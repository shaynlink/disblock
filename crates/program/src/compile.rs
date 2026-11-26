use std::collections::{HashMap, HashSet};

use thiserror::Error;

use crate::model::{
    CommandOption, CompareOp, Expr, Handler, MathOp, Program, Scope, Statement, Trigger,
};
use crate::run::EmojiRef;
use crate::template::MAX_MESSAGE_LEN;

const MAX_HANDLERS: usize = 200;
/// Discord allows 100 global chat input commands per application.
const MAX_COMMANDS: usize = 100;
const MAX_OPTIONS: usize = 25;
/// Statements in one handler, branches included.
const MAX_STEPS: usize = 100;
/// Nesting of `if` blocks.
const MAX_NESTING: usize = 8;
/// Nesting of values inside values.
const MAX_EXPR_DEPTH: usize = 16;
const MAX_PATTERN_LEN: usize = 200;
const MAX_NAME_LEN: usize = 32;
const MAX_DESCRIPTION_LEN: usize = 100;
pub(crate) const MAX_WAIT_SECONDS: u32 = 300;

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
    #[error("a trigger needs at least one action")]
    NoActions,
    #[error("a trigger can have at most {MAX_STEPS} steps")]
    TooManySteps,
    #[error("`if` blocks can be nested at most {MAX_NESTING} times")]
    TooDeep,
    #[error("{path}: {reason}")]
    Step { path: String, reason: StepError },
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum StepError {
    #[error("messages must be 1-{MAX_MESSAGE_LEN} characters")]
    InvalidContent,
    #[error("texts can be at most {MAX_MESSAGE_LEN} characters")]
    TextTooLong,
    #[error("`{0}` is not a valid Discord ID")]
    InvalidId(String),
    #[error("`{0}` is not a unicode emoji or a custom emoji written `name:id`")]
    InvalidEmoji(String),
    #[error("`{action}` cannot be used with a `{trigger}` trigger")]
    NotAvailable {
        action: &'static str,
        trigger: &'static str,
    },
    #[error("`{0}` is not a valid variable name: use 1-32 lowercase letters, digits or `_`")]
    InvalidVariableName(String),
    #[error("this command has no option named `{0}`")]
    UnknownOption(String),
    #[error("waits must last 1 to {MAX_WAIT_SECONDS} seconds")]
    InvalidWait,
    #[error("values can be nested at most {MAX_EXPR_DEPTH} times")]
    ExpressionTooDeep,
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

/// A checked statement: IDs parsed, names validated.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Step {
    Reply {
        content: String,
        ephemeral: bool,
    },
    SendMessage {
        channel_id: u64,
        content: String,
    },
    AddRole(u64),
    RemoveRole(u64),
    React(EmojiRef),
    If {
        condition: Calc,
        then_steps: Vec<Step>,
        else_steps: Vec<Step>,
    },
    Set {
        scope: Scope,
        name: String,
        value: Calc,
    },
    Change {
        scope: Scope,
        name: String,
        by: Calc,
    },
    Wait(u32),
    Stop,
}

/// A checked expression.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Calc {
    Text(String),
    Number(i64),
    Bool(bool),
    Variable(Scope, String),
    Option(String),
    Random(Box<Calc>, Box<Calc>),
    Math(MathOp, Box<Calc>, Box<Calc>),
    Compare(CompareOp, Box<Calc>, Box<Calc>),
    And(Box<Calc>, Box<Calc>),
    Or(Box<Calc>, Box<Calc>),
    Not(Box<Calc>),
    HasRole(u64),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CompiledHandler {
    pub(crate) label: String,
    pub(crate) steps: Vec<Step>,
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
    pub(crate) persistent: bool,
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

    /// Whether some handler reads or writes member or server variables, which
    /// the runtime then loads before running it.
    #[must_use]
    pub fn uses_saved_variables(&self) -> bool {
        self.persistent
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
    /// ID, emoji or variable name, or a step that cannot run with its trigger.
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
    let (kind, label, options) = match &handler.trigger {
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
            (TriggerKind::Slash, format!("/{name}"), options.as_slice())
        }
        Trigger::MemberJoin {} => {
            compiled.member_join.push(index);
            (
                TriggerKind::MemberJoin,
                "member join".to_owned(),
                [].as_slice(),
            )
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
                [].as_slice(),
            )
        }
    };

    if handler.actions.is_empty() {
        return Err(HandlerError::NoActions);
    }
    let mut checker = Checker {
        trigger: kind,
        options,
        steps: 0,
        persistent: false,
    };
    let steps = checker.block(&handler.actions, &mut Vec::new(), 0)?;
    compiled.persistent |= checker.persistent;
    compiled.handlers.push(CompiledHandler { label, steps });
    Ok(())
}

