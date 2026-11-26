/** Mirrors `disblock_program::Program`, the JSON the server validates. */
export interface Program {
  handlers: Handler[];
}

export interface Handler {
  trigger: Trigger;
  actions: Statement[];
}

export type Trigger =
  | { type: "slash_command"; name: string; description: string; options: CommandOption[] }
  | { type: "member_join" }
  | { type: "message_contains"; pattern: string; case_sensitive: boolean };

export type OptionKind = "string" | "integer" | "boolean" | "user";

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

export type Statement =
  | { type: "reply"; content: string; ephemeral: boolean }
  | { type: "send_message"; channel_id: string; content: string }
  | { type: "add_role"; role_id: string }
  | { type: "remove_role"; role_id: string }
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

export interface ExecutionEntry {
  id: number;
  trigger: string;
  success: boolean;
  error: string | null;
  duration_ms: number;
  created_at: string;
}
