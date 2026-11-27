//! Turns gateway events into program executions and performs their effects.

use std::sync::Arc;
use std::time::{Duration, Instant};

use disblock_program::{
    ButtonStyle, CompiledProgram, Context, Effect, EmojiRef, Event, EventEmoji, EventKind,
    Execution, MessagePayload, Random, RenderedButton, RenderedComponentRow, RenderedEmbed,
    SavedVariables, ScheduleSpec,
};
use time::OffsetDateTime;
use twilight_http::api_error::ApiError as DiscordApiError;
use twilight_http::error::ErrorType;
use twilight_http::request::{AuditLogReason, channel::reaction::RequestReactionType};
use twilight_model::application::interaction::application_command::CommandOptionValue;
use twilight_model::application::interaction::{Interaction, InteractionData, InteractionType};
use twilight_model::channel::Message;
use twilight_model::channel::message::component::{
    ActionRow, Button, ButtonStyle as TwilightButtonStyle, Component, ComponentType, SelectMenu,
    SelectMenuOption, SelectMenuType,
};
use twilight_model::channel::message::{
    AllowedMentions, Embed, EmojiReactionType, MentionType, MessageFlags,
};
use twilight_model::gateway::GatewayReaction;
use twilight_model::gateway::event::Event as GatewayEvent;
use twilight_model::guild::Member;
use twilight_model::http::interaction::{
    InteractionResponse, InteractionResponseData, InteractionResponseType,
};
use twilight_model::id::Id;
use twilight_model::id::marker::{
    ChannelMarker, GuildMarker, InteractionMarker, MessageMarker, RoleMarker, UserMarker,
};
use twilight_model::user::User;
use twilight_model::util::Timestamp;
use twilight_util::builder::embed::{
    EmbedAuthorBuilder, EmbedBuilder, EmbedFieldBuilder, EmbedFooterBuilder, ImageSource,
};
use twilight_util::builder::interaction_response::{ChannelMessageBuilder, UpdateMessageBuilder};

use super::BotContext;
use super::log::Record;
use super::variables;

/// Randomness for the `random` block.
struct ThreadRandom;

impl Random for ThreadRandom {
    fn between(&mut self, min: i64, max: i64) -> i64 {
        rand::random_range(min..=max)
    }
}

/// Load the variables the event's member and server saved, when the program
/// uses any, then plan every handler the event matches.
async fn plan(
    bot: &BotContext,
    program: &CompiledProgram,
    event: &Event<'_>,
) -> Result<Vec<Execution>, String> {
    match (event.context.guild_id, event.context.user_id) {
        (Some(guild_id), Some(user_id)) if program.uses_saved_variables() => {
            variables::plan_and_save(&bot.db, bot.id, guild_id, user_id, |saved| {
                program.handle(event, saved, &mut ThreadRandom)
            })
            .await
            .map_err(|error| {
                tracing::error!(bot = %bot.id, %error, "failed to load variables");
                "could not load saved variables".to_owned()
            })?
            .ok_or_else(|| "monthly execution limit reached for this bot's plan".to_owned())
        }
        _ => {
            let executions = program.handle(event, &SavedVariables::default(), &mut ThreadRandom);
            let count =
                i32::try_from(executions.len()).map_err(|_| "too many executions".to_owned())?;
            if super::quota::reserve_many(&bot.db, bot.id, count)
                .await
                .map_err(|error| {
                    tracing::error!(bot = %bot.id, %error, "failed to reserve execution quota");
                    "could not reserve the execution quota".to_owned()
                })?
            {
                Ok(executions)
            } else {
                Err("monthly execution limit reached for this bot's plan".to_owned())
            }
        }
    }
}

/// Record a run that could not even start.
fn record_failure(bot: &BotContext, trigger: &str, error: String) {
    bot.log.record(Record {
        bot_id: bot.id,
        trigger: trigger.to_owned(),
        error: Some(error),
        duration: Duration::ZERO,
    });
}

fn role_ids(roles: &[Id<RoleMarker>]) -> Vec<u64> {
    roles.iter().map(|role| role.get()).collect()
}

pub async fn handle(bot: Arc<BotContext>, program: Arc<CompiledProgram>, event: GatewayEvent) {
    match event {
        GatewayEvent::InteractionCreate(interaction) => {
            // Boxed: this future is large and would bloat every event task.
            Box::pin(on_interaction(&bot, &program, interaction.0)).await;
        }
        GatewayEvent::MemberAdd(member) => {
            on_member_add(&bot, &program, member.guild_id, &member.member).await;
        }
        GatewayEvent::MemberRemove(member) => {
            on_member_remove(&bot, &program, member.guild_id, &member.user).await;
        }
        GatewayEvent::MessageCreate(message) => on_message(&bot, &program, &message).await,
        GatewayEvent::ReactionAdd(reaction) => {
            on_reaction(&bot, &program, &reaction.0, true).await;
        }
        GatewayEvent::ReactionRemove(reaction) => {
            on_reaction(&bot, &program, &reaction.0, false).await;
        }
        _ => {}
    }
}

