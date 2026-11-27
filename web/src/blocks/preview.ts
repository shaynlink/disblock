import type { ComponentRow, Embed, Program, Statement, Trigger } from "../types.ts";

export interface PreviewMessage {
  handlerIndex: number;
  trigger: string;
  kind: "reply" | "message" | "updated message" | "direct message";
  content: string;
  embeds?: Embed[];
  components?: ComponentRow[];
  ephemeral: boolean;
  condition?: "when the condition is met" | "otherwise";
}

function triggerLabel(trigger: Trigger): string {
  switch (trigger.type) {
    case "slash_command":
      return `/${trigger.name || "command"}`;
    case "member_join":
      return "a member joining";
    case "member_leave":
      return "a member leaving";
    case "message_contains":
      return `a message containing “${trigger.pattern || "…"}”`;
    case "reaction_add":
      return "a reaction being added";
    case "reaction_remove":
      return "a reaction being removed";
    case "button_click":
      return `button “${trigger.id || "…"}”`;
    case "select_menu":
      return `menu “${trigger.id || "…"}”`;
    case "schedule":
      return `every ${trigger.interval} ${trigger.unit}`;
  }
}

function asPreviewMessage(
  statement: Statement,
  handlerIndex: number,
  trigger: string,
  condition: PreviewMessage["condition"],
): PreviewMessage | null {
  switch (statement.type) {
    case "reply":
      return {
        handlerIndex,
        trigger,
        kind: "reply",
        content: statement.content,
        embeds: statement.embeds,
        components: statement.components,
        ephemeral: statement.ephemeral,
        condition,
      };
    case "send_message":
      return {
        handlerIndex,
        trigger,
        kind: "message",
        content: statement.content,
        embeds: statement.embeds,
        components: statement.components,
        ephemeral: false,
        condition,
      };
    case "update_message":
      return {
        handlerIndex,
        trigger,
        kind: "updated message",
        content: statement.content,
        embeds: statement.embeds,
        components: statement.components,
        ephemeral: false,
        condition,
      };
    case "direct_message":
      return {
        handlerIndex,
        trigger,
        kind: "direct message",
        content: statement.content,
        ephemeral: false,
        condition,
      };
    default:
      return null;
  }
}

function collectMessages(
  statements: Statement[],
  handlerIndex: number,
  trigger: string,
  condition: PreviewMessage["condition"],
): PreviewMessage[] {
  const messages: PreviewMessage[] = [];
  for (const statement of statements) {
    if (statement.type === "if") {
      messages.push(
        ...collectMessages(statement.then, handlerIndex, trigger, "when the condition is met"),
        ...collectMessages(statement.else, handlerIndex, trigger, "otherwise"),
      );
      continue;
    }
    const message = asPreviewMessage(statement, handlerIndex, trigger, condition);
    if (message) {
      messages.push(message);
    }
  }
  return messages;
}

/** The message-producing parts of a program, in their workspace order. */
export function getPreviewMessages(program: Program): PreviewMessage[] {
  return program.handlers.flatMap((handler, handlerIndex) =>
    collectMessages(handler.actions, handlerIndex, triggerLabel(handler.trigger), undefined),
  );
}

/** Replace runtime placeholders with stable sample data for the visual preview. */
export function previewText(value: string): string {
  return value
    .replaceAll("{user.name}", "SampleUser")
    .replaceAll("{user.id}", "123456789012345678")
    .replaceAll("{user}", "@SampleUser")
    .replaceAll("{channel}", "#general")
    .replaceAll("{message.id}", "123456789012345678")
    .replaceAll("{emoji}", "🎉")
    .replaceAll("{selected}", "First option")
    .replace(/\{option\.[^}]+\}/g, "example")
    .replace(/\{(?:var|member|server):[^}]+\}/g, "42");
}
