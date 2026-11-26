use std::collections::{HashMap, HashSet};

use thiserror::Error;

use crate::model::{Action, CommandOption, Handler, Program, Trigger};
use crate::run::EmojiRef;
use crate::template::MAX_MESSAGE_LEN;

const MAX_HANDLERS: usize = 200;
/// Discord allows 100 global chat input commands per application.
const MAX_COMMANDS: usize = 100;
const MAX_OPTIONS: usize = 25;
const MAX_ACTIONS: usize = 10;
const MAX_PATTERN_LEN: usize = 200;
const MAX_NAME_LEN: usize = 32;
const MAX_DESCRIPTION_LEN: usize = 100;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum CompileError {
    #[error("a bot can have at most {MAX_HANDLERS} triggers")]
    TooManyHandlers,
    #[error("a bot can have at most {MAX_COMMANDS} slash commands")]
    TooManyCommands,
    #[error("trigger #{}: {reason}", .handler + 1)]
    Handler {
        handler: usize,
        reason: HandlerError,
    },
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum HandlerError {
    #[error("`{0}` is not a valid name: use 1-32 lowercase letters, digits, `-` or `_`")]
    InvalidName(String),
    #[error("command `/{0}` is defined twice")]
    DuplicateCommand(String),
    #[error("descriptions must be 1-100 characters")]
    InvalidDescription,
    #[error("a command can have at most {MAX_OPTIONS} options")]
    TooManyOptions,
    #[error("option `{0}` is defined twice")]
    DuplicateOption(String),
    #[error("required option `{0}` must come before the optional ones")]
    RequiredAfterOptional(String),
    #[error("the searched text must be 1-{MAX_PATTERN_LEN} characters")]
    InvalidPattern,
    #[error("a trigger needs between 1 and {MAX_ACTIONS} actions")]
    InvalidActionCount,
    #[error("action #{}: {reason}", .action + 1)]
    Action { action: usize, reason: ActionError },
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum ActionError {
    #[error("messages must be 1-{MAX_MESSAGE_LEN} characters")]
    InvalidContent,
    #[error("`{0}` is not a valid Discord ID")]
    InvalidId(String),
    #[error("`{0}` is not a unicode emoji or a custom emoji written `name:id`")]
    InvalidEmoji(String),
    #[error("`{action}` cannot be used with a `{trigger}` trigger")]
    NotAvailable {
        action: &'static str,
        trigger: &'static str,
    },
}

/// A slash command to register on Discord.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SlashCommandSpec {
    pub name: String,
    pub description: String,
    pub options: Vec<CommandOption>,
}

/// What the gateway connection must subscribe to for a program to work.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct GatewayNeeds {
    /// Member join events (privileged `GUILD_MEMBERS` intent).
    pub members: bool,
    /// Message events with their content (privileged `MESSAGE_CONTENT` intent).
    pub messages: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum CompiledAction {
    Reply { content: String, ephemeral: bool },
    SendMessage { channel_id: u64, content: String },
    AddRole { role_id: u64 },
    RemoveRole { role_id: u64 },
    React { emoji: EmojiRef },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CompiledHandler {
    pub(crate) label: String,
    pub(crate) actions: Vec<CompiledAction>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct MessageMatcher {
    /// Lowercased when the match is case insensitive.
    pub(crate) pattern: String,
    pub(crate) case_sensitive: bool,
    pub(crate) handler: usize,
}

/// A program whose every value has been checked against Discord's limits.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CompiledProgram {
    pub(crate) handlers: Vec<CompiledHandler>,
    pub(crate) commands: Vec<SlashCommandSpec>,
    pub(crate) slash: HashMap<String, usize>,
    pub(crate) member_join: Vec<usize>,
    pub(crate) messages: Vec<MessageMatcher>,
}

impl CompiledProgram {
    /// Slash commands to register, in the order they appear in the program.
    #[must_use]
    pub fn commands(&self) -> &[SlashCommandSpec] {
        &self.commands
    }

    #[must_use]
    pub fn gateway_needs(&self) -> GatewayNeeds {
        GatewayNeeds {
            members: !self.member_join.is_empty(),
            messages: !self.messages.is_empty(),
        }
    }
}

#[derive(Clone, Copy)]
enum TriggerKind {
    Slash,
    MemberJoin,
    Message,
}

impl TriggerKind {
    const fn name(self) -> &'static str {
        match self {
            Self::Slash => "slash command",
            Self::MemberJoin => "member join",
            Self::Message => "message contains",
        }
    }
}

impl Program {
    /// Check the program and turn it into its executable form.
    ///
    /// # Errors
    ///
    /// Returns the first rule the program breaks: a Discord limit, a malformed
    /// ID or emoji, or an action that cannot run with its trigger.
    pub fn compile(&self) -> Result<CompiledProgram, CompileError> {
        if self.handlers.len() > MAX_HANDLERS {
            return Err(CompileError::TooManyHandlers);
        }

        let mut compiled = CompiledProgram::default();
        for (index, handler) in self.handlers.iter().enumerate() {
            compile_handler(&mut compiled, index, handler).map_err(|reason| {
                CompileError::Handler {
                    handler: index,
                    reason,
                }
            })?;
        }

        if compiled.commands.len() > MAX_COMMANDS {
            return Err(CompileError::TooManyCommands);
        }
        Ok(compiled)
    }
}

fn compile_handler(
    compiled: &mut CompiledProgram,
    index: usize,
    handler: &Handler,
) -> Result<(), HandlerError> {
    let (kind, label) = match &handler.trigger {
        Trigger::SlashCommand {
            name,
            description,
            options,
        } => {
            check_name(name)?;
            check_description(description)?;
            check_options(options)?;
            if compiled.slash.insert(name.clone(), index).is_some() {
                return Err(HandlerError::DuplicateCommand(name.clone()));
            }
            compiled.commands.push(SlashCommandSpec {
                name: name.clone(),
                description: description.clone(),
                options: options.clone(),
            });
            (TriggerKind::Slash, format!("/{name}"))
        }
        Trigger::MemberJoin {} => {
            compiled.member_join.push(index);
            (TriggerKind::MemberJoin, "member join".to_owned())
        }
        Trigger::MessageContains {
            pattern,
            case_sensitive,
        } => {
            let length = pattern.chars().count();
            if length == 0 || length > MAX_PATTERN_LEN || pattern.trim().is_empty() {
                return Err(HandlerError::InvalidPattern);
            }
            compiled.messages.push(MessageMatcher {
                pattern: if *case_sensitive {
                    pattern.clone()
                } else {
                    pattern.to_lowercase()
                },
                case_sensitive: *case_sensitive,
                handler: index,
            });
            (
                TriggerKind::Message,
                format!("message contains \"{pattern}\""),
            )
        }
    };

    if handler.actions.is_empty() || handler.actions.len() > MAX_ACTIONS {
        return Err(HandlerError::InvalidActionCount);
    }
    let actions = handler
        .actions
        .iter()
        .enumerate()
        .map(|(action, value)| {
            compile_action(kind, value).map_err(|reason| HandlerError::Action { action, reason })
        })
        .collect::<Result<Vec<_>, _>>()?;

    compiled.handlers.push(CompiledHandler { label, actions });
    Ok(())
}

fn compile_action(trigger: TriggerKind, action: &Action) -> Result<CompiledAction, ActionError> {
    let not_available = |action: &'static str| ActionError::NotAvailable {
        action,
        trigger: trigger.name(),
    };

    Ok(match action {
        Action::Reply { content, ephemeral } => {
            if matches!(trigger, TriggerKind::MemberJoin) {
                return Err(not_available("reply"));
            }
            CompiledAction::Reply {
                content: check_content(content)?,
                ephemeral: *ephemeral && matches!(trigger, TriggerKind::Slash),
            }
        }
        Action::SendMessage {
            channel_id,
            content,
        } => CompiledAction::SendMessage {
            channel_id: parse_id(channel_id)?,
            content: check_content(content)?,
        },
        Action::AddRole { role_id } => CompiledAction::AddRole {
            role_id: parse_id(role_id)?,
        },
        Action::RemoveRole { role_id } => CompiledAction::RemoveRole {
            role_id: parse_id(role_id)?,
        },
        Action::React { emoji } => {
            if !matches!(trigger, TriggerKind::Message) {
                return Err(not_available("react"));
            }
            CompiledAction::React {
                emoji: parse_emoji(emoji)?,
            }
        }
    })
}

