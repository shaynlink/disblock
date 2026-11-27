import * as Blockly from "blockly";
import { registerDiscordPickerField } from "./discordPicker.ts";

const TRIGGER_COLOUR = "#6366f1";
const OPTION_COLOUR = "#ec4899";
const ACTION_COLOUR = "#22c55e";
const EMBED_COLOUR = "#2563eb";
const COMPONENT_COLOUR = "#f97316";
const THREAD_COLOUR = "#0d9488";
const ROLE_COLOUR = "#f59e0b";
const MODERATION_COLOUR = "#ef4444";
const LOGIC_COLOUR = "#0ea5e9";
const VALUE_COLOUR = "#14b8a6";
const VARIABLE_COLOUR = "#a855f7";
const FLOW_COLOUR = "#64748b";

const PLACEHOLDERS =
  "Placeholders: {user}, {user.name}, {user.id}, {channel}, {message.id}, {server.id}, {emoji}, {option.NAME}, {selected}, {modal.ID}, {target}, {target.id}, {var:NAME}, {member:NAME}, {server:NAME}";
const NAME_EXTENSION = "disblock_name_field";
const SUBCOMMAND_EXTENSION = "disblock_subcommand_fields";
const VARIABLE_EXTENSION = "disblock_variable_field";

const SCOPES = [
  ["this run's", "local"],
  ["member's", "member"],
  ["server's", "server"],
];

const ARCHIVE_DURATIONS = [
  ["1 hour", "60"],
  ["24 hours", "1440"],
  ["3 days", "4320"],
  ["1 week", "10080"],
];

/** Variable names: lowercase letters, digits and `_`, 32 characters at most. */
export function sanitizeVariable(value: string): string {
  return value
    .toLowerCase()
    .replace(/[\s-]+/g, "_")
    .replace(/[^a-z0-9_]/g, "")
    .slice(0, 32);
}

/** Discord names: lowercase letters, digits, `-` and `_`, 32 characters at most. */
export function sanitizeName(value: string): string {
  return value
    .toLowerCase()
    .replace(/\s+/g, "-")
    .replace(/[^a-z0-9_-]/g, "")
    .slice(0, 32);
}

