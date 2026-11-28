import type {
  AttachmentSpec,
  ComponentRow,
  Embed,
  ModalInputSpec,
  PollSpec,
  Program,
  Statement,
  Trigger,
} from "../types.ts";

export interface PreviewMessage {
  handlerIndex: number;
  trigger: string;
  kind: "reply" | "message" | "updated message" | "direct message" | "modal" | "forum post";
  content: string;
  embeds?: Embed[];
  components?: ComponentRow[];
  poll?: PollSpec;
  attachments?: AttachmentSpec[];
  modal?: { title: string; inputs: ModalInputSpec[] };
  ephemeral: boolean;
  condition?: "when the condition is met" | "otherwise";
}

function triggerLabel(trigger: Trigger): string {
  switch (trigger.type) {
    case "slash_command":
      return `/${trigger.name || "command"}`;
    case "slash_subcommand":
      return trigger.group
        ? `/${trigger.command || "command"} ${trigger.group} ${trigger.subcommand || "sub"}`
        : `/${trigger.command || "command"} ${trigger.subcommand || "sub"}`;
    case "user_command":
      return `user menu “${trigger.name || "…"}”`;
    case "message_command":
      return `message menu “${trigger.name || "…"}”`;
    case "modal_submit":
      return `modal “${trigger.id || "…"}” is submitted`;
    case "member_join":
      return "a member joining";
    case "member_leave":
      return "a member leaving";
    case "member_update":
      return "a member updating";
    case "member_ban":
      return "a member being banned";
    case "member_unban":
      return "a user being unbanned";
    case "message_contains":
      return `a message containing “${trigger.pattern || "…"}”`;
    case "message_edit":
      return "a message being edited";
    case "message_delete":
      return "a message being deleted";
    case "reaction_add":
      return "a reaction being added";
    case "reaction_remove":
      return "a reaction being removed";
    case "thread_create":
      return "a thread being created";
    case "invite_create":
      return "an invite being created";
    case "automod_execution":
      return trigger.rule_id ? `AutoMod rule ${trigger.rule_id} runs` : "AutoMod runs";
    case "poll_vote_add":
      return "someone voting on a poll";
    case "voice_state_update":
      return "a voice state changing";
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
        ...(statement.poll ? { poll: statement.poll } : {}),
        ...(statement.attachments ? { attachments: statement.attachments } : {}),
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
        ...(statement.poll ? { poll: statement.poll } : {}),
        ...(statement.attachments ? { attachments: statement.attachments } : {}),
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
    case "show_modal":
      return {
        handlerIndex,
        trigger,
        kind: "modal",
        content: "",
        modal: { title: statement.title, inputs: statement.inputs },
        ephemeral: false,
        condition,
      };
    case "create_forum_post":
      return {
        handlerIndex,
        trigger,
        kind: "forum post",
        content: statement.content,
        embeds: [
          {
            title: statement.title,
            fields: [],
            timestamp: false,
          },
          ...(statement.embeds ?? []),
        ],
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
    .replaceAll("{channel.id}", "123456789012345679")
    .replaceAll("{channel}", "#general")
    .replaceAll("{message.id}", "123456789012345678")
    .replaceAll("{message.content}", "Sample message text")
    .replaceAll("{emoji}", "🎉")
    .replaceAll("{selected}", "First option")
    .replaceAll("{target.name}", "TargetUser")
    .replaceAll("{target.id}", "987654321098765432")
    .replaceAll("{target.content}", "Right-clicked message")
    .replaceAll("{target.author.id}", "987654321098765432")
    .replaceAll("{target.author}", "@TargetUser")
    .replaceAll("{target}", "@TargetUser")
    .replaceAll("{thread.name}", "Discussion")
    .replaceAll("{invite.code}", "abc123")
    .replaceAll("{automod.rule_id}", "111222333444555666")
    .replaceAll("{automod.keyword}", "blocked")
    .replaceAll("{automod.content}", "flagged message")
    .replaceAll("{poll.answer_id}", "1")
    .replace(/\{option\.[^}]+\}/g, "example")
    .replace(/\{modal\.[^}]+\}/g, "Submitted text")
    .replace(/\{(?:var|member|server):[^}]+\}/g, "42");
}