fn check_name(name: &str) -> Result<(), HandlerError> {
    let valid = !name.is_empty()
        && name.len() <= MAX_NAME_LEN
        && name
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-' || b == b'_');
    if valid {
        Ok(())
    } else {
        Err(HandlerError::InvalidName(name.to_owned()))
    }
}

fn check_description(description: &str) -> Result<(), HandlerError> {
    let length = description.chars().count();
    if length == 0 || length > MAX_DESCRIPTION_LEN || description.trim().is_empty() {
        Err(HandlerError::InvalidDescription)
    } else {
        Ok(())
    }
}

fn check_options(options: &[CommandOption]) -> Result<(), HandlerError> {
    if options.len() > MAX_OPTIONS {
        return Err(HandlerError::TooManyOptions);
    }
    let mut seen = HashSet::with_capacity(options.len());
    let mut optional_seen = false;
    for option in options {
        check_name(&option.name)?;
        check_description(&option.description)?;
        if !seen.insert(option.name.as_str()) {
            return Err(HandlerError::DuplicateOption(option.name.clone()));
        }
        if option.required && optional_seen {
            return Err(HandlerError::RequiredAfterOptional(option.name.clone()));
        }
        optional_seen |= !option.required;
    }
    Ok(())
}

fn check_content(content: &str) -> Result<String, ActionError> {
    let length = content.chars().count();
    if length == 0 || length > MAX_MESSAGE_LEN || content.trim().is_empty() {
        Err(ActionError::InvalidContent)
    } else {
        Ok(content.to_owned())
    }
}

