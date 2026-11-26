use serde::{Deserialize, Serialize};

/// A bot program as sent by the block editor.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Program {
    pub handlers: Vec<Handler>,
}

/// One trigger and the actions it runs, in order.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Handler {
    pub trigger: Trigger,
    pub actions: Vec<Action>,
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

/// Snowflake IDs are strings because JavaScript numbers cannot hold them.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum Action {
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
}
