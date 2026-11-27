import * as Blockly from "blockly";
import { beforeAll, describe, expect, it } from "vite-plus/test";
import { registerBlocks, sanitizeName, sanitizeVariable, starterWorkspace } from "./blocks.ts";
import {
  GenerateError,
  findMissingIntents,
  generateProgram,
  generateProgramWithMap,
  resolveErrorBlockId,
} from "./generate.ts";

beforeAll(() => {
  registerBlocks();
});

function workspaceFrom(state: object): Blockly.Workspace {
  const workspace = new Blockly.Workspace();
  Blockly.serialization.workspaces.load(state, workspace);
  return workspace;
}

describe("generateProgram", () => {
  it("turns the starter workspace into a /ping command", () => {
    expect(generateProgram(workspaceFrom(starterWorkspace))).toEqual({
      handlers: [
        {
          trigger: {
            type: "slash_command",
            name: "ping",
            description: "Check that the bot is alive",
            options: [],
          },
          actions: [{ type: "reply", content: "Pong! 🏓", ephemeral: false }],
        },
      ],
    });
  });

  it("reads options, stacked actions and every trigger, top to bottom", () => {
    const workspace = workspaceFrom({
      blocks: {
        languageVersion: 0,
        blocks: [
          {
            type: "message_contains",
            x: 0,
            y: 300,
            fields: { PATTERN: "gg", CASE_SENSITIVE: true },
            inputs: { ACTIONS: { block: { type: "react", fields: { EMOJI: "🎉" } } } },
          },
          {
            type: "slash_command",
            x: 0,
            y: 0,
            fields: { NAME: "greet", DESCRIPTION: "Greet someone" },
            inputs: {
              OPTIONS: {
                block: {
                  type: "command_option",
                  fields: { NAME: "who", KIND: "user", REQUIRED: true, DESCRIPTION: "Who" },
                },
              },
              ACTIONS: {
                block: {
                  type: "reply",
                  fields: { CONTENT: "Hi {option.who}", EPHEMERAL: true },
                  next: {
                    block: { type: "add_role", fields: { ROLE_ID: " 123456789012345678 " } },
                  },
                },
              },
            },
          },
          { type: "member_join", x: 0, y: 600 },
          { type: "reply", x: 400, y: 0, fields: { CONTENT: "orphan" } },
        ],
      },
    });

    expect(generateProgram(workspace).handlers).toEqual([
      {
        trigger: {
          type: "slash_command",
          name: "greet",
          description: "Greet someone",
          options: [{ name: "who", description: "Who", kind: "user", required: true }],
        },
        actions: [
          { type: "reply", content: "Hi {option.who}", ephemeral: true },
          { type: "add_role", role_id: "123456789012345678" },
        ],
      },
      {
        trigger: { type: "message_contains", pattern: "gg", case_sensitive: true },
        actions: [{ type: "react", emoji: "🎉" }],
      },
      { trigger: { type: "member_join" }, actions: [] },
    ]);
  });

  it("reads every Discord slash-command option kind", () => {
    const workspace = workspaceFrom({
      blocks: {
        languageVersion: 0,
        blocks: [
          {
            type: "slash_command",
            fields: { NAME: "configure", DESCRIPTION: "Configure the bot" },
            inputs: {
              OPTIONS: {
                block: {
                  type: "command_option",
                  fields: {
                    NAME: "decimal",
                    KIND: "number",
                    REQUIRED: true,
                    DESCRIPTION: "A decimal",
                  },
                  next: {
                    block: {
                      type: "command_option",
                      fields: {
                        NAME: "channel",
                        KIND: "channel",
                        REQUIRED: false,
                        DESCRIPTION: "A channel",
                      },
                      next: {
                        block: {
                          type: "command_option",
                          fields: {
                            NAME: "role",
                            KIND: "role",
                            REQUIRED: false,
                            DESCRIPTION: "A role",
                          },
                          next: {
                            block: {
                              type: "command_option",
                              fields: {
                                NAME: "mention",
                                KIND: "mentionable",
                                REQUIRED: false,
                                DESCRIPTION: "A member or role",
                              },
                              next: {
                                block: {
                                  type: "command_option",
                                  fields: {
                                    NAME: "file",
                                    KIND: "attachment",
                                    REQUIRED: false,
                                    DESCRIPTION: "A file",
                                  },
                                },
                              },
                            },
                          },
                        },
                      },
                    },
                  },
                },
              },
            },
          },
        ],
      },
    });

    const trigger = generateProgram(workspace).handlers[0]?.trigger;
    if (!trigger || trigger.type !== "slash_command") {
      throw new Error("expected a slash command");
    }
    expect(trigger.options.map((option) => option.kind)).toEqual([
      "number",
      "channel",
      "role",
      "mentionable",
      "attachment",
    ]);
  });

  it("generates moderation actions with a member template", () => {
    const workspace = workspaceFrom({
      blocks: {
        languageVersion: 0,
        blocks: [
          {
            type: "message_contains",
            fields: { PATTERN: "spam", CASE_SENSITIVE: false },
            inputs: {
              ACTIONS: {
                block: {
                  type: "timeout_member",
                  fields: { MEMBER_ID: "{user.id}", MINUTES: 5, REASON: "Spam" },
                  next: {
                    block: {
                      type: "delete_triggering_message",
                      next: {
                        block: {
                          type: "direct_message_member",
                          fields: {
                            MEMBER_ID: "{user.id}",
                            CONTENT: "Please do not spam.",
                          },
                        },
                      },
                    },
                  },
                },
              },
            },
          },
        ],
      },
    });

    expect(generateProgram(workspace).handlers[0]?.actions).toEqual([
      { type: "timeout", member_id: "{user.id}", seconds: 300, reason: "Spam" },
      { type: "delete_message" },
      {
        type: "direct_message",
        member_id: "{user.id}",
        content: "Please do not spam.",
      },
    ]);
  });
});