const definitions = [
  {
    type: "slash_command",
    message0: "when /%1 is used",
    args0: [{ type: "field_input", name: "NAME", text: "hello" }],
    message1: "description %1",
    args1: [{ type: "field_input", name: "DESCRIPTION", text: "Say hello" }],
    message2: "options %1",
    args2: [{ type: "input_statement", name: "OPTIONS", check: "option" }],
    message3: "do %1",
    args3: [{ type: "input_statement", name: "ACTIONS", check: "action" }],
    colour: TRIGGER_COLOUR,
    extensions: [NAME_EXTENSION],
    tooltip: "A slash command members can type in your server.",
  },
  {
    type: "slash_subcommand",
    message0: "when /%1 group %2 subcommand %3 is used",
    args0: [
      { type: "field_input", name: "COMMAND", text: "admin" },
      { type: "field_input", name: "GROUP", text: "" },
      { type: "field_input", name: "SUBCOMMAND", text: "warn" },
    ],
    message1: "description %1",
    args1: [{ type: "field_input", name: "DESCRIPTION", text: "Run a subcommand" }],
    message2: "options %1",
    args2: [{ type: "input_statement", name: "OPTIONS", check: "option" }],
    message3: "do %1",
    args3: [{ type: "input_statement", name: "ACTIONS", check: "action" }],
    colour: TRIGGER_COLOUR,
    extensions: [SUBCOMMAND_EXTENSION],
    tooltip:
      "A slash subcommand (leave group blank for /command subcommand, or set group for /command group subcommand).",
  },
  {
    type: "user_command",
    message0: "when user menu %1 is used",
    args0: [{ type: "field_input", name: "NAME", text: "Inspect User" }],
    message1: "do %1",
    args1: [{ type: "input_statement", name: "ACTIONS", check: "action" }],
    colour: TRIGGER_COLOUR,
    tooltip:
      "Appears when right-clicking a user → Apps. Use {target}, {target.id}, or {target.name}.",
  },
  {
    type: "message_command",
    message0: "when message menu %1 is used",
    args0: [{ type: "field_input", name: "NAME", text: "Bookmark Message" }],
    message1: "do %1",
    args1: [{ type: "input_statement", name: "ACTIONS", check: "action" }],
    colour: TRIGGER_COLOUR,
    tooltip:
      "Appears when right-clicking a message → Apps. Use {target.id}, {target.content}, {target.author}, or {target.author.id}.",
  },
  {
    type: "modal_submit",
    message0: "when modal ID %1 is submitted",
    args0: [{ type: "field_input", name: "ID", text: "feedback_modal" }],
    message1: "do %1",
    args1: [{ type: "input_statement", name: "ACTIONS", check: "action" }],
    colour: TRIGGER_COLOUR,
    tooltip:
      "Runs when someone submits a modal opened with 'open modal'. Read inputs with {modal.ID}.",
  },
  {
    type: "member_join",
    message0: "when a member joins",
    message1: "do %1",
    args1: [{ type: "input_statement", name: "ACTIONS", check: "action" }],
    colour: TRIGGER_COLOUR,
    tooltip: "Runs when someone joins a server. Needs the Server Members intent.",
  },
  {
    type: "member_leave",
    message0: "when a member leaves",
    message1: "do %1",
    args1: [{ type: "input_statement", name: "ACTIONS", check: "action" }],
    colour: TRIGGER_COLOUR,
    tooltip: "Runs when someone leaves a server. Needs the Server Members intent.",
  },
  {
    type: "member_update",
    message0: "when a member's roles or profile update",
    message1: "do %1",
    args1: [{ type: "input_statement", name: "ACTIONS", check: "action" }],
    colour: TRIGGER_COLOUR,
    tooltip: "Runs when a member's roles or nickname change. Needs the Server Members intent.",
  },
  {
    type: "member_ban",
    message0: "when a member is banned",
    message1: "do %1",
    args1: [{ type: "input_statement", name: "ACTIONS", check: "action" }],
    colour: TRIGGER_COLOUR,
    tooltip: "Runs when a user is banned from a server. Use {user} and {user.id}.",
  },
  {
    type: "member_unban",
    message0: "when a user is unbanned",
    message1: "do %1",
    args1: [{ type: "input_statement", name: "ACTIONS", check: "action" }],
    colour: TRIGGER_COLOUR,
    tooltip: "Runs when a user is unbanned from a server. Use {user} and {user.id}.",
  },
  {
    type: "message_contains",
    message0: "when a message contains %1",
    args0: [{ type: "field_input", name: "PATTERN", text: "ping" }],
    message1: "match case %1",
    args1: [{ type: "field_checkbox", name: "CASE_SENSITIVE", checked: false }],
    message2: "do %1",
    args2: [{ type: "input_statement", name: "ACTIONS", check: "action" }],
    colour: TRIGGER_COLOUR,
    tooltip: "Runs when a message contains this text. Needs the Message Content intent.",
  },
  {
    type: "message_edit",
    message0: "when a message is edited",
    message1: "do %1",
    args1: [{ type: "input_statement", name: "ACTIONS", check: "action" }],
    colour: TRIGGER_COLOUR,
    tooltip:
      "Runs when a message is edited. Needs the Message Content intent. Use {message.content} and {channel}.",
  },
  {
    type: "message_delete",
    message0: "when a message is deleted",
    message1: "do %1",
    args1: [{ type: "input_statement", name: "ACTIONS", check: "action" }],
    colour: TRIGGER_COLOUR,
    tooltip: "Runs when a message is deleted in a channel. Use {message.id} and {channel}.",
  },
  {
    type: "reaction_add",
    message0: "when reaction %1 is added",
    args0: [{ type: "field_input", name: "EMOJI", text: "" }],
    message1: "on message ID %1",
    args1: [{ type: "field_input", name: "MESSAGE_ID", text: "" }],
    message2: "do %1",
    args2: [{ type: "input_statement", name: "ACTIONS", check: "action" }],
    colour: TRIGGER_COLOUR,
    tooltip:
      "Runs when someone reacts to a message. Leave emoji or message ID blank to match any. Use {emoji} and {message.id}.",
  },
  {
    type: "reaction_remove",
    message0: "when reaction %1 is removed",
    args0: [{ type: "field_input", name: "EMOJI", text: "" }],
    message1: "on message ID %1",
    args1: [{ type: "field_input", name: "MESSAGE_ID", text: "" }],
    message2: "do %1",
    args2: [{ type: "input_statement", name: "ACTIONS", check: "action" }],
    colour: TRIGGER_COLOUR,
    tooltip:
      "Runs when someone removes a reaction. Leave emoji or message ID blank to match any. Use {emoji} and {message.id}.",
  },
  {
    type: "thread_create",
    message0: "when a thread is created",
    message1: "do %1",
    args1: [{ type: "input_statement", name: "ACTIONS", check: "action" }],
    colour: TRIGGER_COLOUR,
    tooltip: "Runs when a new thread or forum post is created. Use {thread.name} and {channel}.",
  },
  {
    type: "invite_create",
    message0: "when an invite is created",
    message1: "do %1",
    args1: [{ type: "input_statement", name: "ACTIONS", check: "action" }],
    colour: TRIGGER_COLOUR,
    tooltip: "Runs when an invite link is created. Use {invite.code}, {user}, and {channel}.",
  },
  {
    type: "automod_execution",
    message0: "when AutoMod runs (rule ID %1)",
    args0: [{ type: "field_input", name: "RULE_ID", text: "" }],
    message1: "do %1",
    args1: [{ type: "input_statement", name: "ACTIONS", check: "action" }],
    colour: TRIGGER_COLOUR,
    tooltip:
      "Runs when Discord AutoMod blocks or flags content. Leave rule ID blank for any rule. Use {automod.keyword}, {automod.rule_id}, {user}.",
  },
  {
    type: "poll_vote_add",
    message0: "when someone votes on a poll",
    message1: "do %1",
    args1: [{ type: "input_statement", name: "ACTIONS", check: "action" }],
    colour: TRIGGER_COLOUR,
    tooltip: "Runs when a member votes on a poll. Use {user}, {poll.answer_id}, {message.id}.",
  },
  {
    type: "voice_state_update",
    message0: "when voice state changes",
    message1: "do %1",
    args1: [{ type: "input_statement", name: "ACTIONS", check: "action" }],
    colour: TRIGGER_COLOUR,
    tooltip:
      "Runs when a member joins, leaves, or switches a voice channel. Use {user} and {channel}.",
  },
  {
    type: "button_click",
    message0: "when button ID %1 is clicked",
    args0: [{ type: "field_input", name: "ID", text: "verify" }],
    message1: "do %1",
    args1: [{ type: "input_statement", name: "ACTIONS", check: "action" }],
    colour: TRIGGER_COLOUR,
    tooltip: "Runs when someone clicks a button with this ID.",
  },
  {
    type: "select_menu_trigger",
    message0: "when select menu ID %1 is chosen",
    args0: [{ type: "field_input", name: "ID", text: "menu" }],
    message1: "do %1",
    args1: [{ type: "input_statement", name: "ACTIONS", check: "action" }],
    colour: TRIGGER_COLOUR,
    tooltip:
      "Runs when someone picks an option or member/role/channel from a select menu with this ID. Use {selected} or the selected value block.",
  },
  {
    type: "schedule",
    message0: "every %1 %2",
    args0: [
      { type: "field_number", name: "INTERVAL", value: 1, min: 1, max: 43200, precision: 1 },
      {
        type: "field_dropdown",
        name: "UNIT",
        options: [
          ["minutes", "minutes"],
          ["hours", "hours"],
          ["days", "days"],
        ],
      },
    ],
    message1: "do %1",
    args1: [{ type: "input_statement", name: "ACTIONS", check: "action" }],
    colour: TRIGGER_COLOUR,
    tooltip:
      "Runs automatically at a fixed interval while the bot is online (1 minute to 30 days).",
  },
  {
    type: "command_option",
    message0: "option %1 %2 required %3",
    args0: [
      { type: "field_input", name: "NAME", text: "text" },
      {
        type: "field_dropdown",
        name: "KIND",
        options: [
          ["text", "string"],
          ["number", "integer"],
          ["decimal number", "number"],
          ["yes / no", "boolean"],
          ["member", "user"],
          ["channel", "channel"],
          ["role", "role"],
          ["member or role", "mentionable"],
          ["attachment", "attachment"],
        ],
      },
      { type: "field_checkbox", name: "REQUIRED", checked: true },
    ],
    message1: "description %1 autocomplete %2",
    args1: [
      { type: "field_input", name: "DESCRIPTION", text: "Some text" },
      { type: "field_input", name: "AUTOCOMPLETE", text: "" },
    ],
    message2: "choices %1",
    args2: [{ type: "input_statement", name: "CHOICES", check: "option_choice" }],
    previousStatement: "option",
    nextStatement: "option",
    colour: OPTION_COLOUR,
    extensions: [NAME_EXTENSION],
    tooltip:
      "Use its value in messages with {option.NAME}. Add optional fixed choices or comma-separated autocomplete suggestions.",
  },
  {
    type: "command_option_choice",
    message0: "choice label %1 value %2",
    args0: [
      { type: "field_input", name: "NAME", text: "Option A" },
      { type: "field_input", name: "VALUE", text: "a" },
    ],
    previousStatement: "option_choice",
    nextStatement: "option_choice",
    colour: OPTION_COLOUR,
    tooltip: "A fixed choice shown to users for text, number, or decimal options.",
  },
  {
    type: "reply",
    message0: "reply %1",
    args0: [{ type: "field_input", name: "CONTENT", text: "Hello {user}!" }],
    message1: "only visible to them %1",
    args1: [{ type: "field_checkbox", name: "EPHEMERAL", checked: false }],
    previousStatement: "action",
    nextStatement: "action",
    colour: ACTION_COLOUR,
    tooltip: `Answer the command or the message. ${PLACEHOLDERS}`,
  },
  {
    type: "reply_rich",
    message0: "reply with rich message %1",
    args0: [{ type: "field_input", name: "CONTENT", text: "" }],
    message1: "only visible to them %1",
    args1: [{ type: "field_checkbox", name: "EPHEMERAL", checked: false }],
    message2: "embeds %1",
    args2: [{ type: "input_statement", name: "EMBEDS", check: "embed" }],
    message3: "buttons & menus %1",
    args3: [{ type: "input_statement", name: "COMPONENTS", check: "component" }],
    message4: "poll %1",
    args4: [{ type: "input_statement", name: "POLL", check: "poll" }],
    message5: "files %1",
    args5: [{ type: "input_statement", name: "ATTACHMENTS", check: "attachment" }],
    previousStatement: "action",
    nextStatement: "action",
    colour: ACTION_COLOUR,
    tooltip: `Answer with optional text, embeds, buttons, select menus, poll, and files. ${PLACEHOLDERS}`,
  },
  {
    type: "send_message",
    message0: "send %1",
    args0: [{ type: "field_input", name: "CONTENT", text: "Welcome {user}!" }],
    message1: "in channel %1",
    args1: [{ type: "field_discord_resource", name: "CHANNEL_ID", text: "", resource: "channel" }],
    previousStatement: "action",
    nextStatement: "action",
    colour: ACTION_COLOUR,
    tooltip: `Pick a channel from your server or enter an ID / {template}. ${PLACEHOLDERS}`,
  },
  {
    type: "send_rich_message",
    message0: "send rich message %1",
    args0: [{ type: "field_input", name: "CONTENT", text: "" }],
    message1: "in channel %1 reply to ID %2",
    args1: [
      { type: "field_discord_resource", name: "CHANNEL_ID", text: "", resource: "channel" },
      { type: "field_input", name: "REPLY_TO", text: "" },
    ],
    message2: "embeds %1",
    args2: [{ type: "input_statement", name: "EMBEDS", check: "embed" }],
    message3: "buttons & menus %1",
    args3: [{ type: "input_statement", name: "COMPONENTS", check: "component" }],
    message4: "poll %1",
    args4: [{ type: "input_statement", name: "POLL", check: "poll" }],
    message5: "files %1",
    args5: [{ type: "input_statement", name: "ATTACHMENTS", check: "attachment" }],
    previousStatement: "action",
    nextStatement: "action",
    colour: ACTION_COLOUR,
    tooltip: `Send a rich message with optional text, embeds, buttons, menus, poll, files, or targeted reply ID. ${PLACEHOLDERS}`,
  },
  {
    type: "update_message",
    message0: "update clicked message %1",
    args0: [{ type: "field_input", name: "CONTENT", text: "" }],
    message1: "embeds %1",
    args1: [{ type: "input_statement", name: "EMBEDS", check: "embed" }],
    message2: "buttons & menus %1",
    args2: [{ type: "input_statement", name: "COMPONENTS", check: "component" }],
    previousStatement: "action",
    nextStatement: "action",
    colour: ACTION_COLOUR,
    tooltip: `Edit the message whose button or menu was clicked. Leave text and embeds empty to only update buttons and menus. ${PLACEHOLDERS}`,
  },
  {
    type: "message_poll",
    message0: "poll %1 duration %2 hours multiselect %3",
    args0: [
      { type: "field_input", name: "QUESTION", text: "What should we do?" },
      { type: "field_number", name: "DURATION", value: 24, min: 1, max: 768, precision: 1 },
      { type: "field_checkbox", name: "MULTISELECT", checked: false },
    ],
    message1: "answers %1",
    args1: [{ type: "input_statement", name: "ANSWERS", check: "poll_answer" }],
    previousStatement: "poll",
    colour: ACTION_COLOUR,
    tooltip: "Attach a native Discord poll (1 to 10 answers, up to 32 days).",
  },
  {
    type: "poll_answer",
    message0: "answer %1 emoji %2",
    args0: [
      { type: "field_input", name: "TEXT", text: "Yes" },
      { type: "field_input", name: "EMOJI", text: "" },
    ],
    previousStatement: "poll_answer",
    nextStatement: "poll_answer",
    colour: ACTION_COLOUR,
    tooltip: "One option in a Discord poll (up to 55 characters).",
  },
  {
    type: "message_attachment",
    message0: "file %1 content %2",
    args0: [
      { type: "field_input", name: "FILENAME", text: "summary.txt" },
      { type: "field_input", name: "CONTENT", text: "Report for {user.name}" },
    ],
    previousStatement: "attachment",
    nextStatement: "attachment",
    colour: ACTION_COLOUR,
    tooltip: `Attach a generated text/JSON/CSV file to the message. ${PLACEHOLDERS}`,
  },
  {
    type: "pin_message",
    message0: "pin message %1 in channel %2",
    args0: [
      { type: "field_input", name: "MESSAGE_ID", text: "{message.id}" },
      {
        type: "field_discord_resource",
        name: "CHANNEL_ID",
        text: "{channel.id}",
        resource: "channel",
      },
    ],
    previousStatement: "action",
    nextStatement: "action",
    colour: ACTION_COLOUR,
    tooltip: "Pins a message in a channel. Needs Manage Messages.",
  },
  {
    type: "unpin_message",
    message0: "unpin message %1 in channel %2",
    args0: [
      { type: "field_input", name: "MESSAGE_ID", text: "{message.id}" },
      {
        type: "field_discord_resource",
        name: "CHANNEL_ID",
        text: "{channel.id}",
        resource: "channel",
      },
    ],
    previousStatement: "action",
    nextStatement: "action",
    colour: ACTION_COLOUR,
    tooltip: "Unpins a message in a channel. Needs Manage Messages.",
  },
  {
    type: "publish_message",
    message0: "publish announcement %1 in channel %2",
    args0: [
      { type: "field_input", name: "MESSAGE_ID", text: "{message.id}" },
      {
        type: "field_discord_resource",
        name: "CHANNEL_ID",
        text: "{channel.id}",
        resource: "channel",
      },
    ],
    previousStatement: "action",
    nextStatement: "action",
    colour: ACTION_COLOUR,
    tooltip: "Crossposts an announcement channel message to following servers.",
  },
  {
    type: "bulk_delete_messages",
    message0: "bulk delete last %1 messages in channel %2",
    args0: [
      { type: "field_number", name: "COUNT", value: 10, min: 2, max: 100, precision: 1 },
      {
        type: "field_discord_resource",
        name: "CHANNEL_ID",
        text: "{channel.id}",
        resource: "channel",
      },
    ],
    previousStatement: "action",
    nextStatement: "action",
    colour: MODERATION_COLOUR,
    tooltip: "Deletes 2 to 100 recent messages (under 14 days old). Needs Manage Messages.",
  },
  {
    type: "create_thread_from_message",
    message0: "start thread %1 on message archive %2",
    args0: [
      { type: "field_input", name: "NAME", text: "Discussion" },
      { type: "field_dropdown", name: "ARCHIVE", options: ARCHIVE_DURATIONS },
    ],
    previousStatement: "action",
    nextStatement: "action",
    colour: THREAD_COLOUR,
    tooltip: "Starts a public thread on the triggering or right-clicked message.",
  },
  {
    type: "create_thread",
    message0: "create thread %1 in channel %2",
    args0: [
      { type: "field_input", name: "NAME", text: "New thread" },
      {
        type: "field_discord_resource",
        name: "CHANNEL_ID",
        text: "{channel.id}",
        resource: "channel",
      },
    ],
    message1: "private %1 archive %2",
    args1: [
      { type: "field_checkbox", name: "PRIVATE", checked: false },
      { type: "field_dropdown", name: "ARCHIVE", options: ARCHIVE_DURATIONS },
    ],
    previousStatement: "action",
    nextStatement: "action",
    colour: THREAD_COLOUR,
    tooltip: "Creates a public or private thread inside a text or announcement channel.",
  },
  {
    type: "create_forum_post",
    message0: "create forum post %1 in channel %2",
    args0: [
      { type: "field_input", name: "TITLE", text: "New topic" },
      { type: "field_discord_resource", name: "CHANNEL_ID", text: "", resource: "channel" },
    ],
    message1: "message %1",
    args1: [{ type: "field_input", name: "CONTENT", text: "Started by {user}" }],
    message2: "embeds %1",
    args2: [{ type: "input_statement", name: "EMBEDS", check: "embed" }],
    previousStatement: "action",
    nextStatement: "action",
    colour: THREAD_COLOUR,
    tooltip: "Creates a new post in a forum or media channel.",
  },
  {
    type: "join_thread",
    message0: "join thread %1",
    args0: [
      {
        type: "field_discord_resource",
        name: "THREAD_ID",
        text: "{channel.id}",
        resource: "channel",
      },
    ],
    previousStatement: "action",
    nextStatement: "action",
    colour: THREAD_COLOUR,
    tooltip: "Makes the bot join the thread.",
  },
  {
    type: "add_thread_member",
    message0: "add member %1 to thread %2",
    args0: [
      { type: "field_discord_resource", name: "MEMBER_ID", text: "{user.id}", resource: "member" },
      {
        type: "field_discord_resource",
        name: "THREAD_ID",
        text: "{channel.id}",
        resource: "channel",
      },
    ],
    previousStatement: "action",
    nextStatement: "action",
    colour: THREAD_COLOUR,
    tooltip: "Adds a member to a thread.",
  },
  {
    type: "remove_thread_member",
    message0: "remove member %1 from thread %2",
    args0: [
      { type: "field_discord_resource", name: "MEMBER_ID", text: "{user.id}", resource: "member" },
      {
        type: "field_discord_resource",
        name: "THREAD_ID",
        text: "{channel.id}",
        resource: "channel",
      },
    ],
    previousStatement: "action",
    nextStatement: "action",
    colour: THREAD_COLOUR,
    tooltip: "Removes a member from a thread.",
  },
  {
    type: "embed",
    message0: "embed title %1",
    args0: [{ type: "field_input", name: "TITLE", text: "Title" }],
    message1: "description %1",
    args1: [{ type: "field_input", name: "DESCRIPTION", text: "Message text" }],
    message2: "color %1 timestamp %2",
    args2: [
      { type: "field_input", name: "COLOR", text: "#5865F2" },
      { type: "field_checkbox", name: "TIMESTAMP", checked: false },
    ],
    message3: "extras %1",
    args3: [{ type: "input_statement", name: "PARTS", check: "embed_part" }],
    previousStatement: "embed",
    nextStatement: "embed",
    colour: EMBED_COLOUR,
    tooltip: `A rich Discord embed card. Color is a hex code like #5865F2. ${PLACEHOLDERS}`,
  },
  {
    type: "embed_field",
    message0: "field %1 value %2 inline %3",
    args0: [
      { type: "field_input", name: "NAME", text: "Field" },
      { type: "field_input", name: "VALUE", text: "Value" },
      { type: "field_checkbox", name: "INLINE", checked: false },
    ],
    previousStatement: "embed_part",
    nextStatement: "embed_part",
    colour: EMBED_COLOUR,
    tooltip: `Add a field to the embed (up to 25). ${PLACEHOLDERS}`,
  },
  {
    type: "embed_author",
    message0: "author %1 icon URL %2",
    args0: [
      { type: "field_input", name: "NAME", text: "{user.name}" },
      { type: "field_input", name: "ICON_URL", text: "" },
    ],
    message1: "link URL %1",
    args1: [{ type: "field_input", name: "URL", text: "" }],
    previousStatement: "embed_part",
    nextStatement: "embed_part",
    colour: EMBED_COLOUR,
    tooltip: `Set the author header of the embed. ${PLACEHOLDERS}`,
  },
  {
    type: "embed_footer",
    message0: "footer %1 icon URL %2",
    args0: [
      { type: "field_input", name: "TEXT", text: "Footer" },
      { type: "field_input", name: "ICON_URL", text: "" },
    ],
    previousStatement: "embed_part",
    nextStatement: "embed_part",
    colour: EMBED_COLOUR,
    tooltip: `Set the footer text of the embed. ${PLACEHOLDERS}`,
  },
  {
    type: "embed_image",
    message0: "image URL %1",
    args0: [{ type: "field_input", name: "URL", text: "" }],
    previousStatement: "embed_part",
    nextStatement: "embed_part",
    colour: EMBED_COLOUR,
    tooltip: "Large image shown at the bottom of the embed (http:// or https://).",
  },
  {
    type: "embed_thumbnail",
    message0: "thumbnail URL %1",
    args0: [{ type: "field_input", name: "URL", text: "" }],
    previousStatement: "embed_part",
    nextStatement: "embed_part",
    colour: EMBED_COLOUR,
    tooltip: "Small image shown at the top-right of the embed (http:// or https://).",
  },
  {
    type: "embed_url",
    message0: "title URL %1",
    args0: [{ type: "field_input", name: "URL", text: "" }],
    previousStatement: "embed_part",
    nextStatement: "embed_part",
    colour: EMBED_COLOUR,
    tooltip: "Makes the embed title a clickable link (http:// or https://).",
  },
  {
    type: "component_button",
    message0: "button %1 style %2 ID %3",
    args0: [
      { type: "field_input", name: "LABEL", text: "Click me" },
      {
        type: "field_dropdown",
        name: "STYLE",
        options: [
          ["blue", "primary"],
          ["grey", "secondary"],
          ["green", "success"],
          ["red", "danger"],
        ],
      },
      { type: "field_input", name: "ID", text: "verify" },
    ],
    message1: "emoji %1 disabled %2",
    args1: [
      { type: "field_input", name: "EMOJI", text: "" },
      { type: "field_checkbox", name: "DISABLED", checked: false },
    ],
    previousStatement: "component",
    nextStatement: "component",
    colour: COMPONENT_COLOUR,
    tooltip: `An interactive button. Handle clicks with "when button ID ... is clicked". ${PLACEHOLDERS}`,
  },
  {
    type: "component_link_button",
    message0: "link button %1 URL %2",
    args0: [
      { type: "field_input", name: "LABEL", text: "Open link" },
      { type: "field_input", name: "URL", text: "https://example.com" },
    ],
    message1: "emoji %1 disabled %2",
    args1: [
      { type: "field_input", name: "EMOJI", text: "" },
      { type: "field_checkbox", name: "DISABLED", checked: false },
    ],
    previousStatement: "component",
    nextStatement: "component",
    colour: COMPONENT_COLOUR,
    tooltip: `Opens a web URL when clicked (does not trigger the bot). ${PLACEHOLDERS}`,
  },
  {
    type: "component_select_menu",
    message0: "select menu ID %1 placeholder %2",
    args0: [
      { type: "field_input", name: "ID", text: "menu" },
      { type: "field_input", name: "PLACEHOLDER", text: "Choose an option…" },
    ],
    message1: "min %1 max %2 disabled %3",
    args1: [
      { type: "field_number", name: "MIN", value: 1, min: 1, max: 25, precision: 1 },
      { type: "field_number", name: "MAX", value: 1, min: 1, max: 25, precision: 1 },
      { type: "field_checkbox", name: "DISABLED", checked: false },
    ],
    message2: "options %1",
    args2: [{ type: "input_statement", name: "OPTIONS", check: "select_option" }],
    previousStatement: "component",
    nextStatement: "component",
    colour: COMPONENT_COLOUR,
    tooltip: `A dropdown menu (up to 25 options). Handle choices with "when select menu ID ... is chosen". ${PLACEHOLDERS}`,
  },
  {
    type: "component_entity_select_menu",
    message0: "Discord %1 menu ID %2 placeholder %3",
    args0: [
      {
        type: "field_dropdown",
        name: "KIND",
        options: [
          ["members", "user"],
          ["roles", "role"],
          ["members & roles", "mentionable"],
          ["channels", "channel"],
        ],
      },
      { type: "field_input", name: "ID", text: "entity_menu" },
      { type: "field_input", name: "PLACEHOLDER", text: "Pick from server…" },
    ],
    message1: "channels %1 min %2 max %3 disabled %4",
    args1: [
      {
        type: "field_dropdown",
        name: "CHANNEL_FILTER",
        options: [
          ["any", "any"],
          ["text only", "text"],
          ["voice only", "voice"],
          ["announcements", "announcement"],
          ["forums", "forum"],
        ],
      },
      { type: "field_number", name: "MIN", value: 1, min: 1, max: 25, precision: 1 },
      { type: "field_number", name: "MAX", value: 1, min: 1, max: 25, precision: 1 },
      { type: "field_checkbox", name: "DISABLED", checked: false },
    ],
    previousStatement: "component",
    nextStatement: "component",
    colour: COMPONENT_COLOUR,
    tooltip:
      "Auto-populated Discord picker for members, roles, or channels. Handle choices with 'when select menu ID ... is chosen' and use {selected}.",
  },
  {
    type: "select_option",
    message0: "option %1 value %2",
    args0: [
      { type: "field_input", name: "LABEL", text: "Option 1" },
      { type: "field_input", name: "VALUE", text: "option_1" },
    ],
    message1: "description %1 emoji %2 default %3",
    args1: [
      { type: "field_input", name: "DESCRIPTION", text: "" },
      { type: "field_input", name: "EMOJI", text: "" },
      { type: "field_checkbox", name: "DEFAULT", checked: false },
    ],
    previousStatement: "select_option",
    nextStatement: "select_option",
    colour: COMPONENT_COLOUR,
    tooltip: `One choice in a select menu. Its value becomes {selected} when chosen. ${PLACEHOLDERS}`,
  },
  {
    type: "component_new_row",
    message0: "new row",
    previousStatement: "component",
    nextStatement: "component",
    colour: COMPONENT_COLOUR,
    tooltip: "Starts a new line of buttons below the previous ones (up to 5 rows per message).",
  },
  {
    type: "show_modal",
    message0: "open modal ID %1 title %2",
    args0: [
      { type: "field_input", name: "ID", text: "feedback_modal" },
      { type: "field_input", name: "TITLE", text: "Send feedback" },
    ],
    message1: "inputs %1",
    args1: [{ type: "input_statement", name: "INPUTS", check: "modal_input" }],
    previousStatement: "action",
    nextStatement: "action",
    colour: COMPONENT_COLOUR,
    tooltip:
      "Opens a popup form (must be the first response on a command, button, or select menu). Handle submissions with 'when modal ID ... is submitted'.",
  },
  {
    type: "modal_text_input",
    message0: "input ID %1 label %2 style %3",
    args0: [
      { type: "field_input", name: "ID", text: "details" },
      { type: "field_input", name: "LABEL", text: "Details" },
      {
        type: "field_dropdown",
        name: "STYLE",
        options: [
          ["short line", "short"],
          ["paragraph", "paragraph"],
        ],
      },
    ],
    message1: "placeholder %1 default %2 required %3",
    args1: [
      { type: "field_input", name: "PLACEHOLDER", text: "" },
      { type: "field_input", name: "VALUE", text: "" },
      { type: "field_checkbox", name: "REQUIRED", checked: true },
    ],
    previousStatement: "modal_input",
    nextStatement: "modal_input",
    colour: COMPONENT_COLOUR,
    tooltip: "A text field inside a modal (up to 5 per modal). Read it on submit with {modal.ID}.",
  },
  {
    type: "react",
    message0: "react with %1",
    args0: [{ type: "field_input", name: "EMOJI", text: "👍" }],
    previousStatement: "action",
    nextStatement: "action",
    colour: ACTION_COLOUR,
    tooltip: "A unicode emoji, or a custom one written name:id.",
  },
  {
    type: "add_role",
    message0: "give role %1",
    args0: [{ type: "field_discord_resource", name: "ROLE_ID", text: "", resource: "role" }],
    previousStatement: "action",
    nextStatement: "action",
    colour: ROLE_COLOUR,
    tooltip: "Pick a role or enter {option.role} / {selected}. The bot's role must be above it.",
  },
  {
    type: "remove_role",
    message0: "remove role %1",
    args0: [{ type: "field_discord_resource", name: "ROLE_ID", text: "", resource: "role" }],
    previousStatement: "action",
    nextStatement: "action",
    colour: ROLE_COLOUR,
    tooltip: "Pick a role or enter {option.role} / {selected}. The bot's role must be above it.",
  },
  {
    type: "timeout_member",
    message0: "timeout member %1 for %2 minutes",
    args0: [
      { type: "field_discord_resource", name: "MEMBER_ID", text: "{user.id}", resource: "member" },
      { type: "field_number", name: "MINUTES", value: 10, min: 1, max: 40320, precision: 1 },
    ],
    message1: "reason %1",
    args1: [{ type: "field_input", name: "REASON", text: "" }],
    previousStatement: "action",
    nextStatement: "action",
    colour: MODERATION_COLOUR,
    tooltip:
      "Prevents a member from communicating for up to 28 days. Use {user.id}, {target.id}, or {option.member}. The bot needs Moderate Members.",
  },
  {
    type: "kick_member",
    message0: "kick member %1",
    args0: [
      { type: "field_discord_resource", name: "MEMBER_ID", text: "{user.id}", resource: "member" },
    ],
    message1: "reason %1",
    args1: [{ type: "field_input", name: "REASON", text: "" }],
    previousStatement: "action",
    nextStatement: "action",
    colour: MODERATION_COLOUR,
    tooltip:
      "Removes a member from the server. Use {user.id}, {target.id}, or {option.member}. The bot needs Kick Members.",
  },
  {
    type: "ban_member",
    message0: "ban member %1",
    args0: [
      { type: "field_discord_resource", name: "MEMBER_ID", text: "{user.id}", resource: "member" },
    ],
    message1: "reason %1",
    args1: [{ type: "field_input", name: "REASON", text: "" }],
    previousStatement: "action",
    nextStatement: "action",
    colour: MODERATION_COLOUR,
    tooltip:
      "Bans a member from the server. Use {user.id}, {target.id}, or {option.member}. The bot needs Ban Members.",
  },
  {
    type: "delete_triggering_message",
    message0: "delete triggering message",
    previousStatement: "action",
    nextStatement: "action",
    colour: MODERATION_COLOUR,
    tooltip:
      "Deletes the message that matched a message trigger. The bot needs Manage Messages in that channel.",
  },
  {
    type: "direct_message_member",
    message0: "direct message member %1",
    args0: [
      { type: "field_discord_resource", name: "MEMBER_ID", text: "{user.id}", resource: "member" },
    ],
    message1: "message %1",
    args1: [{ type: "field_input", name: "CONTENT", text: "Please review the server rules." }],
    previousStatement: "action",
    nextStatement: "action",
    colour: MODERATION_COLOUR,
    tooltip: "Sends a private message. Use {user.id}, {target.id}, or {option.member}.",
  },
  {
    type: "logic_if",
    message0: "if %1",
    args0: [{ type: "input_value", name: "CONDITION" }],
    message1: "then %1",
    args1: [{ type: "input_statement", name: "THEN", check: "action" }],
    message2: "else %1",
    args2: [{ type: "input_statement", name: "ELSE", check: "action" }],
    previousStatement: "action",
    nextStatement: "action",
    colour: LOGIC_COLOUR,
    tooltip: "Runs the first branch when the condition holds, the second one otherwise.",
  },
  {
    type: "logic_compare",
    message0: "%1 %2 %3",
    args0: [
      { type: "input_value", name: "LEFT" },
      {
        type: "field_dropdown",
        name: "OP",
        options: [
          ["=", "equal"],
          ["≠", "not_equal"],
          ["<", "less"],
          ["≤", "less_or_equal"],
          [">", "greater"],
          ["≥", "greater_or_equal"],
          ["contains", "contains"],
        ],
      },
      { type: "input_value", name: "RIGHT" },
    ],
    inputsInline: true,
    output: null,
    colour: LOGIC_COLOUR,
    tooltip: "Numbers compare as numbers, everything else as text. Contains ignores case.",
  },
  {
    type: "logic_operation",
    message0: "%1 %2 %3",
    args0: [
      { type: "input_value", name: "LEFT" },
      {
        type: "field_dropdown",
        name: "OP",
        options: [
          ["and", "and"],
          ["or", "or"],
        ],
      },
      { type: "input_value", name: "RIGHT" },
    ],
    inputsInline: true,
    output: null,
    colour: LOGIC_COLOUR,
  },
  {
    type: "logic_not",
    message0: "not %1",
    args0: [{ type: "input_value", name: "VALUE" }],
    output: null,
    colour: LOGIC_COLOUR,
  },
  {
    type: "logic_boolean",
    message0: "%1",
    args0: [
      {
        type: "field_dropdown",
        name: "VALUE",
        options: [
          ["true", "true"],
          ["false", "false"],
        ],
      },
    ],
    output: null,
    colour: LOGIC_COLOUR,
  },
  {
    type: "has_role",
    message0: "member has role %1",
    args0: [{ type: "field_discord_resource", name: "ROLE_ID", text: "", resource: "role" }],
    output: null,
    colour: LOGIC_COLOUR,
    tooltip: "Whether the member who triggered this has the role.",
  },
  {
    type: "value_text",
    message0: "“ %1 ”",
    args0: [{ type: "field_input", name: "TEXT", text: "" }],
    output: null,
    colour: VALUE_COLOUR,
    tooltip: `Some text. ${PLACEHOLDERS}`,
  },
  {
    type: "value_number",
    message0: "%1",
    args0: [{ type: "field_number", name: "NUM", value: 0, precision: 1 }],
    output: null,
    colour: VALUE_COLOUR,
  },
  {
    type: "value_option",
    message0: "option %1",
    args0: [{ type: "field_input", name: "NAME", text: "text" }],
    output: null,
    colour: VALUE_COLOUR,
    extensions: [NAME_EXTENSION],
    tooltip: "The value given to this slash command option, empty when it was left out.",
  },
  {
    type: "value_selected",
    message0: "selected value",
    output: null,
    colour: VALUE_COLOUR,
    tooltip: "The option value(s) or ID(s) chosen in a select menu trigger.",
  },
  {
    type: "value_modal_field",
    message0: "modal field %1",
    args0: [{ type: "field_input", name: "ID", text: "details" }],
    output: null,
    colour: VALUE_COLOUR,
    tooltip: "The text submitted in a modal input with this ID.",
  },
  {
    type: "value_target",
    message0: "target %1",
    args0: [
      {
        type: "field_dropdown",
        name: "FIELD",
        options: [
          ["ID", "id"],
          ["mention", "mention"],
          ["name", "name"],
          ["message content", "content"],
          ["author ID", "author_id"],
          ["author mention", "author_mention"],
        ],
      },
    ],
    output: null,
    colour: VALUE_COLOUR,
    tooltip: "Information about the right-clicked user or message in a context menu command.",
  },
  {
    type: "value_random",
    message0: "random from %1 to %2",
    args0: [
      { type: "input_value", name: "MIN" },
      { type: "input_value", name: "MAX" },
    ],
    inputsInline: true,
    output: null,
    colour: VALUE_COLOUR,
    tooltip: "A whole number between the two, both included.",
  },
  {
    type: "value_math",
    message0: "%1 %2 %3",
    args0: [
      { type: "input_value", name: "LEFT" },
      {
        type: "field_dropdown",
        name: "OP",
        options: [
          ["+", "add"],
          ["−", "subtract"],
          ["×", "multiply"],
          ["÷", "divide"],
          ["mod", "modulo"],
        ],
      },
      { type: "input_value", name: "RIGHT" },
    ],
    inputsInline: true,
    output: null,
    colour: VALUE_COLOUR,
    tooltip: "Whole numbers only: division drops the remainder.",
  },
  {
    type: "variable_get",
    message0: "%1 %2",
    args0: [
      { type: "field_dropdown", name: "SCOPE", options: SCOPES },
      { type: "field_input", name: "NAME", text: "points" },
    ],
    output: null,
    colour: VARIABLE_COLOUR,
    extensions: [VARIABLE_EXTENSION],
    tooltip: "A variable's value. Member and server variables are saved between runs.",
  },
  {
    type: "variable_set",
    message0: "set %1 %2 to %3",
    args0: [
      { type: "field_dropdown", name: "SCOPE", options: SCOPES },
      { type: "field_input", name: "NAME", text: "points" },
      { type: "input_value", name: "VALUE" },
    ],
    previousStatement: "action",
    nextStatement: "action",
    colour: VARIABLE_COLOUR,
    extensions: [VARIABLE_EXTENSION],
    tooltip: "Member and server variables are saved between runs.",
  },
  {
    type: "variable_change",
    message0: "change %1 %2 by %3",
    args0: [
      { type: "field_dropdown", name: "SCOPE", options: SCOPES },
      { type: "field_input", name: "NAME", text: "points" },
      { type: "input_value", name: "BY" },
    ],
    previousStatement: "action",
    nextStatement: "action",
    colour: VARIABLE_COLOUR,
    extensions: [VARIABLE_EXTENSION],
    tooltip: "Adds a number to the variable, which starts at 0.",
  },
  {
    type: "flow_wait",
    message0: "wait %1 seconds",
    args0: [{ type: "field_number", name: "SECONDS", value: 5, min: 1, max: 300, precision: 1 }],
    previousStatement: "action",
    nextStatement: "action",
    colour: FLOW_COLOUR,
    tooltip: "Up to 300 seconds, and 10 minutes for a whole run.",
  },
  {
    type: "flow_stop",
    message0: "stop",
    previousStatement: "action",
    colour: FLOW_COLOUR,
    tooltip: "Ends this trigger's run here.",
  },
];

