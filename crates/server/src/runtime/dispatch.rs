//! Turns gateway events into program executions and performs their effects.

use std::sync::Arc;
use std::time::{Duration, Instant};

use disblock_program::{
    ButtonStyle, ChannelFilter, CompiledProgram, Context, Effect, EmojiRef, EntitySelectKind,
    Event, EventEmoji, EventKind, Execution, MessagePayload, Random, RenderedAttachment,
    RenderedButton, RenderedComponentRow, RenderedEmbed, RenderedModalInput, RenderedPoll,
    SavedVariables, ScheduleSpec, TextInputStyle,
};
use time::OffsetDateTime;
use twilight_http::api_error::ApiError as DiscordApiError;
use twilight_http::error::ErrorType;
use twilight_http::request::{AuditLogReason, channel::reaction::RequestReactionType};
use twilight_model::application::command::{
    CommandOptionChoice, CommandOptionChoiceValue, CommandType,
};
use twilight_model::application::interaction::application_command::{
    CommandDataOption, CommandOptionValue,
};
use twilight_model::application::interaction::modal::ModalInteractionComponent;
use twilight_model::application::interaction::{
    Interaction, InteractionData, InteractionDataResolved, InteractionType,
};
use twilight_model::channel::message::component::{
    ActionRow, Button, ButtonStyle as TwilightButtonStyle, Component, ComponentType, SelectMenu,
    SelectMenuOption, SelectMenuType, TextInput, TextInputStyle as TwilightTextInputStyle,
};
use twilight_model::channel::message::{
    AllowedMentions, Embed, EmojiReactionType, MentionType, MessageFlags,
};
use twilight_model::channel::thread::AutoArchiveDuration;
use twilight_model::channel::{ChannelType, Message};
use twilight_model::gateway::GatewayReaction;
use twilight_model::gateway::event::Event as GatewayEvent;
use twilight_model::guild::Member;
use twilight_model::http::attachment::Attachment;
use twilight_model::http::interaction::{
    InteractionResponse, InteractionResponseData, InteractionResponseType,
};
use twilight_model::id::Id;
use twilight_model::id::marker::{
    ChannelMarker, GuildMarker, InteractionMarker, MessageMarker, RoleMarker, UserMarker,
};
use twilight_model::poll::{PartialPollMediaEmoji, Poll, PollAnswer, PollLayoutType, PollMedia};
use twilight_model::user::User;
use twilight_model::util::Timestamp;
use twilight_util::builder::embed::{
    EmbedAuthorBuilder, EmbedBuilder, EmbedFieldBuilder, EmbedFooterBuilder, ImageSource,
};
use twilight_util::builder::interaction_response::{
    AutocompleteBuilder, ChannelMessageBuilder, ModalBuilder, UpdateMessageBuilder,
};

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
        GatewayEvent::MemberUpdate(member) => {
            on_member_update(&bot, &program, &member).await;
        }
        GatewayEvent::BanAdd(ban) => {
            on_ban(&bot, &program, ban.guild_id, &ban.user, true).await;
        }
        GatewayEvent::BanRemove(ban) => {
            on_ban(&bot, &program, ban.guild_id, &ban.user, false).await;
        }
        GatewayEvent::MessageCreate(message) => on_message(&bot, &program, &message).await,
        GatewayEvent::MessageUpdate(message) => on_message_update(&bot, &program, &message.0).await,
        GatewayEvent::MessageDelete(deleted) => {
            on_message_delete(
                &bot,
                &program,
                deleted.guild_id,
                deleted.channel_id,
                deleted.id,
            )
            .await;
        }
        GatewayEvent::ReactionAdd(reaction) => {
            on_reaction(&bot, &program, &reaction.0, true).await;
        }
        GatewayEvent::ReactionRemove(reaction) => {
            on_reaction(&bot, &program, &reaction.0, false).await;
        }
        GatewayEvent::ThreadCreate(thread) => {
            on_thread_create(&bot, &program, &thread.0).await;
        }
        GatewayEvent::InviteCreate(invite) => {
            on_invite_create(&bot, &program, &invite).await;
        }
        GatewayEvent::AutoModerationActionExecution(exec) => {
            on_automod_execution(&bot, &program, &exec).await;
        }
        GatewayEvent::MessagePollVoteAdd(vote) => {
            on_poll_vote_add(&bot, &program, &vote).await;
        }
        GatewayEvent::VoiceStateUpdate(voice) => {
            on_voice_state_update(&bot, &program, &voice.0).await;
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
        context: Context::default(),
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
            match data.kind {
                CommandType::ChatInput => {
                    let (group, subcommand, leaf_options) = split_command_options(&data.options);
                    let options: Vec<(String, String)> = leaf_options
                        .iter()
                        .filter_map(|option| {
                            render_option(&option.value, data.resolved.as_ref())
                                .map(|value| (option.name.clone(), value))
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
                    let kind = match subcommand {
                        Some(sub) => EventKind::SlashSubcommand {
                            command: &data.name,
                            group,
                            subcommand: sub,
                        },
                        None => EventKind::SlashCommand { name: &data.name },
                    };
                    let label = match (group, subcommand) {
                        (Some(grp), Some(sub)) => format!("/{} {} {}", data.name, grp, sub),
                        (None, Some(sub)) => format!("/{} {}", data.name, sub),
                        _ => format!("/{}", data.name),
                    };
                    run_interaction(
                        bot,
                        program,
                        &interaction,
                        user.id,
                        &Event { kind, context },
                        false,
                        None,
                        &label,
                        "This command is not available anymore.",
                    )
                    .await;
                }
                CommandType::User => {
                    let target_id = data.target_id.map(|id| id.get());
                    let target_mention = target_id.map(|id| format!("<@{id}>"));
                    let target_name =
                        data.target_id
                            .map(|id| id.cast::<UserMarker>())
                            .and_then(|uid| {
                                data.resolved
                                    .as_ref()
                                    .and_then(|res| res.users.get(&uid))
                                    .map(|u| display_name(u).to_owned())
                            });
                    let mut context = context_for(
                        user,
                        &roles,
                        interaction.guild_id,
                        channel_id,
                        None,
                        &[],
                        None,
                    );
                    context.target_id = target_id;
                    context.target_mention = target_mention.as_deref();
                    context.target_name = target_name.as_deref();
                    run_interaction(
                        bot,
                        program,
                        &interaction,
                        user.id,
                        &Event {
                            kind: EventKind::UserCommand { name: &data.name },
                            context,
                        },
                        false,
                        None,
                        &format!("user menu \"{}\"", data.name),
                        "This command is not available anymore.",
                    )
                    .await;
                }
                CommandType::Message => {
                    let target_msg = data
                        .target_id
                        .map(|id| id.cast::<MessageMarker>())
                        .and_then(|mid| data.resolved.as_ref().and_then(|r| r.messages.get(&mid)));
                    let target_id = data.target_id.map(|id| id.get());
                    let target_author_id = target_msg.map(|m| m.author.id.get());
                    let target_name = target_msg.map(|m| display_name(&m.author));
                    let target_content = target_msg.map(|m| m.content.as_str());
                    let target_mention = target_author_id.map(|id| format!("<@{id}>"));
                    let mut context = context_for(
                        user,
                        &roles,
                        interaction.guild_id,
                        channel_id,
                        target_id.and_then(Id::new_checked),
                        &[],
                        None,
                    );
                    context.target_id = target_id;
                    context.target_mention = target_mention.as_deref();
                    context.target_name = target_name;
                    context.target_content = target_content;
                    context.target_author_id = target_author_id;
                    run_interaction(
                        bot,
                        program,
                        &interaction,
                        user.id,
                        &Event {
                            kind: EventKind::MessageCommand { name: &data.name },
                            context,
                        },
                        false,
                        None,
                        &format!("message menu \"{}\"", data.name),
                        "This command is not available anymore.",
                    )
                    .await;
                }
                _ => {}
            }
        }
        (
            InteractionType::ApplicationCommandAutocomplete,
            Some(InteractionData::ApplicationCommand(data)),
        ) => {
            on_autocomplete(bot, program, &interaction, data).await;
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
        (InteractionType::ModalSubmit, Some(InteractionData::ModalSubmit(data))) => {
            let modal_fields = extract_modal_fields(&data.components);
            let mut context = context_for(
                user,
                &roles,
                interaction.guild_id,
                channel_id,
                interaction.message.as_ref().map(|m| m.id),
                &[],
                None,
            );
            context.modal_fields = &modal_fields;
            run_interaction(
                bot,
                program,
                &interaction,
                user.id,
                &Event {
                    kind: EventKind::ModalSubmit {
                        id: &data.custom_id,
                    },
                    context,
                },
                false,
                None,
                &format!("modal \"{}\"", data.custom_id),
                "This form is not available anymore.",
            )
            .await;
        }
        _ => {}
    }
}

fn split_command_options(
    options: &[CommandDataOption],
) -> (Option<&str>, Option<&str>, &[CommandDataOption]) {
    match options.first() {
        Some(CommandDataOption {
            name,
            value: CommandOptionValue::SubCommand(inner),
        }) => (None, Some(name.as_str()), inner.as_slice()),
        Some(CommandDataOption {
            name: group_name,
            value: CommandOptionValue::SubCommandGroup(group_inner),
        }) => match group_inner.first() {
            Some(CommandDataOption {
                name: sub_name,
                value: CommandOptionValue::SubCommand(inner),
            }) => (
                Some(group_name.as_str()),
                Some(sub_name.as_str()),
                inner.as_slice(),
            ),
            _ => (Some(group_name.as_str()), None, options),
        },
        _ => (None, None, options),
    }
}

async fn on_autocomplete(
    bot: &BotContext,
    program: &CompiledProgram,
    interaction: &Interaction,
    data: &twilight_model::application::interaction::application_command::CommandData,
) {
    let (group, subcommand, leaf_options) = split_command_options(&data.options);
    let Some((option_name, query, kind)) = leaf_options.iter().find_map(|opt| match &opt.value {
        CommandOptionValue::Focused(query, kind) => {
            Some((opt.name.as_str(), query.as_str(), *kind))
        }
        _ => None,
    }) else {
        return;
    };

    let suggestions = program.autocomplete_choices(&data.name, group, subcommand, option_name);
    let lowered = query.to_lowercase();
    let choices: Vec<CommandOptionChoice> = suggestions
        .iter()
        .filter(|item| lowered.is_empty() || item.to_lowercase().contains(&lowered))
        .take(25)
        .filter_map(|item| {
            let value = match kind {
                twilight_model::application::command::CommandOptionType::Integer => {
                    CommandOptionChoiceValue::Integer(item.trim().parse::<i64>().ok()?)
                }
                twilight_model::application::command::CommandOptionType::Number => {
                    CommandOptionChoiceValue::Number(item.trim().parse::<f64>().ok()?)
                }
                _ => CommandOptionChoiceValue::String(item.clone()),
            };
            Some(CommandOptionChoice {
                name: item.clone(),
                name_localizations: None,
                value,
            })
        })
        .collect();

    let response = AutocompleteBuilder::new(choices).build();
    if let Err(error) = bot
        .http
        .interaction(bot.application_id)
        .create_response(interaction.id, &interaction.token, &response)
        .await
    {
        tracing::warn!(bot = %bot.id, %error, "failed to answer autocomplete");
    }
}

fn extract_modal_fields(components: &[ModalInteractionComponent]) -> Vec<(String, String)> {
    let mut fields = Vec::new();
    for component in components {
        match component {
            ModalInteractionComponent::ActionRow(row) => {
                fields.extend(extract_modal_fields(&row.components));
            }
            ModalInteractionComponent::Label(label) => {
                fields.extend(extract_modal_fields(std::slice::from_ref(
                    label.component.as_ref(),
                )));
            }
            ModalInteractionComponent::TextInput(input) => {
                fields.push((input.custom_id.clone(), input.value.clone()));
            }
            ModalInteractionComponent::StringSelect(select) => {
                fields.push((select.custom_id.clone(), select.values.join(", ")));
            }
            ModalInteractionComponent::UserSelect(select) => {
                let joined = select
                    .values
                    .iter()
                    .map(|id| id.get().to_string())
                    .collect::<Vec<_>>()
                    .join(", ");
                fields.push((select.custom_id.clone(), joined));
            }
            ModalInteractionComponent::RoleSelect(select) => {
                let joined = select
                    .values
                    .iter()
                    .map(|id| id.get().to_string())
                    .collect::<Vec<_>>()
                    .join(", ");
                fields.push((select.custom_id.clone(), joined));
            }
            ModalInteractionComponent::MentionableSelect(select) => {
                let joined = select
                    .values
                    .iter()
                    .map(|id| id.get().to_string())
                    .collect::<Vec<_>>()
                    .join(", ");
                fields.push((select.custom_id.clone(), joined));
            }
            ModalInteractionComponent::ChannelSelect(select) => {
                let joined = select
                    .values
                    .iter()
                    .map(|id| id.get().to_string())
                    .collect::<Vec<_>>()
                    .join(", ");
                fields.push((select.custom_id.clone(), joined));
            }
            _ => {}
        }
    }
    fields
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

async fn on_member_update(
    bot: &BotContext,
    program: &CompiledProgram,
    member: &twilight_model::gateway::payload::incoming::MemberUpdate,
) {
    if member.user.bot {
        return;
    }
    let roles = role_ids(&member.roles);
    let context = context_for(
        &member.user,
        &roles,
        Some(member.guild_id),
        None,
        None,
        &[],
        None,
    );
    let target = Target {
        guild_id: Some(member.guild_id),
        user_id: Some(member.user.id),
    };
    let event = Event {
        kind: EventKind::MemberUpdate,
        context,
    };
    run_many(
        bot,
        program,
        &event,
        target,
        || Reply::Unavailable,
        "member updated",
    )
    .await;
}

async fn on_ban(
    bot: &BotContext,
    program: &CompiledProgram,
    guild_id: Id<GuildMarker>,
    user: &User,
    banned: bool,
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
        kind: if banned {
            EventKind::MemberBan
        } else {
            EventKind::MemberUnban
        },
        context,
    };
    let label = if banned {
        "member banned"
    } else {
        "member unbanned"
    };
    run_many(bot, program, &event, target, || Reply::Unavailable, label).await;
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
    let mut context = context_for(
        &message.author,
        &roles,
        message.guild_id,
        Some(message.channel_id),
        Some(message.id),
        &[],
        None,
    );
    context.message_content = Some(&message.content);
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

async fn on_message_update(bot: &BotContext, program: &CompiledProgram, message: &Message) {
    if message.author.bot || message.guild_id.is_none() {
        return;
    }
    let roles = message
        .member
        .as_ref()
        .map(|member| role_ids(&member.roles))
        .unwrap_or_default();
    let mut context = context_for(
        &message.author,
        &roles,
        message.guild_id,
        Some(message.channel_id),
        Some(message.id),
        &[],
        None,
    );
    context.message_content = Some(&message.content);
    let target = Target {
        guild_id: message.guild_id,
        user_id: Some(message.author.id),
    };
    let event = Event {
        kind: EventKind::MessageEdit,
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
        "message edited",
    )
    .await;
}

async fn on_message_delete(
    bot: &BotContext,
    program: &CompiledProgram,
    guild_id: Option<Id<GuildMarker>>,
    channel_id: Id<ChannelMarker>,
    message_id: Id<MessageMarker>,
) {
    if guild_id.is_none() {
        return;
    }
    let context = Context {
        guild_id: guild_id.map(Id::get),
        channel_id: Some(channel_id.get()),
        message_id: Some(message_id.get()),
        ..Context::default()
    };
    let target = Target {
        guild_id,
        user_id: None,
    };
    let event = Event {
        kind: EventKind::MessageDelete,
        context,
    };
    run_many(
        bot,
        program,
        &event,
        target,
        || Reply::Unavailable,
        "message deleted",
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
        emoji: Some(&rendered_emoji),
        ..Context::default()
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

async fn on_thread_create(
    bot: &BotContext,
    program: &CompiledProgram,
    thread: &twilight_model::channel::Channel,
) {
    if thread.guild_id.is_none() || thread.newly_created != Some(true) {
        return;
    }
    let context = Context {
        user_id: thread.owner_id.map(Id::get),
        guild_id: thread.guild_id.map(Id::get),
        channel_id: Some(thread.id.get()),
        thread_name: thread.name.as_deref(),
        ..Context::default()
    };
    let target = Target {
        guild_id: thread.guild_id,
        user_id: thread.owner_id,
    };
    let event = Event {
        kind: EventKind::ThreadCreate,
        context,
    };
    run_many(
        bot,
        program,
        &event,
        target,
        || Reply::Unavailable,
        "thread created",
    )
    .await;
}

async fn on_invite_create(
    bot: &BotContext,
    program: &CompiledProgram,
    invite: &twilight_model::gateway::payload::incoming::InviteCreate,
) {
    let context = Context {
        user_id: invite.inviter.as_ref().map(|u| u.id.get()),
        user_name: invite.inviter.as_ref().map(display_name),
        guild_id: Some(invite.guild_id.get()),
        channel_id: Some(invite.channel_id.get()),
        invite_code: Some(&invite.code),
        ..Context::default()
    };
    let target = Target {
        guild_id: Some(invite.guild_id),
        user_id: invite.inviter.as_ref().map(|u| u.id),
    };
    let event = Event {
        kind: EventKind::InviteCreate,
        context,
    };
    run_many(
        bot,
        program,
        &event,
        target,
        || Reply::Unavailable,
        "invite created",
    )
    .await;
}

async fn on_automod_execution(
    bot: &BotContext,
    program: &CompiledProgram,
    exec: &twilight_model::gateway::payload::incoming::AutoModerationActionExecution,
) {
    let context = Context {
        user_id: Some(exec.user_id.get()),
        guild_id: Some(exec.guild_id.get()),
        channel_id: exec.channel_id.map(Id::get),
        message_id: exec.message_id.map(Id::get),
        automod_rule_id: Some(exec.rule_id.get()),
        automod_keyword: exec.matched_keyword.as_deref(),
        automod_content: Some(&exec.content),
        ..Context::default()
    };
    let target = Target {
        guild_id: Some(exec.guild_id),
        user_id: Some(exec.user_id),
    };
    let event = Event {
        kind: EventKind::AutoModExecution {
            rule_id: exec.rule_id.get(),
        },
        context,
    };
    run_many(
        bot,
        program,
        &event,
        target,
        || Reply::Unavailable,
        "AutoMod execution",
    )
    .await;
}

async fn on_poll_vote_add(
    bot: &BotContext,
    program: &CompiledProgram,
    vote: &twilight_model::gateway::payload::incoming::MessagePollVoteAdd,
) {
    if vote.guild_id.is_none() || bot.bot_user_id.get() == Some(&vote.user_id) {
        return;
    }
    let context = Context {
        user_id: Some(vote.user_id.get()),
        guild_id: vote.guild_id.map(Id::get),
        channel_id: Some(vote.channel_id.get()),
        message_id: Some(vote.message_id.get()),
        poll_answer_id: Some(vote.answer_id),
        ..Context::default()
    };
    let target = Target {
        guild_id: vote.guild_id,
        user_id: Some(vote.user_id),
    };
    let event = Event {
        kind: EventKind::PollVoteAdd,
        context,
    };
    run_many(
        bot,
        program,
        &event,
        target,
        || Reply::Unavailable,
        "poll vote added",
    )
    .await;
}

async fn on_voice_state_update(
    bot: &BotContext,
    program: &CompiledProgram,
    voice: &twilight_model::voice::VoiceState,
) {
    let Some(guild_id) = voice.guild_id else {
        return;
    };
    if voice.member.as_ref().is_some_and(|m| m.user.bot) {
        return;
    }
    let roles = voice
        .member
        .as_ref()
        .map(|member| role_ids(&member.roles))
        .unwrap_or_default();
    let user_name = voice.member.as_ref().map(|m| display_name(&m.user));
    let context = Context {
        user_id: Some(voice.user_id.get()),
        user_name,
        roles: &roles,
        guild_id: Some(guild_id.get()),
        channel_id: voice.channel_id.map(Id::get),
        ..Context::default()
    };
    let target = Target {
        guild_id: Some(guild_id),
        user_id: Some(voice.user_id),
    };
    let event = Event {
        kind: EventKind::VoiceStateUpdate,
        context,
    };
    run_many(
        bot,
        program,
        &event,
        target,
        || Reply::Unavailable,
        "voice state update",
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
        ..Context::default()
    }
}

fn render_option(
    value: &CommandOptionValue,
    resolved: Option<&InteractionDataResolved>,
) -> Option<String> {
    Some(match value {
        CommandOptionValue::Attachment(id) => resolved
            .and_then(|data| data.attachments.get(id))
            .map_or_else(|| id.get().to_string(), |attachment| attachment.url.clone()),
        CommandOptionValue::String(value) => value.clone(),
        CommandOptionValue::Integer(value) => value.to_string(),
        CommandOptionValue::Number(value) => value.to_string(),
        CommandOptionValue::Boolean(value) => if *value { "yes" } else { "no" }.to_owned(),
        CommandOptionValue::User(id) => format!("<@{id}>"),
        CommandOptionValue::Channel(id) => format!("<#{id}>"),
        CommandOptionValue::Role(id) => format!("<@&{id}>"),
        CommandOptionValue::Mentionable(id) => id.get().to_string(),
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
            reply_to_message_id,
        } => send_channel_message(http, *channel_id, message, *reply_to_message_id).await,
        Effect::UpdateMessage { message } => reply.update(bot, message).await,
        Effect::ShowModal { id, title, inputs } => reply.show_modal(bot, id, title, inputs).await,
        Effect::CreateThreadFromMessage {
            name,
            auto_archive_minutes,
        } => {
            let Reply::Message {
                channel_id,
                message_id,
            } = *reply
            else {
                return Err("starting a thread from a message needs a message trigger".to_owned());
            };
            let mut req = http.create_thread_from_message(channel_id, message_id, name);
            if let Some(archive) = auto_archive_duration(*auto_archive_minutes) {
                req = req.auto_archive_duration(archive);
            }
            req.await
                .map(drop)
                .map_err(|error| describe(&error, "create the thread"))
        }
        Effect::CreateThread {
            channel_id,
            name,
            private,
            auto_archive_minutes,
        } => {
            let channel_id = Id::new_checked(*channel_id).ok_or("invalid channel ID")?;
            let kind = if *private {
                ChannelType::PrivateThread
            } else {
                ChannelType::PublicThread
            };
            let mut req = http.create_thread(channel_id, name, kind);
            if let Some(archive) = auto_archive_duration(*auto_archive_minutes) {
                req = req.auto_archive_duration(archive);
            }
            req.await
                .map(drop)
                .map_err(|error| describe(&error, "create the thread"))
        }
        Effect::CreateForumPost {
            channel_id,
            title,
            content,
            embeds,
        } => {
            let channel_id = Id::new_checked(*channel_id).ok_or("invalid forum channel ID")?;
            let built_embeds = embeds
                .iter()
                .map(build_embed)
                .collect::<Result<Vec<_>, _>>()?;
            let mut msg = http.create_forum_thread(channel_id, title).message();
            if let Some(content) = content.as_deref() {
                msg = msg.content(content);
            }
            if !built_embeds.is_empty() {
                msg = msg.embeds(&built_embeds);
            }
            msg.await
                .map(drop)
                .map_err(|error| describe(&error, "create the forum post"))
        }
        Effect::JoinThread { thread_id } => {
            let thread_id = Id::new_checked(*thread_id).ok_or("invalid thread ID")?;
            http.join_thread(thread_id)
                .await
                .map(drop)
                .map_err(|error| describe(&error, "join the thread"))
        }
        Effect::AddThreadMember {
            thread_id,
            member_id,
        } => {
            let thread_id = Id::new_checked(*thread_id).ok_or("invalid thread ID")?;
            let member_id = Id::new_checked(*member_id).ok_or("invalid member ID")?;
            http.add_thread_member(thread_id, member_id)
                .await
                .map(drop)
                .map_err(|error| describe(&error, "add the member to the thread"))
        }
        Effect::RemoveThreadMember {
            thread_id,
            member_id,
        } => {
            let thread_id = Id::new_checked(*thread_id).ok_or("invalid thread ID")?;
            let member_id = Id::new_checked(*member_id).ok_or("invalid member ID")?;
            http.remove_thread_member(thread_id, member_id)
                .await
                .map(drop)
                .map_err(|error| describe(&error, "remove the member from the thread"))
        }
        Effect::PinMessage {
            channel_id,
            message_id,
        } => {
            let channel_id = Id::new_checked(*channel_id).ok_or("invalid channel ID")?;
            let message_id = Id::new_checked(*message_id).ok_or("invalid message ID")?;
            http.create_pin(channel_id, message_id)
                .await
                .map(drop)
                .map_err(|error| describe(&error, "pin the message"))
        }
        Effect::UnpinMessage {
            channel_id,
            message_id,
        } => {
            let channel_id = Id::new_checked(*channel_id).ok_or("invalid channel ID")?;
            let message_id = Id::new_checked(*message_id).ok_or("invalid message ID")?;
            http.delete_pin(channel_id, message_id)
                .await
                .map(drop)
                .map_err(|error| describe(&error, "unpin the message"))
        }
        Effect::PublishMessage {
            channel_id,
            message_id,
        } => {
            let channel_id = Id::new_checked(*channel_id).ok_or("invalid channel ID")?;
            let message_id = Id::new_checked(*message_id).ok_or("invalid message ID")?;
            http.crosspost_message(channel_id, message_id)
                .await
                .map(drop)
                .map_err(|error| describe(&error, "publish the announcement"))
        }
        Effect::BulkDeleteMessages { channel_id, count } => {
            perform_bulk_delete(http, *channel_id, *count).await
        }
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

fn auto_archive_duration(minutes: Option<u16>) -> Option<AutoArchiveDuration> {
    match minutes {
        Some(60) => Some(AutoArchiveDuration::Hour),
        Some(1440) => Some(AutoArchiveDuration::Day),
        Some(4320) => Some(AutoArchiveDuration::ThreeDays),
        Some(10080) => Some(AutoArchiveDuration::Week),
        _ => None,
    }
}

async fn perform_bulk_delete(
    http: &twilight_http::Client,
    channel_id: u64,
    count: u8,
) -> Result<(), String> {
    let channel_id = Id::<ChannelMarker>::new_checked(channel_id).ok_or("invalid channel ID")?;
    let messages = http
        .channel_messages(channel_id)
        .limit(u16::from(count))
        .await
        .map_err(|error| describe(&error, "fetch messages to delete"))?
        .model()
        .await
        .map_err(|error| format!("could not read channel messages: {error}"))?;
    let ids: Vec<Id<MessageMarker>> = messages.into_iter().map(|m| m.id).collect();
    match ids.len() {
        0 => Ok(()),
        1 => http
            .delete_message(channel_id, ids[0])
            .await
            .map(drop)
            .map_err(|error| describe(&error, "delete the message")),
        _ => http
            .delete_messages(channel_id, &ids)
            .await
            .map(drop)
            .map_err(|error| describe(&error, "bulk delete messages")),
    }
}

async fn send_channel_message(
    http: &twilight_http::Client,
    channel_id: u64,
    message: &MessagePayload,
    reply_to_message_id: Option<u64>,
) -> Result<(), String> {
    let channel_id = Id::<ChannelMarker>::new_checked(channel_id).ok_or("invalid channel ID")?;
    let (embeds, components) = build_rich(message)?;
    let poll = message.poll.as_ref().map(build_poll);
    let attachments = build_attachments(&message.attachments);
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
    if let Some(poll) = poll.as_ref() {
        req = req.poll(poll);
    }
    if !attachments.is_empty() {
        req = req.attachments(&attachments);
    }
    if let Some(target_msg_id) = reply_to_message_id {
        let target_msg_id =
            Id::new_checked(target_msg_id).ok_or("invalid reply target message ID")?;
        req = req.reply(target_msg_id).fail_if_not_exists(false);
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

fn build_attachments(attachments: &[RenderedAttachment]) -> Vec<Attachment> {
    attachments
        .iter()
        .enumerate()
        .map(|(index, item)| {
            Attachment::from_bytes(
                item.filename.clone(),
                item.content.as_bytes().to_vec(),
                u64::try_from(index + 1).unwrap_or(1),
            )
        })
        .collect()
}

fn build_poll(poll: &RenderedPoll) -> Poll {
    let expiry = OffsetDateTime::now_utc()
        .checked_add(time::Duration::hours(i64::from(poll.duration_hours)))
        .and_then(|dt| Timestamp::from_secs(dt.unix_timestamp()).ok());
    let answers = poll
        .answers
        .iter()
        .enumerate()
        .map(|(idx, answer)| {
            let emoji = answer.emoji.as_ref().map(|e| match e {
                EmojiRef::Unicode(name) => PartialPollMediaEmoji {
                    animated: false,
                    id: None,
                    name: Some(name.clone()),
                },
                EmojiRef::Custom { name, id, animated } => PartialPollMediaEmoji {
                    animated: *animated,
                    id: Id::new_checked(*id),
                    name: Some(name.clone()),
                },
            });
            PollAnswer {
                answer_id: u8::try_from(idx + 1).unwrap_or(10),
                poll_media: PollMedia {
                    emoji,
                    text: Some(answer.text.clone()),
                },
            }
        })
        .collect();
    Poll {
        answers,
        allow_multiselect: poll.allow_multiselect,
        expiry,
        layout_type: PollLayoutType::Default,
        question: PollMedia {
            emoji: None,
            text: Some(poll.question.clone()),
        },
        results: None,
    }
}

#[allow(deprecated)]
fn build_modal_components(inputs: &[RenderedModalInput]) -> Vec<Component> {
    inputs
        .iter()
        .map(|input| {
            Component::ActionRow(ActionRow {
                id: None,
                components: vec![Component::TextInput(TextInput {
                    custom_id: input.id.clone(),
                    id: None,
                    label: Some(input.label.clone()),
                    max_length: input.max_length,
                    min_length: input.min_length,
                    placeholder: input.placeholder.clone(),
                    required: Some(input.required),
                    style: match input.style {
                        TextInputStyle::Short => TwilightTextInputStyle::Short,
                        TextInputStyle::Paragraph => TwilightTextInputStyle::Paragraph,
                    },
                    value: input.value.clone(),
                })],
            })
        })
        .collect()
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
        RenderedComponentRow::EntitySelectMenu {
            id,
            kind,
            placeholder,
            min_values,
            max_values,
            disabled,
            channel_types,
        } => {
            let mapped_channel_types =
                if *kind == EntitySelectKind::Channel && !channel_types.is_empty() {
                    Some(
                        channel_types
                            .iter()
                            .map(|filter| match filter {
                                ChannelFilter::Text => ChannelType::GuildText,
                                ChannelFilter::Voice => ChannelType::GuildVoice,
                                ChannelFilter::Announcement => ChannelType::GuildAnnouncement,
                                ChannelFilter::Forum => ChannelType::GuildForum,
                                ChannelFilter::Stage => ChannelType::GuildStageVoice,
                                ChannelFilter::Category => ChannelType::GuildCategory,
                                ChannelFilter::Media => ChannelType::GuildMedia,
                            })
                            .collect(),
                    )
                } else {
                    None
                };
            vec![Component::SelectMenu(SelectMenu {
                channel_types: mapped_channel_types,
                custom_id: id.clone(),
                default_values: None,
                disabled: *disabled,
                id: None,
                kind: match kind {
                    EntitySelectKind::User => SelectMenuType::User,
                    EntitySelectKind::Role => SelectMenuType::Role,
                    EntitySelectKind::Mentionable => SelectMenuType::Mentionable,
                    EntitySelectKind::Channel => SelectMenuType::Channel,
                },
                max_values: Some(*max_values),
                min_values: Some(*min_values),
                options: None,
                placeholder: placeholder.clone(),
                required: None,
            })]
        }
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
    /// is not an immediate reply, update, or modal, acknowledge right away.
    async fn prepare(&mut self, bot: &BotContext, effects: &[Effect]) {
        let Self::Interaction(interaction) = self else {
            return;
        };
        if matches!(
            effects.first(),
            Some(Effect::Reply { .. } | Effect::UpdateMessage { .. } | Effect::ShowModal { .. })
        ) || effects
            .iter()
            .any(|effect| matches!(effect, Effect::ShowModal { .. }))
        {
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

    async fn show_modal(
        &mut self,
        bot: &BotContext,
        id: &str,
        title: &str,
        inputs: &[RenderedModalInput],
    ) -> Result<(), String> {
        let Self::Interaction(interaction) = self else {
            return Err("opening a modal needs an interaction trigger".to_owned());
        };
        if interaction.state != ReplyState::Pending {
            return Err("opening a modal must be the first response".to_owned());
        }
        let components = build_modal_components(inputs);
        let response = ModalBuilder::new(id, title, components).build();
        interaction.state = ReplyState::Sent;
        bot.http
            .interaction(bot.application_id)
            .create_response(interaction.id, interaction.token, &response)
            .await
            .map(drop)
            .map_err(|error| describe(&error, "open the modal"))
    }

    async fn send(
        &mut self,
        bot: &BotContext,
        payload: &MessagePayload,
        ephemeral: bool,
    ) -> Result<(), String> {
        let (embeds, components) = build_rich(payload)?;
        let poll = payload.poll.as_ref().map(build_poll);
        let attachments = build_attachments(&payload.attachments);
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
                        if let Some(poll) = poll {
                            builder = builder.poll(poll);
                        }
                        if !attachments.is_empty() {
                            builder = builder.attachments(attachments);
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
                        if !attachments.is_empty() {
                            req = req.attachments(&attachments);
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
                        if let Some(poll) = poll {
                            req = req.poll(poll);
                        }
                        if !attachments.is_empty() {
                            req = req.attachments(&attachments);
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
                if let Some(poll) = poll.as_ref() {
                    req = req.poll(poll);
                }
                if !attachments.is_empty() {
                    req = req.attachments(&attachments);
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
        RenderedEmbedAuthor, RenderedEmbedField, RenderedEmbedFooter, RenderedPollAnswer,
        RenderedSelectOption,
    };
    use twilight_http::request::TryIntoRequest as _;
    use twilight_model::application::interaction::modal::{
        ModalInteractionActionRow, ModalInteractionLabel, ModalInteractionTextInput,
    };

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
                RenderedComponentRow::EntitySelectMenu {
                    id: "pick_channel".to_owned(),
                    kind: EntitySelectKind::Channel,
                    placeholder: Some("Pick a text channel".to_owned()),
                    min_values: 1,
                    max_values: 2,
                    disabled: false,
                    channel_types: vec![ChannelFilter::Text],
                },
            ],
            poll: Some(RenderedPoll {
                question: "Favorite color?".to_owned(),
                answers: vec![
                    RenderedPollAnswer {
                        text: "Blue".to_owned(),
                        emoji: Some(EmojiRef::Unicode("🔵".to_owned())),
                    },
                    RenderedPollAnswer {
                        text: "Green".to_owned(),
                        emoji: None,
                    },
                ],
                duration_hours: 24,
                allow_multiselect: false,
            }),
            attachments: vec![RenderedAttachment {
                filename: "summary.txt".to_owned(),
                content: "Hello attachment".to_owned(),
            }],
        };

        let (embeds, components) = build_rich(&payload).expect("valid rich payload");
        let poll = build_poll(payload.poll.as_ref().unwrap());
        let attachments = build_attachments(&payload.attachments);
        assert_eq!(embeds.len(), 1);
        assert!(embeds[0].timestamp.is_some());
        assert_eq!(components.len(), 3);

        let client = twilight_http::Client::new(String::new());
        client
            .create_message(Id::new(1))
            .content("Hello!")
            .embeds(&embeds)
            .components(&components)
            .poll(&poll)
            .attachments(&attachments)
            .try_into_request()
            .expect("twilight accepts built embeds, components, poll, and attachments");
    }

    #[test]
    fn builds_valid_modals_and_extracts_modal_fields() {
        let components = build_modal_components(&[RenderedModalInput {
            id: "reason".to_owned(),
            label: "Reason".to_owned(),
            style: TextInputStyle::Paragraph,
            placeholder: Some("Write here".to_owned()),
            required: true,
            min_length: Some(3),
            max_length: Some(200),
            value: None,
        }]);
        let modal = ModalBuilder::new("warn_modal", "Warn member", components).build();
        assert_eq!(modal.kind, InteractionResponseType::Modal);

        let extracted = extract_modal_fields(&[
            ModalInteractionComponent::ActionRow(ModalInteractionActionRow {
                id: 1,
                components: vec![ModalInteractionComponent::TextInput(
                    ModalInteractionTextInput {
                        custom_id: "reason".to_owned(),
                        id: 2,
                        value: "Spamming".to_owned(),
                    },
                )],
            }),
            ModalInteractionComponent::Label(ModalInteractionLabel {
                id: 3,
                component: Box::new(ModalInteractionComponent::TextInput(
                    ModalInteractionTextInput {
                        custom_id: "notes".to_owned(),
                        id: 4,
                        value: "Second warning".to_owned(),
                    },
                )),
            }),
        ]);
        assert_eq!(
            extracted,
            vec![
                ("reason".to_owned(), "Spamming".to_owned()),
                ("notes".to_owned(), "Second warning".to_owned()),
            ]
        );
    }
}
