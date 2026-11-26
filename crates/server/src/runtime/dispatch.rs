//! Turns gateway events into program executions and performs their effects.

use std::sync::Arc;
use std::time::Instant;

use disblock_program::{CompiledProgram, Context, Effect, EmojiRef, Event, EventKind, Execution};
use twilight_http::api_error::ApiError as DiscordApiError;
use twilight_http::error::ErrorType;
use twilight_http::request::channel::reaction::RequestReactionType;
use twilight_model::application::interaction::application_command::CommandOptionValue;
use twilight_model::application::interaction::{Interaction, InteractionData, InteractionType};
use twilight_model::channel::Message;
use twilight_model::channel::message::MessageFlags;
use twilight_model::gateway::event::Event as GatewayEvent;
use twilight_model::guild::Member;
use twilight_model::http::interaction::{
    InteractionResponse, InteractionResponseData, InteractionResponseType,
};
use twilight_model::id::Id;
use twilight_model::id::marker::{
    ChannelMarker, GuildMarker, InteractionMarker, MessageMarker, UserMarker,
};
use twilight_model::user::User;
use twilight_util::builder::interaction_response::ChannelMessageBuilder;

use super::BotContext;
use super::log::Record;

pub async fn handle(bot: Arc<BotContext>, program: Arc<CompiledProgram>, event: GatewayEvent) {
    match event {
        GatewayEvent::InteractionCreate(interaction) => {
            on_interaction(&bot, &program, interaction.0).await;
        }
        GatewayEvent::MemberAdd(member) => {
            on_member_add(&bot, &program, member.guild_id, &member.member).await;
        }
        GatewayEvent::MessageCreate(message) => on_message(&bot, &program, &message).await,
        _ => {}
    }
}

async fn on_interaction(bot: &BotContext, program: &CompiledProgram, interaction: Interaction) {
    if interaction.kind != InteractionType::ApplicationCommand {
        return;
    }
    let Some(InteractionData::ApplicationCommand(data)) = &interaction.data else {
        return;
    };
    let Some(user) = interaction
        .member
        .as_ref()
        .and_then(|member| member.user.as_ref())
        .or(interaction.user.as_ref())
    else {
        return;
    };

    let options: Vec<(String, String)> = data
        .options
        .iter()
        .filter_map(|option| render_option(&option.value).map(|value| (option.name.clone(), value)))
        .collect();
    let context = context_for(
        user,
        interaction.guild_id,
        interaction.channel.as_ref().map(|c| c.id),
        &options,
    );
    let executions = program.handle(&Event {
        kind: EventKind::SlashCommand { name: &data.name },
        context,
    });

    let mut reply = Reply::Interaction(InteractionReply {
        id: interaction.id,
        token: &interaction.token,
        state: ReplyState::Pending,
    });
    let target = Target {
        guild_id: interaction.guild_id,
        user_id: user.id,
    };

    let Some(execution) = executions.into_iter().next() else {
        // Discord still shows a command that was removed a moment ago.
        reply
            .finish(bot, Some("This command is not available anymore."))
            .await;
        return;
    };
    run(bot, execution, target, reply).await;
}

async fn on_member_add(
    bot: &BotContext,
    program: &CompiledProgram,
    guild_id: Id<GuildMarker>,
    member: &Member,
) {
    if member.user.bot {
        return;
    }
    let context = context_for(&member.user, Some(guild_id), None, &[]);
    let target = Target {
        guild_id: Some(guild_id),
        user_id: member.user.id,
    };
    for execution in program.handle(&Event {
        kind: EventKind::MemberJoin,
        context,
    }) {
        run(bot, execution, target, Reply::Unavailable).await;
    }
}

async fn on_message(bot: &BotContext, program: &CompiledProgram, message: &Message) {
    // Ignoring bots also prevents a bot from triggering itself in a loop.
    if message.author.bot || message.guild_id.is_none() {
        return;
    }
    let context = context_for(
        &message.author,
        message.guild_id,
        Some(message.channel_id),
        &[],
    );
    let target = Target {
        guild_id: message.guild_id,
        user_id: message.author.id,
    };
    for execution in program.handle(&Event {
        kind: EventKind::Message {
            content: &message.content,
        },
        context,
    }) {
        let reply = Reply::Message {
            channel_id: message.channel_id,
            message_id: message.id,
        };
        run(bot, execution, target, reply).await;
    }
}

