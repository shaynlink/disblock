import * as Blockly from "blockly";

const TRIGGER_COLOUR = "#6366f1";
const OPTION_COLOUR = "#ec4899";
const ACTION_COLOUR = "#22c55e";
const EMBED_COLOUR = "#2563eb";
const COMPONENT_COLOUR = "#f97316";
const ROLE_COLOUR = "#f59e0b";
const MODERATION_COLOUR = "#ef4444";
const LOGIC_COLOUR = "#0ea5e9";
const VALUE_COLOUR = "#14b8a6";
const VARIABLE_COLOUR = "#a855f7";
const FLOW_COLOUR = "#64748b";

const PLACEHOLDERS =
  "Placeholders: {user}, {user.name}, {user.id}, {channel}, {message.id}, {server.id}, {emoji}, {option.NAME}, {selected}, {var:NAME}, {member:NAME}, {server:NAME}";
const NAME_EXTENSION = "disblock_name_field";
const VARIABLE_EXTENSION = "disblock_variable_field";

const SCOPES = [
  ["this run's", "local"],
  ["member's", "member"],
  ["server's", "server"],
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
      "Runs when someone picks an option from a select menu with this ID. Use {selected} or the selected value block.",
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
    message1: "description %1",
    args1: [{ type: "field_input", name: "DESCRIPTION", text: "Some text" }],
    previousStatement: "option",
    nextStatement: "option",
    colour: OPTION_COLOUR,
    extensions: [NAME_EXTENSION],
    tooltip: "Use its value in messages with {option.NAME}.",
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
    previousStatement: "action",
    nextStatement: "action",
    colour: ACTION_COLOUR,
    tooltip: `Answer with optional text, embeds, buttons, and select menus. ${PLACEHOLDERS}`,
  },
  {
    type: "send_message",
    message0: "send %1",
    args0: [{ type: "field_input", name: "CONTENT", text: "Welcome {user}!" }],
    message1: "in channel ID %1",
    args1: [{ type: "field_input", name: "CHANNEL_ID", text: "" }],
    previousStatement: "action",
    nextStatement: "action",
    colour: ACTION_COLOUR,
    tooltip: `Right-click a channel in Discord and copy its ID (Developer Mode). ${PLACEHOLDERS}`,
  },
  {
    type: "send_rich_message",
    message0: "send rich message %1",
    args0: [{ type: "field_input", name: "CONTENT", text: "" }],
    message1: "in channel ID %1",
    args1: [{ type: "field_input", name: "CHANNEL_ID", text: "" }],
    message2: "embeds %1",
    args2: [{ type: "input_statement", name: "EMBEDS", check: "embed" }],
    message3: "buttons & menus %1",
    args3: [{ type: "input_statement", name: "COMPONENTS", check: "component" }],
    previousStatement: "action",
    nextStatement: "action",
    colour: ACTION_COLOUR,
    tooltip: `Send a rich message with optional text, embeds, buttons, and select menus. ${PLACEHOLDERS}`,
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
    message0: "give role ID %1",
    args0: [{ type: "field_input", name: "ROLE_ID", text: "" }],
    previousStatement: "action",
    nextStatement: "action",
    colour: ROLE_COLOUR,
    tooltip: "Gives the role to the member. The bot's role must be above it.",
  },
  {
    type: "remove_role",
    message0: "remove role ID %1",
    args0: [{ type: "field_input", name: "ROLE_ID", text: "" }],
    previousStatement: "action",
    nextStatement: "action",
    colour: ROLE_COLOUR,
    tooltip: "Removes the role from the member. The bot's role must be above it.",
  },
  {
    type: "timeout_member",
    message0: "timeout member %1 for %2 minutes",
    args0: [
      { type: "field_input", name: "MEMBER_ID", text: "{user.id}" },
      { type: "field_number", name: "MINUTES", value: 10, min: 1, max: 40320, precision: 1 },
    ],
    message1: "reason %1",
    args1: [{ type: "field_input", name: "REASON", text: "" }],
    previousStatement: "action",
    nextStatement: "action",
    colour: MODERATION_COLOUR,
    tooltip:
      "Prevents a member from communicating for up to 28 days. Use {user.id} for the member who triggered this, or {option.member} for a user command option. The bot needs Moderate Members.",
  },
  {
    type: "kick_member",
    message0: "kick member %1",
    args0: [{ type: "field_input", name: "MEMBER_ID", text: "{user.id}" }],
    message1: "reason %1",
    args1: [{ type: "field_input", name: "REASON", text: "" }],
    previousStatement: "action",
    nextStatement: "action",
    colour: MODERATION_COLOUR,
    tooltip:
      "Removes a member from the server. Use {user.id} for the member who triggered this, or {option.member} for a user command option. The bot needs Kick Members.",
  },
  {
    type: "ban_member",
    message0: "ban member %1",
    args0: [{ type: "field_input", name: "MEMBER_ID", text: "{user.id}" }],
    message1: "reason %1",
    args1: [{ type: "field_input", name: "REASON", text: "" }],
    previousStatement: "action",
    nextStatement: "action",
    colour: MODERATION_COLOUR,
    tooltip:
      "Bans a member from the server. Use {user.id} for the member who triggered this, or {option.member} for a user command option. The bot needs Ban Members.",
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
    args0: [{ type: "field_input", name: "MEMBER_ID", text: "{user.id}" }],
    message1: "message %1",
    args1: [{ type: "field_input", name: "CONTENT", text: "Please review the server rules." }],
    previousStatement: "action",
    nextStatement: "action",
    colour: MODERATION_COLOUR,
    tooltip:
      "Sends a private message. Use {user.id} for the member who triggered this, or {option.member} for a user command option.",
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
    message0: "member has role ID %1",
    args0: [{ type: "field_input", name: "ROLE_ID", text: "" }],
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
    tooltip: "The option value(s) chosen in a select menu trigger.",
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
  if (!Blockly.Extensions.isRegistered(NAME_EXTENSION)) {
    Blockly.Extensions.register(NAME_EXTENSION, function (this: Blockly.Block) {
      this.getField("NAME")?.setValidator((value: string) => sanitizeName(value) || null);
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
        { kind: "block", type: "member_join" },
        { kind: "block", type: "member_leave" },
        { kind: "block", type: "message_contains" },
        { kind: "block", type: "reaction_add" },
        { kind: "block", type: "reaction_remove" },
        { kind: "block", type: "button_click" },
        { kind: "block", type: "select_menu_trigger" },
        { kind: "block", type: "schedule" },
      ],
    },
    {
      kind: "category",
      name: "Options",
      colour: OPTION_COLOUR,
      contents: [{ kind: "block", type: "command_option" }],
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
        { kind: "block", type: "react" },
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
      name: "Buttons & Menus",
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
        { kind: "block", type: "select_option" },
        { kind: "block", type: "component_new_row" },
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
