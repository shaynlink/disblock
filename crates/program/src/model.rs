use serde::{Deserialize, Serialize};

/// A bot program as sent by the block editor.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Program {
    pub handlers: Vec<Handler>,
}

/// One trigger and the statements it runs, in order.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Handler {
    pub trigger: Trigger,
    pub actions: Vec<Statement>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum Trigger {
    /// A slash command, registered globally on the bot's application.
    SlashCommand {
        name: String,
        description: String,
        #[serde(default)]
        options: Vec<CommandOption>,
    },
    /// A member joins a server the bot is in.
    MemberJoin {},
    /// A member leaves a server the bot is in.
    MemberLeave {},
    /// A message containing `pattern` is posted in a channel the bot can read.
    MessageContains {
        pattern: String,
        #[serde(default)]
        case_sensitive: bool,
    },
    /// A reaction is added to a message in a server.
    ReactionAdd {
        #[serde(default)]
        emoji: Option<String>,
        #[serde(default)]
        message_id: Option<String>,
    },
    /// A reaction is removed from a message in a server.
    ReactionRemove {
        #[serde(default)]
        emoji: Option<String>,
        #[serde(default)]
        message_id: Option<String>,
    },
    /// A button on a message is clicked.
    ButtonClick { id: String },
    /// An option is chosen from a select menu on a message.
    SelectMenu { id: String },
    /// Runs repeatedly on a fixed interval while the bot is online.
    Schedule { interval: u32, unit: ScheduleUnit },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScheduleUnit {
    Minutes,
    Hours,
    Days,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CommandOption {
    pub name: String,
    pub description: String,
    pub kind: OptionKind,
    #[serde(default)]
    pub required: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OptionKind {
    String,
    Integer,
    Boolean,
    User,
}

/// One step of a handler. Message contents are templates: `{user}`,
/// `{option.NAME}`, `{selected}`, `{var:NAME}`, `{member:NAME}` and
/// `{server:NAME}` are replaced when the step runs. Snowflake IDs are strings
/// because JavaScript numbers cannot hold them.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum Statement {
    /// Answer the command, message, or component interaction.
    Reply {
        #[serde(default)]
        content: String,
        #[serde(default)]
        ephemeral: bool,
        #[serde(default)]
        embeds: Vec<EmbedSpec>,
        #[serde(default)]
        components: Vec<ComponentRow>,
    },
    SendMessage {
        channel_id: String,
        #[serde(default)]
        content: String,
        #[serde(default)]
        embeds: Vec<EmbedSpec>,
        #[serde(default)]
        components: Vec<ComponentRow>,
    },
    /// Edit the message whose button or select menu was clicked.
    UpdateMessage {
        #[serde(default)]
        content: String,
        #[serde(default)]
        embeds: Vec<EmbedSpec>,
        #[serde(default)]
        components: Vec<ComponentRow>,
    },
    /// Give a role to the member who triggered the handler.
    AddRole {
        role_id: String,
    },
    /// Take a role from the member who triggered the handler.
    RemoveRole {
        role_id: String,
    },
    /// Prevent a member from communicating for the requested number of seconds.
    Timeout {
        /// A Discord member ID, or a template such as `{user.id}` or
        /// `{option.member}` for a user slash-command option.
        member_id: String,
        seconds: u32,
        #[serde(default)]
        reason: String,
    },
    /// Remove a member from the server.
    Kick {
        /// A Discord member ID, or a template such as `{user.id}` or
        /// `{option.member}` for a user slash-command option.
        member_id: String,
        #[serde(default)]
        reason: String,
    },
    /// Ban a member from the server.
    Ban {
        /// A Discord member ID, or a template such as `{user.id}` or
        /// `{option.member}` for a user slash-command option.
        member_id: String,
        #[serde(default)]
        reason: String,
    },
    /// Delete the message that triggered a message-contains handler.
    DeleteMessage {},
    /// Send a direct message to a member.
    DirectMessage {
        /// A Discord member ID, or a template such as `{user.id}` or
        /// `{option.member}` for a user slash-command option.
        member_id: String,
        content: String,
    },
    /// React to the triggering message with a unicode emoji or `name:id`.
    React {
        emoji: String,
    },
    If {
        condition: Expr,
        #[serde(rename = "then")]
        then_branch: Vec<Statement>,
        #[serde(default, rename = "else")]
        else_branch: Vec<Statement>,
    },
    SetVariable {
        scope: Scope,
        name: String,
        value: Expr,
    },
    /// Add a number to a variable, which counts as 0 when it is not set.
    ChangeVariable {
        scope: Scope,
        name: String,
        by: Expr,
    },
    Wait {
        seconds: u32,
    },
    /// End the handler here.
    Stop {},
}

/// Where a variable lives.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Scope {
    /// Only during this run of the handler.
    Local,
    /// Saved for the member, in the server they triggered the handler in.
    Member,
    /// Saved for the whole server.
    Server,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MathOp {
    Add,
    Subtract,
    Multiply,
    Divide,
    Modulo,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CompareOp {
    Equal,
    NotEqual,
    Less,
    LessOrEqual,
    Greater,
    GreaterOrEqual,
    Contains,
}

/// A value computed when the handler runs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum Expr {
    /// A template, rendered like message contents.
    Text {
        value: String,
    },
    Number {
        value: i64,
    },
    Boolean {
        value: bool,
    },
    Variable {
        scope: Scope,
        name: String,
    },
    /// A slash command option, empty when it was not given.
    Option {
        name: String,
    },
    /// Value(s) chosen in a select menu trigger, empty when none.
    Selected {},
    /// A whole number between `min` and `max`, both included.
    Random {
        min: Box<Expr>,
        max: Box<Expr>,
    },
    Math {
        op: MathOp,
        left: Box<Expr>,
        right: Box<Expr>,
    },
    Compare {
        op: CompareOp,
        left: Box<Expr>,
        right: Box<Expr>,
    },
    And {
        left: Box<Expr>,
        right: Box<Expr>,
    },
    Or {
        left: Box<Expr>,
        right: Box<Expr>,
    },
    Not {
        value: Box<Expr>,
    },
    /// Whether the member who triggered the handler has the role.
    HasRole {
        role_id: String,
    },
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EmbedSpec {
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub url: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub color: Option<String>,
    #[serde(default)]
    pub author: Option<EmbedAuthorSpec>,
    #[serde(default)]
    pub footer: Option<EmbedFooterSpec>,
    #[serde(default)]
    pub thumbnail_url: Option<String>,
    #[serde(default)]
    pub image_url: Option<String>,
    #[serde(default)]
    pub fields: Vec<EmbedFieldSpec>,
    #[serde(default)]
    pub timestamp: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EmbedAuthorSpec {
    pub name: String,
    #[serde(default)]
    pub url: Option<String>,
    #[serde(default)]
    pub icon_url: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EmbedFooterSpec {
    pub text: String,
    #[serde(default)]
    pub icon_url: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EmbedFieldSpec {
    pub name: String,
    pub value: String,
    #[serde(default)]
    pub inline: bool,
}

/// One action row below a message: up to 5 buttons or 1 select menu.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum ComponentRow {
    Buttons {
        buttons: Vec<ButtonSpec>,
    },
    SelectMenu {
        id: String,
        #[serde(default)]
        placeholder: Option<String>,
        #[serde(default)]
        min_values: Option<u8>,
        #[serde(default)]
        max_values: Option<u8>,
        #[serde(default)]
        disabled: bool,
        options: Vec<SelectOptionSpec>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum ButtonSpec {
    Action {
        id: String,
        #[serde(default)]
        label: Option<String>,
        #[serde(default)]
        emoji: Option<String>,
        style: ButtonStyle,
        #[serde(default)]
        disabled: bool,
    },
    Link {
        url: String,
        #[serde(default)]
        label: Option<String>,
        #[serde(default)]
        emoji: Option<String>,
        #[serde(default)]
        disabled: bool,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ButtonStyle {
    Primary,
    Secondary,
    Success,
    Danger,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SelectOptionSpec {
    pub label: String,
    pub value: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub emoji: Option<String>,
    #[serde(default)]
    pub default: bool,
}
