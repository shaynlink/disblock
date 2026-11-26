import * as Blockly from "blockly";
import { beforeAll, describe, expect, it } from "vite-plus/test";
import { registerBlocks, starterWorkspace } from "./blocks.ts";
import { generateProgramWithMap } from "./generate.ts";
import { BOT_TEMPLATES, hasCustomWorkspaceBlocks, insertTemplate } from "./templates.ts";

beforeAll(() => {
  registerBlocks();
});

const SAMPLE_IDS: Record<string, string> = {
  channelId: "123456789012345678",
  roleId: "123456789012345679",
  emoji: "🔔",
  messageId: "123456789012345680",
  modRoleId: "123456789012345681",
  logChannelId: "123456789012345682",
  blockedWord: "spamlink",
  staffChannelId: "123456789012345683",
  staffRoleId: "123456789012345684",
};

describe("ready-made bot templates", () => {
  it("provides all five templates requested by the roadmap", () => {
    expect(BOT_TEMPLATES.map((template) => template.id)).toEqual([
      "welcome",
      "levels",
      "reaction_roles",
      "moderation",
      "tickets",
    ]);
  });

  it.each(BOT_TEMPLATES)("loads and generates valid program blocks for $id", (template) => {
    const workspace = new Blockly.Workspace();
    const first = insertTemplate(workspace, template, SAMPLE_IDS, "replace");
    expect(first).not.toBeNull();

    const { program, map } = generateProgramWithMap(workspace);
    expect(program.handlers.length).toBeGreaterThan(0);
    expect(map.handlers).toHaveLength(program.handlers.length);
  });

  it("appends templates below existing blocks without overwriting them", () => {
    const workspace = new Blockly.Workspace();
    Blockly.serialization.workspaces.load(starterWorkspace, workspace);
    expect(hasCustomWorkspaceBlocks(workspace)).toBe(false);

    const welcome = BOT_TEMPLATES.find((item) => item.id === "welcome");
    const reactionRoles = BOT_TEMPLATES.find((item) => item.id === "reaction_roles");
    expect(welcome).toBeDefined();
    expect(reactionRoles).toBeDefined();
    if (!welcome || !reactionRoles) {
      return;
    }

    insertTemplate(workspace, welcome, SAMPLE_IDS, "append");
    expect(hasCustomWorkspaceBlocks(workspace)).toBe(true);
    insertTemplate(workspace, reactionRoles, SAMPLE_IDS, "append");

    const { program } = generateProgramWithMap(workspace);
    const triggerTypes = program.handlers.map((handler) => handler.trigger.type);
    expect(triggerTypes).toEqual([
      "slash_command",
      "member_join",
      "member_leave",
      "slash_command",
      "reaction_add",
      "reaction_remove",
    ]);
  });
});