fn context_for<'a>(
    user: &'a User,
    guild_id: Option<Id<GuildMarker>>,
    channel_id: Option<Id<ChannelMarker>>,
    options: &'a [(String, String)],
) -> Context<'a> {
    Context {
        user_id: user.id.get(),
        user_name: user.global_name.as_deref().unwrap_or(&user.name),
        guild_id: guild_id.map(Id::get),
        channel_id: channel_id.map(Id::get),
        options,
    }
}

fn render_option(value: &CommandOptionValue) -> Option<String> {
    Some(match value {
        CommandOptionValue::String(value) => value.clone(),
        CommandOptionValue::Integer(value) => value.to_string(),
        CommandOptionValue::Boolean(value) => if *value { "yes" } else { "no" }.to_owned(),
        CommandOptionValue::User(id) => format!("<@{id}>"),
        _ => return None,
    })
}

/// The member an execution acts on.
#[derive(Clone, Copy)]
struct Target {
    guild_id: Option<Id<GuildMarker>>,
    user_id: Id<UserMarker>,
}

async fn run(bot: &BotContext, execution: Execution, target: Target, mut reply: Reply<'_>) {
    let started = Instant::now();
    reply.prepare(bot, &execution.effects).await;

    let mut error = None;
    for effect in &execution.effects {
        if let Err(message) = perform(bot, effect, target, &mut reply).await {
            error = Some(message);
            break;
        }
    }

    let notice = error
        .as_ref()
        .map(|_| "Something went wrong while running this command.");
    reply.finish(bot, notice).await;

    bot.log.record(Record {
        bot_id: bot.id,
        trigger: execution.trigger,
        error,
        duration: started.elapsed(),
    });
}

async fn perform(
    bot: &BotContext,
    effect: &Effect,
    target: Target,
    reply: &mut Reply<'_>,
) -> Result<(), String> {
    let http = &bot.http;
    let guild_id = || {
        target
            .guild_id
            .ok_or_else(|| "roles can only be changed inside a server".to_owned())
    };

    match effect {
        Effect::Reply { content, ephemeral } => reply.send(bot, content, *ephemeral).await,
        Effect::SendMessage {
            channel_id,
            content,
        } => {
            let channel_id =
                Id::<ChannelMarker>::new_checked(*channel_id).ok_or("invalid channel ID")?;
            http.create_message(channel_id)
                .content(content)
                .await
                .map(drop)
                .map_err(|error| describe(&error, "send the message"))
        }
        Effect::AddRole { role_id } => {
            let role_id = Id::new_checked(*role_id).ok_or("invalid role ID")?;
            http.add_guild_member_role(guild_id()?, target.user_id, role_id)
                .await
                .map(drop)
                .map_err(|error| describe(&error, "add the role"))
        }
        Effect::RemoveRole { role_id } => {
            let role_id = Id::new_checked(*role_id).ok_or("invalid role ID")?;
            http.remove_guild_member_role(guild_id()?, target.user_id, role_id)
                .await
                .map(drop)
                .map_err(|error| describe(&error, "remove the role"))
        }
        Effect::React { emoji } => {
            let Reply::Message {
                channel_id,
                message_id,
            } = *reply
            else {
                return Err("reactions need a message".to_owned());
            };
            let emoji = match emoji {
                EmojiRef::Unicode(name) => RequestReactionType::Unicode { name },
                EmojiRef::Custom { name, id } => RequestReactionType::Custom {
                    id: Id::new_checked(*id).ok_or("invalid emoji ID")?,
                    name: Some(name),
                },
            };
            http.create_reaction(channel_id, message_id, &emoji)
                .await
                .map(drop)
                .map_err(|error| describe(&error, "react"))
        }
    }
}

/// Where `Reply` effects go.
enum Reply<'a> {
    Interaction(InteractionReply<'a>),
    Message {
        channel_id: Id<ChannelMarker>,
        message_id: Id<MessageMarker>,
    },
    Unavailable,
}

struct InteractionReply<'a> {
    id: Id<InteractionMarker>,
    token: &'a str,
    state: ReplyState,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ReplyState {
    /// Nothing sent yet: Discord expects an answer within 3 seconds.
    Pending,
    /// "Thinking…" was sent; the first reply edits it.
    Deferred,
    /// The first reply was sent; later replies are follow-ups.
    Sent,
}