describe("sanitizeName", () => {
  it("keeps names Discord accepts", () => {
    expect(sanitizeName("Say Hello!")).toBe("say-hello");
    expect(sanitizeName("x".repeat(40))).toHaveLength(32);
  });
});

describe("logic and variables", () => {
  it("builds conditions, saved variables and flow steps", () => {
    const workspace = workspaceFrom({
      blocks: {
        languageVersion: 0,
        blocks: [
          {
            type: "message_contains",
            fields: { PATTERN: "gg", CASE_SENSITIVE: false },
            inputs: {
              ACTIONS: {
                block: {
                  type: "variable_change",
                  fields: { SCOPE: "member", NAME: "XP Points" },
                  inputs: {
                    BY: {
                      block: {
                        type: "value_random",
                        inputs: {
                          MIN: { block: { type: "value_number", fields: { NUM: 5 } } },
                          // Blockly rounds number fields to whole numbers.
                          MAX: { block: { type: "value_number", fields: { NUM: 10.7 } } },
                        },
                      },
                    },
                  },
                  next: {
                    block: {
                      type: "logic_if",
                      inputs: {
                        CONDITION: {
                          block: {
                            type: "logic_compare",
                            fields: { OP: "greater_or_equal" },
                            inputs: {
                              LEFT: {
                                block: {
                                  type: "variable_get",
                                  fields: { SCOPE: "member", NAME: "xp_points" },
                                },
                              },
                              RIGHT: { block: { type: "value_number", fields: { NUM: 100 } } },
                            },
                          },
                        },
                        THEN: {
                          block: {
                            type: "reply",
                            fields: { CONTENT: "Level up! {member:xp_points}" },
                            next: { block: { type: "flow_stop" } },
                          },
                        },
                        ELSE: { block: { type: "flow_wait", fields: { SECONDS: 3 } } },
                      },
                    },
                  },
                },
              },
            },
          },
        ],
      },
    });

    expect(generateProgram(workspace).handlers[0]?.actions).toEqual([
      {
        type: "change_variable",
        scope: "member",
        name: "xp_points",
        by: {
          type: "random",
          min: { type: "number", value: 5 },
          max: { type: "number", value: 11 },
        },
      },
      {
        type: "if",
        condition: {
          type: "compare",
          op: "greater_or_equal",
          left: { type: "variable", scope: "member", name: "xp_points" },
          right: { type: "number", value: 100 },
        },
        then: [
          { type: "reply", content: "Level up! {member:xp_points}", ephemeral: false },
          { type: "stop" },
        ],
        else: [{ type: "wait", seconds: 3 }],
      },
    ]);
  });

  it("points at a block with an empty slot", () => {
    const workspace = workspaceFrom({
      blocks: {
        languageVersion: 0,
        blocks: [
          {
            type: "member_join",
            inputs: { ACTIONS: { block: { type: "logic_if", id: "incomplete" } } },
          },
        ],
      },
    });

    expect(() => generateProgram(workspace)).toThrow(GenerateError);
    try {
      generateProgram(workspace);
    } catch (error) {
      expect(error instanceof GenerateError && error.blockId).toBe("incomplete");
    }
  });
});

