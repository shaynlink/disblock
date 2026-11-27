/** Mirrors `disblock_program::Program`, the JSON the server validates. */
export interface Program {
  handlers: Handler[];
}

export interface Handler {
  trigger: Trigger;
  actions: Statement[];
}

export type ScheduleUnit = "minutes" | "hours" | "days";

export type Trigger =
  | { type: "slash_command"; name: string; description: string; options: CommandOption[] }
  | { type: "member_join" }
  | { type: "member_leave" }
  | { type: "message_contains"; pattern: string; case_sensitive: boolean }
  | { type: "reaction_add"; emoji?: string; message_id?: string }
  | { type: "reaction_remove"; emoji?: string; message_id?: string }
  | { type: "button_click"; id: string }
  | { type: "select_menu"; id: string }
  | { type: "schedule"; interval: number; unit: ScheduleUnit };

export type OptionKind =
  | "string"
  | "integer"
  | "number"
  | "boolean"
  | "user"
  | "channel"
  | "role"
  | "mentionable"
  | "attachment";

export interface CommandOption {
  name: string;
  description: string;
  kind: OptionKind;
  required: boolean;
}

/** Where a variable lives: this run only, the member, or the whole server. */
export type Scope = "local" | "member" | "server";

export type MathOp = "add" | "subtract" | "multiply" | "divide" | "modulo";

export type CompareOp =
  | "equal"
  | "not_equal"
  | "less"
  | "less_or_equal"
  | "greater"
  | "greater_or_equal"
  | "contains";

export interface EmbedAuthor {
  name: string;
  url?: string;
  icon_url?: string;
}

export interface EmbedFooter {
  text: string;
  icon_url?: string;
}

export interface EmbedField {
  name: string;
  value: string;
  inline: boolean;
}

export interface Embed {
  title?: string;
  url?: string;
  description?: string;
  color?: string;
  author?: EmbedAuthor;
  footer?: EmbedFooter;
  thumbnail_url?: string;
  image_url?: string;
  fields: EmbedField[];
  timestamp: boolean;
}

export type ButtonStyle = "primary" | "secondary" | "success" | "danger";

export type ButtonSpec =
  | {
      type: "action";
      id: string;
      label?: string;
      emoji?: string;
      style: ButtonStyle;
      disabled: boolean;
    }
  | {
      type: "link";
      url: string;
      label?: string;
      emoji?: string;
      disabled: boolean;
    };

export interface SelectOption {
  label: string;
  value: string;
  description?: string;
  emoji?: string;
  default: boolean;
}

export type ComponentRow =
  | { type: "buttons"; buttons: ButtonSpec[] }
  | {
      type: "select_menu";
      id: string;
      placeholder?: string;
      min_values?: number;
      max_values?: number;
      disabled: boolean;
      options: SelectOption[];
    };

export type Statement =
  | {
      type: "reply";
      content: string;
      ephemeral: boolean;
      embeds?: Embed[];
      components?: ComponentRow[];
    }
  | {
      type: "send_message";
      channel_id: string;
      content: string;
      embeds?: Embed[];
      components?: ComponentRow[];
    }
  | {
      type: "update_message";
      content: string;
      embeds?: Embed[];
      components?: ComponentRow[];
    }
  | { type: "add_role"; role_id: string }
  | { type: "remove_role"; role_id: string }
  | { type: "timeout"; member_id: string; seconds: number; reason: string }
  | { type: "kick"; member_id: string; reason: string }
  | { type: "ban"; member_id: string; reason: string }
  | { type: "delete_message" }
  | { type: "direct_message"; member_id: string; content: string }
  | { type: "react"; emoji: string }
  | { type: "if"; condition: Expr; then: Statement[]; else: Statement[] }
  | { type: "set_variable"; scope: Scope; name: string; value: Expr }
  | { type: "change_variable"; scope: Scope; name: string; by: Expr }
  | { type: "wait"; seconds: number }
  | { type: "stop" };

export type Expr =
  | { type: "text"; value: string }
  | { type: "number"; value: number }
  | { type: "boolean"; value: boolean }
  | { type: "variable"; scope: Scope; name: string }
  | { type: "option"; name: string }
  | { type: "selected" }
  | { type: "random"; min: Expr; max: Expr }
  | { type: "math"; op: MathOp; left: Expr; right: Expr }
  | { type: "compare"; op: CompareOp; left: Expr; right: Expr }
  | { type: "and"; left: Expr; right: Expr }
  | { type: "or"; left: Expr; right: Expr }
  | { type: "not"; value: Expr }
  | { type: "has_role"; role_id: string };

export interface User {
  id: string;
  discord_id: string;
  username: string;
  avatar: string | null;
}

export type BotStatus = "stopped" | "starting" | "online" | "error";

export interface Bot {
  id: string;
  application_id: string;
  name: string;
  avatar: string | null;
  enabled: boolean;
  status: BotStatus;
  status_detail: string | null;
  created_at: string;
}

export interface BotDetail extends Bot {
  workspace: object;
  program: Program;
}

export interface BotIntents {
  members: boolean;
  message_content: boolean;
}

export interface StepRef {
  index: number;
  branch?: "then" | "else";
}

export type EmbedPartLocation =
  | { type: "title_url" }
  | { type: "author" }
  | { type: "footer" }
  | { type: "thumbnail" }
  | { type: "image" }
  | { type: "field"; index: number };

export interface EmbedLocation {
  index: number;
  part?: EmbedPartLocation;
}

export interface ComponentLocation {
  row: number;
  button?: number;
  option?: number;
}

export interface ErrorLocation {
  handler: number;
  option?: number;
  steps?: StepRef[];
  expr?: number[];
  embed?: EmbedLocation;
  component?: ComponentLocation;
}

export interface ExecutionEntry {
  id: number;
  trigger: string;
  success: boolean;
  error: string | null;
  duration_ms: number;
  created_at: string;
}