impl Reply<'_> {
    /// Interactions must be answered within 3 seconds. When the first effect
    /// is not a reply, acknowledge right away and edit the answer later.
    async fn prepare(&mut self, bot: &BotContext, effects: &[Effect]) {
        let Self::Interaction(interaction) = self else {
            return;
        };
        if matches!(effects.first(), Some(Effect::Reply { .. })) {
            return;
        }
        let ephemeral = effects.iter().find_map(|effect| match effect {
            Effect::Reply { ephemeral, .. } => Some(*ephemeral),
            _ => None,
        });
        let response = InteractionResponse {
            kind: InteractionResponseType::DeferredChannelMessageWithSource,
            data: Some(InteractionResponseData {
                flags: Some(flags(ephemeral.unwrap_or(true))),
                ..InteractionResponseData::default()
            }),
        };
        match bot
            .http
            .interaction(bot.application_id)
            .create_response(interaction.id, interaction.token, &response)
            .await
        {
            Ok(_) => interaction.state = ReplyState::Deferred,
            Err(error) => tracing::warn!(bot = %bot.id, %error, "failed to defer interaction"),
        }
    }

    async fn send(
        &mut self,
        bot: &BotContext,
        content: &str,
        ephemeral: bool,
    ) -> Result<(), String> {
        match self {
            Self::Interaction(interaction) => {
                let client = bot.http.interaction(bot.application_id);
                let result = match interaction.state {
                    ReplyState::Pending => {
                        let response = ChannelMessageBuilder::new()
                            .content(content)
                            .flags(flags(ephemeral))
                            .build();
                        client
                            .create_response(interaction.id, interaction.token, &response)
                            .await
                            .map(drop)
                    }
                    ReplyState::Deferred => client
                        .update_response(interaction.token)
                        .content(Some(content))
                        .await
                        .map(drop),
                    ReplyState::Sent => client
                        .create_followup(interaction.token)
                        .content(content)
                        .flags(flags(ephemeral))
                        .await
                        .map(drop),
                };
                interaction.state = ReplyState::Sent;
                result.map_err(|error| describe(&error, "reply"))
            }
            Self::Message {
                channel_id,
                message_id,
            } => bot
                .http
                .create_message(*channel_id)
                .content(content)
                .reply(*message_id)
                .fail_if_not_exists(false)
                .await
                .map(drop)
                .map_err(|error| describe(&error, "reply")),
            Self::Unavailable => Err("there is no message to reply to".to_owned()),
        }
    }

    /// Make sure the person who used a slash command always gets an answer.
    async fn finish(&mut self, bot: &BotContext, notice: Option<&str>) {
        let Self::Interaction(interaction) = self else {
            return;
        };
        let content = match notice {
            Some(notice) => notice,
            None if interaction.state == ReplyState::Sent => return,
            None => "✅ Done.",
        };
        let client = bot.http.interaction(bot.application_id);
        let result = match interaction.state {
            ReplyState::Pending => {
                let response = ChannelMessageBuilder::new()
                    .content(content)
                    .flags(MessageFlags::EPHEMERAL)
                    .build();
                client
                    .create_response(interaction.id, interaction.token, &response)
                    .await
                    .map(drop)
            }
            ReplyState::Deferred => client
                .update_response(interaction.token)
                .content(Some(content))
                .await
                .map(drop),
            ReplyState::Sent => client
                .create_followup(interaction.token)
                .content(content)
                .flags(MessageFlags::EPHEMERAL)
                .await
                .map(drop),
        };
        interaction.state = ReplyState::Sent;
        if let Err(error) = result {
            tracing::warn!(bot = %bot.id, %error, "failed to answer interaction");
        }
    }
}

fn flags(ephemeral: bool) -> MessageFlags {
    if ephemeral {
        MessageFlags::EPHEMERAL
    } else {
        MessageFlags::empty()
    }
}

/// A message the bot owner can act on, shown in the execution history.
fn describe(error: &twilight_http::Error, action: &str) -> String {
    let ErrorType::Response {
        error: DiscordApiError::General(api),
        ..
    } = error.kind()
    else {
        return format!("could not {action}: {error}");
    };
    let hint = match api.code {
        50_013 => "the bot is missing a permission, or its role is below the role it manages",
        50_001 => "the bot cannot see this channel",
        10_003 => "this channel does not exist anymore",
        10_011 => "this role does not exist anymore",
        10_014 => "this emoji is unknown to the bot",
        10_008 => "the message was deleted",
        _ => return format!("could not {action}: {} (code {})", api.message, api.code),
    };
    format!("could not {action}: {hint}")
}
