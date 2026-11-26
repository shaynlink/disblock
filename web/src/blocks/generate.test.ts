import * as Blockly from "blockly";
import { beforeAll, describe, expect, it } from "vite-plus/test";
import { registerBlocks, sanitizeName, sanitizeVariable, starterWorkspace } from "./blocks.ts";
import { GenerateError, generateProgram } from "./generate.ts";

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
