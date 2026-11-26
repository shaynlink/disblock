import * as Blockly from "blockly";
import { beforeAll, describe, expect, it } from "vite-plus/test";
import { registerBlocks, sanitizeName, starterWorkspace } from "./blocks.ts";
import { generateProgram } from "./generate.ts";

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
