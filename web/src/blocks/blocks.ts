import * as Blockly from "blockly";

const TRIGGER_COLOUR = "#5865f2";
const OPTION_COLOUR = "#eb459e";
const ACTION_COLOUR = "#23a55a";
const ROLE_COLOUR = "#f0b232";

const PLACEHOLDERS =
  "Placeholders: {user}, {user.name}, {user.id}, {channel}, {server.id}, {option.NAME}";
const NAME_EXTENSION = "disblock_name_field";

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
          ["yes / no", "boolean"],
          ["member", "user"],
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
        { kind: "block", type: "message_contains" },
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
      name: "Messages",
      colour: ACTION_COLOUR,
      contents: [
        { kind: "block", type: "reply" },
        { kind: "block", type: "send_message" },
        { kind: "block", type: "react" },
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
  ],
};

export function createTheme(): Blockly.Theme {
  return Blockly.Theme.defineTheme("disblock", {
    name: "disblock",
    base: Blockly.Themes.Classic,
    componentStyles: {
      workspaceBackgroundColour: "#1e1f22",
      toolboxBackgroundColour: "#2b2d31",
      toolboxForegroundColour: "#f2f3f5",
      flyoutBackgroundColour: "#313338",
      flyoutForegroundColour: "#dbdee1",
      flyoutOpacity: 0.97,
      scrollbarColour: "#4e5058",
      scrollbarOpacity: 0.7,
      insertionMarkerColour: "#ffffff",
      insertionMarkerOpacity: 0.3,
      cursorColour: "#dbdee1",
    },
    fontStyle: { family: "inherit", size: 11 },
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