let registered = false;

/** Define Disblock's blocks once per page load. */
export function registerBlocks(): void {
  if (registered) {
    return;
  }
  registerDiscordPickerField();
  if (!Blockly.Extensions.isRegistered(NAME_EXTENSION)) {
    Blockly.Extensions.register(NAME_EXTENSION, function (this: Blockly.Block) {
      this.getField("NAME")?.setValidator((value: string) => sanitizeName(value) || null);
    });
  }
  if (!Blockly.Extensions.isRegistered(SUBCOMMAND_EXTENSION)) {
    Blockly.Extensions.register(SUBCOMMAND_EXTENSION, function (this: Blockly.Block) {
      this.getField("COMMAND")?.setValidator((value: string) => sanitizeName(value) || null);
      this.getField("SUBCOMMAND")?.setValidator((value: string) => sanitizeName(value) || null);
      this.getField("GROUP")?.setValidator((value: string) =>
        value.trim() ? sanitizeName(value) || null : "",
      );
    });
  }
  if (!Blockly.Extensions.isRegistered(VARIABLE_EXTENSION)) {
    Blockly.Extensions.register(VARIABLE_EXTENSION, function (this: Blockly.Block) {
      this.getField("NAME")?.setValidator((value: string) => sanitizeVariable(value) || null);
    });
  }
  Blockly.common.defineBlocksWithJsonArray(definitions);
  registered = true;
}