describe("sanitizeVariable", () => {
  it("keeps names the server accepts", () => {
    expect(sanitizeVariable("XP Points-2")).toBe("xp_points_2");
  });
});

describe("rich messages and on-click triggers", () => {
  it("generates embeds, grouped button rows, select menus and component triggers", () => {
    const workspace = workspaceFrom({
      blocks: {
        languageVersion: 0,
        blocks: [
          {
            type: "slash_command",
            x: 0,
            y: 0,
            fields: { NAME: "panel", DESCRIPTION: "Open panel" },
            inputs: {
              ACTIONS: {
                block: {
                  type: "reply_rich",
                  fields: { CONTENT: "", EPHEMERAL: false },
                  inputs: {
                    EMBEDS: {
                      block: {
                        type: "embed",
                        fields: {
                          TITLE: "Roles",
                          DESCRIPTION: "Pick yours",
                          COLOR: " #5865F2 ",
                          TIMESTAMP: true,
                        },
                        inputs: {
                          PARTS: {
                            block: {
                              type: "embed_field",
                              fields: { NAME: "Count", VALUE: "{server:count}", INLINE: true },
                              next: {
                                block: {
                                  type: "embed_footer",
                                  fields: { TEXT: "Disblock", ICON_URL: "" },
                                },
                              },
                            },
                          },
                        },
                      },
                    },
                    COMPONENTS: {
                      block: {
                        type: "component_button",
                        fields: {
                          LABEL: "Accept",
                          STYLE: "success",
                          ID: " accept ",
                          EMOJI: "✅",
                          DISABLED: false,
                        },
                        next: {
                          block: {
                            type: "component_link_button",
                            fields: {
                              LABEL: "Docs",
                              URL: "https://example.com",
                              EMOJI: "",
                              DISABLED: false,
                            },
                            next: {
                              block: {
                                type: "component_new_row",
                                next: {
                                  block: {
                                    type: "component_button",
                                    fields: {
                                      LABEL: "Reset",
                                      STYLE: "danger",
                                      ID: "reset",
                                      EMOJI: "",
                                      DISABLED: true,
                                    },
                                    next: {
                                      block: {
                                        type: "component_select_menu",
                                        fields: {
                                          ID: "color",
                                          PLACEHOLDER: "Pick one",
                                          MIN: 1,
                                          MAX: 1,
                                          DISABLED: false,
                                        },
                                        inputs: {
                                          OPTIONS: {
                                            block: {
                                              type: "select_option",
                                              fields: {
                                                LABEL: "Red",
                                                VALUE: "red",
                                                DESCRIPTION: "Crimson",
                                                EMOJI: "🔴",
                                                DEFAULT: true,
                                              },
                                            },
                                          },
                                        },
                                      },
                                    },
                                  },
                                },
                              },
                            },
                          },
                        },
                      },
                    },
                  },
                },
              },
            },
          },
          {
            type: "button_click",
            x: 0,
            y: 300,
            fields: { ID: " accept " },
            inputs: {
              ACTIONS: {
                block: {
                  type: "update_message",
                  fields: { CONTENT: "Done by {user}" },
                },
              },
            },
          },
          {
            type: "select_menu_trigger",
            x: 0,
            y: 500,
            fields: { ID: "color" },
            inputs: {
              ACTIONS: {
                block: {
                  type: "variable_set",
                  fields: { SCOPE: "member", NAME: "color" },
                  inputs: { VALUE: { block: { type: "value_selected" } } },
                },
              },
            },
          },
        ],
      },
    });

    expect(generateProgram(workspace).handlers).toEqual([
      {
        trigger: { type: "slash_command", name: "panel", description: "Open panel", options: [] },
        actions: [
          {
            type: "reply",
            content: "",
            ephemeral: false,
            embeds: [
              {
                title: "Roles",
                description: "Pick yours",
                color: "#5865F2",
                footer: { text: "Disblock" },
                fields: [{ name: "Count", value: "{server:count}", inline: true }],
                timestamp: true,
              },
            ],
            components: [
              {
                type: "buttons",
                buttons: [
                  {
                    type: "action",
                    id: "accept",
                    label: "Accept",
                    emoji: "✅",
                    style: "success",
                    disabled: false,
                  },
                  {
                    type: "link",
                    url: "https://example.com",
                    label: "Docs",
                    disabled: false,
                  },
                ],
              },
              {
                type: "buttons",
                buttons: [
                  {
                    type: "action",
                    id: "reset",
                    label: "Reset",
                    style: "danger",
                    disabled: true,
                  },
                ],
              },
              {
                type: "select_menu",
                id: "color",
                placeholder: "Pick one",
                min_values: 1,
                max_values: 1,
                disabled: false,
                options: [
                  {
                    label: "Red",
                    value: "red",
                    description: "Crimson",
                    emoji: "🔴",
                    default: true,
                  },
                ],
              },
            ],
          },
        ],
      },
      {
        trigger: { type: "button_click", id: "accept" },
        actions: [{ type: "update_message", content: "Done by {user}" }],
      },
      {
        trigger: { type: "select_menu", id: "color" },
        actions: [
          {
            type: "set_variable",
            scope: "member",
            name: "color",
            value: { type: "selected" },
          },
        ],
      },
    ]);
  });

  it("points at duplicate embed parts", () => {
    const workspace = workspaceFrom({
      blocks: {
        languageVersion: 0,
        blocks: [
          {
            type: "button_click",
            fields: { ID: "go" },
            inputs: {
              ACTIONS: {
                block: {
                  type: "reply_rich",
                  inputs: {
                    EMBEDS: {
                      block: {
                        type: "embed",
                        inputs: {
                          PARTS: {
                            block: {
                              type: "embed_footer",
                              fields: { TEXT: "One" },
                              next: {
                                block: {
                                  type: "embed_footer",
                                  id: "dup_footer",
                                  fields: { TEXT: "Two" },
                                },
                              },
                            },
                          },
                        },
                      },
                    },
                  },
                },
              },
            },
          },
        ],
      },
    });

    expect(() => generateProgram(workspace)).toThrow(GenerateError);
    try {
      generateProgram(workspace);
    } catch (error) {
      expect(error instanceof GenerateError && error.blockId).toBe("dup_footer");
    }
  });
});

