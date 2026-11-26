/** Mirrors `disblock_program::Program`, the JSON the server validates. */
export interface Program {
  handlers: Handler[];
}

export interface Handler {
  trigger: Trigger;
  actions: Action[];
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

export type Action =
  | { type: "reply"; content: string; ephemeral: boolean }
  | { type: "send_message"; channel_id: string; content: string }
  | { type: "add_role"; role_id: string }
  | { type: "remove_role"; role_id: string }
  | { type: "react"; emoji: string };

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