pub async fn on_schedule(bot: &BotContext, program: &CompiledProgram, spec: ScheduleSpec) {
    let event = Event {
        kind: EventKind::Schedule {
            interval: spec.interval,
            unit: spec.unit,
        },
        context: Context {
            user_id: None,
            user_name: None,
            roles: &[],
            guild_id: None,
            channel_id: None,
            message_id: None,
            options: &[],
            selected: None,
            emoji: None,
        },
    };
    let target = Target {
        guild_id: None,
        user_id: None,
    };
    run_many(
        bot,
        program,
        &event,
        target,
        || Reply::Unavailable,
        "scheduled task",
    )
    .await;
}

async fn run_many(
    bot: &BotContext,
    program: &CompiledProgram,
    event: &Event<'_>,
    target: Target,
    make_reply: impl Fn() -> Reply<'static>,
    failure_label: &str,
) {
    match plan(bot, program, event).await {
        Ok(executions) => {
            for execution in executions {
                run(bot, execution, target, make_reply()).await;
            }
        }
        Err(error) => record_failure(bot, failure_label, error),
    }
}

async fn on_interaction(bot: &BotContext, program: &CompiledProgram, interaction: Interaction) {
    let Some(user) = interaction
        .member
        .as_ref()
        .and_then(|member| member.user.as_ref())
        .or(interaction.user.as_ref())
    else {
        return;
    };
    let roles = interaction
        .member
        .as_ref()
        .map(|member| role_ids(&member.roles))
        .unwrap_or_default();
    let channel_id = interaction.channel.as_ref().map(|c| c.id);

    match (interaction.kind, &interaction.data) {
        (InteractionType::ApplicationCommand, Some(InteractionData::ApplicationCommand(data))) => {
            let options: Vec<(String, String)> = data
                .options
                .iter()
                .filter_map(|option| {
                    render_option(&option.value).map(|value| (option.name.clone(), value))
                })
                .collect();
            let context = context_for(
                user,
                &roles,
                interaction.guild_id,
                channel_id,
                None,
                &options,
                None,
            );
            run_interaction(
                bot,
                program,
                &interaction,
                user.id,
                &Event {
                    kind: EventKind::SlashCommand { name: &data.name },
                    context,
                },
                false,
                None,
                &format!("/{}", data.name),
                "This command is not available anymore.",
            )
            .await;
        }
        (InteractionType::MessageComponent, Some(InteractionData::MessageComponent(data))) => {
            let selected = data.values.join(", ");
            let (kind, selected_ref, label, missing) = match data.component_type {
                ComponentType::Button => (
                    EventKind::ButtonClick {
                        id: &data.custom_id,
                    },
                    None,
                    format!("button \"{}\"", data.custom_id),
                    "This button is not available anymore.",
                ),
                ComponentType::TextSelectMenu
                | ComponentType::UserSelectMenu
                | ComponentType::RoleSelectMenu
                | ComponentType::MentionableSelectMenu
                | ComponentType::ChannelSelectMenu => (
                    EventKind::SelectMenu {
                        id: &data.custom_id,
                    },
                    Some(selected.as_str()),
                    format!("select menu \"{}\"", data.custom_id),
                    "This menu is not available anymore.",
                ),
                _ => return,
            };
            let source_message = interaction.message.as_ref().map(|m| (m.channel_id, m.id));
            let context = context_for(
                user,
                &roles,
                interaction.guild_id,
                channel_id,
                source_message.map(|(_, id)| id),
                &[],
                selected_ref,
            );
            run_interaction(
                bot,
                program,
                &interaction,
                user.id,
                &Event { kind, context },
                true,
                source_message,
                &label,
                missing,
            )
            .await;
        }
        _ => {}
    }
}

