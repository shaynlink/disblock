import type * as Blockly from "blockly";
import type {
  AttachmentSpec,
  BotIntents,
  ButtonSpec,
  ButtonStyle,
  ChannelFilter,
  CommandOption,
  CommandOptionChoice,
  CompareOp,
  ComponentRow,
  Embed,
  EmbedAuthor,
  EmbedField,
  EmbedFooter,
  EntitySelectKind,
  ErrorLocation,
  Expr,
  Handler,
  MathOp,
  ModalInputSpec,
  OptionKind,
  PollAnswerSpec,
  PollSpec,
  Program,
  ScheduleUnit,
  Scope,
  SelectOption,
  Statement,
  TargetField,
  TextInputStyle,
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

export interface ExprNode {
  blockId: string;
  children: ExprNode[];
}

export interface EmbedNode {
  blockId: string;
  titleUrlBlockId?: string;
  authorBlockId?: string;
  footerBlockId?: string;
  thumbnailBlockId?: string;
  imageBlockId?: string;
  fieldBlockIds: string[];
}

export interface ComponentRowNode {
  blockId: string;
  buttonBlockIds?: string[];
  optionBlockIds?: string[];
}

export interface StatementNode {
  blockId: string;
  expr?: ExprNode;
  then?: StatementNode[];
  else?: StatementNode[];
  embeds?: EmbedNode[];
  components?: ComponentRowNode[];
}

export interface HandlerNode {
  triggerBlockId: string;
  optionBlockIds: string[];
  actions: StatementNode[];
}

export interface ProgramMap {
  handlers: HandlerNode[];
}

export interface MissingIntent {
  intent: "members" | "message_content";
  label: string;
  blockId: string;
  message: string;
}

const OPTION_KINDS: readonly OptionKind[] = [
  "string",
  "integer",
  "number",
  "boolean",
  "user",
  "channel",
  "role",
  "mentionable",
  "attachment",
];
const SCHEDULE_UNITS: readonly ScheduleUnit[] = ["minutes", "hours", "days"];
const BUTTON_STYLES: readonly ButtonStyle[] = ["primary", "secondary", "success", "danger"];
const ENTITY_SELECT_KINDS: readonly EntitySelectKind[] = ["user", "role", "mentionable", "channel"];
const CHANNEL_FILTERS: readonly ChannelFilter[] = ["text", "voice", "announcement", "forum"];
const TEXT_INPUT_STYLES: readonly TextInputStyle[] = ["short", "paragraph"];
const TARGET_FIELDS: readonly TargetField[] = [
  "id",
  "mention",
  "name",
  "content",
  "author_id",
  "author_mention",
];
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

function optionalText(block: Blockly.Block, field: string): string | undefined {
  const value = text(block, field).trim();
  return value ? value : undefined;
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
  if (!block.getInput(input)) {
    return [];
  }
  const blocks: Blockly.Block[] = [];
  for (let current = block.getInputTargetBlock(input); current; current = current.getNextBlock()) {
    if (current.isEnabled()) {
      blocks.push(current);
    }
  }
  return blocks;
}

function readOptionChoice(block: Blockly.Block): CommandOptionChoice {
  if (block.type !== "command_option_choice") {
    throw new GenerateError("Only choice blocks can go in the choices slot.", block.id);
  }
  return {
    name: text(block, "NAME"),
    value: text(block, "VALUE"),
  };
}

function readOption(block: Blockly.Block): CommandOption | null {
  if (block.type !== "command_option") {
    return null;
  }
  const choices = stack(block, "CHOICES").map(readOptionChoice);
  const rawAutocomplete = text(block, "AUTOCOMPLETE")
    .split(",")
    .map((entry) => entry.trim())
    .filter(Boolean);
  return {
    name: text(block, "NAME"),
    description: text(block, "DESCRIPTION"),
    kind: oneOf(block, "KIND", OPTION_KINDS),
    required: checked(block, "REQUIRED"),
    ...(choices.length > 0 ? { choices } : {}),
    ...(rawAutocomplete.length > 0 ? { autocomplete: rawAutocomplete } : {}),
  };
}

function readOptions(block: Blockly.Block): {
  options: CommandOption[];
  optionBlockIds: string[];
} {
  const options: CommandOption[] = [];
  const optionBlockIds: string[] = [];
  for (const optBlock of stack(block, "OPTIONS")) {
    const option = readOption(optBlock);
    if (option !== null) {
      options.push(option);
      optionBlockIds.push(optBlock.id);
    }
  }
  return { options, optionBlockIds };
}

function readTriggerWithMap(
  block: Blockly.Block,
): { trigger: Trigger; optionBlockIds: string[] } | null {
  switch (block.type) {
    case "slash_command": {
      const { options, optionBlockIds } = readOptions(block);
      return {
        trigger: {
          type: "slash_command",
          name: text(block, "NAME"),
          description: text(block, "DESCRIPTION"),
          options,
        },
        optionBlockIds,
      };
    }
    case "slash_subcommand": {
      const { options, optionBlockIds } = readOptions(block);
      const group = optionalText(block, "GROUP");
      return {
        trigger: {
          type: "slash_subcommand",
          command: text(block, "COMMAND"),
          ...(group !== undefined ? { group } : {}),
          subcommand: text(block, "SUBCOMMAND"),
          description: text(block, "DESCRIPTION"),
          options,
        },
        optionBlockIds,
      };
    }
    case "user_command":
      return {
        trigger: { type: "user_command", name: text(block, "NAME").trim() },
        optionBlockIds: [],
      };
    case "message_command":
      return {
        trigger: { type: "message_command", name: text(block, "NAME").trim() },
        optionBlockIds: [],
      };
    case "modal_submit":
      return {
        trigger: { type: "modal_submit", id: text(block, "ID").trim() },
        optionBlockIds: [],
      };
    case "member_join":
    case "member_leave":
    case "member_update":
    case "member_ban":
    case "member_unban":
    case "message_edit":
    case "message_delete":
    case "thread_create":
    case "invite_create":
    case "poll_vote_add":
    case "voice_state_update":
      return { trigger: { type: block.type }, optionBlockIds: [] };
    case "message_contains":
      return {
        trigger: {
          type: "message_contains",
          pattern: text(block, "PATTERN"),
          case_sensitive: checked(block, "CASE_SENSITIVE"),
        },
        optionBlockIds: [],
      };
    case "reaction_add":
    case "reaction_remove": {
      const emoji = optionalText(block, "EMOJI");
      const message_id = optionalText(block, "MESSAGE_ID");
      return {
        trigger: {
          type: block.type,
          ...(emoji !== undefined ? { emoji } : {}),
          ...(message_id !== undefined ? { message_id } : {}),
        },
        optionBlockIds: [],
      };
    }
    case "automod_execution": {
      const rule_id = optionalText(block, "RULE_ID");
      return {
        trigger: {
          type: "automod_execution",
          ...(rule_id !== undefined ? { rule_id } : {}),
        },
        optionBlockIds: [],
      };
    }
    case "button_click":
      return {
        trigger: {
          type: "button_click",
          id: text(block, "ID").trim(),
        },
        optionBlockIds: [],
      };
    case "select_menu_trigger":
      return {
        trigger: {
          type: "select_menu",
          id: text(block, "ID").trim(),
        },
        optionBlockIds: [],
      };
    case "schedule":
      return {
        trigger: {
          type: "schedule",
          interval: Math.trunc(num(block, "INTERVAL")),
          unit: oneOf(block, "UNIT", SCHEDULE_UNITS),
        },
        optionBlockIds: [],
      };
    default:
      return null;
  }
}

/** The value plugged into `input`, which must not be empty. */
function value(block: Blockly.Block, input: string): { expr: Expr; node: ExprNode } {
  const target = block.getInputTargetBlock(input);
  if (!target || !target.isEnabled()) {
    throw new GenerateError("This block has an empty slot: drop a value in it.", block.id);
  }
  return readExpr(target);
}

function readExpr(block: Blockly.Block): { expr: Expr; node: ExprNode } {
  const leaf = (expr: Expr): { expr: Expr; node: ExprNode } => ({
    expr,
    node: { blockId: block.id, children: [] },
  });
  switch (block.type) {
    case "value_text":
      return leaf({ type: "text", value: text(block, "TEXT") });
    case "value_number":
      return leaf({ type: "number", value: Math.trunc(num(block, "NUM")) });
    case "logic_boolean":
      return leaf({ type: "boolean", value: text(block, "VALUE") === "true" });
    case "variable_get":
      return leaf({
        type: "variable",
        scope: oneOf(block, "SCOPE", SCOPES),
        name: text(block, "NAME"),
      });
    case "value_option":
      return leaf({ type: "option", name: text(block, "NAME") });
    case "value_selected":
      return leaf({ type: "selected" });
    case "value_modal_field":
      return leaf({ type: "modal_field", id: text(block, "ID").trim() });
    case "value_target":
      return leaf({ type: "target", field: oneOf(block, "FIELD", TARGET_FIELDS) });
    case "value_random": {
      const min = value(block, "MIN");
      const max = value(block, "MAX");
      return {
        expr: { type: "random", min: min.expr, max: max.expr },
        node: { blockId: block.id, children: [min.node, max.node] },
      };
    }
    case "value_math": {
      const left = value(block, "LEFT");
      const right = value(block, "RIGHT");
      return {
        expr: {
          type: "math",
          op: oneOf(block, "OP", MATH_OPS),
          left: left.expr,
          right: right.expr,
        },
        node: { blockId: block.id, children: [left.node, right.node] },
      };
    }
    case "logic_compare": {
      const left = value(block, "LEFT");
      const right = value(block, "RIGHT");
      return {
        expr: {
          type: "compare",
          op: oneOf(block, "OP", COMPARE_OPS),
          left: left.expr,
          right: right.expr,
        },
        node: { blockId: block.id, children: [left.node, right.node] },
      };
    }
    case "logic_operation": {
      const left = value(block, "LEFT");
      const right = value(block, "RIGHT");
      return {
        expr: {
          type: text(block, "OP") === "or" ? "or" : "and",
          left: left.expr,
          right: right.expr,
        },
        node: { blockId: block.id, children: [left.node, right.node] },
      };
    }
    case "logic_not": {
      const inner = value(block, "VALUE");
      return {
        expr: { type: "not", value: inner.expr },
        node: { blockId: block.id, children: [inner.node] },
      };
    }
    case "has_role":
      return leaf({ type: "has_role", role_id: text(block, "ROLE_ID").trim() });
    default:
      throw new GenerateError("This block cannot be used as a value.", block.id);
  }
}

function requiredUrl(block: Blockly.Block, field: string, label: string): string {
  const url = text(block, field).trim();
  if (!url) {
    throw new GenerateError(`Enter a ${label} URL.`, block.id);
  }
  return url;
}

function readEmbed(block: Blockly.Block): { embed: Embed; node: EmbedNode } {
  if (block.type !== "embed") {
    throw new GenerateError("Only embed blocks can go in the embeds slot.", block.id);
  }
  const title = text(block, "TITLE");
  const description = text(block, "DESCRIPTION");
  const color = text(block, "COLOR").trim();
  const fields: EmbedField[] = [];
  const fieldBlockIds: string[] = [];
  let author: EmbedAuthor | undefined;
  let authorBlockId: string | undefined;
  let footer: EmbedFooter | undefined;
  let footerBlockId: string | undefined;
  let imageUrl: string | undefined;
  let imageBlockId: string | undefined;
  let thumbnailUrl: string | undefined;
  let thumbnailBlockId: string | undefined;
  let url: string | undefined;
  let titleUrlBlockId: string | undefined;

  for (const part of stack(block, "PARTS")) {
    switch (part.type) {
      case "embed_field":
        fields.push({
          name: text(part, "NAME"),
          value: text(part, "VALUE"),
          inline: checked(part, "INLINE"),
        });
        fieldBlockIds.push(part.id);
        break;
      case "embed_author": {
        if (author) {
          throw new GenerateError("An embed can only have one author.", part.id);
        }
        const iconUrl = text(part, "ICON_URL").trim();
        const linkUrl = text(part, "URL").trim();
        author = {
          name: text(part, "NAME"),
          ...(linkUrl ? { url: linkUrl } : {}),
          ...(iconUrl ? { icon_url: iconUrl } : {}),
        };
        authorBlockId = part.id;
        break;
      }
      case "embed_footer": {
        if (footer) {
          throw new GenerateError("An embed can only have one footer.", part.id);
        }
        const iconUrl = text(part, "ICON_URL").trim();
        footer = {
          text: text(part, "TEXT"),
          ...(iconUrl ? { icon_url: iconUrl } : {}),
        };
        footerBlockId = part.id;
        break;
      }
      case "embed_image":
        if (imageUrl !== undefined) {
          throw new GenerateError("An embed can only have one image.", part.id);
        }
        imageUrl = requiredUrl(part, "URL", "image");
        imageBlockId = part.id;
        break;
      case "embed_thumbnail":
        if (thumbnailUrl !== undefined) {
          throw new GenerateError("An embed can only have one thumbnail.", part.id);
        }
        thumbnailUrl = requiredUrl(part, "URL", "thumbnail");
        thumbnailBlockId = part.id;
        break;
      case "embed_url":
        if (url !== undefined) {
          throw new GenerateError("An embed can only have one title URL.", part.id);
        }
        url = requiredUrl(part, "URL", "title");
        titleUrlBlockId = part.id;
        break;
      default:
        throw new GenerateError("This block cannot be placed inside an embed.", part.id);
    }
  }

  return {
    embed: {
      ...(title.trim() ? { title } : {}),
      ...(url !== undefined ? { url } : {}),
      ...(description.trim() ? { description } : {}),
      ...(color ? { color } : {}),
      ...(author ? { author } : {}),
      ...(footer ? { footer } : {}),
      ...(thumbnailUrl !== undefined ? { thumbnail_url: thumbnailUrl } : {}),
      ...(imageUrl !== undefined ? { image_url: imageUrl } : {}),
      fields,
      timestamp: checked(block, "TIMESTAMP"),
    },
    node: {
      blockId: block.id,
      titleUrlBlockId,
      authorBlockId,
      footerBlockId,
      thumbnailBlockId,
      imageBlockId,
      fieldBlockIds,
    },
  };
}

function readSelectOption(block: Blockly.Block): { option: SelectOption; blockId: string } {
  if (block.type !== "select_option") {
    throw new GenerateError("Only option blocks can go inside a select menu.", block.id);
  }
  const description = text(block, "DESCRIPTION");
  const emoji = text(block, "EMOJI").trim();
  return {
    option: {
      label: text(block, "LABEL"),
      value: text(block, "VALUE").trim(),
      ...(description.trim() ? { description } : {}),
      ...(emoji ? { emoji } : {}),
      default: checked(block, "DEFAULT"),
    },
    blockId: block.id,
  };
}

function readComponents(block: Blockly.Block): {
  components: ComponentRow[];
  nodes: ComponentRowNode[];
} {
  const rows: ComponentRow[] = [];
  const nodes: ComponentRowNode[] = [];
  let currentButtons: ButtonSpec[] = [];
  let currentButtonIds: string[] = [];

  const flushButtons = () => {
    if (currentButtons.length > 0) {
      rows.push({ type: "buttons", buttons: currentButtons });
      nodes.push({ blockId: currentButtonIds[0], buttonBlockIds: currentButtonIds });
      currentButtons = [];
      currentButtonIds = [];
    }
  };
  const pushButton = (button: ButtonSpec, blockId: string) => {
    if (currentButtons.length >= 5) {
      flushButtons();
    }
    currentButtons.push(button);
    currentButtonIds.push(blockId);
  };

  for (const item of stack(block, "COMPONENTS")) {
    switch (item.type) {
      case "component_button": {
        const label = text(item, "LABEL");
        const emoji = text(item, "EMOJI").trim();
        pushButton(
          {
            type: "action",
            id: text(item, "ID").trim(),
            ...(label.trim() ? { label } : {}),
            ...(emoji ? { emoji } : {}),
            style: oneOf(item, "STYLE", BUTTON_STYLES),
            disabled: checked(item, "DISABLED"),
          },
          item.id,
        );
        break;
      }
      case "component_link_button": {
        const label = text(item, "LABEL");
        const emoji = text(item, "EMOJI").trim();
        pushButton(
          {
            type: "link",
            url: text(item, "URL").trim(),
            ...(label.trim() ? { label } : {}),
            ...(emoji ? { emoji } : {}),
            disabled: checked(item, "DISABLED"),
          },
          item.id,
        );
        break;
      }
      case "component_new_row":
        flushButtons();
        break;
      case "component_select_menu": {
        flushButtons();
        const placeholder = text(item, "PLACEHOLDER");
        const options = stack(item, "OPTIONS").map(readSelectOption);
        rows.push({
          type: "select_menu",
          id: text(item, "ID").trim(),
          ...(placeholder.trim() ? { placeholder } : {}),
          min_values: Math.trunc(num(item, "MIN")),
          max_values: Math.trunc(num(item, "MAX")),
          disabled: checked(item, "DISABLED"),
          options: options.map((entry) => entry.option),
        });
        nodes.push({
          blockId: item.id,
          optionBlockIds: options.map((entry) => entry.blockId),
        });
        break;
      }
      case "component_entity_select_menu": {
        flushButtons();
        const kind = oneOf(item, "KIND", ENTITY_SELECT_KINDS);
        const placeholder = text(item, "PLACEHOLDER");
        const channelFilterRaw = text(item, "CHANNEL_FILTER");
        const channel_types =
          kind === "channel" &&
          channelFilterRaw !== "any" &&
          CHANNEL_FILTERS.includes(channelFilterRaw as ChannelFilter)
            ? [channelFilterRaw as ChannelFilter]
            : undefined;
        rows.push({
          type: "entity_select_menu",
          id: text(item, "ID").trim(),
          kind,
          ...(placeholder.trim() ? { placeholder } : {}),
          min_values: Math.trunc(num(item, "MIN")),
          max_values: Math.trunc(num(item, "MAX")),
          disabled: checked(item, "DISABLED"),
          ...(channel_types ? { channel_types } : {}),
        });
        nodes.push({ blockId: item.id });
        break;
      }
      default:
        throw new GenerateError("This block cannot be used as a message component.", item.id);
    }
  }

  flushButtons();
  return { components: rows, nodes };
}

function readPoll(block: Blockly.Block): PollSpec | undefined {
  const pollBlocks = stack(block, "POLL");
  if (pollBlocks.length === 0) {
    return undefined;
  }
  if (pollBlocks.length > 1) {
    throw new GenerateError("A message can only have one poll.", pollBlocks[1].id);
  }
  const pollBlock = pollBlocks[0];
  if (pollBlock.type !== "message_poll") {
    throw new GenerateError("Only a poll block can go in the poll slot.", pollBlock.id);
  }
  const answers: PollAnswerSpec[] = stack(pollBlock, "ANSWERS").map((ans) => {
    if (ans.type !== "poll_answer") {
      throw new GenerateError("Only poll answer blocks can go inside a poll.", ans.id);
    }
    const emoji = optionalText(ans, "EMOJI");
    return {
      text: text(ans, "TEXT"),
      ...(emoji !== undefined ? { emoji } : {}),
    };
  });
  return {
    question: text(pollBlock, "QUESTION"),
    answers,
    duration_hours: Math.trunc(num(pollBlock, "DURATION")),
    allow_multiselect: checked(pollBlock, "MULTISELECT"),
  };
}

function readAttachments(block: Blockly.Block): AttachmentSpec[] {
  return stack(block, "ATTACHMENTS").map((att) => {
    if (att.type !== "message_attachment") {
      throw new GenerateError("Only file blocks can go in the files slot.", att.id);
    }
    return {
      filename: text(att, "FILENAME"),
      content: text(att, "CONTENT"),
    };
  });
}

function readRichBody(
  block: Blockly.Block,
  allowPollAndFiles: boolean,
): {
  body: {
    content: string;
    embeds?: Embed[];
    components?: ComponentRow[];
    poll?: PollSpec;
    attachments?: AttachmentSpec[];
  };
  embeds?: EmbedNode[];
  components?: ComponentRowNode[];
} {
  const embeds = stack(block, "EMBEDS").map(readEmbed);
  const { components, nodes: componentNodes } = readComponents(block);
  const poll = allowPollAndFiles ? readPoll(block) : undefined;
  const attachments = allowPollAndFiles ? readAttachments(block) : [];
  return {
    body: {
      content: text(block, "CONTENT"),
      ...(embeds.length > 0 ? { embeds: embeds.map((entry) => entry.embed) } : {}),
      ...(components.length > 0 ? { components } : {}),
      ...(poll !== undefined ? { poll } : {}),
      ...(attachments.length > 0 ? { attachments } : {}),
    },
    ...(embeds.length > 0 ? { embeds: embeds.map((entry) => entry.node) } : {}),
    ...(componentNodes.length > 0 ? { components: componentNodes } : {}),
  };
}

function readModalInput(block: Blockly.Block): ModalInputSpec {
  if (block.type !== "modal_text_input") {
    throw new GenerateError("Only modal input blocks can go inside a modal.", block.id);
  }
  const placeholder = optionalText(block, "PLACEHOLDER");
  const valueText = optionalText(block, "VALUE");
  return {
    id: text(block, "ID").trim(),
    label: text(block, "LABEL"),
    style: oneOf(block, "STYLE", TEXT_INPUT_STYLES),
    ...(placeholder !== undefined ? { placeholder } : {}),
    required: checked(block, "REQUIRED"),
    ...(valueText !== undefined ? { value: valueText } : {}),
  };
}

function readStatement(block: Blockly.Block): { statement: Statement; node: StatementNode } {
  switch (block.type) {
    case "reply":
      return {
        statement: {
          type: "reply",
          content: text(block, "CONTENT"),
          ephemeral: checked(block, "EPHEMERAL"),
        },
        node: { blockId: block.id },
      };
    case "reply_rich": {
      const rich = readRichBody(block, true);
      return {
        statement: {
          type: "reply",
          ...rich.body,
          ephemeral: checked(block, "EPHEMERAL"),
        },
        node: { blockId: block.id, embeds: rich.embeds, components: rich.components },
      };
    }
    case "send_message":
      return {
        statement: {
          type: "send_message",
          channel_id: text(block, "CHANNEL_ID").trim(),
          content: text(block, "CONTENT"),
        },
        node: { blockId: block.id },
      };
    case "send_rich_message": {
      const rich = readRichBody(block, true);
      const replyTo = optionalText(block, "REPLY_TO");
      return {
        statement: {
          type: "send_message",
          channel_id: text(block, "CHANNEL_ID").trim(),
          ...rich.body,
          ...(replyTo !== undefined ? { reply_to_message_id: replyTo } : {}),
        },
        node: { blockId: block.id, embeds: rich.embeds, components: rich.components },
      };
    }
    case "update_message": {
      const rich = readRichBody(block, false);
      return {
        statement: {
          type: "update_message",
          ...rich.body,
        },
        node: { blockId: block.id, embeds: rich.embeds, components: rich.components },
      };
    }
    case "show_modal":
      return {
        statement: {
          type: "show_modal",
          id: text(block, "ID").trim(),
          title: text(block, "TITLE"),
          inputs: stack(block, "INPUTS").map(readModalInput),
        },
        node: { blockId: block.id },
      };
    case "create_thread_from_message":
      return {
        statement: {
          type: "create_thread_from_message",
          name: text(block, "NAME"),
          auto_archive_minutes: Math.trunc(num(block, "ARCHIVE")),
        },
        node: { blockId: block.id },
      };
    case "create_thread":
      return {
        statement: {
          type: "create_thread",
          channel_id: text(block, "CHANNEL_ID").trim(),
          name: text(block, "NAME"),
          private: checked(block, "PRIVATE"),
          auto_archive_minutes: Math.trunc(num(block, "ARCHIVE")),
        },
        node: { blockId: block.id },
      };
    case "create_forum_post": {
      const embeds = stack(block, "EMBEDS").map(readEmbed);
      return {
        statement: {
          type: "create_forum_post",
          channel_id: text(block, "CHANNEL_ID").trim(),
          title: text(block, "TITLE"),
          content: text(block, "CONTENT"),
          ...(embeds.length > 0 ? { embeds: embeds.map((entry) => entry.embed) } : {}),
        },
        node: {
          blockId: block.id,
          ...(embeds.length > 0 ? { embeds: embeds.map((entry) => entry.node) } : {}),
        },
      };
    }
    case "join_thread":
      return {
        statement: {
          type: "join_thread",
          thread_id: text(block, "THREAD_ID").trim(),
        },
        node: { blockId: block.id },
      };
    case "add_thread_member":
      return {
        statement: {
          type: "add_thread_member",
          thread_id: text(block, "THREAD_ID").trim(),
          member_id: text(block, "MEMBER_ID").trim(),
        },
        node: { blockId: block.id },
      };
    case "remove_thread_member":
      return {
        statement: {
          type: "remove_thread_member",
          thread_id: text(block, "THREAD_ID").trim(),
          member_id: text(block, "MEMBER_ID").trim(),
        },
        node: { blockId: block.id },
      };
    case "pin_message":
      return {
        statement: {
          type: "pin_message",
          channel_id: text(block, "CHANNEL_ID").trim(),
          message_id: text(block, "MESSAGE_ID").trim(),
        },
        node: { blockId: block.id },
      };
    case "unpin_message":
      return {
        statement: {
          type: "unpin_message",
          channel_id: text(block, "CHANNEL_ID").trim(),
          message_id: text(block, "MESSAGE_ID").trim(),
        },
        node: { blockId: block.id },
      };
    case "publish_message":
      return {
        statement: {
          type: "publish_message",
          channel_id: text(block, "CHANNEL_ID").trim(),
          message_id: text(block, "MESSAGE_ID").trim(),
        },
        node: { blockId: block.id },
      };
    case "bulk_delete_messages":
      return {
        statement: {
          type: "bulk_delete_messages",
          channel_id: text(block, "CHANNEL_ID").trim(),
          count: Math.trunc(num(block, "COUNT")),
        },
        node: { blockId: block.id },
      };
    case "react":
      return {
        statement: { type: "react", emoji: text(block, "EMOJI").trim() },
        node: { blockId: block.id },
      };
    case "add_role":
      return {
        statement: { type: "add_role", role_id: text(block, "ROLE_ID").trim() },
        node: { blockId: block.id },
      };
    case "remove_role":
      return {
        statement: { type: "remove_role", role_id: text(block, "ROLE_ID").trim() },
        node: { blockId: block.id },
      };
    case "timeout_member":
      return {
        statement: {
          type: "timeout",
          member_id: text(block, "MEMBER_ID").trim(),
          seconds: Math.trunc(num(block, "MINUTES")) * 60,
          reason: text(block, "REASON"),
        },
        node: { blockId: block.id },
      };
    case "kick_member":
      return {
        statement: {
          type: "kick",
          member_id: text(block, "MEMBER_ID").trim(),
          reason: text(block, "REASON"),
        },
        node: { blockId: block.id },
      };
    case "ban_member":
      return {
        statement: {
          type: "ban",
          member_id: text(block, "MEMBER_ID").trim(),
          reason: text(block, "REASON"),
        },
        node: { blockId: block.id },
      };
    case "delete_triggering_message":
      return { statement: { type: "delete_message" }, node: { blockId: block.id } };
    case "direct_message_member":
      return {
        statement: {
          type: "direct_message",
          member_id: text(block, "MEMBER_ID").trim(),
          content: text(block, "CONTENT"),
        },
        node: { blockId: block.id },
      };
    case "logic_if": {
      const cond = value(block, "CONDITION");
      const thenBranch = stack(block, "THEN").map(readStatement);
      const elseBranch = stack(block, "ELSE").map(readStatement);
      return {
        statement: {
          type: "if",
          condition: cond.expr,
          then: thenBranch.map((entry) => entry.statement),
          else: elseBranch.map((entry) => entry.statement),
        },
        node: {
          blockId: block.id,
          expr: cond.node,
          then: thenBranch.map((entry) => entry.node),
          else: elseBranch.map((entry) => entry.node),
        },
      };
    }
    case "variable_set": {
      const val = value(block, "VALUE");
      return {
        statement: {
          type: "set_variable",
          scope: oneOf(block, "SCOPE", SCOPES),
          name: text(block, "NAME"),
          value: val.expr,
        },
        node: { blockId: block.id, expr: val.node },
      };
    }
    case "variable_change": {
      const by = value(block, "BY");
      return {
        statement: {
          type: "change_variable",
          scope: oneOf(block, "SCOPE", SCOPES),
          name: text(block, "NAME"),
          by: by.expr,
        },
        node: { blockId: block.id, expr: by.node },
      };
    }
    case "flow_wait":
      return {
        statement: { type: "wait", seconds: Math.trunc(num(block, "SECONDS")) },
        node: { blockId: block.id },
      };
    case "flow_stop":
      return { statement: { type: "stop" }, node: { blockId: block.id } };
    default:
      throw new GenerateError("This block cannot be used as an action.", block.id);
  }
}

/**
 * Turn the editor's blocks into the program the server runs, along with a
 * source map from server error locations back to Blockly block IDs.
 *
 * @throws {GenerateError} when an attached block is incomplete.
 */
export function generateProgramWithMap(workspace: Blockly.Workspace): {
  program: Program;
  map: ProgramMap;
} {
  const handlers: Handler[] = [];
  const handlerNodes: HandlerNode[] = [];
  for (const block of workspace.getTopBlocks(true)) {
    if (!block.isEnabled()) {
      continue;
    }
    const parsedTrigger = readTriggerWithMap(block);
    if (!parsedTrigger) {
      continue;
    }
    const actions = stack(block, "ACTIONS").map(readStatement);
    handlers.push({
      trigger: parsedTrigger.trigger,
      actions: actions.map((entry) => entry.statement),
    });
    handlerNodes.push({
      triggerBlockId: block.id,
      optionBlockIds: parsedTrigger.optionBlockIds,
      actions: actions.map((entry) => entry.node),
    });
  }
  return { program: { handlers }, map: { handlers: handlerNodes } };
}

/**
 * Turn the editor's blocks into the program the server runs. Triggers are
 * ordered top to bottom, as the server numbers them in its error messages.
 * Blocks that are not attached to a trigger are ignored.
 *
 * @throws {GenerateError} when an attached block is incomplete.
 */
export function generateProgram(workspace: Blockly.Workspace): Program {
  return generateProgramWithMap(workspace).program;
}

/** Resolve a server `ErrorLocation` to the most specific Blockly block ID. */
export function resolveErrorBlockId(map: ProgramMap, location: ErrorLocation): string | null {
  const handler = map.handlers[location.handler];
  if (!handler) {
    return null;
  }
  let best = handler.triggerBlockId;
  if (location.option !== undefined) {
    return handler.optionBlockIds[location.option] ?? best;
  }

  let list = handler.actions;
  let currentStmt: StatementNode | undefined;
  for (const stepRef of location.steps ?? []) {
    const stmt = list[stepRef.index];
    if (!stmt) {
      return best;
    }
    best = stmt.blockId;
    currentStmt = stmt;
    if (stepRef.branch === "then") {
      list = stmt.then ?? [];
    } else if (stepRef.branch === "else") {
      list = stmt.else ?? [];
    }
  }

  if (!currentStmt) {
    return best;
  }

  if (location.expr && location.expr.length > 0 && currentStmt.expr) {
    let exprNode: ExprNode | undefined = location.expr[0] === 0 ? currentStmt.expr : undefined;
    for (let i = 1; exprNode && i < location.expr.length; i += 1) {
      best = exprNode.blockId;
      exprNode = exprNode.children[location.expr[i]];
    }
    if (exprNode) {
      best = exprNode.blockId;
    }
    return best;
  }

  if (location.embed) {
    const embedNode = currentStmt.embeds?.[location.embed.index];
    if (embedNode) {
      best = embedNode.blockId;
      const part = location.embed.part;
      if (part) {
        switch (part.type) {
          case "title_url":
            return embedNode.titleUrlBlockId ?? best;
          case "author":
            return embedNode.authorBlockId ?? best;
          case "footer":
            return embedNode.footerBlockId ?? best;
          case "thumbnail":
            return embedNode.thumbnailBlockId ?? best;
          case "image":
            return embedNode.imageBlockId ?? best;
          case "field":
            return embedNode.fieldBlockIds[part.index] ?? best;
        }
      }
    }
    return best;
  }

  if (location.component) {
    const rowNode = currentStmt.components?.[location.component.row];
    if (rowNode) {
      best = rowNode.blockId;
      if (location.component.button !== undefined) {
        best = rowNode.buttonBlockIds?.[location.component.button] ?? best;
      } else if (location.component.option !== undefined) {
        best = rowNode.optionBlockIds?.[location.component.option] ?? best;
      }
    }
  }

  return best;
}

/** Find enabled trigger blocks whose required privileged intent is disabled on Discord. */
export function findMissingIntents(
  workspace: Blockly.Workspace,
  intents: BotIntents,
): MissingIntent[] {
  const missing: MissingIntent[] = [];
  for (const block of workspace.getTopBlocks(true)) {
    if (!block.isEnabled()) {
      continue;
    }
    if (
      (block.type === "member_join" ||
        block.type === "member_leave" ||
        block.type === "member_update") &&
      !intents.members
    ) {
      missing.push({
        intent: "members",
        label: "Server Members Intent",
        blockId: block.id,
        message:
          "Server Members Intent is disabled in the Discord developer portal. Enable it under Bot before saving.",
      });
    } else if (
      (block.type === "message_contains" || block.type === "message_edit") &&
      !intents.message_content
    ) {
      missing.push({
        intent: "message_content",
        label: "Message Content Intent",
        blockId: block.id,
        message:
          "Message Content Intent is disabled in the Discord developer portal. Enable it under Bot before saving.",
      });
    }
  }
  return missing;
}