export const toolbox: Blockly.utils.toolbox.ToolboxDefinition = {
  kind: "categoryToolbox",
  contents: [
    {
      kind: "category",
      name: "Triggers",
      colour: TRIGGER_COLOUR,
      contents: [
        { kind: "block", type: "slash_command" },
        { kind: "block", type: "slash_subcommand" },
        { kind: "block", type: "user_command" },
        { kind: "block", type: "message_command" },
        { kind: "block", type: "button_click" },
        { kind: "block", type: "select_menu_trigger" },
        { kind: "block", type: "modal_submit" },
        { kind: "block", type: "member_join" },
        { kind: "block", type: "member_leave" },
        { kind: "block", type: "member_update" },
        { kind: "block", type: "member_ban" },
        { kind: "block", type: "member_unban" },
        { kind: "block", type: "message_contains" },
        { kind: "block", type: "message_edit" },
        { kind: "block", type: "message_delete" },
        { kind: "block", type: "reaction_add" },
        { kind: "block", type: "reaction_remove" },
        { kind: "block", type: "thread_create" },
        { kind: "block", type: "invite_create" },
        { kind: "block", type: "automod_execution" },
        { kind: "block", type: "poll_vote_add" },
        { kind: "block", type: "voice_state_update" },
        { kind: "block", type: "schedule" },
      ],
    },
    {
      kind: "category",
      name: "Options",
      colour: OPTION_COLOUR,
      contents: [
        { kind: "block", type: "command_option" },
        { kind: "block", type: "command_option_choice" },
      ],
    },
    {
      kind: "category",
      name: "Logic",
      colour: LOGIC_COLOUR,
      contents: [
        { kind: "block", type: "logic_if" },
        {
          kind: "block",
          type: "logic_compare",
          inputs: {
            LEFT: { block: { type: "variable_get", fields: { SCOPE: "member", NAME: "points" } } },
            RIGHT: { block: { type: "value_number", fields: { NUM: 10 } } },
          },
        },
        { kind: "block", type: "logic_operation" },
        { kind: "block", type: "logic_not" },
        { kind: "block", type: "has_role" },
        { kind: "block", type: "logic_boolean" },
      ],
    },
    {
      kind: "category",
      name: "Values",
      colour: VALUE_COLOUR,
      contents: [
        { kind: "block", type: "value_text" },
        { kind: "block", type: "value_number" },
        { kind: "block", type: "value_option" },
        { kind: "block", type: "value_selected" },
        { kind: "block", type: "value_modal_field" },
        { kind: "block", type: "value_target" },
        {
          kind: "block",
          type: "value_random",
          inputs: {
            MIN: { block: { type: "value_number", fields: { NUM: 1 } } },
            MAX: { block: { type: "value_number", fields: { NUM: 6 } } },
          },
        },
        { kind: "block", type: "value_math" },
      ],
    },
    {
      kind: "category",
      name: "Variables",
      colour: VARIABLE_COLOUR,
      contents: [
        {
          kind: "block",
          type: "variable_change",
          fields: { SCOPE: "member" },
          inputs: { BY: { block: { type: "value_number", fields: { NUM: 1 } } } },
        },
        {
          kind: "block",
          type: "variable_set",
          fields: { SCOPE: "member" },
          inputs: { VALUE: { block: { type: "value_number", fields: { NUM: 0 } } } },
        },
        { kind: "block", type: "variable_get", fields: { SCOPE: "member" } },
      ],
    },
    {
      kind: "category",
      name: "Messages",
      colour: ACTION_COLOUR,
      contents: [
        { kind: "block", type: "reply" },
        { kind: "block", type: "reply_rich" },
        { kind: "block", type: "send_message" },
        { kind: "block", type: "send_rich_message" },
        { kind: "block", type: "update_message" },
        {
          kind: "block",
          type: "message_poll",
          inputs: {
            ANSWERS: {
              block: {
                type: "poll_answer",
                fields: { TEXT: "Yes", EMOJI: "👍" },
                next: {
                  block: {
                    type: "poll_answer",
                    fields: { TEXT: "No", EMOJI: "👎" },
                  },
                },
              },
            },
          },
        },
        { kind: "block", type: "poll_answer" },
        { kind: "block", type: "message_attachment" },
        { kind: "block", type: "react" },
        { kind: "block", type: "pin_message" },
        { kind: "block", type: "unpin_message" },
        { kind: "block", type: "publish_message" },
      ],
    },
    {
      kind: "category",
      name: "Embeds",
      colour: EMBED_COLOUR,
      contents: [
        { kind: "block", type: "embed" },
        { kind: "block", type: "embed_field" },
        { kind: "block", type: "embed_author" },
        { kind: "block", type: "embed_footer" },
        { kind: "block", type: "embed_image" },
        { kind: "block", type: "embed_thumbnail" },
        { kind: "block", type: "embed_url" },
      ],
    },
    {
      kind: "category",
      name: "Buttons, Menus & Modals",
      colour: COMPONENT_COLOUR,
      contents: [
        { kind: "block", type: "component_button" },
        { kind: "block", type: "component_link_button" },
        {
          kind: "block",
          type: "component_select_menu",
          inputs: {
            OPTIONS: {
              block: {
                type: "select_option",
                fields: { LABEL: "Option 1", VALUE: "option_1" },
                next: {
                  block: {
                    type: "select_option",
                    fields: { LABEL: "Option 2", VALUE: "option_2" },
                  },
                },
              },
            },
          },
        },
        { kind: "block", type: "component_entity_select_menu" },
        { kind: "block", type: "select_option" },
        { kind: "block", type: "component_new_row" },
        {
          kind: "block",
          type: "show_modal",
          inputs: {
            INPUTS: {
              block: {
                type: "modal_text_input",
                fields: { ID: "details", LABEL: "Details", STYLE: "paragraph", REQUIRED: true },
              },
            },
          },
        },
        { kind: "block", type: "modal_text_input" },
      ],
    },
    {
      kind: "category",
      name: "Threads & Forums",
      colour: THREAD_COLOUR,
      contents: [
        { kind: "block", type: "create_thread_from_message" },
        { kind: "block", type: "create_thread" },
        { kind: "block", type: "create_forum_post" },
        { kind: "block", type: "join_thread" },
        { kind: "block", type: "add_thread_member" },
        { kind: "block", type: "remove_thread_member" },
      ],
    },
    {
      kind: "category",
      name: "Roles",
      colour: ROLE_COLOUR,
      contents: [
        { kind: "block", type: "add_role" },
        { kind: "block", type: "remove_role" },
      ],
    },
    {
      kind: "category",
      name: "Moderation",
      colour: MODERATION_COLOUR,
      contents: [
        { kind: "block", type: "timeout_member" },
        { kind: "block", type: "kick_member" },
        { kind: "block", type: "ban_member" },
        { kind: "block", type: "delete_triggering_message" },
        { kind: "block", type: "bulk_delete_messages" },
        { kind: "block", type: "direct_message_member" },
      ],
    },
    {
      kind: "category",
      name: "Flow",
      colour: FLOW_COLOUR,
      contents: [
        { kind: "block", type: "flow_wait" },
        { kind: "block", type: "flow_stop" },
      ],
    },
  ],
};