#[allow(clippy::too_many_arguments)]
async fn run_interaction(
    bot: &BotContext,
    program: &CompiledProgram,
    interaction: &Interaction,
    user_id: Id<UserMarker>,
    event: &Event<'_>,
    is_component: bool,
    source_message: Option<(Id<ChannelMarker>, Id<MessageMarker>)>,
    trigger_label: &str,
    missing_notice: &str,
) {
    let planned = plan(bot, program, event).await;
    let mut reply = Reply::Interaction(InteractionReply {
        id: interaction.id,
        token: &interaction.token,
        is_component,
        source_message,
        original_is_source: false,
        state: ReplyState::Pending,
    });
    let target = Target {
        guild_id: interaction.guild_id,
        user_id: Some(user_id),
    };

    let executions = match planned {
        Ok(executions) => executions,
        Err(error) => {
            reply.finish(bot, Some(FAILURE_NOTICE)).await;
            record_failure(bot, trigger_label, error);
            return;
        }
    };
    let Some(execution) = executions.into_iter().next() else {
        // Discord still shows a command or component from an older message.
        reply.finish(bot, Some(missing_notice)).await;
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
    let roles = role_ids(&member.roles);
    let context = context_for(&member.user, &roles, Some(guild_id), None, None, &[], None);
    let target = Target {
        guild_id: Some(guild_id),
        user_id: Some(member.user.id),
    };
    let event = Event {
        kind: EventKind::MemberJoin,
        context,
    };
    run_many(
        bot,
        program,
        &event,
        target,
        || Reply::Unavailable,
        "member join",
    )
    .await;
}

async fn on_member_remove(
    bot: &BotContext,
    program: &CompiledProgram,
    guild_id: Id<GuildMarker>,
    user: &User,
) {
    if user.bot {
        return;
    }
    let context = context_for(user, &[], Some(guild_id), None, None, &[], None);
    let target = Target {
        guild_id: Some(guild_id),
        user_id: Some(user.id),
    };
    let event = Event {
        kind: EventKind::MemberLeave,
        context,
    };
    run_many(
        bot,
        program,
        &event,
        target,
        || Reply::Unavailable,
        "member leave",
    )
    .await;
}

async fn on_message(bot: &BotContext, program: &CompiledProgram, message: &Message) {
    // Ignoring bots also prevents a bot from triggering itself in a loop.
    if message.author.bot || message.guild_id.is_none() {
        return;
    }
    let roles = message
        .member
        .as_ref()
        .map(|member| role_ids(&member.roles))
        .unwrap_or_default();
    let context = context_for(
        &message.author,
        &roles,
        message.guild_id,
        Some(message.channel_id),
        Some(message.id),
        &[],
        None,
    );
    let target = Target {
        guild_id: message.guild_id,
        user_id: Some(message.author.id),
    };
    let event = Event {
        kind: EventKind::Message {
            content: &message.content,
        },
        context,
    };
    run_many(
        bot,
        program,
        &event,
        target,
        || Reply::Message {
            channel_id: message.channel_id,
            message_id: message.id,
        },
        "message",
    )
    .await;
}

async fn on_reaction(
    bot: &BotContext,
    program: &CompiledProgram,
    reaction: &GatewayReaction,
    added: bool,
) {
    let Some(guild_id) = reaction.guild_id else {
        return;
    };
    if bot.bot_user_id.get() == Some(&reaction.user_id)
        || reaction
            .member
            .as_ref()
            .is_some_and(|member| member.user.bot)
    {
        return;
    }
    let roles = reaction
        .member
        .as_ref()
        .map(|member| role_ids(&member.roles))
        .unwrap_or_default();
    let user_name = reaction
        .member
        .as_ref()
        .map(|member| display_name(&member.user));
    let (event_emoji, rendered_emoji) = reaction_emoji(&reaction.emoji);
    let context = Context {
        user_id: Some(reaction.user_id.get()),
        user_name,
        roles: &roles,
        guild_id: Some(guild_id.get()),
        channel_id: Some(reaction.channel_id.get()),
        message_id: Some(reaction.message_id.get()),
        options: &[],
        selected: None,
        emoji: Some(&rendered_emoji),
    };
    let target = Target {
        guild_id: Some(guild_id),
        user_id: Some(reaction.user_id),
    };
    let message_id = reaction.message_id.get();
    let kind = if added {
        EventKind::ReactionAdd {
            emoji: event_emoji,
            message_id,
        }
    } else {
        EventKind::ReactionRemove {
            emoji: event_emoji,
            message_id,
        }
    };
    let event = Event { kind, context };
    let trigger_label = if added {
        "reaction added"
    } else {
        "reaction removed"
    };
    run_many(
        bot,
        program,
        &event,
        target,
        || Reply::Message {
            channel_id: reaction.channel_id,
            message_id: reaction.message_id,
        },
        trigger_label,
    )
    .await;
}

fn reaction_emoji(emoji: &EmojiReactionType) -> (EventEmoji<'_>, String) {
    match emoji {
        EmojiReactionType::Unicode { name } => (EventEmoji::Unicode(name), name.clone()),
        EmojiReactionType::Custom { animated, id, name } => {
            let text = match name.as_deref() {
                Some(name) if *animated => format!("<a:{name}:{id}>"),
                Some(name) => format!("<:{name}:{id}>"),
                None => id.to_string(),
            };
            (EventEmoji::Custom { id: id.get() }, text)
        }
    }
}

fn display_name(user: &User) -> &str {
    user.global_name.as_deref().unwrap_or(&user.name)
}

fn context_for<'a>(
    user: &'a User,
    roles: &'a [u64],
    guild_id: Option<Id<GuildMarker>>,
    channel_id: Option<Id<ChannelMarker>>,
    message_id: Option<Id<MessageMarker>>,
    options: &'a [(String, String)],
    selected: Option<&'a str>,
) -> Context<'a> {
    Context {
        user_id: Some(user.id.get()),
        user_name: Some(display_name(user)),
        roles,
        guild_id: guild_id.map(Id::get),
        channel_id: channel_id.map(Id::get),
        message_id: message_id.map(Id::get),
        options,
        selected,
        emoji: None,
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
    user_id: Option<Id<UserMarker>>,
}

