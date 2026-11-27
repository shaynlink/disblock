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
    /// A slash subcommand (optionally inside a group), merged into its parent command.
    SlashSubcommand {
        command: String,
        #[serde(default)]
        group: Option<String>,
        subcommand: String,
        description: String,
        #[serde(default)]
        options: Vec<CommandOption>,
    },
    /// Right-click context menu command on a Discord user.
    UserCommand { name: String },
    /// Right-click context menu command on a Discord message.
    MessageCommand { name: String },
    /// A modal dialog submitted by a user.
    ModalSubmit { id: String },
    /// A member joins a server the bot is in.
    MemberJoin {},
    /// A member leaves a server the bot is in.
    MemberLeave {},
    /// A member's roles or profile in a server is updated.
    MemberUpdate {},
    /// A member is banned from a server.
    MemberBan {},
    /// A user is unbanned from a server.
    MemberUnban {},
    /// A message containing `pattern` is posted in a channel the bot can read.
    MessageContains {
        pattern: String,
        #[serde(default)]
        case_sensitive: bool,
    },
    /// A message is edited in a channel the bot can see.
    MessageEdit {},
    /// A message is deleted in a channel the bot can see.
    MessageDelete {},
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
    /// A thread is created in a server.
    ThreadCreate {},
    /// An invite is created in a server.
    InviteCreate {},
    /// An AutoMod rule executes on a message or member.
    AutoModExecution {
        #[serde(default)]
        rule_id: Option<String>,
    },
    /// A vote is added to a poll.
    PollVoteAdd {},
    /// A member joins, leaves, or moves in a voice channel.
    VoiceStateUpdate {},
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
    #[serde(default)]
    pub choices: Vec<CommandOptionChoiceSpec>,
    #[serde(default)]
    pub autocomplete: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CommandOptionChoiceSpec {
    pub name: String,
    pub value: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OptionKind {
    String,
    Integer,
    Number,
    Boolean,
    User,
    Channel,
    Role,
    Mentionable,
    Attachment,
}

/// One step of a handler. Message contents are templates: `{user}`,
/// `{option.NAME}`, `{selected}`, `{modal.ID}`, `{target}`, `{var:NAME}`,
/// `{member:NAME}` and `{server:NAME}` are replaced when the step runs.
/// Snowflake IDs are strings because JavaScript numbers cannot hold them.
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
        #[serde(default)]
        poll: Option<PollSpec>,
        #[serde(default)]
        attachments: Vec<AttachmentSpec>,
    },
    SendMessage {
        channel_id: String,
        #[serde(default)]
        content: String,
        #[serde(default)]
        embeds: Vec<EmbedSpec>,
        #[serde(default)]
        components: Vec<ComponentRow>,
        #[serde(default)]
        poll: Option<PollSpec>,
        #[serde(default)]
        attachments: Vec<AttachmentSpec>,
        #[serde(default)]
        reply_to_message_id: Option<String>,
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
    /// Open a modal form as the initial response to an interaction.
    ShowModal {
        id: String,
        title: String,
        inputs: Vec<ModalInputSpec>,
    },
    /// Start a public thread attached to the triggering message.
    CreateThreadFromMessage {
        name: String,
        #[serde(default)]
        auto_archive_minutes: Option<u16>,
    },
    /// Create a standalone thread in a text or announcement channel.
    CreateThread {
        channel_id: String,
        name: String,
        #[serde(default)]
        private: bool,
        #[serde(default)]
        auto_archive_minutes: Option<u16>,
    },
    /// Create a post (thread with starter message) in a forum or media channel.
    CreateForumPost {
        channel_id: String,
        title: String,
        #[serde(default)]
        content: String,
        #[serde(default)]
        embeds: Vec<EmbedSpec>,
    },
    /// Make the bot join a thread.
    JoinThread {
        thread_id: String,
    },
    /// Add a member to a thread.
    AddThreadMember {
        thread_id: String,
        member_id: String,
    },
    /// Remove a member from a thread.
    RemoveThreadMember {
        thread_id: String,
        member_id: String,
    },
    /// Pin a message in a channel.
    PinMessage {
        channel_id: String,
        message_id: String,
    },
    /// Unpin a message in a channel.
    UnpinMessage {
        channel_id: String,
        message_id: String,
    },
    /// Crosspost (publish) an announcement channel message.
    PublishMessage {
        channel_id: String,
        message_id: String,
    },
    /// Delete the most recent `count` messages in a channel (2 to 100).
    BulkDeleteMessages {
        channel_id: String,
        count: u8,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TargetField {
    Id,
    Mention,
    Name,
    Content,
    AuthorId,
    AuthorMention,
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
    /// A text input value submitted in a modal trigger.
    ModalField {
        id: String,
    },
    /// Information about the right-clicked user or message in a context command.
    Target {
        field: TargetField,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EntitySelectKind {
    User,
    Role,
    Mentionable,
    Channel,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChannelFilter {
    Text,
    Voice,
    Announcement,
    Forum,
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
    EntitySelectMenu {
        id: String,
        kind: EntitySelectKind,
        #[serde(default)]
        placeholder: Option<String>,
        #[serde(default)]
        min_values: Option<u8>,
        #[serde(default)]
        max_values: Option<u8>,
        #[serde(default)]
        disabled: bool,
        #[serde(default)]
        channel_types: Vec<ChannelFilter>,
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

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TextInputStyle {
    #[default]
    Short,
    Paragraph,
}

const fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ModalInputSpec {
    pub id: String,
    pub label: String,
    #[serde(default)]
    pub style: TextInputStyle,
    #[serde(default)]
    pub placeholder: Option<String>,
    #[serde(default = "default_true")]
    pub required: bool,
    #[serde(default)]
    pub min_length: Option<u16>,
    #[serde(default)]
    pub max_length: Option<u16>,
    #[serde(default)]
    pub value: Option<String>,
}

const fn default_poll_hours() -> u32 {
    24
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PollSpec {
    pub question: String,
    pub answers: Vec<PollAnswerSpec>,
    #[serde(default = "default_poll_hours")]
    pub duration_hours: u32,
    #[serde(default)]
    pub allow_multiselect: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PollAnswerSpec {
    pub text: String,
    #[serde(default)]
    pub emoji: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AttachmentSpec {
    pub filename: String,
    pub content: String,
}
