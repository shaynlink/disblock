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
    /// A message containing `pattern` is posted in a channel the bot can read.
    MessageContains {
        pattern: String,
        #[serde(default)]
        case_sensitive: bool,
    },
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
/// `{option.NAME}`, `{var:NAME}`, `{member:NAME}` and `{server:NAME}` are
/// replaced when the step runs. Snowflake IDs are strings because JavaScript
/// numbers cannot hold them.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum Statement {
    /// Answer the slash command, or reply to the triggering message.
    Reply {
        content: String,
        #[serde(default)]
        ephemeral: bool,
    },
    SendMessage {
        channel_id: String,
        content: String,
    },
    /// Give a role to the member who triggered the handler.
    AddRole {
        role_id: String,
    },
    /// Take a role from the member who triggered the handler.
    RemoveRole {
        role_id: String,
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