const FAILURE_NOTICE: &str = "Something went wrong while running this command.";

async fn run(bot: &BotContext, execution: Execution, target: Target, mut reply: Reply<'_>) {
    let started = Instant::now();
    let mut error = save_writes(bot, &execution, target).await.err();

    if error.is_none() {
        reply.prepare(bot, &execution.effects).await;
        for effect in &execution.effects {
            if let Err(message) = perform(bot, effect, target, &mut reply).await {
                error = Some(message);
                break;
            }
        }
    }
    // The plan stopped early: its effects ran, then report why it stopped.
    if error.is_none() {
        error = execution
            .error
            .as_ref()
            .map(|reason| format!("stopped: {reason}"));
    }

    reply
        .finish(bot, error.as_ref().map(|_| FAILURE_NOTICE))
        .await;

    bot.log.record(Record {
        bot_id: bot.id,
        trigger: execution.trigger,
        error,
        duration: started.elapsed(),
    });
}

/// Saved before the effects run, so a slow effect cannot hold stale values.
async fn save_writes(
    bot: &BotContext,
    execution: &Execution,
    target: Target,
) -> Result<(), String> {
    if execution.writes.is_empty() {
        return Ok(());
    }
    let guild_id = target
        .guild_id
        .ok_or_else(|| "variables can only be saved inside a server".to_owned())?;
    let user_id = target
        .user_id
        .ok_or_else(|| "variables require a member".to_owned())?;
    variables::save(
        &bot.db,
        bot.id,
        guild_id.get(),
        user_id.get(),
        &execution.writes,
    )
    .await
    .map_err(|error| {
        tracing::error!(bot = %bot.id, %error, "failed to save variables");
        "could not save variables".to_owned()
    })
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
            .ok_or_else(|| "this action can only be used inside a server".to_owned())
    };
    let member_target = || {
        target
            .user_id
            .ok_or("roles can only be changed for a member")
    };

    match effect {
        Effect::Reply { message, ephemeral } => reply.send(bot, message, *ephemeral).await,
        Effect::Wait { seconds } => {
            tokio::time::sleep(Duration::from_secs(u64::from(*seconds))).await;
            Ok(())
        }
        Effect::SendMessage {
            channel_id,
            message,
        } => send_channel_message(http, *channel_id, message).await,
        Effect::UpdateMessage { message } => reply.update(bot, message).await,
        Effect::AddRole { role_id } => {
            let role_id = Id::new_checked(*role_id).ok_or("invalid role ID")?;
            http.add_guild_member_role(guild_id()?, member_target()?, role_id)
                .await
                .map(drop)
                .map_err(|error| describe(&error, "add the role"))
        }
        Effect::RemoveRole { role_id } => {
            let role_id = Id::new_checked(*role_id).ok_or("invalid role ID")?;
            http.remove_guild_member_role(guild_id()?, member_target()?, role_id)
                .await
                .map(drop)
                .map_err(|error| describe(&error, "remove the role"))
        }
        Effect::Timeout {
            member_id,
            seconds,
            reason,
        } => perform_timeout(http, guild_id()?, *member_id, *seconds, reason.as_deref()).await,
        Effect::Kick { member_id, reason } => {
            let member_id = Id::new_checked(*member_id).ok_or("invalid member ID")?;
            let mut request = http.remove_guild_member(guild_id()?, member_id);
            if let Some(reason) = reason {
                request = request.reason(reason);
            }
            request
                .await
                .map(drop)
                .map_err(|error| describe(&error, "kick the member"))
        }
        Effect::Ban { member_id, reason } => {
            let member_id = Id::new_checked(*member_id).ok_or("invalid member ID")?;
            let mut request = http.create_ban(guild_id()?, member_id);
            if let Some(reason) = reason {
                request = request.reason(reason);
            }
            request
                .await
                .map(drop)
                .map_err(|error| describe(&error, "ban the member"))
        }
        Effect::DeleteMessage => {
            let Reply::Message {
                channel_id,
                message_id,
            } = *reply
            else {
                return Err("deleting a message needs a message trigger".to_owned());
            };
            http.delete_message(channel_id, message_id)
                .await
                .map(drop)
                .map_err(|error| describe(&error, "delete the message"))
        }
        Effect::DirectMessage { member_id, content } => perform_dm(http, *member_id, content).await,
        Effect::React { emoji } => perform_react(http, reply, emoji).await,
    }
}