/// Walks one handler's statements, keeping track of where it is for errors.
struct Checker<'a> {
    trigger: TriggerKind,
    options: &'a [CommandOption],
    steps: usize,
    persistent: bool,
}

impl Checker<'_> {
    fn block(
        &mut self,
        statements: &[Statement],
        path: &mut Vec<String>,
        nesting: usize,
    ) -> Result<Vec<Step>, HandlerError> {
        if nesting > MAX_NESTING {
            return Err(HandlerError::TooDeep);
        }
        let mut steps = Vec::with_capacity(statements.len());
        for (index, statement) in statements.iter().enumerate() {
            self.steps += 1;
            if self.steps > MAX_STEPS {
                return Err(HandlerError::TooManySteps);
            }
            path.push(format!("step {}", index + 1));
            steps.push(self.statement(statement, path, nesting)?);
            path.pop();
        }
        Ok(steps)
    }

    fn statement(
        &mut self,
        statement: &Statement,
        path: &mut Vec<String>,
        nesting: usize,
    ) -> Result<Step, HandlerError> {
        let at = |path: &[String], reason: StepError| HandlerError::Step {
            path: path.join(" › "),
            reason,
        };
        let trigger = self.trigger;
        let not_available = |action: &'static str| StepError::NotAvailable {
            action,
            trigger: trigger.name(),
        };

        Ok(match statement {
            Statement::Reply { content, ephemeral } => {
                if matches!(trigger, TriggerKind::MemberJoin) {
                    return Err(at(path, not_available("reply")));
                }
                Step::Reply {
                    content: self.content(content).map_err(|reason| at(path, reason))?,
                    ephemeral: *ephemeral && matches!(trigger, TriggerKind::Slash),
                }
            }
            Statement::SendMessage {
                channel_id,
                content,
            } => Step::SendMessage {
                channel_id: parse_id(channel_id).map_err(|reason| at(path, reason))?,
                content: self.content(content).map_err(|reason| at(path, reason))?,
            },
            Statement::AddRole { role_id } => {
                Step::AddRole(parse_id(role_id).map_err(|reason| at(path, reason))?)
            }
            Statement::RemoveRole { role_id } => {
                Step::RemoveRole(parse_id(role_id).map_err(|reason| at(path, reason))?)
            }
            Statement::React { emoji } => {
                if !matches!(trigger, TriggerKind::Message) {
                    return Err(at(path, not_available("react")));
                }
                Step::React(parse_emoji(emoji).map_err(|reason| at(path, reason))?)
            }
            Statement::If {
                condition,
                then_branch,
                else_branch,
            } => {
                let condition = self.expr(condition, 0).map_err(|reason| at(path, reason))?;
                path.push("then".to_owned());
                let then_steps = self.block(then_branch, path, nesting + 1)?;
                path.pop();
                path.push("else".to_owned());
                let else_steps = self.block(else_branch, path, nesting + 1)?;
                path.pop();
                Step::If {
                    condition,
                    then_steps,
                    else_steps,
                }
            }
            Statement::SetVariable { scope, name, value } => Step::Set {
                scope: *scope,
                name: self
                    .variable(*scope, name)
                    .map_err(|reason| at(path, reason))?,
                value: self.expr(value, 0).map_err(|reason| at(path, reason))?,
            },
            Statement::ChangeVariable { scope, name, by } => Step::Change {
                scope: *scope,
                name: self
                    .variable(*scope, name)
                    .map_err(|reason| at(path, reason))?,
                by: self.expr(by, 0).map_err(|reason| at(path, reason))?,
            },
            Statement::Wait { seconds } => {
                if *seconds == 0 || *seconds > MAX_WAIT_SECONDS {
                    return Err(at(path, StepError::InvalidWait));
                }
                Step::Wait(*seconds)
            }
            Statement::Stop {} => Step::Stop,
        })
    }

    fn expr(&mut self, expr: &Expr, depth: usize) -> Result<Calc, StepError> {
        if depth > MAX_EXPR_DEPTH {
            return Err(StepError::ExpressionTooDeep);
        }
        let next = depth + 1;

        Ok(match expr {
            Expr::Text { value } => {
                if value.chars().count() > MAX_MESSAGE_LEN {
                    return Err(StepError::TextTooLong);
                }
                self.note_template(value);
                Calc::Text(value.clone())
            }
            Expr::Number { value } => Calc::Number(*value),
            Expr::Boolean { value } => Calc::Bool(*value),
            Expr::Variable { scope, name } => Calc::Variable(*scope, self.variable(*scope, name)?),
            Expr::Option { name } => {
                if !matches!(self.trigger, TriggerKind::Slash) {
                    return Err(StepError::NotAvailable {
                        action: "option",
                        trigger: self.trigger.name(),
                    });
                }
                if !self.options.iter().any(|option| &option.name == name) {
                    return Err(StepError::UnknownOption(name.clone()));
                }
                Calc::Option(name.clone())
            }
            Expr::Random { min, max } => {
                Calc::Random(self.boxed(min, next)?, self.boxed(max, next)?)
            }
            Expr::Math { op, left, right } => {
                Calc::Math(*op, self.boxed(left, next)?, self.boxed(right, next)?)
            }
            Expr::Compare { op, left, right } => {
                Calc::Compare(*op, self.boxed(left, next)?, self.boxed(right, next)?)
            }
            Expr::And { left, right } => {
                Calc::And(self.boxed(left, next)?, self.boxed(right, next)?)
            }
            Expr::Or { left, right } => Calc::Or(self.boxed(left, next)?, self.boxed(right, next)?),
            Expr::Not { value } => Calc::Not(self.boxed(value, next)?),
            Expr::HasRole { role_id } => Calc::HasRole(parse_id(role_id)?),
        })
    }

    fn boxed(&mut self, expr: &Expr, depth: usize) -> Result<Box<Calc>, StepError> {
        self.expr(expr, depth).map(Box::new)
    }

    fn content(&mut self, content: &str) -> Result<String, StepError> {
        let length = content.chars().count();
        if length == 0 || length > MAX_MESSAGE_LEN || content.trim().is_empty() {
            return Err(StepError::InvalidContent);
        }
        self.note_template(content);
        Ok(content.to_owned())
    }

    fn variable(&mut self, scope: Scope, name: &str) -> Result<String, StepError> {
        let valid = !name.is_empty()
            && name.len() <= MAX_NAME_LEN
            && name
                .bytes()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_');
        if !valid {
            return Err(StepError::InvalidVariableName(name.to_owned()));
        }
        self.persistent |= scope != Scope::Local;
        Ok(name.to_owned())
    }

    fn note_template(&mut self, template: &str) {
        self.persistent |= template.contains("{member:") || template.contains("{server:");
    }
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