describe("editor validation mapping and intents", () => {
  it("resolves nested server error locations back to the exact block ID", () => {
    const workspace = workspaceFrom({
      blocks: {
        languageVersion: 0,
        blocks: [
          {
            type: "slash_command",
            id: "trig_cmd",
            x: 0,
            y: 0,
            fields: { NAME: "test", DESCRIPTION: "Test" },
            inputs: {
              OPTIONS: {
                block: {
                  type: "command_option",
                  id: "opt_0",
                  fields: { NAME: "a", KIND: "string", REQUIRED: false, DESCRIPTION: "A" },
                  next: {
                    block: {
                      type: "command_option",
                      id: "opt_1",
                      fields: { NAME: "b", KIND: "string", REQUIRED: true, DESCRIPTION: "B" },
                    },
                  },
                },
              },
              ACTIONS: {
                block: {
                  type: "logic_if",
                  id: "if_outer",
                  inputs: {
                    CONDITION: {
                      block: {
                        type: "logic_operation",
                        id: "and_node",
                        fields: { OP: "and" },
                        inputs: {
                          LEFT: { block: { type: "logic_boolean", fields: { VALUE: "true" } } },
                          RIGHT: {
                            block: {
                              type: "has_role",
                              id: "bad_role",
                              fields: { ROLE_ID: "bad" },
                            },
                          },
                        },
                      },
                    },
                    THEN: {
                      block: {
                        type: "reply_rich",
                        id: "rich_reply",
                        inputs: {
                          EMBEDS: {
                            block: {
                              type: "embed",
                              id: "embed_0",
                              inputs: {
                                PARTS: {
                                  block: {
                                    type: "embed_field",
                                    id: "field_0",
                                    fields: { NAME: "A", VALUE: "B" },
                                    next: {
                                      block: {
                                        type: "embed_field",
                                        id: "field_1",
                                        fields: { NAME: "", VALUE: "X" },
                                      },
                                    },
                                  },
                                },
                              },
                            },
                          },
                          COMPONENTS: {
                            block: {
                              type: "component_button",
                              id: "btn_0",
                              fields: { ID: "ok", LABEL: "OK", STYLE: "primary" },
                              next: {
                                block: {
                                  type: "component_select_menu",
                                  id: "menu_1",
                                  fields: { ID: "pick", PLACEHOLDER: "Pick" },
                                  inputs: {
                                    OPTIONS: {
                                      block: {
                                        type: "select_option",
                                        id: "sel_opt_0",
                                        fields: { LABEL: "A", VALUE: "same" },
                                        next: {
                                          block: {
                                            type: "select_option",
                                            id: "sel_opt_1",
                                            fields: { LABEL: "B", VALUE: "same" },
                                          },
                                        },
                                      },
                                    },
                                  },
                                },
                              },
                            },
                          },
                        },
                      },
                    },
                  },
                },
              },
            },
          },
        ],
      },
    });

    const { map } = generateProgramWithMap(workspace);
    expect(resolveErrorBlockId(map, { handler: 0 })).toBe("trig_cmd");
    expect(resolveErrorBlockId(map, { handler: 0, option: 1 })).toBe("opt_1");
    expect(
      resolveErrorBlockId(map, {
        handler: 0,
        steps: [{ index: 0 }],
        expr: [0, 1],
      }),
    ).toBe("bad_role");
    expect(
      resolveErrorBlockId(map, {
        handler: 0,
        steps: [{ index: 0, branch: "then" }, { index: 0 }],
        embed: { index: 0, part: { type: "field", index: 1 } },
      }),
    ).toBe("field_1");
    expect(
      resolveErrorBlockId(map, {
        handler: 0,
        steps: [{ index: 0, branch: "then" }, { index: 0 }],
        component: { row: 1, option: 1 },
      }),
    ).toBe("sel_opt_1");
  });

  it("detects missing privileged intents on enabled trigger blocks", () => {
    const workspace = workspaceFrom({
      blocks: {
        languageVersion: 0,
        blocks: [
          { type: "member_join", id: "join_blk", x: 0, y: 0 },
          { type: "member_leave", id: "leave_blk", x: 0, y: 100 },
          {
            type: "message_contains",
            id: "msg_blk",
            x: 0,
            y: 200,
            fields: { PATTERN: "hi", CASE_SENSITIVE: false },
          },
          {
            type: "message_contains",
            id: "disabled_msg",
            x: 0,
            y: 400,
            enabled: false,
            fields: { PATTERN: "off", CASE_SENSITIVE: false },
          },
        ],
      },
    });

    expect(findMissingIntents(workspace, { members: false, message_content: true })).toEqual([
      {
        intent: "members",
        label: "Server Members Intent",
        blockId: "join_blk",
        message:
          "Server Members Intent is disabled in the Discord developer portal. Enable it under Bot before saving.",
      },
      {
        intent: "members",
        label: "Server Members Intent",
        blockId: "leave_blk",
        message:
          "Server Members Intent is disabled in the Discord developer portal. Enable it under Bot before saving.",
      },
    ]);
    expect(findMissingIntents(workspace, { members: true, message_content: true })).toEqual([]);
  });

  it("reads member_leave, reaction triggers, and schedules", () => {
    const workspace = workspaceFrom({
      blocks: {
        languageVersion: 0,
        blocks: [
          {
            type: "member_leave",
            id: "trig_leave",
            x: 0,
            y: 0,
            inputs: {
              ACTIONS: {
                block: {
                  type: "send_message",
                  fields: { CONTENT: "Bye {user}", CHANNEL_ID: "123456789012345678" },
                },
              },
            },
          },
          {
            type: "reaction_add",
            id: "trig_react_add",
            x: 0,
            y: 120,
            fields: { EMOJI: "👍", MESSAGE_ID: "123456789012345679" },
            inputs: {
              ACTIONS: {
                block: { type: "add_role", fields: { ROLE_ID: "123456789012345680" } },
              },
            },
          },
          {
            type: "reaction_remove",
            id: "trig_react_rem",
            x: 0,
            y: 260,
            fields: { EMOJI: "", MESSAGE_ID: "" },
            inputs: {
              ACTIONS: {
                block: { type: "remove_role", fields: { ROLE_ID: "123456789012345680" } },
              },
            },
          },
          {
            type: "schedule",
            id: "trig_sched",
            x: 0,
            y: 400,
            fields: { INTERVAL: 6, UNIT: "hours" },
            inputs: {
              ACTIONS: {
                block: { type: "flow_stop" },
              },
            },
          },
        ],
      },
    });

    const { program, map } = generateProgramWithMap(workspace);
    expect(program).toEqual({
      handlers: [
        {
          trigger: { type: "member_leave" },
          actions: [
            {
              type: "send_message",
              channel_id: "123456789012345678",
              content: "Bye {user}",
            },
          ],
        },
        {
          trigger: {
            type: "reaction_add",
            emoji: "👍",
            message_id: "123456789012345679",
          },
          actions: [{ type: "add_role", role_id: "123456789012345680" }],
        },
        {
          trigger: { type: "reaction_remove" },
          actions: [{ type: "remove_role", role_id: "123456789012345680" }],
        },
        {
          trigger: { type: "schedule", interval: 6, unit: "hours" },
          actions: [{ type: "stop" }],
        },
      ],
    });
    expect(resolveErrorBlockId(map, { handler: 1 })).toBe("trig_react_add");
    expect(resolveErrorBlockId(map, { handler: 3 })).toBe("trig_sched");
  });

  it("generates subcommands, context commands, modals, entity selects, polls, files, threads, and advanced events", () => {
    const workspace = workspaceFrom({
      blocks: {
        languageVersion: 0,
        blocks: [
          {
            type: "slash_subcommand",
            x: 0,
            y: 0,
            fields: {
              COMMAND: "admin",
              GROUP: "user",
              SUBCOMMAND: "warn",
              DESCRIPTION: "Warn a user",
            },
            inputs: {
              OPTIONS: {
                block: {
                  type: "command_option",
                  fields: {
                    NAME: "level",
                    KIND: "string",
                    REQUIRED: true,
                    DESCRIPTION: "Severity",
                    AUTOCOMPLETE: "",
                  },
                  inputs: {
                    CHOICES: {
                      block: {
                        type: "command_option_choice",
                        fields: { NAME: "Low", VALUE: "low" },
                      },
                    },
                  },
                },
              },
              ACTIONS: {
                block: {
                  type: "show_modal",
                  fields: { ID: "warn_form", TITLE: "Warn details" },
                  inputs: {
                    INPUTS: {
                      block: {
                        type: "modal_text_input",
                        fields: {
                          ID: "reason",
                          LABEL: "Reason",
                          STYLE: "paragraph",
                          PLACEHOLDER: "Why?",
                          VALUE: "",
                          REQUIRED: true,
                        },
                      },
                    },
                  },
                },
              },
            },
          },
          {
            type: "modal_submit",
            x: 0,
            y: 250,
            fields: { ID: "warn_form" },
            inputs: {
              ACTIONS: {
                block: {
                  type: "variable_set",
                  fields: { SCOPE: "local", NAME: "r" },
                  inputs: {
                    VALUE: { block: { type: "value_modal_field", fields: { ID: "reason" } } },
                  },
                  next: {
                    block: {
                      type: "reply_rich",
                      fields: { CONTENT: "Logged", EPHEMERAL: true },
                      inputs: {
                        COMPONENTS: {
                          block: {
                            type: "component_entity_select_menu",
                            fields: {
                              KIND: "channel",
                              ID: "target_ch",
                              PLACEHOLDER: "Pick channel",
                              CHANNEL_FILTER: "text",
                              MIN: 1,
                              MAX: 1,
                              DISABLED: false,
                            },
                          },
                        },
                        POLL: {
                          block: {
                            type: "message_poll",
                            fields: { QUESTION: "Escalate?", DURATION: 24, MULTISELECT: false },
                            inputs: {
                              ANSWERS: {
                                block: {
                                  type: "poll_answer",
                                  fields: { TEXT: "Yes", EMOJI: "👍" },
                                },
                              },
                            },
                          },
                        },
                        ATTACHMENTS: {
                          block: {
                            type: "message_attachment",
                            fields: { FILENAME: "warn.txt", CONTENT: "{modal.reason}" },
                          },
                        },
                      },
                    },
                  },
                },
              },
            },
          },
          {
            type: "user_command",
            x: 0,
            y: 500,
            fields: { NAME: "Inspect" },
            inputs: {
              ACTIONS: {
                block: {
                  type: "variable_set",
                  fields: { SCOPE: "local", NAME: "t" },
                  inputs: {
                    VALUE: { block: { type: "value_target", fields: { FIELD: "id" } } },
                  },
                },
              },
            },
          },
          {
            type: "message_command",
            x: 0,
            y: 650,
            fields: { NAME: "Thread It" },
            inputs: {
              ACTIONS: {
                block: {
                  type: "create_thread_from_message",
                  fields: { NAME: "Topic", ARCHIVE: "1440" },
                  next: {
                    block: {
                      type: "pin_message",
                      fields: { MESSAGE_ID: "{target.id}", CHANNEL_ID: "{channel.id}" },
                    },
                  },
                },
              },
            },
          },
          {
            type: "automod_execution",
            x: 0,
            y: 800,
            fields: { RULE_ID: "" },
            inputs: {
              ACTIONS: {
                block: {
                  type: "bulk_delete_messages",
                  fields: { COUNT: 5, CHANNEL_ID: "{channel.id}" },
                },
              },
            },
          },
          {
            type: "member_update",
            id: "upd_blk",
            x: 0,
            y: 950,
          },
          {
            type: "message_edit",
            id: "edit_blk",
            x: 0,
            y: 1050,
          },
        ],
      },
    });

    const program = generateProgram(workspace);
    expect(program.handlers[0]).toEqual({
      trigger: {
        type: "slash_subcommand",
        command: "admin",
        group: "user",
        subcommand: "warn",
        description: "Warn a user",
        options: [
          {
            name: "level",
            description: "Severity",
            kind: "string",
            required: true,
            choices: [{ name: "Low", value: "low" }],
          },
        ],
      },
      actions: [
        {
          type: "show_modal",
          id: "warn_form",
          title: "Warn details",
          inputs: [
            {
              id: "reason",
              label: "Reason",
              style: "paragraph",
              placeholder: "Why?",
              required: true,
            },
          ],
        },
      ],
    });
    expect(program.handlers[1].trigger).toEqual({ type: "modal_submit", id: "warn_form" });
    expect(program.handlers[1].actions[1]).toEqual({
      type: "reply",
      content: "Logged",
      ephemeral: true,
      components: [
        {
          type: "entity_select_menu",
          id: "target_ch",
          kind: "channel",
          placeholder: "Pick channel",
          min_values: 1,
          max_values: 1,
          disabled: false,
          channel_types: ["text"],
        },
      ],
      poll: {
        question: "Escalate?",
        answers: [{ text: "Yes", emoji: "👍" }],
        duration_hours: 24,
        allow_multiselect: false,
      },
      attachments: [{ filename: "warn.txt", content: "{modal.reason}" }],
    });
    expect(program.handlers[2].trigger).toEqual({ type: "user_command", name: "Inspect" });
    expect(program.handlers[3].actions).toEqual([
      { type: "create_thread_from_message", name: "Topic", auto_archive_minutes: 1440 },
      { type: "pin_message", channel_id: "{channel.id}", message_id: "{target.id}" },
    ]);
    expect(program.handlers[4]).toEqual({
      trigger: { type: "automod_execution" },
      actions: [{ type: "bulk_delete_messages", channel_id: "{channel.id}", count: 5 }],
    });

    expect(findMissingIntents(workspace, { members: false, message_content: false })).toEqual([
      {
        intent: "members",
        label: "Server Members Intent",
        blockId: "upd_blk",
        message:
          "Server Members Intent is disabled in the Discord developer portal. Enable it under Bot before saving.",
      },
      {
        intent: "message_content",
        label: "Message Content Intent",
        blockId: "edit_blk",
        message:
          "Message Content Intent is disabled in the Discord developer portal. Enable it under Bot before saving.",
      },
    ]);
  });
});