async fn send_channel_message(
    http: &twilight_http::Client,
    channel_id: u64,
    message: &MessagePayload,
) -> Result<(), String> {
    let channel_id = Id::<ChannelMarker>::new_checked(channel_id).ok_or("invalid channel ID")?;
    let (embeds, components) = build_rich(message)?;
    let mut req = http.create_message(channel_id);
    if let Some(content) = message.content.as_deref() {
        req = req.content(content);
    }
    if !embeds.is_empty() {
        req = req.embeds(&embeds);
    }
    if !components.is_empty() {
        req = req.components(&components);
    }
    req.await
        .map(drop)
        .map_err(|error| describe(&error, "send the message"))
}

async fn perform_timeout(
    http: &twilight_http::Client,
    guild_id: Id<GuildMarker>,
    member_id: u64,
    seconds: u32,
    reason: Option<&str>,
) -> Result<(), String> {
    let member_id = Id::new_checked(member_id).ok_or("invalid member ID")?;
    let until = OffsetDateTime::now_utc()
        .checked_add(time::Duration::seconds(i64::from(seconds)))
        .ok_or("invalid timeout duration")?;
    let until = Timestamp::from_secs(until.unix_timestamp())
        .map_err(|error| format!("invalid timeout expiry: {error}"))?;
    let mut request = http
        .update_guild_member(guild_id, member_id)
        .communication_disabled_until(Some(until));
    if let Some(reason) = reason {
        request = request.reason(reason);
    }
    request
        .await
        .map(drop)
        .map_err(|error| describe(&error, "timeout the member"))
}

async fn perform_dm(
    http: &twilight_http::Client,
    member_id: u64,
    content: &str,
) -> Result<(), String> {
    let member_id = Id::new_checked(member_id).ok_or("invalid member ID")?;
    let channel = http
        .create_private_channel(member_id)
        .await
        .map_err(|error| describe(&error, "open a private channel"))?
        .model()
        .await
        .map_err(|error| format!("could not open a private channel: {error}"))?;
    http.create_message(channel.id)
        .content(content)
        .await
        .map(drop)
        .map_err(|error| describe(&error, "send the direct message"))
}

async fn perform_react(
    http: &twilight_http::Client,
    reply: &Reply<'_>,
    emoji: &EmojiRef,
) -> Result<(), String> {
    let Reply::Message {
        channel_id,
        message_id,
    } = *reply
    else {
        return Err("reactions need a message".to_owned());
    };
    let emoji = match emoji {
        EmojiRef::Unicode(name) => RequestReactionType::Unicode { name },
        EmojiRef::Custom { name, id, .. } => RequestReactionType::Custom {
            id: Id::new_checked(*id).ok_or("invalid emoji ID")?,
            name: Some(name),
        },
    };
    http.create_reaction(channel_id, message_id, &emoji)
        .await
        .map(drop)
        .map_err(|error| describe(&error, "react"))
}

fn allowed_mentions() -> AllowedMentions {
    AllowedMentions {
        parse: vec![MentionType::Users],
        ..AllowedMentions::default()
    }
}

fn build_rich(payload: &MessagePayload) -> Result<(Vec<Embed>, Vec<Component>), String> {
    let embeds = payload
        .embeds
        .iter()
        .map(build_embed)
        .collect::<Result<Vec<_>, _>>()?;
    let components = payload
        .components
        .iter()
        .map(build_component_row)
        .collect::<Result<Vec<_>, _>>()?;
    Ok((embeds, components))
}

fn build_embed(rendered: &RenderedEmbed) -> Result<Embed, String> {
    let mut builder = EmbedBuilder::new();
    if let Some(title) = &rendered.title {
        builder = builder.title(title);
    }
    if let Some(url) = &rendered.url {
        builder = builder.url(url);
    }
    if let Some(description) = &rendered.description {
        builder = builder.description(description);
    }
    if let Some(color) = rendered.color {
        builder = builder.color(color);
    }
    if let Some(author) = &rendered.author {
        let mut author_builder = EmbedAuthorBuilder::new(&author.name);
        if let Some(url) = &author.url {
            author_builder = author_builder.url(url);
        }
        if let Some(icon_url) = &author.icon_url {
            author_builder = author_builder.icon_url(
                ImageSource::url(icon_url)
                    .map_err(|error| format!("invalid author icon URL: {error}"))?,
            );
        }
        builder = builder.author(author_builder);
    }
    if let Some(footer) = &rendered.footer {
        let mut footer_builder = EmbedFooterBuilder::new(&footer.text);
        if let Some(icon_url) = &footer.icon_url {
            footer_builder = footer_builder.icon_url(
                ImageSource::url(icon_url)
                    .map_err(|error| format!("invalid footer icon URL: {error}"))?,
            );
        }
        builder = builder.footer(footer_builder);
    }
    if let Some(thumbnail_url) = &rendered.thumbnail_url {
        builder = builder.thumbnail(
            ImageSource::url(thumbnail_url)
                .map_err(|error| format!("invalid thumbnail URL: {error}"))?,
        );
    }
    if let Some(image_url) = &rendered.image_url {
        builder = builder.image(
            ImageSource::url(image_url).map_err(|error| format!("invalid image URL: {error}"))?,
        );
    }
    for field in &rendered.fields {
        let mut field_builder = EmbedFieldBuilder::new(&field.name, &field.value);
        if field.inline {
            field_builder = field_builder.inline();
        }
        builder = builder.field(field_builder);
    }
    if rendered.timestamp
        && let Ok(now) = Timestamp::from_secs(OffsetDateTime::now_utc().unix_timestamp())
    {
        builder = builder.timestamp(now);
    }
    builder
        .validate()
        .map_err(|error| format!("invalid embed: {error}"))
        .map(EmbedBuilder::build)
}

