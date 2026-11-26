import * as Blockly from "blockly";

const TRIGGER_COLOUR = "#6366f1";
const OPTION_COLOUR = "#ec4899";
const ACTION_COLOUR = "#22c55e";
const ROLE_COLOUR = "#f59e0b";
const LOGIC_COLOUR = "#0ea5e9";
const VALUE_COLOUR = "#14b8a6";
const VARIABLE_COLOUR = "#a855f7";
const FLOW_COLOUR = "#64748b";

const PLACEHOLDERS =
  "Placeholders: {user}, {user.name}, {user.id}, {channel}, {server.id}, {option.NAME}, {var:NAME}, {member:NAME}, {server:NAME}";
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
