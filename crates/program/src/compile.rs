use std::collections::{BTreeSet, HashMap, HashSet};

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::model::{
    AttachmentSpec, ButtonSpec, ButtonStyle, ChannelFilter, CommandOption, CommandOptionChoiceSpec,
    CompareOp, ComponentRow, EmbedFieldSpec, EmbedSpec, EntitySelectKind, Expr, Handler, MathOp,
    ModalInputSpec, OptionKind, PollSpec, Program, ScheduleUnit, Scope, SelectOptionSpec,
    Statement, TargetField, TextInputStyle, Trigger,
};
use crate::run::EmojiRef;
use crate::template::MAX_MESSAGE_LEN;

const MAX_HANDLERS: usize = 200;
/// Discord allows 100 global chat input commands per application.
const MAX_COMMANDS: usize = 100;
/// Discord allows 5 global user and 5 global message context commands each.
const MAX_CONTEXT_COMMANDS: usize = 5;
const MAX_SCHEDULES: usize = 20;
const MAX_OPTIONS: usize = 25;
const MAX_CHOICES: usize = 25;
const MAX_OPTION_STRING_LENGTH: u16 = 6_000;
const MAX_ATTACHMENT_FILE_TYPES: usize = 10;
const MIN_INTEGER_OPTION_VALUE: i64 = -9_007_199_254_740_991;
const MAX_INTEGER_OPTION_VALUE: i64 = 9_007_199_254_740_991;
const MAX_NUMBER_OPTION_VALUE: f64 = 9_007_199_254_740_992.0;
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
/// Scheduled triggers run at most once every 30 days.
const MAX_SCHEDULE_SECONDS: u32 = 30 * 24 * 60 * 60;
const MAX_ID_TEMPLATE_LEN: usize = 100;
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
const MAX_MODAL_INPUTS: usize = 5;
pub(crate) const MAX_MODAL_TITLE_LEN: usize = 45;
pub(crate) const MAX_MODAL_LABEL_LEN: usize = 45;
pub(crate) const MAX_MODAL_PLACEHOLDER_LEN: usize = 100;
pub(crate) const MAX_MODAL_VALUE_LEN: usize = 4000;
const MAX_POLL_ANSWERS: usize = 10;
pub(crate) const MAX_POLL_QUESTION_LEN: usize = 300;
pub(crate) const MAX_POLL_ANSWER_LEN: usize = 55;
const MAX_POLL_HOURS: u32 = 768;
const MAX_ATTACHMENTS: usize = 10;
const MAX_ATTACHMENT_FILENAME_LEN: usize = 100;
pub(crate) const MAX_ATTACHMENT_CONTENT_LEN: usize = 50_000;
pub(crate) const MAX_THREAD_NAME_LEN: usize = 100;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum CompileError {
    #[error("a bot can have at most {MAX_HANDLERS} triggers")]
    TooManyHandlers,
    #[error("a bot can have at most {MAX_COMMANDS} slash commands")]
    TooManyCommands,
    #[error("a bot can have at most {MAX_CONTEXT_COMMANDS} {0} menu commands")]
    TooManyContextCommands(&'static str),
    #[error("a bot can have at most {MAX_SCHEDULES} scheduled triggers")]
    TooManySchedules,
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
    #[error("menu command name must be 1-32 non-empty characters")]
    InvalidContextName,
    #[error("command `/{0}` is defined twice")]
    DuplicateCommand(String),
    #[error("subcommand `/{0}` is defined twice")]
    DuplicateSubcommand(String),
    #[error("command `/{0}` cannot mix top-level actions and subcommands")]
    MixedSubcommands(String),
    #[error("subcommand group or name `{0}` conflicts on `/{1}`")]
    ConflictingSubcommandGroup(String, String),
    #[error("a slash command can have at most {MAX_OPTIONS} subcommands or groups")]
    TooManySubcommands,
    #[error("{0} menu command `{1}` is defined twice")]
    DuplicateContextCommand(&'static str, String),
    #[error("`{0}` is not a valid component ID: use 1-{MAX_CUSTOM_ID_LEN} characters")]
    InvalidComponentId(String),
    #[error("button `{0}` is defined twice")]
    DuplicateButton(String),
    #[error("select menu `{0}` is defined twice")]
    DuplicateSelectMenu(String),
    #[error("modal `{0}` is defined twice")]
    DuplicateModal(String),
    #[error("descriptions must be 1-100 characters")]
    InvalidDescription,
    #[error("a command can have at most {MAX_OPTIONS} options")]
    TooManyOptions,
    #[error("option `{0}` is defined twice")]
    DuplicateOption(String),
    #[error("required option `{0}` must come before the optional ones")]
    RequiredAfterOptional(String),
    #[error("option `{0}` supports choices only for text, integer, or decimal options")]
    ChoicesUnsupported(String),
    #[error("option `{0}` can have at most {MAX_CHOICES} choices")]
    TooManyChoices(String),
    #[error("option `{0}` cannot use both fixed choices and autocomplete suggestions")]
    ChoicesAndAutocompleteConflict(String),
    #[error("option `{0}` has an invalid choice `{1}`")]
    InvalidChoice(String, String),
    #[error("option `{0}` only supports {1}")]
    UnsupportedOptionConstraint(String, &'static str),
    #[error("option `{0}` has an invalid {1}")]
    InvalidOptionConstraint(String, &'static str),
    #[error("option `{0}` has a minimum greater than its maximum {1}")]
    InvalidOptionConstraintBounds(String, &'static str),
    #[error("option `{0}` can filter at most {MAX_ATTACHMENT_FILE_TYPES} attachment types")]
    TooManyAttachmentFileTypes(String),
    #[error("the searched text must be 1-{MAX_PATTERN_LEN} characters")]
    InvalidPattern,
    #[error("`{0}` is not a unicode emoji or a custom emoji written `name:id`")]
    InvalidEmoji(String),
    #[error("`{0}` is not a valid Discord message ID")]
    InvalidMessageId(String),
    #[error("`{0}` is not a valid AutoMod rule ID")]
    InvalidRuleId(String),
    #[error("scheduled triggers must run every 1 minute to 30 days")]
    InvalidSchedule,
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
    #[error("messages must have text, an embed, a poll, or an attachment")]
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
    #[error("opening a modal must happen before replying, updating, or waiting")]
    ModalNotFirstResponse,
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
    #[error("a modal needs 1 to {MAX_MODAL_INPUTS} text inputs")]
    InvalidModalInputCount,
    #[error("modal text input `{0}` min length ({1}) cannot exceed max length ({2})")]
    InvalidModalInputBounds(String, u16, u16),
    #[error("modal input ID `{0}` is used more than once in this modal")]
    DuplicateModalInputId(String),
    #[error("a poll needs 1 to {MAX_POLL_ANSWERS} answers")]
    InvalidPollAnswerCount,
    #[error("poll duration must be 1 to {MAX_POLL_HOURS} hours")]
    InvalidPollDuration,
    #[error("a message can have at most {MAX_ATTACHMENTS} attachments")]
    TooManyAttachments,
    #[error("`{0}` is not a valid attachment filename")]
    InvalidFilename(String),
    #[error("auto-archive duration must be 60, 1440, 4320, or 10080 minutes")]
    InvalidArchiveDuration,
    #[error("bulk delete count must be between 2 and 100")]
    InvalidBulkDeleteCount,
}

/// A slash command to register on Discord.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SlashCommandSpec {
    pub name: String,
    pub description: String,
    pub options: Vec<CommandOption>,
    pub subcommands: Vec<SlashSubcommandSpec>,
    pub groups: Vec<SlashSubcommandGroupSpec>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SlashSubcommandSpec {
    pub name: String,
    pub description: String,
    pub options: Vec<CommandOption>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SlashSubcommandGroupSpec {
    pub name: String,
    pub description: String,
    pub subcommands: Vec<SlashSubcommandSpec>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContextCommandKind {
    User,
    Message,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContextCommandSpec {
    pub name: String,
    pub kind: ContextCommandKind,
}

/// A scheduled trigger to tick periodically while the bot is online.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ScheduleSpec {
    pub interval: u32,
    pub unit: ScheduleUnit,
}

impl ScheduleSpec {
    /// Duration between runs in seconds.
    #[must_use]
    pub fn seconds(self) -> u64 {
        let mult: u64 = match self.unit {
            ScheduleUnit::Minutes => 60,
            ScheduleUnit::Hours => 3_600,
            ScheduleUnit::Days => 86_400,
        };
        u64::from(self.interval) * mult
    }
}

/// What the gateway connection must subscribe to for a program to work.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct GatewayNeeds {
    /// Member join, leave, and update events (privileged `GUILD_MEMBERS` intent).
    pub members: bool,
    /// Message content triggers (privileged `MESSAGE_CONTENT` intent).
    pub messages: bool,
    /// Non-content message events like message delete (`GUILD_MESSAGES` intent).
    pub message_events: bool,
    /// Message reaction add and remove events (`GUILD_MESSAGE_REACTIONS` intent).
    pub reactions: bool,
    /// Ban add and remove events (`GUILD_MODERATION` intent).
    pub moderation: bool,
    /// Invite create events (`GUILD_INVITES` intent).
    pub invites: bool,
    /// Voice state update events (`GUILD_VOICE_STATES` intent).
    pub voice_states: bool,
    /// AutoMod execution events (`AUTO_MODERATION_EXECUTION` intent).
    pub automod: bool,
    /// Poll vote add events (`GUILD_MESSAGE_POLLS` intent).
    pub polls: bool,
}

/// A checked statement: IDs parsed, names validated.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Step {
    Reply {
        message: CompiledMessage,
        ephemeral: bool,
    },
    SendMessage {
        channel_id: String,
        message: CompiledMessage,
        reply_to_message_id: Option<String>,
    },
    UpdateMessage {
        message: CompiledMessage,
    },
    ShowModal {
        id: String,
        title: String,
        inputs: Vec<CompiledModalInput>,
    },
    CreateThreadFromMessage {
        name: String,
        auto_archive_minutes: Option<u16>,
    },
    CreateThread {
        channel_id: String,
        name: String,
        private: bool,
        auto_archive_minutes: Option<u16>,
    },
    CreateForumPost {
        channel_id: String,
        title: String,
        content: Option<String>,
        embeds: Vec<CompiledEmbed>,
    },
    JoinThread {
        thread_id: String,
    },
    AddThreadMember {
        thread_id: String,
        member_id: String,
    },
    RemoveThreadMember {
        thread_id: String,
        member_id: String,
    },
    PinMessage {
        channel_id: String,
        message_id: String,
    },
    UnpinMessage {
        channel_id: String,
        message_id: String,
    },
    PublishMessage {
        channel_id: String,
        message_id: String,
    },
    BulkDeleteMessages {
        channel_id: String,
        count: u8,
    },
    AddRole(String),
    RemoveRole(String),
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
    pub(crate) poll: Option<CompiledPoll>,
    pub(crate) attachments: Vec<CompiledAttachment>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CompiledPoll {
    pub(crate) question: String,
    pub(crate) answers: Vec<CompiledPollAnswer>,
    pub(crate) duration_hours: u32,
    pub(crate) allow_multiselect: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CompiledPollAnswer {
    pub(crate) text: String,
    pub(crate) emoji: Option<EmojiRef>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CompiledAttachment {
    pub(crate) filename: String,
    pub(crate) content: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CompiledModalInput {
    pub(crate) id: String,
    pub(crate) label: String,
    pub(crate) style: TextInputStyle,
    pub(crate) placeholder: Option<String>,
    pub(crate) required: bool,
    pub(crate) min_length: Option<u16>,
    pub(crate) max_length: Option<u16>,
    pub(crate) value: Option<String>,
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
    EntitySelectMenu {
        id: String,
        kind: EntitySelectKind,
        placeholder: Option<String>,
        min_values: u8,
        max_values: u8,
        disabled: bool,
        channel_types: Vec<ChannelFilter>,
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
    ModalField(String),
    Target(TargetField),
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ReactionMatcher {
    pub(crate) emoji: Option<EmojiRef>,
    pub(crate) message_id: Option<u64>,
    pub(crate) handler: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct AutoModMatcher {
    pub(crate) rule_id: Option<u64>,
    pub(crate) handler: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ScheduleEntry {
    pub(crate) spec: ScheduleSpec,
    pub(crate) handler: usize,
}

/// A program whose every value has been checked against Discord's limits.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CompiledProgram {
    pub(crate) handlers: Vec<CompiledHandler>,
    pub(crate) commands: Vec<SlashCommandSpec>,
    pub(crate) context_commands: Vec<ContextCommandSpec>,
    pub(crate) slash: HashMap<String, usize>,
    pub(crate) subcommands: HashMap<(String, Option<String>, String), usize>,
    pub(crate) user_commands: HashMap<String, usize>,
    pub(crate) message_commands: HashMap<String, usize>,
    pub(crate) modals: HashMap<String, usize>,
    pub(crate) member_join: Vec<usize>,
    pub(crate) member_leave: Vec<usize>,
    pub(crate) member_update: Vec<usize>,
    pub(crate) member_ban: Vec<usize>,
    pub(crate) member_unban: Vec<usize>,
    pub(crate) messages: Vec<MessageMatcher>,
    pub(crate) message_edit: Vec<usize>,
    pub(crate) message_delete: Vec<usize>,
    pub(crate) reaction_add: Vec<ReactionMatcher>,
    pub(crate) reaction_remove: Vec<ReactionMatcher>,
    pub(crate) thread_create: Vec<usize>,
    pub(crate) invite_create: Vec<usize>,
    pub(crate) automod_execution: Vec<AutoModMatcher>,
    pub(crate) poll_vote_add: Vec<usize>,
    pub(crate) voice_state_update: Vec<usize>,
    pub(crate) buttons: HashMap<String, usize>,
    pub(crate) selects: HashMap<String, usize>,
    pub(crate) schedules: Vec<ScheduleEntry>,
    /// Member and server variables the program reads or writes.
    pub(crate) saved: BTreeSet<(Scope, String)>,
}

impl CompiledProgram {
    /// Slash commands to register, in the order they appear in the program.
    #[must_use]
    pub fn commands(&self) -> &[SlashCommandSpec] {
        &self.commands
    }

    /// Context menu commands (user and message) to register, in program order.
    #[must_use]
    pub fn context_commands(&self) -> &[ContextCommandSpec] {
        &self.context_commands
    }

    /// Find autocomplete suggestions configured for an option on a slash command or subcommand.
    #[must_use]
    pub fn autocomplete_choices(
        &self,
        command: &str,
        group: Option<&str>,
        subcommand: Option<&str>,
        option_name: &str,
    ) -> &[String] {
        let Some(cmd) = self.commands.iter().find(|spec| spec.name == command) else {
            return &[];
        };
        let options = match (group, subcommand) {
            (None, None) => cmd.options.as_slice(),
            (None, Some(sub)) => cmd
                .subcommands
                .iter()
                .find(|item| item.name == sub)
                .map_or(&[][..], |item| item.options.as_slice()),
            (Some(grp), Some(sub)) => cmd
                .groups
                .iter()
                .find(|item| item.name == grp)
                .and_then(|item| item.subcommands.iter().find(|entry| entry.name == sub))
                .map_or(&[][..], |entry| entry.options.as_slice()),
            (Some(_), None) => &[],
        };
        options
            .iter()
            .find(|option| option.name == option_name)
            .map_or(&[][..], |option| option.autocomplete.as_slice())
    }

    /// Distinct scheduled intervals needed by the program, in handler order.
    #[must_use]
    pub fn schedules(&self) -> Vec<ScheduleSpec> {
        let mut distinct = Vec::new();
        for entry in &self.schedules {
            if !distinct.contains(&entry.spec) {
                distinct.push(entry.spec);
            }
        }
        distinct
    }

    #[must_use]
    pub fn gateway_needs(&self) -> GatewayNeeds {
        GatewayNeeds {
            members: !self.member_join.is_empty()
                || !self.member_leave.is_empty()
                || !self.member_update.is_empty(),
            messages: !self.messages.is_empty() || !self.message_edit.is_empty(),
            message_events: !self.message_delete.is_empty(),
            reactions: !self.reaction_add.is_empty() || !self.reaction_remove.is_empty(),
            moderation: !self.member_ban.is_empty() || !self.member_unban.is_empty(),
            invites: !self.invite_create.is_empty(),
            voice_states: !self.voice_state_update.is_empty(),
            automod: !self.automod_execution.is_empty(),
            polls: !self.poll_vote_add.is_empty(),
        }
    }

    /// Whether some handler reads or writes member or server variables, which
    /// the runtime then loads before running it.
    #[must_use]
    pub fn uses_saved_variables(&self) -> bool {
        !self.saved.is_empty()
    }

    /// Distinct member and server variables, which plans may cap.
    #[must_use]
    pub fn saved_variable_count(&self) -> usize {
        self.saved.len()
    }
}

#[derive(Clone, Copy)]
enum TriggerKind {
    Slash,
    SlashSubcommand,
    UserCommand,
    MessageCommand,
    ModalSubmit,
    MemberJoin,
    MemberLeave,
    MemberUpdate,
    MemberBan,
    MemberUnban,
    Message,
    MessageEdit,
    MessageDelete,
    ReactionAdd,
    ReactionRemove,
    ThreadCreate,
    InviteCreate,
    AutoModExecution,
    PollVoteAdd,
    VoiceStateUpdate,
    ButtonClick,
    SelectMenu,
    Schedule,
}

impl TriggerKind {
    const fn name(self) -> &'static str {
        match self {
            Self::Slash | Self::SlashSubcommand => "slash command",
            Self::UserCommand => "user menu command",
            Self::MessageCommand => "message menu command",
            Self::ModalSubmit => "modal submit",
            Self::MemberJoin => "member join",
            Self::MemberLeave => "member leave",
            Self::MemberUpdate => "member update",
            Self::MemberBan => "member ban",
            Self::MemberUnban => "member unban",
            Self::Message => "message contains",
            Self::MessageEdit => "message edited",
            Self::MessageDelete => "message deleted",
            Self::ReactionAdd => "reaction added",
            Self::ReactionRemove => "reaction removed",
            Self::ThreadCreate => "thread created",
            Self::InviteCreate => "invite created",
            Self::AutoModExecution => "AutoMod execution",
            Self::PollVoteAdd => "poll vote added",
            Self::VoiceStateUpdate => "voice state update",
            Self::ButtonClick => "button click",
            Self::SelectMenu => "select menu",
            Self::Schedule => "scheduled task",
        }
    }

    const fn is_interaction(self) -> bool {
        matches!(
            self,
            Self::Slash
                | Self::SlashSubcommand
                | Self::UserCommand
                | Self::MessageCommand
                | Self::ModalSubmit
                | Self::ButtonClick
                | Self::SelectMenu
        )
    }

    const fn is_component(self) -> bool {
        matches!(self, Self::ButtonClick | Self::SelectMenu)
    }

    const fn can_open_modal(self) -> bool {
        self.is_interaction() && !matches!(self, Self::ModalSubmit)
    }

    const fn has_message(self) -> bool {
        matches!(
            self,
            Self::Message
                | Self::MessageEdit
                | Self::MessageCommand
                | Self::ReactionAdd
                | Self::ReactionRemove
        )
    }

    const fn has_member(self) -> bool {
        !matches!(
            self,
            Self::MemberLeave | Self::MemberUnban | Self::MessageDelete | Self::Schedule
        )
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
        let mut overflow_schedule = None;
        for (index, handler) in self.handlers.iter().enumerate() {
            if matches!(handler.trigger, Trigger::Schedule { .. })
                && compiled.schedules.len() == MAX_SCHEDULES
                && overflow_schedule.is_none()
            {
                overflow_schedule = Some(index);
            }
            compile_handler(&mut compiled, index, handler)?;
        }

        if compiled.commands.len() > MAX_COMMANDS {
            return Err(CompileFailure {
                error: CompileError::TooManyCommands,
                location: Box::new(ErrorLocation::handler(MAX_COMMANDS)),
            });
        }
        if compiled.user_commands.len() > MAX_CONTEXT_COMMANDS {
            return Err(CompileFailure {
                error: CompileError::TooManyContextCommands("user"),
                location: Box::new(ErrorLocation::handler(MAX_CONTEXT_COMMANDS)),
            });
        }
        if compiled.message_commands.len() > MAX_CONTEXT_COMMANDS {
            return Err(CompileFailure {
                error: CompileError::TooManyContextCommands("message"),
                location: Box::new(ErrorLocation::handler(MAX_CONTEXT_COMMANDS)),
            });
        }
        if compiled.schedules.len() > MAX_SCHEDULES {
            return Err(CompileFailure {
                error: CompileError::TooManySchedules,
                location: Box::new(ErrorLocation::handler(
                    overflow_schedule.unwrap_or(MAX_SCHEDULES),
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
    let (kind, label, options) = compile_trigger(compiled, index, &handler.trigger)?;
    if handler.actions.is_empty() {
        return Err(handler_failure(index, None, HandlerError::NoActions));
    }
    let mut checker = Checker {
        handler: index,
        trigger: kind,
        options,
        steps: 0,
        can_show_modal: kind.can_open_modal(),
        saved: BTreeSet::new(),
        step_refs: Vec::new(),
        expr_path: Vec::new(),
        embed_loc: None,
        component_loc: None,
    };
    let steps = checker.block(&handler.actions, 0)?;
    compiled.saved.extend(checker.saved);
    compiled.handlers.push(CompiledHandler { label, steps });
    Ok(())
}

fn compile_trigger<'a>(
    compiled: &mut CompiledProgram,
    index: usize,
    trigger: &'a Trigger,
) -> Result<(TriggerKind, String, &'a [CommandOption]), CompileFailure> {
    let at_trigger = |reason| handler_failure(index, None, reason);
    Ok(match trigger {
        Trigger::SlashCommand {
            name,
            description,
            options,
        } => {
            compile_slash_trigger(compiled, index, name, description, options)?;
            (TriggerKind::Slash, format!("/{name}"), options.as_slice())
        }
        Trigger::SlashSubcommand {
            command,
            group,
            subcommand,
            description,
            options,
        } => {
            let label = compile_subcommand_trigger(
                compiled,
                index,
                command,
                group.as_deref(),
                subcommand,
                description,
                options,
            )?;
            (TriggerKind::SlashSubcommand, label, options.as_slice())
        }
        Trigger::UserCommand { name } => {
            let trimmed = name.trim();
            if trimmed.is_empty() || trimmed.chars().count() > MAX_NAME_LEN {
                return Err(at_trigger(HandlerError::InvalidContextName));
            }
            if compiled
                .user_commands
                .insert(trimmed.to_owned(), index)
                .is_some()
            {
                return Err(at_trigger(HandlerError::DuplicateContextCommand(
                    "user",
                    trimmed.to_owned(),
                )));
            }
            compiled.context_commands.push(ContextCommandSpec {
                name: trimmed.to_owned(),
                kind: ContextCommandKind::User,
            });
            (
                TriggerKind::UserCommand,
                format!("user menu \"{trimmed}\""),
                &[],
            )
        }
        Trigger::MessageCommand { name } => {
            let trimmed = name.trim();
            if trimmed.is_empty() || trimmed.chars().count() > MAX_NAME_LEN {
                return Err(at_trigger(HandlerError::InvalidContextName));
            }
            if compiled
                .message_commands
                .insert(trimmed.to_owned(), index)
                .is_some()
            {
                return Err(at_trigger(HandlerError::DuplicateContextCommand(
                    "message",
                    trimmed.to_owned(),
                )));
            }
            compiled.context_commands.push(ContextCommandSpec {
                name: trimmed.to_owned(),
                kind: ContextCommandKind::Message,
            });
            (
                TriggerKind::MessageCommand,
                format!("message menu \"{trimmed}\""),
                &[],
            )
        }
        Trigger::ModalSubmit { id } => {
            let id = check_component_id(id)
                .map_err(|()| HandlerError::InvalidComponentId(id.clone()))
                .map_err(at_trigger)?;
            if compiled.modals.insert(id.to_owned(), index).is_some() {
                return Err(at_trigger(HandlerError::DuplicateModal(id.to_owned())));
            }
            (TriggerKind::ModalSubmit, format!("modal \"{id}\""), &[])
        }
        Trigger::MemberJoin {} => {
            compiled.member_join.push(index);
            (TriggerKind::MemberJoin, "member join".to_owned(), &[])
        }
        Trigger::MemberLeave {} => {
            compiled.member_leave.push(index);
            (TriggerKind::MemberLeave, "member leave".to_owned(), &[])
        }
        Trigger::MemberUpdate {} => {
            compiled.member_update.push(index);
            (TriggerKind::MemberUpdate, "member updated".to_owned(), &[])
        }
        Trigger::MemberBan {} => {
            compiled.member_ban.push(index);
            (TriggerKind::MemberBan, "member banned".to_owned(), &[])
        }
        Trigger::MemberUnban {} => {
            compiled.member_unban.push(index);
            (TriggerKind::MemberUnban, "member unbanned".to_owned(), &[])
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
                &[],
            )
        }
        Trigger::MessageEdit {} => {
            compiled.message_edit.push(index);
            (TriggerKind::MessageEdit, "message edited".to_owned(), &[])
        }
        Trigger::MessageDelete {} => {
            compiled.message_delete.push(index);
            (
                TriggerKind::MessageDelete,
                "message deleted".to_owned(),
                &[],
            )
        }
        Trigger::ReactionAdd { emoji, message_id } => {
            let label = push_reaction_handler(
                &mut compiled.reaction_add,
                index,
                "reaction added",
                emoji.as_deref(),
                message_id.as_deref(),
            )
            .map_err(at_trigger)?;
            (TriggerKind::ReactionAdd, label, &[])
        }
        Trigger::ReactionRemove { emoji, message_id } => {
            let label = push_reaction_handler(
                &mut compiled.reaction_remove,
                index,
                "reaction removed",
                emoji.as_deref(),
                message_id.as_deref(),
            )
            .map_err(at_trigger)?;
            (TriggerKind::ReactionRemove, label, &[])
        }
        Trigger::ThreadCreate {} => {
            compiled.thread_create.push(index);
            (TriggerKind::ThreadCreate, "thread created".to_owned(), &[])
        }
        Trigger::InviteCreate {} => {
            compiled.invite_create.push(index);
            (TriggerKind::InviteCreate, "invite created".to_owned(), &[])
        }
        Trigger::AutoModExecution { rule_id } => {
            let parsed_rule = match rule_id.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
                Some(raw) => Some(
                    parse_id(raw)
                        .map_err(|_| at_trigger(HandlerError::InvalidRuleId(raw.to_owned())))?,
                ),
                None => None,
            };
            compiled.automod_execution.push(AutoModMatcher {
                rule_id: parsed_rule,
                handler: index,
            });
            let label = match parsed_rule {
                Some(id) => format!("AutoMod rule {id}"),
                None => "AutoMod execution".to_owned(),
            };
            (TriggerKind::AutoModExecution, label, &[])
        }
        Trigger::PollVoteAdd {} => {
            compiled.poll_vote_add.push(index);
            (TriggerKind::PollVoteAdd, "poll vote added".to_owned(), &[])
        }
        Trigger::VoiceStateUpdate {} => {
            compiled.voice_state_update.push(index);
            (
                TriggerKind::VoiceStateUpdate,
                "voice state update".to_owned(),
                &[],
            )
        }
        Trigger::ButtonClick { id } => {
            let id = compile_component_trigger(&mut compiled.buttons, index, id, true)
                .map_err(at_trigger)?;
            (TriggerKind::ButtonClick, format!("button \"{id}\""), &[])
        }
        Trigger::SelectMenu { id } => {
            let id = compile_component_trigger(&mut compiled.selects, index, id, false)
                .map_err(at_trigger)?;
            (
                TriggerKind::SelectMenu,
                format!("select menu \"{id}\""),
                &[],
            )
        }
        Trigger::Schedule { interval, unit } => {
            let spec = parse_schedule(*interval, *unit).map_err(at_trigger)?;
            compiled.schedules.push(ScheduleEntry {
                spec,
                handler: index,
            });
            (TriggerKind::Schedule, schedule_label(*interval, *unit), &[])
        }
    })
}

fn compile_slash_trigger(
    compiled: &mut CompiledProgram,
    index: usize,
    name: &str,
    description: &str,
    options: &[CommandOption],
) -> Result<(), CompileFailure> {
    let at_trigger = |reason| handler_failure(index, None, reason);
    check_name(name).map_err(at_trigger)?;
    check_description(description).map_err(at_trigger)?;
    check_options(options).map_err(|(opt, reason)| handler_failure(index, Some(opt), reason))?;
    if compiled
        .commands
        .iter()
        .any(|cmd| cmd.name == name && (!cmd.subcommands.is_empty() || !cmd.groups.is_empty()))
    {
        return Err(at_trigger(HandlerError::MixedSubcommands(name.to_owned())));
    }
    if compiled.slash.insert(name.to_owned(), index).is_some() {
        return Err(at_trigger(HandlerError::DuplicateCommand(name.to_owned())));
    }
    compiled.commands.push(SlashCommandSpec {
        name: name.to_owned(),
        description: description.to_owned(),
        options: options.to_vec(),
        subcommands: Vec::new(),
        groups: Vec::new(),
    });
    Ok(())
}

fn compile_subcommand_trigger(
    compiled: &mut CompiledProgram,
    index: usize,
    command: &str,
    group: Option<&str>,
    subcommand: &str,
    description: &str,
    options: &[CommandOption],
) -> Result<String, CompileFailure> {
    let at_trigger = |reason| handler_failure(index, None, reason);
    check_name(command).map_err(at_trigger)?;
    let group = group.map(str::trim).filter(|s| !s.is_empty());
    if let Some(group_name) = group {
        check_name(group_name).map_err(at_trigger)?;
    }
    check_name(subcommand).map_err(at_trigger)?;
    check_description(description).map_err(at_trigger)?;
    check_options(options).map_err(|(opt, reason)| handler_failure(index, Some(opt), reason))?;

    if compiled.slash.contains_key(command) {
        return Err(at_trigger(HandlerError::MixedSubcommands(
            command.to_owned(),
        )));
    }

    let key = (
        command.to_owned(),
        group.map(str::to_owned),
        subcommand.to_owned(),
    );
    let full_label = match group {
        Some(group_name) => format!("/{command} {group_name} {subcommand}"),
        None => format!("/{command} {subcommand}"),
    };
    if compiled.subcommands.insert(key, index).is_some() {
        return Err(at_trigger(HandlerError::DuplicateSubcommand(
            full_label.trim_start_matches('/').to_owned(),
        )));
    }

    let cmd = match compiled.commands.iter_mut().find(|c| c.name == command) {
        Some(existing) => existing,
        None => {
            compiled.commands.push(SlashCommandSpec {
                name: command.to_owned(),
                description: format!("{command} commands"),
                options: Vec::new(),
                subcommands: Vec::new(),
                groups: Vec::new(),
            });
            compiled
                .commands
                .last_mut()
                .expect("just pushed command spec")
        }
    };

    let sub_spec = SlashSubcommandSpec {
        name: subcommand.to_owned(),
        description: description.to_owned(),
        options: options.to_vec(),
    };

    match group {
        None => {
            if cmd.groups.iter().any(|g| g.name == subcommand) {
                return Err(at_trigger(HandlerError::ConflictingSubcommandGroup(
                    subcommand.to_owned(),
                    command.to_owned(),
                )));
            }
            if cmd.subcommands.len() + cmd.groups.len() >= MAX_OPTIONS {
                return Err(at_trigger(HandlerError::TooManySubcommands));
            }
            cmd.subcommands.push(sub_spec);
        }
        Some(group_name) => {
            if cmd.subcommands.iter().any(|s| s.name == group_name) {
                return Err(at_trigger(HandlerError::ConflictingSubcommandGroup(
                    group_name.to_owned(),
                    command.to_owned(),
                )));
            }
            if let Some(existing_group) = cmd.groups.iter_mut().find(|g| g.name == group_name) {
                if existing_group.subcommands.len() >= MAX_OPTIONS {
                    return Err(at_trigger(HandlerError::TooManySubcommands));
                }
                existing_group.subcommands.push(sub_spec);
            } else {
                if cmd.subcommands.len() + cmd.groups.len() >= MAX_OPTIONS {
                    return Err(at_trigger(HandlerError::TooManySubcommands));
                }
                cmd.groups.push(SlashSubcommandGroupSpec {
                    name: group_name.to_owned(),
                    description: format!("{group_name} commands"),
                    subcommands: vec![sub_spec],
                });
            }
        }
    }

    Ok(full_label)
}

fn compile_component_trigger<'a>(
    map: &mut HashMap<String, usize>,
    index: usize,
    id: &'a str,
    is_button: bool,
) -> Result<&'a str, HandlerError> {
    let id =
        check_component_id(id).map_err(|()| HandlerError::InvalidComponentId(id.to_owned()))?;
    if map.insert(id.to_owned(), index).is_some() {
        return Err(if is_button {
            HandlerError::DuplicateButton(id.to_owned())
        } else {
            HandlerError::DuplicateSelectMenu(id.to_owned())
        });
    }
    Ok(id)
}

/// Walks one handler's statements, keeping track of where it is for errors.
struct Checker<'a> {
    handler: usize,
    trigger: TriggerKind,
    options: &'a [CommandOption],
    steps: usize,
    can_show_modal: bool,
    saved: BTreeSet<(Scope, String)>,
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
        Ok(match statement {
            Statement::Reply {
                content,
                ephemeral,
                embeds,
                components,
                poll,
                attachments,
            } => {
                self.can_show_modal = false;
                self.reply_step(
                    content,
                    *ephemeral,
                    embeds,
                    components,
                    poll.as_ref(),
                    attachments,
                )?
            }
            Statement::SendMessage {
                channel_id,
                content,
                embeds,
                components,
                poll,
                attachments,
                reply_to_message_id,
            } => Step::SendMessage {
                channel_id: self.id_or_template(channel_id)?,
                message: self.check_message(
                    content,
                    embeds,
                    components,
                    poll.as_ref(),
                    attachments,
                    true,
                )?,
                reply_to_message_id: reply_to_message_id
                    .as_deref()
                    .map(str::trim)
                    .filter(|s| !s.is_empty())
                    .map(|raw| self.id_or_template(raw))
                    .transpose()?,
            },
            Statement::UpdateMessage {
                content,
                embeds,
                components,
            } => {
                self.require(trigger.is_component(), "update message")?;
                self.can_show_modal = false;
                Step::UpdateMessage {
                    message: self.check_message(content, embeds, components, None, &[], false)?,
                }
            }
            Statement::ShowModal { id, title, inputs } => {
                self.require(trigger.can_open_modal(), "open modal")?;
                if !self.can_show_modal {
                    return Err(self.fail(StepError::ModalNotFirstResponse));
                }
                self.can_show_modal = false;
                self.modal_step(id, title, inputs)?
            }
            Statement::CreateThreadFromMessage {
                name,
                auto_archive_minutes,
            } => {
                self.require(trigger.has_message(), "create thread from message")?;
                Step::CreateThreadFromMessage {
                    name: self
                        .required_text(name, MAX_THREAD_NAME_LEN, "thread name")
                        .map_err(|reason| self.fail(reason))?,
                    auto_archive_minutes: check_archive_duration(*auto_archive_minutes)
                        .map_err(|reason| self.fail(reason))?,
                }
            }
            Statement::CreateThread {
                channel_id,
                name,
                private,
                auto_archive_minutes,
            } => Step::CreateThread {
                channel_id: self.id_or_template(channel_id)?,
                name: self
                    .required_text(name, MAX_THREAD_NAME_LEN, "thread name")
                    .map_err(|reason| self.fail(reason))?,
                private: *private,
                auto_archive_minutes: check_archive_duration(*auto_archive_minutes)
                    .map_err(|reason| self.fail(reason))?,
            },
            Statement::CreateForumPost {
                channel_id,
                title,
                content,
                embeds,
            } => {
                let compiled_embeds = self
                    .compile_embeds(embeds)
                    .map_err(|reason| self.fail(reason))?;
                let content = self
                    .optional_content(content)
                    .map_err(|reason| self.fail(reason))?;
                if content.is_none() && compiled_embeds.is_empty() {
                    return Err(self.fail(StepError::EmptyMessage));
                }
                Step::CreateForumPost {
                    channel_id: self.id_or_template(channel_id)?,
                    title: self
                        .required_text(title, MAX_THREAD_NAME_LEN, "forum post title")
                        .map_err(|reason| self.fail(reason))?,
                    content,
                    embeds: compiled_embeds,
                }
            }
            Statement::JoinThread { thread_id } => Step::JoinThread {
                thread_id: self.id_or_template(thread_id)?,
            },
            Statement::AddThreadMember {
                thread_id,
                member_id,
            } => Step::AddThreadMember {
                thread_id: self.id_or_template(thread_id)?,
                member_id: self
                    .member_id(member_id)
                    .map_err(|reason| self.fail(reason))?,
            },
            Statement::RemoveThreadMember {
                thread_id,
                member_id,
            } => Step::RemoveThreadMember {
                thread_id: self.id_or_template(thread_id)?,
                member_id: self
                    .member_id(member_id)
                    .map_err(|reason| self.fail(reason))?,
            },
            Statement::PinMessage {
                channel_id,
                message_id,
            } => Step::PinMessage {
                channel_id: self.id_or_template(channel_id)?,
                message_id: self.id_or_template(message_id)?,
            },
            Statement::UnpinMessage {
                channel_id,
                message_id,
            } => Step::UnpinMessage {
                channel_id: self.id_or_template(channel_id)?,
                message_id: self.id_or_template(message_id)?,
            },
            Statement::PublishMessage {
                channel_id,
                message_id,
            } => Step::PublishMessage {
                channel_id: self.id_or_template(channel_id)?,
                message_id: self.id_or_template(message_id)?,
            },
            Statement::BulkDeleteMessages { channel_id, count } => {
                if !(2..=100).contains(count) {
                    return Err(self.fail(StepError::InvalidBulkDeleteCount));
                }
                Step::BulkDeleteMessages {
                    channel_id: self.id_or_template(channel_id)?,
                    count: *count,
                }
            }
            Statement::AddRole { role_id } => {
                Step::AddRole(self.member_role_id("add role", role_id)?)
            }
            Statement::RemoveRole { role_id } => {
                Step::RemoveRole(self.member_role_id("remove role", role_id)?)
            }
            Statement::Timeout {
                member_id,
                seconds,
                reason,
            } => self.timeout_step(member_id, *seconds, reason)?,
            Statement::Kick { member_id, reason } => {
                let (member_id, reason) = self.moderation_target("kick", member_id, reason)?;
                Step::Kick { member_id, reason }
            }
            Statement::Ban { member_id, reason } => {
                let (member_id, reason) = self.moderation_target("ban", member_id, reason)?;
                Step::Ban { member_id, reason }
            }
            Statement::DeleteMessage {} => {
                self.require(
                    matches!(trigger, TriggerKind::Message | TriggerKind::MessageEdit),
                    "delete message",
                )?;
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
                self.require(trigger.has_message(), "react")?;
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
                self.can_show_modal = false;
                Step::Wait(*seconds)
            }
            Statement::Stop {} => Step::Stop,
        })
    }

    fn reply_step(
        &mut self,
        content: &str,
        ephemeral: bool,
        embeds: &[EmbedSpec],
        components: &[ComponentRow],
        poll: Option<&PollSpec>,
        attachments: &[AttachmentSpec],
    ) -> Result<Step, CompileFailure> {
        self.require(
            self.trigger.is_interaction() || self.trigger.has_message(),
            "reply",
        )?;
        Ok(Step::Reply {
            message: self.check_message(content, embeds, components, poll, attachments, true)?,
            ephemeral: ephemeral && self.trigger.is_interaction(),
        })
    }

    fn modal_step(
        &mut self,
        id: &str,
        title: &str,
        inputs: &[ModalInputSpec],
    ) -> Result<Step, CompileFailure> {
        let id = check_component_id(id)
            .map_err(|()| self.fail(StepError::InvalidComponentId(id.to_owned())))?
            .to_owned();
        let title = self
            .required_text(title, MAX_MODAL_TITLE_LEN, "modal title")
            .map_err(|reason| self.fail(reason))?;
        if inputs.is_empty() || inputs.len() > MAX_MODAL_INPUTS {
            return Err(self.fail(StepError::InvalidModalInputCount));
        }

        let mut seen_ids = HashSet::with_capacity(inputs.len());
        let mut compiled_inputs = Vec::with_capacity(inputs.len());
        for input in inputs {
            let input_id = check_component_id(&input.id)
                .map_err(|()| self.fail(StepError::InvalidComponentId(input.id.clone())))?
                .to_owned();
            if !seen_ids.insert(input_id.clone()) {
                return Err(self.fail(StepError::DuplicateModalInputId(input_id)));
            }
            let label = self
                .required_text(&input.label, MAX_MODAL_LABEL_LEN, "modal input label")
                .map_err(|reason| self.fail(reason))?;
            let placeholder = self
                .optional_text(
                    input.placeholder.as_deref(),
                    MAX_MODAL_PLACEHOLDER_LEN,
                    "modal input placeholder",
                )
                .map_err(|reason| self.fail(reason))?;
            let value = self
                .optional_text(
                    input.value.as_deref(),
                    MAX_MODAL_VALUE_LEN,
                    "modal input value",
                )
                .map_err(|reason| self.fail(reason))?;
            if let Some(min) = input.min_length
                && usize::from(min) > MAX_MODAL_VALUE_LEN
            {
                return Err(self.fail(StepError::InvalidModalInputBounds(
                    input_id,
                    min,
                    MAX_MODAL_VALUE_LEN as u16,
                )));
            }
            if let Some(max) = input.max_length
                && (max == 0 || usize::from(max) > MAX_MODAL_VALUE_LEN)
            {
                return Err(self.fail(StepError::InvalidModalInputBounds(
                    input_id,
                    input.min_length.unwrap_or(0),
                    max,
                )));
            }
            if let (Some(min), Some(max)) = (input.min_length, input.max_length)
                && min > max
            {
                return Err(self.fail(StepError::InvalidModalInputBounds(input_id, min, max)));
            }
            compiled_inputs.push(CompiledModalInput {
                id: input_id,
                label,
                style: input.style,
                placeholder,
                required: input.required,
                min_length: input.min_length,
                max_length: input.max_length,
                value,
            });
        }
        Ok(Step::ShowModal {
            id,
            title,
            inputs: compiled_inputs,
        })
    }

    fn require(&self, allowed: bool, action: &'static str) -> Result<(), CompileFailure> {
        if allowed {
            Ok(())
        } else {
            Err(self.fail(StepError::NotAvailable {
                action,
                trigger: self.trigger.name(),
            }))
        }
    }

    fn timeout_step(
        &mut self,
        member_id: &str,
        seconds: u32,
        reason: &str,
    ) -> Result<Step, CompileFailure> {
        if seconds == 0 || seconds > MAX_TIMEOUT_SECONDS {
            return Err(self.fail(StepError::InvalidTimeout));
        }
        let (member_id, reason) = self.moderation_target("timeout", member_id, reason)?;
        Ok(Step::Timeout {
            member_id,
            seconds,
            reason,
        })
    }

    fn check_message(
        &mut self,
        content: &str,
        embeds: &[EmbedSpec],
        components: &[ComponentRow],
        poll: Option<&PollSpec>,
        attachments: &[AttachmentSpec],
        require_body: bool,
    ) -> Result<CompiledMessage, CompileFailure> {
        self.message(content, embeds, components, poll, attachments, require_body)
            .map_err(|reason| self.fail(reason))
    }

    fn id_or_template(&mut self, value: &str) -> Result<String, CompileFailure> {
        self.member_id(value).map_err(|reason| self.fail(reason))
    }

    fn member_role_id(
        &mut self,
        action: &'static str,
        role_id: &str,
    ) -> Result<String, CompileFailure> {
        if matches!(
            self.trigger,
            TriggerKind::MemberLeave
                | TriggerKind::MemberUnban
                | TriggerKind::MessageDelete
                | TriggerKind::Schedule
        ) {
            return Err(self.fail(StepError::NotAvailable {
                action,
                trigger: self.trigger.name(),
            }));
        }
        self.id_or_template(role_id)
    }

    fn check_var(&mut self, scope: Scope, name: &str) -> Result<String, CompileFailure> {
        self.variable(scope, name)
            .map_err(|reason| self.fail(reason))
    }

    fn moderation_target(
        &mut self,
        action: &'static str,
        member_id: &str,
        reason: &str,
    ) -> Result<(String, Option<String>), CompileFailure> {
        if matches!(self.trigger, TriggerKind::Schedule) {
            return Err(self.fail(StepError::NotAvailable {
                action,
                trigger: self.trigger.name(),
            }));
        }
        let member_id = self.member_id(member_id).map_err(|err| self.fail(err))?;
        let reason = self
            .optional_text(Some(reason), MAX_AUDIT_REASON_LEN, "moderation reason")
            .map_err(|err| self.fail(err))?;
        Ok((member_id, reason))
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
        let saved_modal_gate = self.can_show_modal;
        let then_steps = self.sub_block(Branch::Then, then_branch, nesting)?;
        let after_then = self.can_show_modal;
        self.can_show_modal = saved_modal_gate;
        let else_steps = self.sub_block(Branch::Else, else_branch, nesting)?;
        self.can_show_modal = after_then && self.can_show_modal;
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
                if !matches!(
                    self.trigger,
                    TriggerKind::Slash | TriggerKind::SlashSubcommand
                ) {
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
            Expr::ModalField { id } => {
                if !matches!(self.trigger, TriggerKind::ModalSubmit) {
                    return Err(StepError::NotAvailable {
                        action: "modal field",
                        trigger: self.trigger.name(),
                    });
                }
                let id = check_component_id(id)
                    .map_err(|()| StepError::InvalidComponentId(id.clone()))?
                    .to_owned();
                Calc::ModalField(id)
            }
            Expr::Target { field } => {
                if !matches!(
                    self.trigger,
                    TriggerKind::UserCommand | TriggerKind::MessageCommand
                ) {
                    return Err(StepError::NotAvailable {
                        action: "target",
                        trigger: self.trigger.name(),
                    });
                }
                Calc::Target(*field)
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
            Expr::HasRole { role_id } => {
                if !self.trigger.has_member() || matches!(self.trigger, TriggerKind::ReactionRemove)
                {
                    return Err(StepError::NotAvailable {
                        action: "role check",
                        trigger: self.trigger.name(),
                    });
                }
                Calc::HasRole(parse_id(role_id)?)
            }
        })
    }

    fn boxed(&mut self, slot: usize, expr: &Expr, depth: usize) -> Result<Box<Calc>, StepError> {
        self.in_expr(slot, |this| this.expr(expr, depth))
            .map(Box::new)
    }

    fn compile_embeds(&mut self, embeds: &[EmbedSpec]) -> Result<Vec<CompiledEmbed>, StepError> {
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
        Ok(compiled_embeds)
    }

    fn message(
        &mut self,
        content: &str,
        embeds: &[EmbedSpec],
        components: &[ComponentRow],
        poll: Option<&PollSpec>,
        attachments: &[AttachmentSpec],
        require_body: bool,
    ) -> Result<CompiledMessage, StepError> {
        let content = self.optional_content(content)?;
        let compiled_embeds = self.compile_embeds(embeds)?;
        let compiled_components = self.components(components)?;
        self.component_loc = None;
        let compiled_poll = poll.map(|spec| self.poll(spec)).transpose()?;
        let compiled_attachments = self.attachments(attachments)?;

        if require_body
            && content.is_none()
            && compiled_embeds.is_empty()
            && compiled_poll.is_none()
            && compiled_attachments.is_empty()
        {
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
            poll: compiled_poll,
            attachments: compiled_attachments,
        })
    }

    fn poll(&mut self, spec: &PollSpec) -> Result<CompiledPoll, StepError> {
        let question =
            self.required_text(&spec.question, MAX_POLL_QUESTION_LEN, "poll question")?;
        if spec.answers.is_empty() || spec.answers.len() > MAX_POLL_ANSWERS {
            return Err(StepError::InvalidPollAnswerCount);
        }
        if spec.duration_hours == 0 || spec.duration_hours > MAX_POLL_HOURS {
            return Err(StepError::InvalidPollDuration);
        }
        let mut answers = Vec::with_capacity(spec.answers.len());
        for answer in &spec.answers {
            let text = self.required_text(&answer.text, MAX_POLL_ANSWER_LEN, "poll answer")?;
            let emoji = optional_emoji(answer.emoji.as_deref())?;
            answers.push(CompiledPollAnswer { text, emoji });
        }
        Ok(CompiledPoll {
            question,
            answers,
            duration_hours: spec.duration_hours,
            allow_multiselect: spec.allow_multiselect,
        })
    }

    fn attachments(
        &mut self,
        specs: &[AttachmentSpec],
    ) -> Result<Vec<CompiledAttachment>, StepError> {
        if specs.len() > MAX_ATTACHMENTS {
            return Err(StepError::TooManyAttachments);
        }
        let mut compiled = Vec::with_capacity(specs.len());
        for spec in specs {
            let filename = check_filename(&spec.filename)?;
            let content = self.required_text(
                &spec.content,
                MAX_ATTACHMENT_CONTENT_LEN,
                "attachment content",
            )?;
            compiled.push(CompiledAttachment { filename, content });
        }
        Ok(compiled)
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
        if value.is_empty() || value.chars().count() > MAX_ID_TEMPLATE_LEN {
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

        let fields = self.embed_fields(embed_idx, &spec.fields)?;

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

    fn embed_fields(
        &mut self,
        embed_idx: usize,
        specs: &[EmbedFieldSpec],
    ) -> Result<Vec<CompiledEmbedField>, StepError> {
        if specs.len() > MAX_EMBED_FIELDS {
            self.set_embed_part(
                embed_idx,
                Some(EmbedPart::Field {
                    index: MAX_EMBED_FIELDS,
                }),
            );
            return Err(StepError::TooManyEmbedFields);
        }
        let mut fields = Vec::with_capacity(specs.len());
        for (field_idx, field) in specs.iter().enumerate() {
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
        Ok(fields)
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
                ComponentRow::EntitySelectMenu {
                    id,
                    kind,
                    placeholder,
                    min_values,
                    max_values,
                    disabled,
                    channel_types,
                } => self.entity_select_menu(
                    id,
                    *kind,
                    placeholder.as_deref(),
                    *min_values,
                    *max_values,
                    *disabled,
                    channel_types,
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

    #[allow(clippy::too_many_arguments)]
    fn entity_select_menu(
        &mut self,
        id: &str,
        kind: EntitySelectKind,
        placeholder: Option<&str>,
        min_values: Option<u8>,
        max_values: Option<u8>,
        disabled: bool,
        channel_types: &[ChannelFilter],
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
        let min = min_values.unwrap_or(1);
        let max = max_values.unwrap_or(1);
        if min == 0 || max == 0 || min > max || max > 25 {
            return Err(StepError::InvalidSelectBounds {
                min,
                max,
                options: 25,
            });
        }
        let channel_types = if kind == EntitySelectKind::Channel {
            channel_types.to_vec()
        } else {
            Vec::new()
        };
        Ok(CompiledComponentRow::EntitySelectMenu {
            id,
            kind,
            placeholder,
            min_values: min,
            max_values: max,
            disabled,
            channel_types,
        })
    }

    fn variable(&mut self, scope: Scope, name: &str) -> Result<String, StepError> {
        if scope != Scope::Local && matches!(self.trigger, TriggerKind::Schedule) {
            return Err(StepError::NotAvailable {
                action: "saved variable",
                trigger: self.trigger.name(),
            });
        }
        let valid = !name.is_empty()
            && name.len() <= MAX_NAME_LEN
            && name
                .bytes()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_');
        if !valid {
            return Err(StepError::InvalidVariableName(name.to_owned()));
        }
        if scope != Scope::Local {
            self.saved.insert((scope, name.to_owned()));
        }
        Ok(name.to_owned())
    }

    /// Record the `{member:NAME}` and `{server:NAME}` placeholders of a template.
    fn note_template(&mut self, template: &str) {
        for (prefix, scope) in [("{member:", Scope::Member), ("{server:", Scope::Server)] {
            let mut rest = template;
            while let Some(start) = rest.find(prefix) {
                rest = &rest[start + prefix.len()..];
                if let Some(end) = rest.find('}') {
                    let name = &rest[..end];
                    if !name.is_empty() && !name.contains('{') {
                        self.saved.insert((scope, name.to_owned()));
                    }
                    rest = &rest[end + 1..];
                }
            }
        }
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
        check_option_choices_and_autocomplete(option).map_err(|reason| (index, reason))?;
        check_option_constraints(option).map_err(|reason| (index, reason))?;
        optional_seen |= !option.required;
    }
    Ok(())
}

fn check_option_constraints(option: &CommandOption) -> Result<(), HandlerError> {
    let has_value_bounds = option.min_value.is_some() || option.max_value.is_some();
    let has_length_bounds = option.min_length.is_some() || option.max_length.is_some();

    if has_value_bounds && !matches!(option.kind, OptionKind::Integer | OptionKind::Number) {
        return Err(HandlerError::UnsupportedOptionConstraint(
            option.name.clone(),
            "minimum and maximum values for integer or decimal options",
        ));
    }
    if has_length_bounds && option.kind != OptionKind::String {
        return Err(HandlerError::UnsupportedOptionConstraint(
            option.name.clone(),
            "minimum and maximum lengths for text options",
        ));
    }
    if !option.channel_types.is_empty() && option.kind != OptionKind::Channel {
        return Err(HandlerError::UnsupportedOptionConstraint(
            option.name.clone(),
            "channel type filters for channel options",
        ));
    }
    if !option.file_types.is_empty() && option.kind != OptionKind::Attachment {
        return Err(HandlerError::UnsupportedOptionConstraint(
            option.name.clone(),
            "attachment type filters for attachment options",
        ));
    }

    match option.kind {
        OptionKind::Integer => {
            let min = option
                .min_value
                .as_deref()
                .map(|value| parse_integer_option_bound(value, option, "minimum value"))
                .transpose()?;
            let max = option
                .max_value
                .as_deref()
                .map(|value| parse_integer_option_bound(value, option, "maximum value"))
                .transpose()?;
            if min.zip(max).is_some_and(|(min, max)| min > max) {
                return Err(HandlerError::InvalidOptionConstraintBounds(
                    option.name.clone(),
                    "value",
                ));
            }
        }
        OptionKind::Number => {
            let min = option
                .min_value
                .as_deref()
                .map(|value| parse_number_option_bound(value, option, "minimum value"))
                .transpose()?;
            let max = option
                .max_value
                .as_deref()
                .map(|value| parse_number_option_bound(value, option, "maximum value"))
                .transpose()?;
            if min.zip(max).is_some_and(|(min, max)| min > max) {
                return Err(HandlerError::InvalidOptionConstraintBounds(
                    option.name.clone(),
                    "value",
                ));
            }
        }
        OptionKind::String => {
            let min = option
                .min_length
                .as_deref()
                .map(|value| parse_string_option_bound(value, option, "minimum length", true))
                .transpose()?;
            let max = option
                .max_length
                .as_deref()
                .map(|value| parse_string_option_bound(value, option, "maximum length", false))
                .transpose()?;
            if min.zip(max).is_some_and(|(min, max)| min > max) {
                return Err(HandlerError::InvalidOptionConstraintBounds(
                    option.name.clone(),
                    "length",
                ));
            }
        }
        OptionKind::Attachment => {
            if option.file_types.len() > MAX_ATTACHMENT_FILE_TYPES {
                return Err(HandlerError::TooManyAttachmentFileTypes(
                    option.name.clone(),
                ));
            }
            let mut seen = HashSet::with_capacity(option.file_types.len());
            for file_type in &option.file_types {
                let trimmed = file_type.trim();
                let is_category = matches!(trimmed, "image" | "video" | "audio");
                let is_extension = trimmed.starts_with('.')
                    && trimmed.len() > 1
                    && !trimmed.contains(char::is_whitespace)
                    && !trimmed.contains(['/', '\\']);
                if !(is_category || is_extension) || !seen.insert(trimmed) {
                    return Err(HandlerError::InvalidOptionConstraint(
                        option.name.clone(),
                        "attachment type filter",
                    ));
                }
            }
        }
        _ => {}
    }

    Ok(())
}

fn parse_integer_option_bound(
    value: &str,
    option: &CommandOption,
    name: &'static str,
) -> Result<i64, HandlerError> {
    let parsed = value
        .trim()
        .parse::<i64>()
        .ok()
        .filter(|value| (MIN_INTEGER_OPTION_VALUE..=MAX_INTEGER_OPTION_VALUE).contains(value));
    parsed.ok_or_else(|| HandlerError::InvalidOptionConstraint(option.name.clone(), name))
}

fn parse_number_option_bound(
    value: &str,
    option: &CommandOption,
    name: &'static str,
) -> Result<f64, HandlerError> {
    let parsed = value
        .trim()
        .parse::<f64>()
        .ok()
        .filter(|value| value.is_finite() && value.abs() <= MAX_NUMBER_OPTION_VALUE);
    parsed.ok_or_else(|| HandlerError::InvalidOptionConstraint(option.name.clone(), name))
}

fn parse_string_option_bound(
    value: &str,
    option: &CommandOption,
    name: &'static str,
    allows_zero: bool,
) -> Result<u16, HandlerError> {
    let parsed = value
        .trim()
        .parse::<u16>()
        .ok()
        .filter(|value| *value <= MAX_OPTION_STRING_LENGTH && (allows_zero || *value > 0));
    parsed.ok_or_else(|| HandlerError::InvalidOptionConstraint(option.name.clone(), name))
}

fn check_option_choices_and_autocomplete(option: &CommandOption) -> Result<(), HandlerError> {
    let supports_choices = matches!(
        option.kind,
        OptionKind::String | OptionKind::Integer | OptionKind::Number
    );
    if (!option.choices.is_empty() || !option.autocomplete.is_empty()) && !supports_choices {
        return Err(HandlerError::ChoicesUnsupported(option.name.clone()));
    }
    if !option.choices.is_empty() && !option.autocomplete.is_empty() {
        return Err(HandlerError::ChoicesAndAutocompleteConflict(
            option.name.clone(),
        ));
    }
    if option.choices.len() > MAX_CHOICES || option.autocomplete.len() > MAX_CHOICES {
        return Err(HandlerError::TooManyChoices(option.name.clone()));
    }
    for CommandOptionChoiceSpec { name, value } in &option.choices {
        let name_len = name.chars().count();
        if name_len == 0 || name_len > MAX_DESCRIPTION_LEN || name.trim().is_empty() {
            return Err(HandlerError::InvalidChoice(
                option.name.clone(),
                name.clone(),
            ));
        }
        validate_choice_value(option.kind, &option.name, value)?;
    }
    for suggestion in &option.autocomplete {
        validate_choice_value(option.kind, &option.name, suggestion)?;
    }
    Ok(())
}

fn validate_choice_value(
    kind: OptionKind,
    option_name: &str,
    value: &str,
) -> Result<(), HandlerError> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(HandlerError::InvalidChoice(
            option_name.to_owned(),
            value.to_owned(),
        ));
    }
    match kind {
        OptionKind::String => {
            if value.chars().count() > MAX_DESCRIPTION_LEN {
                return Err(HandlerError::InvalidChoice(
                    option_name.to_owned(),
                    value.to_owned(),
                ));
            }
        }
        OptionKind::Integer => {
            if trimmed.parse::<i64>().is_err() {
                return Err(HandlerError::InvalidChoice(
                    option_name.to_owned(),
                    value.to_owned(),
                ));
            }
        }
        OptionKind::Number => {
            let Ok(parsed) = trimmed.parse::<f64>() else {
                return Err(HandlerError::InvalidChoice(
                    option_name.to_owned(),
                    value.to_owned(),
                ));
            };
            if !parsed.is_finite() {
                return Err(HandlerError::InvalidChoice(
                    option_name.to_owned(),
                    value.to_owned(),
                ));
            }
        }
        _ => {}
    }
    Ok(())
}

fn check_archive_duration(minutes: Option<u16>) -> Result<Option<u16>, StepError> {
    match minutes {
        None | Some(60 | 1440 | 4320 | 10080) => Ok(minutes),
        Some(_) => Err(StepError::InvalidArchiveDuration),
    }
}

fn check_filename(value: &str) -> Result<String, StepError> {
    let trimmed = value.trim();
    if trimmed.is_empty()
        || trimmed.chars().count() > MAX_ATTACHMENT_FILENAME_LEN
        || trimmed.contains(['/', '\\'])
        || trimmed.contains("..")
    {
        return Err(StepError::InvalidFilename(value.to_owned()));
    }
    Ok(trimmed.to_owned())
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

fn push_reaction_handler(
    list: &mut Vec<ReactionMatcher>,
    handler: usize,
    verb: &str,
    emoji: Option<&str>,
    message_id: Option<&str>,
) -> Result<String, HandlerError> {
    let emoji = optional_trigger_emoji(emoji)?;
    let message_id = optional_trigger_message_id(message_id)?;
    let label = reaction_label(verb, emoji.as_ref(), message_id);
    list.push(ReactionMatcher {
        emoji,
        message_id,
        handler,
    });
    Ok(label)
}

fn optional_trigger_emoji(value: Option<&str>) -> Result<Option<EmojiRef>, HandlerError> {
    let Some(value) = value else {
        return Ok(None);
    };
    if value.trim().is_empty() {
        return Ok(None);
    }
    parse_emoji(value)
        .map(Some)
        .map_err(|_| HandlerError::InvalidEmoji(value.to_owned()))
}

fn optional_trigger_message_id(value: Option<&str>) -> Result<Option<u64>, HandlerError> {
    let Some(value) = value else {
        return Ok(None);
    };
    if value.trim().is_empty() {
        return Ok(None);
    }
    parse_id(value)
        .map(Some)
        .map_err(|_| HandlerError::InvalidMessageId(value.to_owned()))
}

fn reaction_label(verb: &str, emoji: Option<&EmojiRef>, message_id: Option<u64>) -> String {
    let mut parts = vec![verb.to_owned()];
    if let Some(emoji) = emoji {
        parts.push(match emoji {
            EmojiRef::Unicode(name) => name.clone(),
            EmojiRef::Custom { name, .. } => format!(":{name}:"),
        });
    }
    if let Some(message_id) = message_id {
        parts.push(format!("on {message_id}"));
    }
    parts.join(" ")
}

fn parse_schedule(interval: u32, unit: ScheduleUnit) -> Result<ScheduleSpec, HandlerError> {
    let spec = ScheduleSpec { interval, unit };
    let mult: u64 = match unit {
        ScheduleUnit::Minutes => 60,
        ScheduleUnit::Hours => 3_600,
        ScheduleUnit::Days => 86_400,
    };
    let valid = u64::from(interval)
        .checked_mul(mult)
        .is_some_and(|secs| (60..=u64::from(MAX_SCHEDULE_SECONDS)).contains(&secs));
    if valid {
        Ok(spec)
    } else {
        Err(HandlerError::InvalidSchedule)
    }
}

fn schedule_label(interval: u32, unit: ScheduleUnit) -> String {
    let unit_name = match (unit, interval == 1) {
        (ScheduleUnit::Minutes, true) => "minute",
        (ScheduleUnit::Minutes, false) => "minutes",
        (ScheduleUnit::Hours, true) => "hour",
        (ScheduleUnit::Hours, false) => "hours",
        (ScheduleUnit::Days, true) => "day",
        (ScheduleUnit::Days, false) => "days",
    };
    format!("every {interval} {unit_name}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{ModalInputSpec, OptionKind, PollAnswerSpec};

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
            poll: None,
            attachments: Vec::new(),
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
                ..GatewayNeeds::default()
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
        assert_eq!(in_template.saved_variable_count(), 1);
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
            choices: Vec::new(),
            autocomplete: Vec::new(),
            min_value: None,
            max_value: None,
            min_length: None,
            max_length: None,
            channel_types: Vec::new(),
            file_types: Vec::new(),
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
    fn validates_discord_option_constraints() {
        let program: Program = serde_json::from_str(
            r#"{"handlers":[{"trigger":{"type":"slash_command","name":"configure","description":"Configure the bot","options":[{"name":"title","description":"A title","kind":"string","min_length":"2","max_length":"64"},{"name":"count","description":"A count","kind":"integer","min_value":"1","max_value":"10"},{"name":"ratio","description":"A ratio","kind":"number","min_value":"-1.5","max_value":"3.5"},{"name":"destination","description":"A channel","kind":"channel","channel_types":["text","forum","stage","category","media"]},{"name":"file","description":"A file","kind":"attachment","file_types":["image",".pdf"]}]},"actions":[{"type":"reply","content":"Done"}]}]}"#,
        )
        .expect("valid program");
        assert!(program.compile().is_ok());

        let invalid: Program = serde_json::from_str(
            r#"{"handlers":[{"trigger":{"type":"slash_command","name":"configure","description":"Configure the bot","options":[{"name":"count","description":"A count","kind":"integer","min_value":"10","max_value":"1"}]},"actions":[{"type":"reply","content":"Done"}]}]}"#,
        )
        .expect("valid JSON");
        assert!(matches!(
            invalid.compile(),
            Err(CompileError::Handler {
                reason: HandlerError::InvalidOptionConstraintBounds(_, "value"),
                ..
            })
        ));
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

    #[test]
    fn compiles_member_leave_reactions_and_schedules() {
        let program = Program {
            handlers: vec![
                Handler {
                    trigger: Trigger::MemberLeave {},
                    actions: vec![Statement::SendMessage {
                        channel_id: "123456789012345678".to_owned(),
                        content: "Bye {user.name}".to_owned(),
                        embeds: Vec::new(),
                        components: Vec::new(),
                        poll: None,
                        attachments: Vec::new(),
                        reply_to_message_id: None,
                    }],
                },
                Handler {
                    trigger: Trigger::ReactionAdd {
                        emoji: Some("👍".to_owned()),
                        message_id: Some("123456789012345679".to_owned()),
                    },
                    actions: vec![Statement::AddRole {
                        role_id: "123456789012345680".to_owned(),
                    }],
                },
                Handler {
                    trigger: Trigger::ReactionRemove {
                        emoji: Some("<:star:123456789012345681>".to_owned()),
                        message_id: None,
                    },
                    actions: vec![Statement::React {
                        emoji: "👀".to_owned(),
                    }],
                },
                Handler {
                    trigger: Trigger::Schedule {
                        interval: 2,
                        unit: ScheduleUnit::Hours,
                    },
                    actions: vec![Statement::SendMessage {
                        channel_id: "123456789012345678".to_owned(),
                        content: "Hourly check".to_owned(),
                        embeds: Vec::new(),
                        components: Vec::new(),
                        poll: None,
                        attachments: Vec::new(),
                        reply_to_message_id: None,
                    }],
                },
            ],
        };

        let compiled = program.compile().expect("new event triggers compile");
        assert_eq!(
            compiled.gateway_needs(),
            GatewayNeeds {
                members: true,
                reactions: true,
                ..GatewayNeeds::default()
            }
        );
        assert_eq!(
            compiled.schedules(),
            vec![ScheduleSpec {
                interval: 2,
                unit: ScheduleUnit::Hours
            }]
        );
    }

    #[test]
    fn validates_reaction_triggers_and_schedules() {
        let bad_emoji = Program {
            handlers: vec![Handler {
                trigger: Trigger::ReactionAdd {
                    emoji: Some("thumbsup".to_owned()),
                    message_id: None,
                },
                actions: vec![reply("hi")],
            }],
        };
        assert_eq!(
            bad_emoji.compile(),
            Err(CompileError::Handler {
                handler: 0,
                reason: HandlerError::InvalidEmoji("thumbsup".to_owned()),
            })
        );

        let bad_message_id = Program {
            handlers: vec![Handler {
                trigger: Trigger::ReactionRemove {
                    emoji: None,
                    message_id: Some("not-an-id".to_owned()),
                },
                actions: vec![reply("hi")],
            }],
        };
        assert_eq!(
            bad_message_id.compile(),
            Err(CompileError::Handler {
                handler: 0,
                reason: HandlerError::InvalidMessageId("not-an-id".to_owned()),
            })
        );

        let bad_schedule = Program {
            handlers: vec![Handler {
                trigger: Trigger::Schedule {
                    interval: 31,
                    unit: ScheduleUnit::Days,
                },
                actions: vec![Statement::Stop {}],
            }],
        };
        assert_eq!(
            bad_schedule.compile(),
            Err(CompileError::Handler {
                handler: 0,
                reason: HandlerError::InvalidSchedule,
            })
        );
    }

    #[test]
    fn rejects_incompatible_actions_and_vars_on_new_triggers() {
        let leave_role = Program {
            handlers: vec![Handler {
                trigger: Trigger::MemberLeave {},
                actions: vec![Statement::AddRole {
                    role_id: "123456789012345678".to_owned(),
                }],
            }],
        };
        assert!(matches!(
            leave_role.compile(),
            Err(CompileError::Handler {
                reason: HandlerError::Step {
                    reason: StepError::NotAvailable {
                        action: "add role",
                        trigger: "member leave"
                    },
                    ..
                },
                ..
            })
        ));

        let remove_has_role = Program {
            handlers: vec![Handler {
                trigger: Trigger::ReactionRemove {
                    emoji: None,
                    message_id: None,
                },
                actions: vec![Statement::If {
                    condition: Expr::HasRole {
                        role_id: "123456789012345678".to_owned(),
                    },
                    then_branch: vec![Statement::Stop {}],
                    else_branch: Vec::new(),
                }],
            }],
        };
        assert!(matches!(
            remove_has_role.compile(),
            Err(CompileError::Handler {
                reason: HandlerError::Step {
                    reason: StepError::NotAvailable {
                        action: "role check",
                        trigger: "reaction removed"
                    },
                    ..
                },
                ..
            })
        ));

        let schedule_saved_var = Program {
            handlers: vec![Handler {
                trigger: Trigger::Schedule {
                    interval: 5,
                    unit: ScheduleUnit::Minutes,
                },
                actions: vec![Statement::SetVariable {
                    scope: Scope::Server,
                    name: "count".to_owned(),
                    value: Expr::Number { value: 1 },
                }],
            }],
        };
        assert!(matches!(
            schedule_saved_var.compile(),
            Err(CompileError::Handler {
                reason: HandlerError::Step {
                    reason: StepError::NotAvailable {
                        action: "saved variable",
                        trigger: "scheduled task"
                    },
                    ..
                },
                ..
            })
        ));
    }

    #[test]
    fn compiles_subcommands_context_commands_modals_polls_and_threads() {
        let program = Program {
            handlers: vec![
                Handler {
                    trigger: Trigger::SlashSubcommand {
                        command: "admin".to_owned(),
                        group: Some("user".to_owned()),
                        subcommand: "warn".to_owned(),
                        description: "Warn a member".to_owned(),
                        options: vec![CommandOption {
                            name: "severity".to_owned(),
                            description: "Severity".to_owned(),
                            kind: OptionKind::String,
                            required: true,
                            choices: vec![CommandOptionChoiceSpec {
                                name: "Low".to_owned(),
                                value: "low".to_owned(),
                            }],
                            autocomplete: Vec::new(),
                            min_value: None,
                            max_value: None,
                            min_length: None,
                            max_length: None,
                            channel_types: Vec::new(),
                            file_types: Vec::new(),
                        }],
                    },
                    actions: vec![Statement::ShowModal {
                        id: "warn_modal".to_owned(),
                        title: "Warn details".to_owned(),
                        inputs: vec![ModalInputSpec {
                            id: "reason".to_owned(),
                            label: "Reason".to_owned(),
                            style: TextInputStyle::Paragraph,
                            placeholder: Some("Explain why".to_owned()),
                            required: true,
                            min_length: Some(3),
                            max_length: Some(200),
                            value: None,
                        }],
                    }],
                },
                Handler {
                    trigger: Trigger::ModalSubmit {
                        id: "warn_modal".to_owned(),
                    },
                    actions: vec![Statement::Reply {
                        content: "Submitted: {modal.reason}".to_owned(),
                        ephemeral: true,
                        embeds: Vec::new(),
                        components: vec![ComponentRow::EntitySelectMenu {
                            id: "pick_channel".to_owned(),
                            kind: EntitySelectKind::Channel,
                            placeholder: Some("Pick channel".to_owned()),
                            min_values: Some(1),
                            max_values: Some(1),
                            disabled: false,
                            channel_types: vec![ChannelFilter::Text],
                        }],
                        poll: None,
                        attachments: vec![AttachmentSpec {
                            filename: "log.txt".to_owned(),
                            content: "Warned by {user.name}".to_owned(),
                        }],
                    }],
                },
                Handler {
                    trigger: Trigger::UserCommand {
                        name: "Inspect Member".to_owned(),
                    },
                    actions: vec![reply("User {target} ({target.id})")],
                },
                Handler {
                    trigger: Trigger::MessageCommand {
                        name: "Create Poll Thread".to_owned(),
                    },
                    actions: vec![
                        Statement::CreateThreadFromMessage {
                            name: "Discussion".to_owned(),
                            auto_archive_minutes: Some(1440),
                        },
                        Statement::Reply {
                            content: "".to_owned(),
                            ephemeral: false,
                            embeds: Vec::new(),
                            components: Vec::new(),
                            poll: Some(PollSpec {
                                question: "Agree?".to_owned(),
                                answers: vec![
                                    PollAnswerSpec {
                                        text: "Yes".to_owned(),
                                        emoji: Some("👍".to_owned()),
                                    },
                                    PollAnswerSpec {
                                        text: "No".to_owned(),
                                        emoji: None,
                                    },
                                ],
                                duration_hours: 24,
                                allow_multiselect: false,
                            }),
                            attachments: Vec::new(),
                        },
                    ],
                },
                Handler {
                    trigger: Trigger::AutoModExecution { rule_id: None },
                    actions: vec![Statement::SendMessage {
                        channel_id: "123456789012345678".to_owned(),
                        content: "AutoMod hit: {automod.keyword}".to_owned(),
                        embeds: Vec::new(),
                        components: Vec::new(),
                        poll: None,
                        attachments: Vec::new(),
                        reply_to_message_id: None,
                    }],
                },
            ],
        };

        let compiled = program.compile().expect("new features compile");
        assert_eq!(compiled.commands().len(), 1);
        assert_eq!(compiled.commands()[0].groups.len(), 1);
        assert_eq!(compiled.context_commands().len(), 2);
        assert!(compiled.gateway_needs().automod);

        // Modal after reply must fail.
        let modal_after_reply = Program {
            handlers: vec![slash(
                "bad",
                vec![
                    reply("first"),
                    Statement::ShowModal {
                        id: "m".to_owned(),
                        title: "Title".to_owned(),
                        inputs: vec![ModalInputSpec {
                            id: "f".to_owned(),
                            label: "Field".to_owned(),
                            style: TextInputStyle::Short,
                            placeholder: None,
                            required: true,
                            min_length: None,
                            max_length: None,
                            value: None,
                        }],
                    },
                ],
            )],
        };
        assert!(matches!(
            modal_after_reply.compile(),
            Err(CompileError::Handler {
                reason: HandlerError::Step {
                    reason: StepError::ModalNotFirstResponse,
                    ..
                },
                ..
            })
        ));
    }
}
