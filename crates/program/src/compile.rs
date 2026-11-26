use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::model::{
    ButtonSpec, ButtonStyle, CommandOption, CompareOp, ComponentRow, EmbedSpec, Expr, Handler,
    MathOp, Program, Scope, SelectOptionSpec, Statement, Trigger,
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
/// Discord limits communication timeouts to 28 days.
pub(crate) const MAX_TIMEOUT_SECONDS: u32 = 28 * 24 * 60 * 60;
const MAX_MEMBER_ID_TEMPLATE_LEN: usize = 100;
const MAX_AUDIT_REASON_LEN: usize = 512;
const MAX_CUSTOM_ID_LEN: usize = 100;
const MAX_EMBEDS: usize = 10;
pub(crate) const MAX_EMBED_TOTAL_LEN: usize = 6000;
pub(crate) const MAX_EMBED_TITLE_LEN: usize = 256;
pub(crate) const MAX_EMBED_DESCRIPTION_LEN: usize = 4096;
pub(crate) const MAX_EMBED_AUTHOR_NAME_LEN: usize = 256;
pub(crate) const MAX_EMBED_FOOTER_TEXT_LEN: usize = 2048;
pub(crate) const MAX_EMBED_FIELD_NAME_LEN: usize = 256;
pub(crate) const MAX_EMBED_FIELD_VALUE_LEN: usize = 1024;
const MAX_EMBED_FIELDS: usize = 25;
const MAX_URL_LEN: usize = 2048;
const MAX_BUTTON_URL_LEN: usize = 512;
const MAX_COMPONENT_ROWS: usize = 5;
const MAX_ROW_BUTTONS: usize = 5;
pub(crate) const MAX_BUTTON_LABEL_LEN: usize = 80;
pub(crate) const MAX_SELECT_PLACEHOLDER_LEN: usize = 150;
const MAX_SELECT_OPTIONS: usize = 25;
pub(crate) const MAX_SELECT_OPTION_LABEL_LEN: usize = 100;
const MAX_SELECT_OPTION_VALUE_LEN: usize = 100;
pub(crate) const MAX_SELECT_OPTION_DESCRIPTION_LEN: usize = 100;

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

#[derive(Debug, PartialEq, Eq, Error)]
#[error("{error}")]
pub struct CompileFailure {
    pub error: CompileError,
    pub location: Box<ErrorLocation>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ErrorLocation {
    pub handler: usize,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub option: Option<usize>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub steps: Vec<StepRef>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub expr: Vec<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub embed: Option<EmbedLocation>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub component: Option<ComponentLocation>,
}

impl ErrorLocation {
    #[must_use]
    pub const fn handler(handler: usize) -> Self {
        Self {
            handler,
            option: None,
            steps: Vec::new(),
            expr: Vec::new(),
            embed: None,
            component: None,
        }
    }

    #[must_use]
    pub const fn option(handler: usize, option: usize) -> Self {
        Self {
            handler,
            option: Some(option),
            steps: Vec::new(),
            expr: Vec::new(),
            embed: None,
            component: None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Branch {
    Then,
    Else,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct StepRef {
    pub index: usize,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub branch: Option<Branch>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EmbedLocation {
    pub index: usize,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub part: Option<EmbedPart>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum EmbedPart {
    TitleUrl,
    Author,
    Footer,
    Thumbnail,
    Image,
    Field { index: usize },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ComponentLocation {
    pub row: usize,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub button: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub option: Option<usize>,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum HandlerError {
    #[error("`{0}` is not a valid name: use 1-32 lowercase letters, digits, `-` or `_`")]
    InvalidName(String),
    #[error("command `/{0}` is defined twice")]
    DuplicateCommand(String),
    #[error("`{0}` is not a valid component ID: use 1-{MAX_CUSTOM_ID_LEN} characters")]
    InvalidComponentId(String),
    #[error("button `{0}` is defined twice")]
    DuplicateButton(String),
    #[error("select menu `{0}` is defined twice")]
    DuplicateSelectMenu(String),
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
    #[error("messages must have text or an embed")]
    EmptyMessage,
    #[error("an update needs text, an embed, or at least one component")]
    EmptyUpdate,
    #[error("messages can be at most {MAX_MESSAGE_LEN} characters")]
    ContentTooLong,
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
    #[error("timeouts must last 1 to {MAX_TIMEOUT_SECONDS} seconds")]
    InvalidTimeout,
    #[error("values can be nested at most {MAX_EXPR_DEPTH} times")]
    ExpressionTooDeep,
    #[error("a message can have at most {MAX_EMBEDS} embeds")]
    TooManyEmbeds,
    #[error("an embed needs a title, description, author, footer, image, or field")]
    EmptyEmbed,
    #[error("embeds in one message can total at most {MAX_EMBED_TOTAL_LEN} characters")]
    EmbedTooLarge,
    #[error("{field} cannot be empty")]
    FieldRequired { field: &'static str },
    #[error("{field} can be at most {max} characters")]
    FieldTooLong { field: &'static str, max: usize },
    #[error("an embed can have at most {MAX_EMBED_FIELDS} fields")]
    TooManyEmbedFields,
    #[error("`{0}` is not a valid hex color like `#5865F2`")]
    InvalidColor(String),
    #[error("`{0}` is not a valid http:// or https:// URL")]
    InvalidUrl(String),
    #[error("a message can have at most {MAX_COMPONENT_ROWS} component rows")]
    TooManyComponentRows,
    #[error("a button row must have 1 to {MAX_ROW_BUTTONS} buttons")]
    InvalidRowButtons,
    #[error("`{0}` is not a valid component ID: use 1-{MAX_CUSTOM_ID_LEN} characters")]
    InvalidComponentId(String),
    #[error("component ID `{0}` is used more than once in this message")]
    DuplicateComponentId(String),
    #[error("a button needs a label or an emoji")]
    EmptyButton,
    #[error("a select menu needs 1 to {MAX_SELECT_OPTIONS} options")]
    InvalidSelectOptionCount,
    #[error("select menu min/max choices must be between 1 and {options} (got {min}..{max})")]
    InvalidSelectBounds { min: u8, max: u8, options: usize },
    #[error("select option value `{0}` is used twice in this menu")]
    DuplicateSelectValue(String),
    #[error("more select options are marked default than the max choices allowed")]
    TooManyDefaultOptions,
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
        message: CompiledMessage,
        ephemeral: bool,
    },
    SendMessage {
        channel_id: u64,
        message: CompiledMessage,
    },
    UpdateMessage {
        message: CompiledMessage,
    },
    AddRole(u64),
    RemoveRole(u64),
    Timeout {
        member_id: String,
        seconds: u32,
        reason: Option<String>,
    },
    Kick {
        member_id: String,
        reason: Option<String>,
    },
    Ban {
        member_id: String,
        reason: Option<String>,
    },
    DeleteMessage,
    DirectMessage {
        member_id: String,
        content: String,
    },
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CompiledMessage {
    pub(crate) content: Option<String>,
    pub(crate) embeds: Vec<CompiledEmbed>,
    pub(crate) components: Vec<CompiledComponentRow>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CompiledEmbed {
    pub(crate) title: Option<String>,
    pub(crate) url: Option<String>,
    pub(crate) description: Option<String>,
    pub(crate) color: Option<u32>,
    pub(crate) author: Option<CompiledEmbedAuthor>,
    pub(crate) footer: Option<CompiledEmbedFooter>,
    pub(crate) thumbnail_url: Option<String>,
    pub(crate) image_url: Option<String>,
    pub(crate) fields: Vec<CompiledEmbedField>,
    pub(crate) timestamp: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CompiledEmbedAuthor {
    pub(crate) name: String,
    pub(crate) url: Option<String>,
    pub(crate) icon_url: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CompiledEmbedFooter {
    pub(crate) text: String,
    pub(crate) icon_url: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CompiledEmbedField {
    pub(crate) name: String,
    pub(crate) value: String,
    pub(crate) inline: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum CompiledComponentRow {
    Buttons(Vec<CompiledButton>),
    SelectMenu {
        id: String,
        placeholder: Option<String>,
        min_values: u8,
        max_values: u8,
        disabled: bool,
        options: Vec<CompiledSelectOption>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum CompiledButton {
    Action {
        id: String,
        label: Option<String>,
        emoji: Option<EmojiRef>,
        style: ButtonStyle,
        disabled: bool,
    },
    Link {
        url: String,
        label: Option<String>,
        emoji: Option<EmojiRef>,
        disabled: bool,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CompiledSelectOption {
    pub(crate) label: String,
    pub(crate) value: String,
    pub(crate) description: Option<String>,
    pub(crate) emoji: Option<EmojiRef>,
    pub(crate) default: bool,
}

/// A checked expression.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Calc {
    Text(String),
    Number(i64),
    Bool(bool),
    Variable(Scope, String),
    Option(String),
    Selected,
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
    pub(crate) buttons: HashMap<String, usize>,
    pub(crate) selects: HashMap<String, usize>,
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
    ButtonClick,
    SelectMenu,
}

impl TriggerKind {
    const fn name(self) -> &'static str {
        match self {
            Self::Slash => "slash command",
            Self::MemberJoin => "member join",
            Self::Message => "message contains",
            Self::ButtonClick => "button click",
            Self::SelectMenu => "select menu",
        }
    }

    const fn is_interaction(self) -> bool {
        matches!(self, Self::Slash | Self::ButtonClick | Self::SelectMenu)
    }

    const fn is_component(self) -> bool {
        matches!(self, Self::ButtonClick | Self::SelectMenu)
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
        self.compile_diagnosed().map_err(|failure| failure.error)
    }

    /// Check the program and return both the error and the block path when invalid.
    ///
    /// # Errors
    ///
    /// Returns [`CompileFailure`] for the first broken rule.
    pub fn compile_diagnosed(&self) -> Result<CompiledProgram, CompileFailure> {
        if self.handlers.len() > MAX_HANDLERS {
            return Err(CompileFailure {
                error: CompileError::TooManyHandlers,
                location: Box::new(ErrorLocation::handler(MAX_HANDLERS)),
            });
        }

        let mut compiled = CompiledProgram::default();
        let mut overflow_command = None;
        for (index, handler) in self.handlers.iter().enumerate() {
            if matches!(handler.trigger, Trigger::SlashCommand { .. })
                && compiled.commands.len() == MAX_COMMANDS
                && overflow_command.is_none()
            {
                overflow_command = Some(index);
            }
            compile_handler(&mut compiled, index, handler)?;
        }

        if compiled.commands.len() > MAX_COMMANDS {
            return Err(CompileFailure {
                error: CompileError::TooManyCommands,
                location: Box::new(ErrorLocation::handler(
                    overflow_command.unwrap_or(MAX_COMMANDS),
                )),
            });
        }
        Ok(compiled)
    }
}

fn handler_failure(handler: usize, option: Option<usize>, reason: HandlerError) -> CompileFailure {
    CompileFailure {
        error: CompileError::Handler { handler, reason },
        location: Box::new(match option {
            Some(opt) => ErrorLocation::option(handler, opt),
            None => ErrorLocation::handler(handler),
        }),
    }
}

fn compile_handler(
    compiled: &mut CompiledProgram,
    index: usize,
    handler: &Handler,
) -> Result<(), CompileFailure> {
    let at_trigger = |reason| handler_failure(index, None, reason);
    let (kind, label, options) = match &handler.trigger {
        Trigger::SlashCommand {
            name,
            description,
            options,
        } => {
            check_name(name).map_err(at_trigger)?;
            check_description(description).map_err(at_trigger)?;
            check_options(options)
                .map_err(|(opt, reason)| handler_failure(index, Some(opt), reason))?;
            if compiled.slash.insert(name.clone(), index).is_some() {
                return Err(at_trigger(HandlerError::DuplicateCommand(name.clone())));
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
                return Err(at_trigger(HandlerError::InvalidPattern));
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
        Trigger::ButtonClick { id } => {
            let id = check_component_id(id)
                .map_err(|()| at_trigger(HandlerError::InvalidComponentId(id.clone())))?;
            if compiled.buttons.insert(id.to_owned(), index).is_some() {
                return Err(at_trigger(HandlerError::DuplicateButton(id.to_owned())));
            }
            (
                TriggerKind::ButtonClick,
                format!("button \"{id}\""),
                [].as_slice(),
            )
        }
        Trigger::SelectMenu { id } => {
            let id = check_component_id(id)
                .map_err(|()| at_trigger(HandlerError::InvalidComponentId(id.clone())))?;
            if compiled.selects.insert(id.to_owned(), index).is_some() {
                return Err(at_trigger(HandlerError::DuplicateSelectMenu(id.to_owned())));
            }
            (
                TriggerKind::SelectMenu,
                format!("select menu \"{id}\""),
                [].as_slice(),
            )
        }
    };

    if handler.actions.is_empty() {
        return Err(at_trigger(HandlerError::NoActions));
    }
    let mut checker = Checker {
        handler: index,
        trigger: kind,
        options,
        steps: 0,
        persistent: false,
        step_refs: Vec::new(),
        expr_path: Vec::new(),
        embed_loc: None,
        component_loc: None,
    };
    let steps = checker.block(&handler.actions, 0)?;
    compiled.persistent |= checker.persistent;
    compiled.handlers.push(CompiledHandler { label, steps });
    Ok(())
}

/// Walks one handler's statements, keeping track of where it is for errors.
struct Checker<'a> {
    handler: usize,
    trigger: TriggerKind,
    options: &'a [CommandOption],
    steps: usize,
    persistent: bool,
    step_refs: Vec<StepRef>,
    expr_path: Vec<usize>,
    embed_loc: Option<EmbedLocation>,
    component_loc: Option<ComponentLocation>,
}

fn format_steps(refs: &[StepRef]) -> String {
    let mut parts = Vec::with_capacity(refs.len() * 2);
    for step in refs {
        parts.push(format!("step {}", step.index + 1));
        if let Some(branch) = step.branch {
            parts.push(match branch {
                Branch::Then => "then".to_owned(),
                Branch::Else => "else".to_owned(),
            });
        }
    }
    parts.join(" › ")
}

impl Checker<'_> {
    fn fail_handler(&self, reason: HandlerError) -> CompileFailure {
        CompileFailure {
            error: CompileError::Handler {
                handler: self.handler,
                reason,
            },
            location: Box::new(ErrorLocation {
                handler: self.handler,
                option: None,
                steps: self.step_refs.clone(),
                expr: Vec::new(),
                embed: None,
                component: None,
            }),
        }
    }

    fn fail(&self, reason: StepError) -> CompileFailure {
        CompileFailure {
            error: CompileError::Handler {
                handler: self.handler,
                reason: HandlerError::Step {
                    path: format_steps(&self.step_refs),
                    reason,
                },
            },
            location: Box::new(ErrorLocation {
                handler: self.handler,
                option: None,
                steps: self.step_refs.clone(),
                expr: self.expr_path.clone(),
                embed: self.embed_loc.clone(),
                component: self.component_loc,
            }),
        }
    }

    fn block(
        &mut self,
        statements: &[Statement],
        nesting: usize,
    ) -> Result<Vec<Step>, CompileFailure> {
        if nesting > MAX_NESTING {
            return Err(self.fail_handler(HandlerError::TooDeep));
        }
        let mut steps = Vec::with_capacity(statements.len());
        for (index, statement) in statements.iter().enumerate() {
            self.step_refs.push(StepRef {
                index,
                branch: None,
            });
            self.steps += 1;
            if self.steps > MAX_STEPS {
                return Err(self.fail_handler(HandlerError::TooManySteps));
            }
            steps.push(self.statement(statement, nesting)?);
            self.step_refs.pop();
        }
        Ok(steps)
    }

    fn statement(&mut self, statement: &Statement, nesting: usize) -> Result<Step, CompileFailure> {
        let trigger = self.trigger;
        let not_available = |action: &'static str| StepError::NotAvailable {
            action,
            trigger: trigger.name(),
        };

        Ok(match statement {
            Statement::Reply {
                content,
                ephemeral,
                embeds,
                components,
            } => {
                if matches!(trigger, TriggerKind::MemberJoin) {
                    return Err(self.fail(not_available("reply")));
                }
                let message = self
                    .message(content, embeds, components, true)
                    .map_err(|reason| self.fail(reason))?;
                Step::Reply {
                    message,
                    ephemeral: *ephemeral && trigger.is_interaction(),
                }
            }
            Statement::SendMessage {
                channel_id,
                content,
                embeds,
                components,
            } => Step::SendMessage {
                channel_id: parse_id(channel_id).map_err(|reason| self.fail(reason))?,
                message: self
                    .message(content, embeds, components, true)
                    .map_err(|reason| self.fail(reason))?,
            },
            Statement::UpdateMessage {
                content,
                embeds,
                components,
            } => {
                if !trigger.is_component() {
                    return Err(self.fail(not_available("update message")));
                }
                Step::UpdateMessage {
                    message: self
                        .message(content, embeds, components, false)
                        .map_err(|reason| self.fail(reason))?,
                }
            }
            Statement::AddRole { role_id } => {
                Step::AddRole(parse_id(role_id).map_err(|reason| self.fail(reason))?)
            }
            Statement::RemoveRole { role_id } => {
                Step::RemoveRole(parse_id(role_id).map_err(|reason| self.fail(reason))?)
            }
            Statement::Timeout {
                member_id,
                seconds,
                reason,
            } => {
                if *seconds == 0 || *seconds > MAX_TIMEOUT_SECONDS {
                    return Err(self.fail(StepError::InvalidTimeout));
                }
                Step::Timeout {
                    member_id: self
                        .member_id(member_id)
                        .map_err(|reason| self.fail(reason))?,
                    seconds: *seconds,
                    reason: self
                        .optional_text(Some(reason), MAX_AUDIT_REASON_LEN, "moderation reason")
                        .map_err(|reason| self.fail(reason))?,
                }
            }
            Statement::Kick { member_id, reason } => Step::Kick {
                member_id: self
                    .member_id(member_id)
                    .map_err(|reason| self.fail(reason))?,
                reason: self
                    .optional_text(Some(reason), MAX_AUDIT_REASON_LEN, "moderation reason")
                    .map_err(|reason| self.fail(reason))?,
            },
            Statement::Ban { member_id, reason } => Step::Ban {
                member_id: self
                    .member_id(member_id)
                    .map_err(|reason| self.fail(reason))?,
                reason: self
                    .optional_text(Some(reason), MAX_AUDIT_REASON_LEN, "moderation reason")
                    .map_err(|reason| self.fail(reason))?,
            },
            Statement::DeleteMessage {} => {
                if !matches!(trigger, TriggerKind::Message) {
                    return Err(self.fail(not_available("delete message")));
                }
                Step::DeleteMessage
            }
            Statement::DirectMessage { member_id, content } => Step::DirectMessage {
                member_id: self
                    .member_id(member_id)
                    .map_err(|reason| self.fail(reason))?,
                content: self
                    .required_text(content, MAX_MESSAGE_LEN, "direct message")
                    .map_err(|reason| self.fail(reason))?,
            },
            Statement::React { emoji } => {
                if !matches!(trigger, TriggerKind::Message) {
                    return Err(self.fail(not_available("react")));
                }
                Step::React(parse_emoji(emoji).map_err(|reason| self.fail(reason))?)
            }
            Statement::If {
                condition,
                then_branch,
                else_branch,
            } => self.if_statement(condition, then_branch, else_branch, nesting)?,
            Statement::SetVariable { scope, name, value } => Step::Set {
                scope: *scope,
                name: self.check_var(*scope, name)?,
                value: self.check_expr(value)?,
            },
            Statement::ChangeVariable { scope, name, by } => Step::Change {
                scope: *scope,
                name: self.check_var(*scope, name)?,
                by: self.check_expr(by)?,
            },
            Statement::Wait { seconds } => {
                if *seconds == 0 || *seconds > MAX_WAIT_SECONDS {
                    return Err(self.fail(StepError::InvalidWait));
                }
                Step::Wait(*seconds)
            }
            Statement::Stop {} => Step::Stop,
        })
    }

    fn check_var(&mut self, scope: Scope, name: &str) -> Result<String, CompileFailure> {
        self.variable(scope, name)
            .map_err(|reason| self.fail(reason))
    }

    fn check_expr(&mut self, expr: &Expr) -> Result<Calc, CompileFailure> {
        self.in_expr(0, |this| this.expr(expr, 0))
            .map_err(|reason| self.fail(reason))
    }

    fn if_statement(
        &mut self,
        condition: &Expr,
        then_branch: &[Statement],
        else_branch: &[Statement],
        nesting: usize,
    ) -> Result<Step, CompileFailure> {
        let condition = self.check_expr(condition)?;
        let then_steps = self.sub_block(Branch::Then, then_branch, nesting)?;
        let else_steps = self.sub_block(Branch::Else, else_branch, nesting)?;
        Ok(Step::If {
            condition,
            then_steps,
            else_steps,
        })
    }

    fn sub_block(
        &mut self,
        branch: Branch,
        statements: &[Statement],
        nesting: usize,
    ) -> Result<Vec<Step>, CompileFailure> {
        if let Some(current) = self.step_refs.last_mut() {
            current.branch = Some(branch);
        }
        let steps = self.block(statements, nesting + 1)?;
        if let Some(current) = self.step_refs.last_mut() {
            current.branch = None;
        }
        Ok(steps)
    }

    fn in_expr<T>(
        &mut self,
        slot: usize,
        f: impl FnOnce(&mut Self) -> Result<T, StepError>,
    ) -> Result<T, StepError> {
        self.expr_path.push(slot);
        let result = f(self);
        if result.is_ok() {
            self.expr_path.pop();
        }
        result
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
            Expr::Selected {} => {
                if !matches!(self.trigger, TriggerKind::SelectMenu) {
                    return Err(StepError::NotAvailable {
                        action: "selected value",
                        trigger: self.trigger.name(),
                    });
                }
                Calc::Selected
            }
            Expr::Random { min, max } => {
                Calc::Random(self.boxed(0, min, next)?, self.boxed(1, max, next)?)
            }
            Expr::Math { op, left, right } => {
                Calc::Math(*op, self.boxed(0, left, next)?, self.boxed(1, right, next)?)
            }
            Expr::Compare { op, left, right } => {
                Calc::Compare(*op, self.boxed(0, left, next)?, self.boxed(1, right, next)?)
            }
            Expr::And { left, right } => {
                Calc::And(self.boxed(0, left, next)?, self.boxed(1, right, next)?)
            }
            Expr::Or { left, right } => {
                Calc::Or(self.boxed(0, left, next)?, self.boxed(1, right, next)?)
            }
            Expr::Not { value } => Calc::Not(self.boxed(0, value, next)?),
            Expr::HasRole { role_id } => Calc::HasRole(parse_id(role_id)?),
        })
    }

    fn boxed(&mut self, slot: usize, expr: &Expr, depth: usize) -> Result<Box<Calc>, StepError> {
        self.in_expr(slot, |this| this.expr(expr, depth))
            .map(Box::new)
    }

    fn message(
        &mut self,
        content: &str,
        embeds: &[EmbedSpec],
        components: &[ComponentRow],
        require_body: bool,
    ) -> Result<CompiledMessage, StepError> {
        let content = self.optional_content(content)?;
        if embeds.len() > MAX_EMBEDS {
            self.embed_loc = Some(EmbedLocation {
                index: MAX_EMBEDS,
                part: None,
            });
            return Err(StepError::TooManyEmbeds);
        }
        let mut compiled_embeds = Vec::with_capacity(embeds.len());
        let mut total_embed_chars = 0usize;
        for (embed_idx, spec) in embeds.iter().enumerate() {
            let (embed, chars) = self.embed(embed_idx, spec)?;
            total_embed_chars += chars;
            if total_embed_chars > MAX_EMBED_TOTAL_LEN {
                self.embed_loc = Some(EmbedLocation {
                    index: embed_idx,
                    part: None,
                });
                return Err(StepError::EmbedTooLarge);
            }
            compiled_embeds.push(embed);
        }
        self.embed_loc = None;

        let compiled_components = self.components(components)?;
        self.component_loc = None;

        if require_body && content.is_none() && compiled_embeds.is_empty() {
            return Err(StepError::EmptyMessage);
        }
        if !require_body
            && content.is_none()
            && compiled_embeds.is_empty()
            && compiled_components.is_empty()
        {
            return Err(StepError::EmptyUpdate);
        }

        Ok(CompiledMessage {
            content,
            embeds: compiled_embeds,
            components: compiled_components,
        })
    }

    fn optional_content(&mut self, content: &str) -> Result<Option<String>, StepError> {
        if content.trim().is_empty() {
            return Ok(None);
        }
        if content.chars().count() > MAX_MESSAGE_LEN {
            return Err(StepError::ContentTooLong);
        }
        self.note_template(content);
        Ok(Some(content.to_owned()))
    }

    fn optional_text(
        &mut self,
        value: Option<&str>,
        max: usize,
        field: &'static str,
    ) -> Result<Option<String>, StepError> {
        let Some(value) = value else {
            return Ok(None);
        };
        if value.trim().is_empty() {
            return Ok(None);
        }
        if value.chars().count() > max {
            return Err(StepError::FieldTooLong { field, max });
        }
        self.note_template(value);
        Ok(Some(value.to_owned()))
    }

    fn required_text(
        &mut self,
        value: &str,
        max: usize,
        field: &'static str,
    ) -> Result<String, StepError> {
        if value.trim().is_empty() {
            return Err(StepError::FieldRequired { field });
        }
        if value.chars().count() > max {
            return Err(StepError::FieldTooLong { field, max });
        }
        self.note_template(value);
        Ok(value.to_owned())
    }

    fn member_id(&mut self, value: &str) -> Result<String, StepError> {
        let value = value.trim();
        if value.is_empty() || value.chars().count() > MAX_MEMBER_ID_TEMPLATE_LEN {
            return Err(StepError::InvalidId(value.to_owned()));
        }
        if !value.contains(['{', '}']) {
            parse_id(value)?;
        }
        self.note_template(value);
        Ok(value.to_owned())
    }

    fn set_embed_part(&mut self, index: usize, part: Option<EmbedPart>) {
        self.embed_loc = Some(EmbedLocation { index, part });
    }

    fn embed(
        &mut self,
        embed_idx: usize,
        spec: &EmbedSpec,
    ) -> Result<(CompiledEmbed, usize), StepError> {
        self.set_embed_part(embed_idx, None);
        let title =
            self.optional_text(spec.title.as_deref(), MAX_EMBED_TITLE_LEN, "embed title")?;
        self.set_embed_part(embed_idx, Some(EmbedPart::TitleUrl));
        let url = spec
            .url
            .as_deref()
            .map(|raw| parse_url(raw, MAX_URL_LEN))
            .transpose()?;
        self.set_embed_part(embed_idx, None);
        let description = self.optional_text(
            spec.description.as_deref(),
            MAX_EMBED_DESCRIPTION_LEN,
            "embed description",
        )?;
        let color = spec
            .color
            .as_deref()
            .map(parse_color)
            .transpose()?
            .flatten();
        self.set_embed_part(embed_idx, Some(EmbedPart::Author));
        let author = match &spec.author {
            Some(author) => Some(CompiledEmbedAuthor {
                name: self.required_text(
                    &author.name,
                    MAX_EMBED_AUTHOR_NAME_LEN,
                    "embed author",
                )?,
                url: optional_url(author.url.as_deref(), MAX_URL_LEN)?,
                icon_url: optional_url(author.icon_url.as_deref(), MAX_URL_LEN)?,
            }),
            None => None,
        };
        self.set_embed_part(embed_idx, Some(EmbedPart::Footer));
        let footer = match &spec.footer {
            Some(footer) => Some(CompiledEmbedFooter {
                text: self.required_text(
                    &footer.text,
                    MAX_EMBED_FOOTER_TEXT_LEN,
                    "embed footer",
                )?,
                icon_url: optional_url(footer.icon_url.as_deref(), MAX_URL_LEN)?,
            }),
            None => None,
        };
        self.set_embed_part(embed_idx, Some(EmbedPart::Thumbnail));
        let thumbnail_url = spec
            .thumbnail_url
            .as_deref()
            .map(|raw| parse_url(raw, MAX_URL_LEN))
            .transpose()?;
        self.set_embed_part(embed_idx, Some(EmbedPart::Image));
        let image_url = spec
            .image_url
            .as_deref()
            .map(|raw| parse_url(raw, MAX_URL_LEN))
            .transpose()?;

        if spec.fields.len() > MAX_EMBED_FIELDS {
            self.set_embed_part(
                embed_idx,
                Some(EmbedPart::Field {
                    index: MAX_EMBED_FIELDS,
                }),
            );
            return Err(StepError::TooManyEmbedFields);
        }
        let mut fields = Vec::with_capacity(spec.fields.len());
        for (field_idx, field) in spec.fields.iter().enumerate() {
            self.set_embed_part(embed_idx, Some(EmbedPart::Field { index: field_idx }));
            fields.push(CompiledEmbedField {
                name: self.required_text(
                    &field.name,
                    MAX_EMBED_FIELD_NAME_LEN,
                    "embed field name",
                )?,
                value: self.required_text(
                    &field.value,
                    MAX_EMBED_FIELD_VALUE_LEN,
                    "embed field value",
                )?,
                inline: field.inline,
            });
        }

        self.set_embed_part(embed_idx, None);
        let has_content = title.is_some()
            || description.is_some()
            || author.is_some()
            || footer.is_some()
            || thumbnail_url.is_some()
            || image_url.is_some()
            || !fields.is_empty();
        if !has_content {
            return Err(StepError::EmptyEmbed);
        }

        let compiled = CompiledEmbed {
            title,
            url,
            description,
            color,
            author,
            footer,
            thumbnail_url,
            image_url,
            fields,
            timestamp: spec.timestamp,
        };
        let chars = embed_chars(&compiled);
        if chars > MAX_EMBED_TOTAL_LEN {
            return Err(StepError::EmbedTooLarge);
        }
        Ok((compiled, chars))
    }

    fn components(
        &mut self,
        rows: &[ComponentRow],
    ) -> Result<Vec<CompiledComponentRow>, StepError> {
        if rows.len() > MAX_COMPONENT_ROWS {
            self.component_loc = Some(ComponentLocation {
                row: MAX_COMPONENT_ROWS,
                button: None,
                option: None,
            });
            return Err(StepError::TooManyComponentRows);
        }
        let mut seen_ids = HashSet::new();
        let mut compiled = Vec::with_capacity(rows.len());
        for (row_idx, row) in rows.iter().enumerate() {
            self.component_loc = Some(ComponentLocation {
                row: row_idx,
                button: None,
                option: None,
            });
            compiled.push(match row {
                ComponentRow::Buttons { buttons } => {
                    if buttons.is_empty() || buttons.len() > MAX_ROW_BUTTONS {
                        return Err(StepError::InvalidRowButtons);
                    }
                    let mut compiled_buttons = Vec::with_capacity(buttons.len());
                    for (btn_idx, button) in buttons.iter().enumerate() {
                        self.component_loc = Some(ComponentLocation {
                            row: row_idx,
                            button: Some(btn_idx),
                            option: None,
                        });
                        compiled_buttons.push(self.button(button, &mut seen_ids)?);
                    }
                    CompiledComponentRow::Buttons(compiled_buttons)
                }
                ComponentRow::SelectMenu {
                    id,
                    placeholder,
                    min_values,
                    max_values,
                    disabled,
                    options,
                } => self.select_menu(
                    row_idx,
                    id,
                    placeholder.as_deref(),
                    *min_values,
                    *max_values,
                    *disabled,
                    options,
                    &mut seen_ids,
                )?,
            });
        }
        Ok(compiled)
    }

    fn button(
        &mut self,
        spec: &ButtonSpec,
        seen_ids: &mut HashSet<String>,
    ) -> Result<CompiledButton, StepError> {
        match spec {
            ButtonSpec::Action {
                id,
                label,
                emoji,
                style,
                disabled,
            } => {
                let id = check_component_id(id)
                    .map_err(|()| StepError::InvalidComponentId(id.clone()))?
                    .to_owned();
                if !seen_ids.insert(id.clone()) {
                    return Err(StepError::DuplicateComponentId(id));
                }
                let label =
                    self.optional_text(label.as_deref(), MAX_BUTTON_LABEL_LEN, "button label")?;
                let emoji = optional_emoji(emoji.as_deref())?;
                if label.is_none() && emoji.is_none() {
                    return Err(StepError::EmptyButton);
                }
                Ok(CompiledButton::Action {
                    id,
                    label,
                    emoji,
                    style: *style,
                    disabled: *disabled,
                })
            }
            ButtonSpec::Link {
                url,
                label,
                emoji,
                disabled,
            } => {
                let url = parse_url(url, MAX_BUTTON_URL_LEN)?;
                let label =
                    self.optional_text(label.as_deref(), MAX_BUTTON_LABEL_LEN, "button label")?;
                let emoji = optional_emoji(emoji.as_deref())?;
                if label.is_none() && emoji.is_none() {
                    return Err(StepError::EmptyButton);
                }
                Ok(CompiledButton::Link {
                    url,
                    label,
                    emoji,
                    disabled: *disabled,
                })
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn select_menu(
        &mut self,
        row_idx: usize,
        id: &str,
        placeholder: Option<&str>,
        min_values: Option<u8>,
        max_values: Option<u8>,
        disabled: bool,
        options: &[SelectOptionSpec],
        seen_ids: &mut HashSet<String>,
    ) -> Result<CompiledComponentRow, StepError> {
        let id = check_component_id(id)
            .map_err(|()| StepError::InvalidComponentId(id.to_owned()))?
            .to_owned();
        if !seen_ids.insert(id.clone()) {
            return Err(StepError::DuplicateComponentId(id));
        }
        let placeholder = self.optional_text(
            placeholder,
            MAX_SELECT_PLACEHOLDER_LEN,
            "select menu placeholder",
        )?;
        if options.is_empty() || options.len() > MAX_SELECT_OPTIONS {
            return Err(StepError::InvalidSelectOptionCount);
        }
        let min = min_values.unwrap_or(1);
        let max = max_values.unwrap_or(1);
        if min == 0 || max == 0 || min > max || usize::from(max) > options.len() {
            return Err(StepError::InvalidSelectBounds {
                min,
                max,
                options: options.len(),
            });
        }

        let mut seen_values = HashSet::with_capacity(options.len());
        let mut compiled_options = Vec::with_capacity(options.len());
        let mut defaults = 0usize;
        for (opt_idx, option) in options.iter().enumerate() {
            self.component_loc = Some(ComponentLocation {
                row: row_idx,
                button: None,
                option: Some(opt_idx),
            });
            let label = self.required_text(
                &option.label,
                MAX_SELECT_OPTION_LABEL_LEN,
                "select option label",
            )?;
            let value = check_select_value(&option.value)?;
            if !seen_values.insert(value.clone()) {
                return Err(StepError::DuplicateSelectValue(value));
            }
            let description = self.optional_text(
                option.description.as_deref(),
                MAX_SELECT_OPTION_DESCRIPTION_LEN,
                "select option description",
            )?;
            let emoji = optional_emoji(option.emoji.as_deref())?;
            if option.default {
                defaults += 1;
            }
            compiled_options.push(CompiledSelectOption {
                label,
                value,
                description,
                emoji,
                default: option.default,
            });
        }
        self.component_loc = Some(ComponentLocation {
            row: row_idx,
            button: None,
            option: None,
        });
        if defaults > usize::from(max) {
            return Err(StepError::TooManyDefaultOptions);
        }

        Ok(CompiledComponentRow::SelectMenu {
            id,
            placeholder,
            min_values: min,
            max_values: max,
            disabled,
            options: compiled_options,
        })
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

fn check_options(options: &[CommandOption]) -> Result<(), (usize, HandlerError)> {
    if options.len() > MAX_OPTIONS {
        return Err((MAX_OPTIONS, HandlerError::TooManyOptions));
    }
    let mut seen = HashSet::with_capacity(options.len());
    let mut optional_seen = false;
    for (index, option) in options.iter().enumerate() {
        check_name(&option.name).map_err(|reason| (index, reason))?;
        check_description(&option.description).map_err(|reason| (index, reason))?;
        if !seen.insert(option.name.as_str()) {
            return Err((index, HandlerError::DuplicateOption(option.name.clone())));
        }
        if option.required && optional_seen {
            return Err((
                index,
                HandlerError::RequiredAfterOptional(option.name.clone()),
            ));
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

fn embed_chars(embed: &CompiledEmbed) -> usize {
    embed.title.as_deref().map_or(0, |s| s.chars().count())
        + embed
            .description
            .as_deref()
            .map_or(0, |s| s.chars().count())
        + embed.author.as_ref().map_or(0, |a| a.name.chars().count())
        + embed.footer.as_ref().map_or(0, |f| f.text.chars().count())
        + embed
            .fields
            .iter()
            .map(|f| f.name.chars().count() + f.value.chars().count())
            .sum::<usize>()
}

fn check_component_id(id: &str) -> Result<&str, ()> {
    let valid = !id.is_empty()
        && id.chars().count() <= MAX_CUSTOM_ID_LEN
        && !id.trim().is_empty()
        && id == id.trim();
    if valid { Ok(id) } else { Err(()) }
}

fn check_select_value(value: &str) -> Result<String, StepError> {
    if value.trim().is_empty() {
        return Err(StepError::FieldRequired {
            field: "select option value",
        });
    }
    if value.chars().count() > MAX_SELECT_OPTION_VALUE_LEN {
        return Err(StepError::FieldTooLong {
            field: "select option value",
            max: MAX_SELECT_OPTION_VALUE_LEN,
        });
    }
    Ok(value.to_owned())
}

fn parse_color(value: &str) -> Result<Option<u32>, StepError> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }
    let hex = trimmed.strip_prefix('#').unwrap_or(trimmed);
    if hex.len() != 6 || !hex.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err(StepError::InvalidColor(value.to_owned()));
    }
    u32::from_str_radix(hex, 16)
        .map(Some)
        .map_err(|_| StepError::InvalidColor(value.to_owned()))
}

fn optional_url(value: Option<&str>, max_len: usize) -> Result<Option<String>, StepError> {
    let Some(value) = value else {
        return Ok(None);
    };
    if value.trim().is_empty() {
        return Ok(None);
    }
    parse_url(value, max_len).map(Some)
}

fn parse_url(value: &str, max_len: usize) -> Result<String, StepError> {
    let trimmed = value.trim();
    let rest = trimmed
        .strip_prefix("https://")
        .or_else(|| trimmed.strip_prefix("http://"))
        .ok_or_else(|| StepError::InvalidUrl(value.to_owned()))?;
    if rest.is_empty()
        || trimmed.chars().count() > max_len
        || trimmed.chars().any(char::is_whitespace)
    {
        return Err(StepError::InvalidUrl(value.to_owned()));
    }
    Ok(trimmed.to_owned())
}

fn optional_emoji(value: Option<&str>) -> Result<Option<EmojiRef>, StepError> {
    let Some(value) = value else {
        return Ok(None);
    };
    if value.trim().is_empty() {
        return Ok(None);
    }
    parse_emoji(value).map(Some)
}

/// Accepts `😀`, `name:id`, `<:name:id>` and `<a:name:id>`.
fn parse_emoji(value: &str) -> Result<EmojiRef, StepError> {
    let invalid = || StepError::InvalidEmoji(value.to_owned());
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(invalid());
    }

    if trimmed.contains(':') {
        let (animated, inner) = match trimmed
            .strip_prefix('<')
            .and_then(|rest| rest.strip_suffix('>'))
        {
            Some(inner) => match inner.strip_prefix("a:") {
                Some(rest) => (true, rest),
                None => (false, inner),
            },
            None => (false, trimmed),
        };
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
            animated,
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
            embeds: Vec::new(),
            components: Vec::new(),
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

        let bad_timeout = program(
            r#"{"handlers":[{"trigger":{"type":"member_join"},"actions":[
                {"type":"timeout","member_id":"123456789012345678","seconds":2419201}
            ]}]}"#,
        );
        assert!(matches!(
            bad_timeout,
            Err(CompileError::Handler {
                reason: HandlerError::Step {
                    reason: StepError::InvalidTimeout,
                    ..
                },
                ..
            })
        ));

        let delete_from_command = program(
            r#"{"handlers":[{"trigger":{"type":"slash_command","name":"clean","description":"Clean"},"actions":[
                {"type":"delete_message"}
            ]}]}"#,
        );
        assert!(matches!(
            delete_from_command,
            Err(CompileError::Handler {
                reason: HandlerError::Step {
                    reason: StepError::NotAvailable {
                        action: "delete message",
                        ..
                    },
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
            animated: false,
        };
        assert_eq!(parse_emoji("party:123456789012345678"), Ok(custom.clone()));
        assert_eq!(
            parse_emoji("<:party:123456789012345678>"),
            Ok(custom.clone())
        );
        assert_eq!(
            parse_emoji("<a:party:123456789012345678>"),
            Ok(EmojiRef::Custom {
                name: "party".to_owned(),
                id: 123_456_789_012_345_678,
                animated: true,
            })
        );
        assert!(parse_emoji("thumbs up").is_err());
    }

    #[test]
    fn validates_rich_messages_and_component_triggers() {
        let rich = program(
            r##"{"handlers":[
                {"trigger":{"type":"slash_command","name":"panel","description":"Panel"},
                 "actions":[
                    {"type":"reply","content":"","embeds":[{"title":"Rules","description":"Be nice","color":"#5865F2","fields":[{"name":"Rule 1","value":"Respect","inline":true}]}],
                     "components":[
                        {"type":"buttons","buttons":[{"type":"action","id":"accept","label":"Accept","style":"success","emoji":"✅"}]},
                        {"type":"select_menu","id":"color","placeholder":"Pick color","options":[{"label":"Red","value":"red"},{"label":"Blue","value":"blue"}]}
                     ]}
                 ]},
                {"trigger":{"type":"button_click","id":"accept"},"actions":[{"type":"reply","content":"Accepted","ephemeral":true},{"type":"update_message","content":"Updated"}]},
                {"trigger":{"type":"select_menu","id":"color"},"actions":[{"type":"set_variable","scope":"member","name":"color","value":{"type":"selected"}},{"type":"reply","content":"{selected}"}]}
            ]}"##,
        )
        .expect("valid rich program");
        assert!(rich.uses_saved_variables());
        assert_eq!(rich.gateway_needs(), GatewayNeeds::default());

        assert!(program(
            r#"{"handlers":[{"trigger":{"type":"button_click","id":"a"},"actions":[{"type":"reply","content":""}]}]}"#
        ).is_err());

        assert!(program(
            r#"{"handlers":[{"trigger":{"type":"button_click","id":"a"},"actions":[{"type":"reply","content":"x","components":[{"type":"buttons","buttons":[{"type":"action","id":"same","label":"1","style":"primary"},{"type":"action","id":"same","label":"2","style":"secondary"}]}]}]}]}"#
        ).is_err());

        assert!(program(
            r#"{"handlers":[
                {"trigger":{"type":"button_click","id":"a"},"actions":[{"type":"reply","content":"1"}]},
                {"trigger":{"type":"button_click","id":"a"},"actions":[{"type":"reply","content":"2"}]}
            ]}"#
        ).is_err());

        assert!(program(
            r#"{"handlers":[{"trigger":{"type":"slash_command","name":"x","description":"x"},"actions":[{"type":"update_message","content":"no"}]}]}"#
        ).is_err());

        assert!(program(
            r#"{"handlers":[{"trigger":{"type":"button_click","id":"a"},"actions":[{"type":"reply","content":"{selected}","components":[]}]}]}"#
        ).is_ok());

        assert!(program(
            r#"{"handlers":[{"trigger":{"type":"button_click","id":"a"},"actions":[{"type":"set_variable","scope":"local","name":"v","value":{"type":"selected"}}]}]}"#
        ).is_err());
    }

    #[test]
    fn diagnoses_nested_step_and_expression_locations() {
        let nested_expr = serde_json::from_str::<Program>(
            r#"{"handlers":[
                {"trigger":{"type":"slash_command","name":"ok","description":"ok"},"actions":[{"type":"reply","content":"hi"}]},
                {"trigger":{"type":"member_join"},"actions":[
                    {"type":"wait","seconds":1},
                    {"type":"if","condition":{"type":"boolean","value":true},
                     "then":[
                        {"type":"if","condition":{"type":"and","left":{"type":"boolean","value":true},"right":{"type":"has_role","role_id":"bad"}},
                         "then":[{"type":"stop"}]}
                     ]}
                ]}
            ]}"#,
        )
        .unwrap()
        .compile_diagnosed()
        .expect_err("invalid role id in nested expr");
        assert_eq!(
            *nested_expr.location,
            ErrorLocation {
                handler: 1,
                option: None,
                steps: vec![
                    StepRef {
                        index: 1,
                        branch: Some(Branch::Then)
                    },
                    StepRef {
                        index: 0,
                        branch: None
                    },
                ],
                expr: vec![0, 1],
                embed: None,
                component: None,
            }
        );

        let bad_option = serde_json::from_str::<Program>(
            r#"{"handlers":[{"trigger":{"type":"slash_command","name":"p","description":"p",
                "options":[
                    {"name":"first","description":"ok","kind":"string","required":false},
                    {"name":"second","description":"ok","kind":"string","required":true}
                ]},"actions":[{"type":"reply","content":"hi"}]}]}"#,
        )
        .unwrap()
        .compile_diagnosed()
        .expect_err("required after optional");
        assert_eq!(*bad_option.location, ErrorLocation::option(0, 1));
    }

    #[test]
    fn diagnoses_embed_and_component_locations() {
        let embed_part = serde_json::from_str::<Program>(
            r#"{"handlers":[{"trigger":{"type":"slash_command","name":"p","description":"p"},"actions":[
                {"type":"reply","content":"","embeds":[
                    {"title":"A","fields":[{"name":"k","value":"v"},{"name":"","value":"x"}]}
                ]}
            ]}]}"#,
        )
        .unwrap()
        .compile_diagnosed()
        .expect_err("empty field name");
        assert_eq!(
            *embed_part.location,
            ErrorLocation {
                handler: 0,
                option: None,
                steps: vec![StepRef {
                    index: 0,
                    branch: None
                }],
                expr: Vec::new(),
                embed: Some(EmbedLocation {
                    index: 0,
                    part: Some(EmbedPart::Field { index: 1 }),
                }),
                component: None,
            }
        );

        let select_opt = serde_json::from_str::<Program>(
            r#"{"handlers":[{"trigger":{"type":"slash_command","name":"p","description":"p"},"actions":[
                {"type":"reply","content":"pick","components":[
                    {"type":"select_menu","id":"m","options":[{"label":"A","value":"same"},{"label":"B","value":"same"}]}
                ]}
            ]}]}"#,
        )
        .unwrap()
        .compile_diagnosed()
        .expect_err("duplicate option value");
        assert_eq!(
            *select_opt.location,
            ErrorLocation {
                handler: 0,
                option: None,
                steps: vec![StepRef {
                    index: 0,
                    branch: None
                }],
                expr: Vec::new(),
                embed: None,
                component: Some(ComponentLocation {
                    row: 0,
                    button: None,
                    option: Some(1),
                }),
            }
        );
    }
}