fn component_emoji(emoji: &EmojiRef) -> Result<EmojiReactionType, String> {
    Ok(match emoji {
        EmojiRef::Unicode(name) => EmojiReactionType::Unicode { name: name.clone() },
        EmojiRef::Custom { name, id, animated } => EmojiReactionType::Custom {
            animated: *animated,
            id: Id::new_checked(*id).ok_or("invalid emoji ID")?,
            name: Some(name.clone()),
        },
    })
}

fn build_component_row(row: &RenderedComponentRow) -> Result<Component, String> {
    let components = match row {
        RenderedComponentRow::Buttons(buttons) => buttons
            .iter()
            .map(|button| {
                Ok(Component::Button(match button {
                    RenderedButton::Action {
                        id,
                        label,
                        emoji,
                        style,
                        disabled,
                    } => Button {
                        id: None,
                        custom_id: Some(id.clone()),
                        disabled: *disabled,
                        emoji: emoji.as_ref().map(component_emoji).transpose()?,
                        label: label.clone(),
                        sku_id: None,
                        style: match style {
                            ButtonStyle::Primary => TwilightButtonStyle::Primary,
                            ButtonStyle::Secondary => TwilightButtonStyle::Secondary,
                            ButtonStyle::Success => TwilightButtonStyle::Success,
                            ButtonStyle::Danger => TwilightButtonStyle::Danger,
                        },
                        url: None,
                    },
                    RenderedButton::Link {
                        url,
                        label,
                        emoji,
                        disabled,
                    } => Button {
                        id: None,
                        custom_id: None,
                        disabled: *disabled,
                        emoji: emoji.as_ref().map(component_emoji).transpose()?,
                        label: label.clone(),
                        sku_id: None,
                        style: TwilightButtonStyle::Link,
                        url: Some(url.clone()),
                    },
                }))
            })
            .collect::<Result<Vec<_>, String>>()?,
        RenderedComponentRow::SelectMenu {
            id,
            placeholder,
            min_values,
            max_values,
            disabled,
            options,
        } => vec![Component::SelectMenu(SelectMenu {
            channel_types: None,
            custom_id: id.clone(),
            default_values: None,
            disabled: *disabled,
            id: None,
            kind: SelectMenuType::Text,
            max_values: Some(*max_values),
            min_values: Some(*min_values),
            options: Some(
                options
                    .iter()
                    .map(|option| {
                        Ok(SelectMenuOption {
                            default: option.default,
                            description: option.description.clone(),
                            emoji: option.emoji.as_ref().map(component_emoji).transpose()?,
                            label: option.label.clone(),
                            value: option.value.clone(),
                        })
                    })
                    .collect::<Result<Vec<_>, String>>()?,
            ),
            placeholder: placeholder.clone(),
            required: None,
        })],
    };
    Ok(Component::ActionRow(ActionRow {
        id: None,
        components,
    }))
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
    is_component: bool,
    source_message: Option<(Id<ChannelMarker>, Id<MessageMarker>)>,
    /// Whether `@original` points to the clicked message rather than a reply.
    original_is_source: bool,
    state: ReplyState,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ReplyState {
    /// Nothing sent yet: Discord expects an answer within 3 seconds.
    Pending,
    /// "Thinking…" was sent; the first reply edits it.
    DeferredMessage,
    /// Silent component ACK was sent; the first update edits the clicked message.
    DeferredUpdate,
    /// An initial response was sent; later replies are follow-ups.
    Sent,
}

impl Reply<'_> {
    /// Interactions must be answered within 3 seconds. When the first effect
    /// is not an immediate reply or update, acknowledge right away.
    async fn prepare(&mut self, bot: &BotContext, effects: &[Effect]) {
        let Self::Interaction(interaction) = self else {
            return;
        };
        if matches!(
            effects.first(),
            Some(Effect::Reply { .. } | Effect::UpdateMessage { .. })
        ) {
            return;
        }
        let first_response = effects
            .iter()
            .find(|effect| matches!(effect, Effect::Reply { .. } | Effect::UpdateMessage { .. }));
        let (response, next_state, original_is_source) = match first_response {
            Some(Effect::UpdateMessage { .. }) => (
                InteractionResponse {
                    kind: InteractionResponseType::DeferredUpdateMessage,
                    data: None,
                },
                ReplyState::DeferredUpdate,
                true,
            ),
            Some(Effect::Reply { ephemeral, .. }) => (
                InteractionResponse {
                    kind: InteractionResponseType::DeferredChannelMessageWithSource,
                    data: Some(InteractionResponseData {
                        flags: Some(flags(*ephemeral)),
                        ..InteractionResponseData::default()
                    }),
                },
                ReplyState::DeferredMessage,
                false,
            ),
            _ if interaction.is_component => (
                InteractionResponse {
                    kind: InteractionResponseType::DeferredUpdateMessage,
                    data: None,
                },
                ReplyState::DeferredUpdate,
                true,
            ),
            _ => (
                InteractionResponse {
                    kind: InteractionResponseType::DeferredChannelMessageWithSource,
                    data: Some(InteractionResponseData {
                        flags: Some(MessageFlags::EPHEMERAL),
                        ..InteractionResponseData::default()
                    }),
                },
                ReplyState::DeferredMessage,
                false,
            ),
        };
        match bot
            .http
            .interaction(bot.application_id)
            .create_response(interaction.id, interaction.token, &response)
            .await
        {
            Ok(_) => {
                interaction.state = next_state;
                interaction.original_is_source = original_is_source;
            }
            Err(error) => tracing::warn!(bot = %bot.id, %error, "failed to defer interaction"),
        }
    }

    async fn send(
        &mut self,
        bot: &BotContext,
        payload: &MessagePayload,
        ephemeral: bool,
    ) -> Result<(), String> {
        let (embeds, components) = build_rich(payload)?;
        match self {
            Self::Interaction(interaction) => {
                let client = bot.http.interaction(bot.application_id);
                let result = match interaction.state {
                    ReplyState::Pending => {
                        let mut builder = ChannelMessageBuilder::new()
                            .allowed_mentions(allowed_mentions())
                            .flags(flags(ephemeral));
                        if let Some(content) = payload.content.as_deref() {
                            builder = builder.content(content);
                        }
                        if !embeds.is_empty() {
                            builder = builder.embeds(embeds);
                        }
                        if !components.is_empty() {
                            builder = builder.components(components);
                        }
                        let response = builder.build();
                        interaction.original_is_source = false;
                        client
                            .create_response(interaction.id, interaction.token, &response)
                            .await
                            .map(drop)
                    }
                    ReplyState::DeferredMessage => {
                        let mut req = client
                            .update_response(interaction.token)
                            .content(payload.content.as_deref());
                        if !embeds.is_empty() {
                            req = req.embeds(Some(&embeds));
                        }
                        if !components.is_empty() {
                            req = req.components(Some(&components));
                        }
                        req.await.map(drop)
                    }
                    ReplyState::DeferredUpdate | ReplyState::Sent => {
                        let mut req = client
                            .create_followup(interaction.token)
                            .flags(flags(ephemeral));
                        if let Some(content) = payload.content.as_deref() {
                            req = req.content(content);
                        }
                        if !embeds.is_empty() {
                            req = req.embeds(&embeds);
                        }
                        if !components.is_empty() {
                            req = req.components(&components);
                        }
                        req.await.map(drop)
                    }
                };
                interaction.state = ReplyState::Sent;
                result.map_err(|error| describe(&error, "reply"))
            }
            Self::Message {
                channel_id,
                message_id,
            } => {
                let mut req = bot
                    .http
                    .create_message(*channel_id)
                    .reply(*message_id)
                    .fail_if_not_exists(false);
                if let Some(content) = payload.content.as_deref() {
                    req = req.content(content);
                }
                if !embeds.is_empty() {
                    req = req.embeds(&embeds);
                }
                if !components.is_empty() {
                    req = req.components(&components);
                }
                req.await
                    .map(drop)
                    .map_err(|error| describe(&error, "reply"))
            }
            Self::Unavailable => Err("there is no message to reply to".to_owned()),
        }
    }

    async fn update(&mut self, bot: &BotContext, payload: &MessagePayload) -> Result<(), String> {
        let Self::Interaction(interaction) = self else {
            return Err(
                "only button and select menu triggers can update the clicked message".to_owned(),
            );
        };
        if !interaction.is_component {
            return Err(
                "only button and select menu triggers can update the clicked message".to_owned(),
            );
        }
        let (embeds, components) = build_rich(payload)?;
        let keep_body = payload.content.is_none() && embeds.is_empty();
        let client = bot.http.interaction(bot.application_id);

        let result = match (interaction.state, interaction.original_is_source) {
            (ReplyState::Pending, _) => {
                let mut builder = UpdateMessageBuilder::new()
                    .allowed_mentions(allowed_mentions())
                    .components(components);
                if !keep_body {
                    builder = builder
                        .content(payload.content.as_deref().unwrap_or(""))
                        .embeds(embeds);
                }
                let response = builder.build();
                interaction.original_is_source = true;
                interaction.state = ReplyState::Sent;
                client
                    .create_response(interaction.id, interaction.token, &response)
                    .await
                    .map(drop)
            }
            (ReplyState::DeferredUpdate, _) | (ReplyState::Sent, true) => {
                interaction.state = ReplyState::Sent;
                let mut req = client
                    .update_response(interaction.token)
                    .components(Some(&components));
                if !keep_body {
                    req = req
                        .content(payload.content.as_deref())
                        .embeds(Some(&embeds));
                }
                req.await.map(drop)
            }
            (ReplyState::DeferredMessage, _) | (ReplyState::Sent, false) => {
                let Some((channel_id, message_id)) = interaction.source_message else {
                    return Err("there is no message to update".to_owned());
                };
                let mut req = bot
                    .http
                    .update_message(channel_id, message_id)
                    .components(Some(&components));
                if !keep_body {
                    req = req
                        .content(payload.content.as_deref())
                        .embeds(Some(&embeds));
                }
                req.await.map(drop)
            }
        };
        result.map_err(|error| describe(&error, "update the message"))
    }

    /// Make sure every interaction is answered, and report errors ephemerally.
    async fn finish(&mut self, bot: &BotContext, notice: Option<&str>) {
        let Self::Interaction(interaction) = self else {
            return;
        };
        let content = match notice {
            Some(notice) => notice,
            None if interaction.state == ReplyState::Sent => return,
            None if interaction.is_component && interaction.state == ReplyState::DeferredUpdate => {
                return;
            }
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
            ReplyState::DeferredMessage => client
                .update_response(interaction.token)
                .content(Some(content))
                .await
                .map(drop),
            ReplyState::DeferredUpdate | ReplyState::Sent => client
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
        10_007 => "this member is no longer in the server",
        10_013 => "this user does not exist anymore",
        50_007 => "this user does not accept direct messages from the bot",
        _ => return format!("could not {action}: {} (code {})", api.message, api.code),
    };
    format!("could not {action}: {hint}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use disblock_program::{
        RenderedEmbedAuthor, RenderedEmbedField, RenderedEmbedFooter, RenderedSelectOption,
    };
    use twilight_http::request::TryIntoRequest as _;

    #[tokio::test]
    async fn builds_valid_discord_embeds_and_components() {
        let payload = MessagePayload {
            content: Some("Hello!".to_owned()),
            embeds: vec![RenderedEmbed {
                title: Some("Welcome".to_owned()),
                url: Some("https://example.com".to_owned()),
                description: Some("Pick a role below.".to_owned()),
                color: Some(0x0058_65F2),
                author: Some(RenderedEmbedAuthor {
                    name: "Bot".to_owned(),
                    url: Some("https://example.com".to_owned()),
                    icon_url: Some("https://example.com/a.png".to_owned()),
                }),
                footer: Some(RenderedEmbedFooter {
                    text: "Footer".to_owned(),
                    icon_url: Some("https://example.com/f.png".to_owned()),
                }),
                thumbnail_url: Some("https://example.com/t.png".to_owned()),
                image_url: Some("https://example.com/i.png".to_owned()),
                fields: vec![RenderedEmbedField {
                    name: "Members".to_owned(),
                    value: "42".to_owned(),
                    inline: true,
                }],
                timestamp: true,
            }],
            components: vec![
                RenderedComponentRow::Buttons(vec![
                    RenderedButton::Action {
                        id: "verify".to_owned(),
                        label: Some("Verify".to_owned()),
                        emoji: Some(EmojiRef::Unicode("✅".to_owned())),
                        style: ButtonStyle::Success,
                        disabled: false,
                    },
                    RenderedButton::Link {
                        url: "https://example.com".to_owned(),
                        label: Some("Docs".to_owned()),
                        emoji: None,
                        disabled: false,
                    },
                ]),
                RenderedComponentRow::SelectMenu {
                    id: "role".to_owned(),
                    placeholder: Some("Choose...".to_owned()),
                    min_values: 1,
                    max_values: 1,
                    disabled: false,
                    options: vec![RenderedSelectOption {
                        label: "Updates".to_owned(),
                        value: "updates".to_owned(),
                        description: Some("Get pinged".to_owned()),
                        emoji: Some(EmojiRef::Custom {
                            name: "bell".to_owned(),
                            id: 123_456_789_012_345_678,
                            animated: true,
                        }),
                        default: false,
                    }],
                },
            ],
        };

        let (embeds, components) = build_rich(&payload).expect("valid rich payload");
        assert_eq!(embeds.len(), 1);
        assert!(embeds[0].timestamp.is_some());
        assert_eq!(components.len(), 2);

        let client = twilight_http::Client::new(String::new());
        client
            .create_message(Id::new(1))
            .content("Hello!")
            .embeds(&embeds)
            .components(&components)
            .try_into_request()
            .expect("twilight accepts built embeds and components");
    }
}
