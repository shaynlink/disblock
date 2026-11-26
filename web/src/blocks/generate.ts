import type * as Blockly from "blockly";
import type {
  CommandOption,
  CompareOp,
  Expr,
  Handler,
  MathOp,
  OptionKind,
  Program,
  Scope,
  Statement,
  Trigger,
} from "../types.ts";

/** A block the program cannot be built from, to point the user at. */
export class GenerateError extends Error {
  readonly blockId: string;

  constructor(message: string, blockId: string) {
    super(message);
    this.name = "GenerateError";
    this.blockId = blockId;
  }
}

const OPTION_KINDS: readonly OptionKind[] = ["string", "integer", "boolean", "user"];
const SCOPES: readonly Scope[] = ["local", "member", "server"];
const MATH_OPS: readonly MathOp[] = ["add", "subtract", "multiply", "divide", "modulo"];
const COMPARE_OPS: readonly CompareOp[] = [
  "equal",
  "not_equal",
  "less",
  "less_or_equal",
  "greater",
  "greater_or_equal",
  "contains",
];

function text(block: Blockly.Block, field: string): string {
  return String(block.getFieldValue(field) ?? "");
}

function num(block: Blockly.Block, field: string): number {
  return Number(block.getFieldValue(field) ?? 0);
}

function checked(block: Blockly.Block, field: string): boolean {
  return block.getFieldValue(field) === "TRUE";
}

/** The field's value if it is one of `allowed`, so a stale workspace cannot inject anything else. */
function oneOf<T extends string>(block: Blockly.Block, field: string, allowed: readonly T[]): T {
  const value = text(block, field);
  const match = allowed.find((candidate) => candidate === value);
  if (!match) {
    throw new GenerateError(
      `Pick a value in this block's "${field.toLowerCase()}" list.`,
      block.id,
    );
  }
  return match;
}

/** Enabled blocks stacked under `input`, top to bottom. */
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
  return {
    name: text(block, "NAME"),
    description: text(block, "DESCRIPTION"),
    kind: oneOf(block, "KIND", OPTION_KINDS),
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

/** The value plugged into `input`, which must not be empty. */
function value(block: Blockly.Block, input: string): Expr {
  const target = block.getInputTargetBlock(input);
  if (!target || !target.isEnabled()) {
    throw new GenerateError("This block has an empty slot: drop a value in it.", block.id);
  }
  return readExpr(target);
}

function readExpr(block: Blockly.Block): Expr {
  switch (block.type) {
    case "value_text":
      return { type: "text", value: text(block, "TEXT") };
    case "value_number":
      return { type: "number", value: Math.trunc(num(block, "NUM")) };
    case "logic_boolean":
      return { type: "boolean", value: text(block, "VALUE") === "true" };
    case "variable_get":
      return { type: "variable", scope: oneOf(block, "SCOPE", SCOPES), name: text(block, "NAME") };
    case "value_option":
      return { type: "option", name: text(block, "NAME") };
    case "value_random":
      return { type: "random", min: value(block, "MIN"), max: value(block, "MAX") };
    case "value_math":
      return {
        type: "math",
        op: oneOf(block, "OP", MATH_OPS),
        left: value(block, "LEFT"),
        right: value(block, "RIGHT"),
      };
    case "logic_compare":
      return {
        type: "compare",
        op: oneOf(block, "OP", COMPARE_OPS),
        left: value(block, "LEFT"),
        right: value(block, "RIGHT"),
      };
    case "logic_operation":
      return {
        type: text(block, "OP") === "or" ? "or" : "and",
        left: value(block, "LEFT"),
        right: value(block, "RIGHT"),
      };
    case "logic_not":
      return { type: "not", value: value(block, "VALUE") };
    case "has_role":
      return { type: "has_role", role_id: text(block, "ROLE_ID").trim() };
    default:
      throw new GenerateError("This block cannot be used as a value.", block.id);
  }
}

function readStatement(block: Blockly.Block): Statement {
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
    case "logic_if":
      return {
        type: "if",
        condition: value(block, "CONDITION"),
        then: stack(block, "THEN").map(readStatement),
        else: stack(block, "ELSE").map(readStatement),
      };
    case "variable_set":
      return {
        type: "set_variable",
        scope: oneOf(block, "SCOPE", SCOPES),
        name: text(block, "NAME"),
        value: value(block, "VALUE"),
      };
    case "variable_change":
      return {
        type: "change_variable",
        scope: oneOf(block, "SCOPE", SCOPES),
        name: text(block, "NAME"),
        by: value(block, "BY"),
      };
    case "flow_wait":
      return { type: "wait", seconds: Math.trunc(num(block, "SECONDS")) };
    case "flow_stop":
      return { type: "stop" };
    default:
      throw new GenerateError("This block cannot be used as an action.", block.id);
  }
}

/**
 * Turn the editor's blocks into the program the server runs. Triggers are
 * ordered top to bottom, as the server numbers them in its error messages.
 * Blocks that are not attached to a trigger are ignored.
 *
 * @throws {GenerateError} when an attached block is incomplete.
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
    handlers.push({ trigger, actions: stack(block, "ACTIONS").map(readStatement) });
  }
  return { handlers };
}
