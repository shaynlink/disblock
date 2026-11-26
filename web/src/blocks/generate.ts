import type * as Blockly from "blockly";
import type { Action, CommandOption, Handler, OptionKind, Program, Trigger } from "../types.ts";

const OPTION_KINDS: readonly OptionKind[] = ["string", "integer", "boolean", "user"];

function text(block: Blockly.Block, field: string): string {
  return String(block.getFieldValue(field) ?? "");
}

function checked(block: Blockly.Block, field: string): boolean {
  return block.getFieldValue(field) === "TRUE";
}

/** Blocks stacked under `input`, top to bottom. */
function stack(block: Blockly.Block, input: string): Blockly.Block[] {
  const blocks: Blockly.Block[] = [];
  for (let current = block.getInputTargetBlock(input); current; current = current.getNextBlock()) {
    if (current.isEnabled()) {
      blocks.push(current);
    }
  }
  return blocks;
}

function readOption(block: Blockly.Block): CommandOption | null {
  if (block.type !== "command_option") {
    return null;
  }
  const kind = text(block, "KIND");
  return {
    name: text(block, "NAME"),
    description: text(block, "DESCRIPTION"),
    kind: OPTION_KINDS.find((candidate) => candidate === kind) ?? "string",
    required: checked(block, "REQUIRED"),
  };
}

function readTrigger(block: Blockly.Block): Trigger | null {
  switch (block.type) {
    case "slash_command":
      return {
        type: "slash_command",
        name: text(block, "NAME"),
        description: text(block, "DESCRIPTION"),
        options: stack(block, "OPTIONS")
          .map(readOption)
          .filter((option) => option !== null),
      };
    case "member_join":
      return { type: "member_join" };
    case "message_contains":
      return {
        type: "message_contains",
        pattern: text(block, "PATTERN"),
        case_sensitive: checked(block, "CASE_SENSITIVE"),
      };
    default:
      return null;
  }
}

function readAction(block: Blockly.Block): Action | null {
  switch (block.type) {
    case "reply":
      return {
        type: "reply",
        content: text(block, "CONTENT"),
        ephemeral: checked(block, "EPHEMERAL"),
      };
    case "send_message":
      return {
        type: "send_message",
        channel_id: text(block, "CHANNEL_ID").trim(),
        content: text(block, "CONTENT"),
      };
    case "react":
      return { type: "react", emoji: text(block, "EMOJI").trim() };
    case "add_role":
      return { type: "add_role", role_id: text(block, "ROLE_ID").trim() };
    case "remove_role":
      return { type: "remove_role", role_id: text(block, "ROLE_ID").trim() };
    default:
      return null;
  }
}

/**
 * Turn the editor's blocks into the program the server runs. Triggers are
 * ordered top to bottom, as the server numbers them in its error messages.
 * Blocks that are not attached to a trigger are ignored.
 */
export function generateProgram(workspace: Blockly.Workspace): Program {
  const handlers: Handler[] = [];
  for (const block of workspace.getTopBlocks(true)) {
    if (!block.isEnabled()) {
      continue;
    }
    const trigger = readTrigger(block);
    if (!trigger) {
      continue;
    }
    const actions = stack(block, "ACTIONS")
      .map(readAction)
      .filter((action) => action !== null);
    handlers.push({ trigger, actions });
  }
  return { handlers };
}