/** Light theme matching the app's neutral palette. */
export function createTheme(): Blockly.Theme {
  return Blockly.Theme.defineTheme("disblock", {
    name: "disblock",
    base: Blockly.Themes.Classic,
    componentStyles: {
      workspaceBackgroundColour: "#ffffff",
      toolboxBackgroundColour: "#ffffff",
      toolboxForegroundColour: "#171717",
      flyoutBackgroundColour: "#f5f5f5",
      flyoutForegroundColour: "#404040",
      flyoutOpacity: 1,
      scrollbarColour: "#a3a3a3",
      scrollbarOpacity: 0.5,
      insertionMarkerColour: "#171717",
      insertionMarkerOpacity: 0.2,
      cursorColour: "#171717",
    },
    fontStyle: { family: "Inter Variable, system-ui, sans-serif", size: 11 },
  });
}

/** Shown in a bot's editor until its owner saves something. */
export const starterWorkspace = {
  blocks: {
    languageVersion: 0,
    blocks: [
      {
        type: "slash_command",
        x: 40,
        y: 40,
        fields: { NAME: "ping", DESCRIPTION: "Check that the bot is alive" },
        inputs: {
          ACTIONS: {
            block: { type: "reply", fields: { CONTENT: "Pong! 🏓", EPHEMERAL: false } },
          },
        },
      },
    ],
  },
};