fn parse_id(value: &str) -> Result<u64, ActionError> {
    let trimmed = value.trim();
    let valid_shape =
        (17..=20).contains(&trimmed.len()) && trimmed.bytes().all(|b| b.is_ascii_digit());
    match trimmed.parse::<u64>() {
        Ok(id) if valid_shape && id != 0 => Ok(id),
        _ => Err(ActionError::InvalidId(value.to_owned())),
    }
}

/// Accepts `😀`, `name:id`, `<:name:id>` and `<a:name:id>`.
fn parse_emoji(value: &str) -> Result<EmojiRef, ActionError> {
    let invalid = || ActionError::InvalidEmoji(value.to_owned());
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(invalid());
    }

    if trimmed.contains(':') {
        let inner = trimmed
            .strip_prefix('<')
            .and_then(|rest| rest.strip_suffix('>'))
            .map_or(trimmed, |inner| inner.strip_prefix("a:").unwrap_or(inner));
        let inner = inner.strip_prefix(':').unwrap_or(inner);
        let (name, id) = inner.split_once(':').ok_or_else(invalid)?;
        let name_valid = (2..=32).contains(&name.len())
            && name.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_');
        if !name_valid {
            return Err(invalid());
        }
        let id = parse_id(id).map_err(|_| invalid())?;
        return Ok(EmojiRef::Custom {
            name: name.to_owned(),
            id,
        });
    }

    let looks_like_text = trimmed
        .chars()
        .any(|c| c.is_ascii_alphanumeric() || c.is_whitespace());
    if looks_like_text || trimmed.len() > 64 {
        return Err(invalid());
    }
    Ok(EmojiRef::Unicode(trimmed.to_owned()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::OptionKind;

    fn slash(name: &str, actions: Vec<Action>) -> Handler {
        Handler {
            trigger: Trigger::SlashCommand {
                name: name.to_owned(),
                description: "A command".to_owned(),
                options: Vec::new(),
            },
            actions,
        }
    }

    fn reply(content: &str) -> Action {
        Action::Reply {
            content: content.to_owned(),
            ephemeral: false,
        }
    }

    #[test]
    fn compiles_a_valid_program() {
        let program = Program {
            handlers: vec![
                slash("hello", vec![reply("Hi {user}!")]),
                Handler {
                    trigger: Trigger::MemberJoin {},
                    actions: vec![Action::AddRole {
                        role_id: "123456789012345678".to_owned(),
                    }],
                },
            ],
        };
        let compiled = program.compile().expect("valid program");
        assert_eq!(compiled.commands().len(), 1);
        assert_eq!(
            compiled.gateway_needs(),
            GatewayNeeds {
                members: true,
                messages: false
            }
        );
    }

    #[test]
    fn rejects_invalid_and_duplicate_names() {
        let program = Program {
            handlers: vec![slash("Hello", vec![reply("hi")])],
        };
        assert_eq!(
            program.compile(),
            Err(CompileError::Handler {
                handler: 0,
                reason: HandlerError::InvalidName("Hello".to_owned()),
            })
        );

        let program = Program {
            handlers: vec![slash("hi", vec![reply("a")]), slash("hi", vec![reply("b")])],
        };
        assert_eq!(
            program.compile(),
            Err(CompileError::Handler {
                handler: 1,
                reason: HandlerError::DuplicateCommand("hi".to_owned()),
            })
        );
    }

    #[test]
    fn rejects_actions_that_do_not_fit_the_trigger() {
        let program = Program {
            handlers: vec![Handler {
                trigger: Trigger::MemberJoin {},
                actions: vec![reply("welcome")],
            }],
        };
        assert!(matches!(
            program.compile(),
            Err(CompileError::Handler {
                reason: HandlerError::Action {
                    reason: ActionError::NotAvailable { .. },
                    ..
                },
                ..
            })
        ));
    }

    #[test]
    fn rejects_required_option_after_optional_one() {
        let option = |name: &str, required| CommandOption {
            name: name.to_owned(),
            description: "An option".to_owned(),
            kind: OptionKind::String,
            required,
        };
        let program = Program {
            handlers: vec![Handler {
                trigger: Trigger::SlashCommand {
                    name: "say".to_owned(),
                    description: "Say something".to_owned(),
                    options: vec![option("a", false), option("b", true)],
                },
                actions: vec![reply("{option.a}")],
            }],
        };
        assert_eq!(
            program.compile(),
            Err(CompileError::Handler {
                handler: 0,
                reason: HandlerError::RequiredAfterOptional("b".to_owned()),
            })
        );
    }

    #[test]
    fn parses_ids_and_emojis() {
        assert_eq!(parse_id("123456789012345678"), Ok(123_456_789_012_345_678));
        assert!(parse_id("12").is_err());
        assert!(parse_id("abc").is_err());

        assert_eq!(parse_emoji("👍"), Ok(EmojiRef::Unicode("👍".to_owned())));
        let custom = EmojiRef::Custom {
            name: "party".to_owned(),
            id: 123_456_789_012_345_678,
        };
        assert_eq!(parse_emoji("party:123456789012345678"), Ok(custom.clone()));
        assert_eq!(
            parse_emoji("<:party:123456789012345678>"),
            Ok(custom.clone())
        );
        assert_eq!(parse_emoji("<a:party:123456789012345678>"), Ok(custom));
        assert!(parse_emoji("thumbs up").is_err());
    }
}