fn parse_id(value: &str) -> Result<u64, StepError> {
    let trimmed = value.trim();
    let valid_shape =
        (17..=20).contains(&trimmed.len()) && trimmed.bytes().all(|b| b.is_ascii_digit());
    match trimmed.parse::<u64>() {
        Ok(id) if valid_shape && id != 0 => Ok(id),
        _ => Err(StepError::InvalidId(value.to_owned())),
    }
}

/// Accepts `😀`, `name:id`, `<:name:id>` and `<a:name:id>`.
fn parse_emoji(value: &str) -> Result<EmojiRef, StepError> {
    let invalid = || StepError::InvalidEmoji(value.to_owned());
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

    fn slash(name: &str, actions: Vec<Statement>) -> Handler {
        Handler {
            trigger: Trigger::SlashCommand {
                name: name.to_owned(),
                description: "A command".to_owned(),
                options: Vec::new(),
            },
            actions,
        }
    }

    fn reply(content: &str) -> Statement {
        Statement::Reply {
            content: content.to_owned(),
            ephemeral: false,
        }
    }

    fn program(json: &str) -> Result<CompiledProgram, CompileError> {
        serde_json::from_str::<Program>(json)
            .expect("valid json")
            .compile()
    }

    #[test]
    fn compiles_a_valid_program() {
        let program = Program {
            handlers: vec![
                slash("hello", vec![reply("Hi {user}!")]),
                Handler {
                    trigger: Trigger::MemberJoin {},
                    actions: vec![Statement::AddRole {
                        role_id: "123456789012345678".to_owned(),
                    }],
                },
            ],
        };
        let compiled = program.compile().expect("valid program");
        assert_eq!(compiled.commands().len(), 1);
        assert!(!compiled.uses_saved_variables());
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
    fn locates_errors_inside_branches() {
        let error = program(
            r#"{"handlers":[{"trigger":{"type":"member_join"},"actions":[
                {"type":"wait","seconds":1},
                {"type":"if","condition":{"type":"boolean","value":true},
                 "then":[{"type":"reply","content":"welcome"}]}
            ]}]}"#,
        )
        .expect_err("reply is not available on member join");
        assert_eq!(
            error.to_string(),
            "trigger #1: step 2 › then › step 1: `reply` cannot be used with a `member join` trigger"
        );
    }

    #[test]
    fn checks_variables_options_and_waits() {
        let bad_variable = program(
            r#"{"handlers":[{"trigger":{"type":"member_join"},"actions":[
                {"type":"set_variable","scope":"member","name":"Bad Name","value":{"type":"number","value":1}}
            ]}]}"#,
        );
        assert!(matches!(
            bad_variable,
            Err(CompileError::Handler {
                reason: HandlerError::Step {
                    reason: StepError::InvalidVariableName(_),
                    ..
                },
                ..
            })
        ));

        let unknown_option = program(
            r#"{"handlers":[{"trigger":{"type":"slash_command","name":"say","description":"Say"},"actions":[
                {"type":"set_variable","scope":"local","name":"x","value":{"type":"option","name":"text"}}
            ]}]}"#,
        );
        assert!(matches!(
            unknown_option,
            Err(CompileError::Handler {
                reason: HandlerError::Step {
                    reason: StepError::UnknownOption(_),
                    ..
                },
                ..
            })
        ));

        let long_wait = program(
            r#"{"handlers":[{"trigger":{"type":"member_join"},"actions":[{"type":"wait","seconds":301}]}]}"#,
        );
        assert!(matches!(
            long_wait,
            Err(CompileError::Handler {
                reason: HandlerError::Step {
                    reason: StepError::InvalidWait,
                    ..
                },
                ..
            })
        ));
    }

    #[test]
    fn detects_saved_variables_in_steps_and_templates() {
        let in_step = program(
            r#"{"handlers":[{"trigger":{"type":"member_join"},"actions":[
                {"type":"change_variable","scope":"server","name":"joins","by":{"type":"number","value":1}}
            ]}]}"#,
        )
        .expect("valid");
        assert!(in_step.uses_saved_variables());

        let in_template = program(
            r#"{"handlers":[{"trigger":{"type":"slash_command","name":"xp","description":"XP"},"actions":[
                {"type":"reply","content":"You have {member:xp} XP"}
            ]}]}"#,
        )
        .expect("valid");
        assert!(in_template.uses_saved_variables());
    }

    #[test]
    fn limits_nesting() {
        let mut statement = reply("deep");
        for _ in 0..=MAX_NESTING {
            statement = Statement::If {
                condition: Expr::Boolean { value: true },
                then_branch: vec![statement],
                else_branch: Vec::new(),
            };
        }
        let program = Program {
            handlers: vec![slash("deep", vec![statement])],
        };
        assert!(matches!(
            program.compile(),
            Err(CompileError::Handler {
                reason: HandlerError::TooDeep,
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
