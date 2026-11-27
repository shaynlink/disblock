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
  | {
      type: "slash_subcommand";
      command: string;
      group?: string;
      subcommand: string;
      description: string;
      options: CommandOption[];
    }
  | { type: "user_command"; name: string }
  | { type: "message_command"; name: string }
  | { type: "modal_submit"; id: string }
  | { type: "member_join" }
  | { type: "member_leave" }
  | { type: "member_update" }
  | { type: "member_ban" }
  | { type: "member_unban" }
  | { type: "message_contains"; pattern: string; case_sensitive: boolean }
  | { type: "message_edit" }
  | { type: "message_delete" }
  | { type: "reaction_add"; emoji?: string; message_id?: string }
  | { type: "reaction_remove"; emoji?: string; message_id?: string }
  | { type: "thread_create" }
  | { type: "invite_create" }
  | { type: "automod_execution"; rule_id?: string }
  | { type: "poll_vote_add" }
  | { type: "voice_state_update" }
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

export interface CommandOptionChoice {
  name: string;
  value: string;
}

export interface CommandOption {
  name: string;
  description: string;
  kind: OptionKind;
  required: boolean;
  choices?: CommandOptionChoice[];
  autocomplete?: string[];
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

export type TargetField = "id" | "mention" | "name" | "content" | "author_id" | "author_mention";

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

export type EntitySelectKind = "user" | "role" | "mentionable" | "channel";

export type ChannelFilter = "text" | "voice" | "announcement" | "forum";

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
    }
  | {
      type: "entity_select_menu";
      id: string;
      kind: EntitySelectKind;
      placeholder?: string;
      min_values?: number;
      max_values?: number;
      disabled: boolean;
      channel_types?: ChannelFilter[];
    };

export type TextInputStyle = "short" | "paragraph";

export interface ModalInputSpec {
  id: string;
  label: string;
  style: TextInputStyle;
  placeholder?: string;
  required: boolean;
  min_length?: number;
  max_length?: number;
  value?: string;
}

export interface PollAnswerSpec {
  text: string;
  emoji?: string;
}

export interface PollSpec {
  question: string;
  answers: PollAnswerSpec[];
  duration_hours: number;
  allow_multiselect: boolean;
}

export interface AttachmentSpec {
  filename: string;
  content: string;
}

export type Statement =
  | {
      type: "reply";
      content: string;
      ephemeral: boolean;
      embeds?: Embed[];
      components?: ComponentRow[];
      poll?: PollSpec;
      attachments?: AttachmentSpec[];
    }
  | {
      type: "send_message";
      channel_id: string;
      content: string;
      embeds?: Embed[];
      components?: ComponentRow[];
      poll?: PollSpec;
      attachments?: AttachmentSpec[];
      reply_to_message_id?: string;
    }
  | {
      type: "update_message";
      content: string;
      embeds?: Embed[];
      components?: ComponentRow[];
    }
  | {
      type: "show_modal";
      id: string;
      title: string;
      inputs: ModalInputSpec[];
    }
  | {
      type: "create_thread_from_message";
      name: string;
      auto_archive_minutes?: number;
    }
  | {
      type: "create_thread";
      channel_id: string;
      name: string;
      private: boolean;
      auto_archive_minutes?: number;
    }
  | {
      type: "create_forum_post";
      channel_id: string;
      title: string;
      content: string;
      embeds?: Embed[];
    }
  | { type: "join_thread"; thread_id: string }
  | { type: "add_thread_member"; thread_id: string; member_id: string }
  | { type: "remove_thread_member"; thread_id: string; member_id: string }
  | { type: "pin_message"; channel_id: string; message_id: string }
  | { type: "unpin_message"; channel_id: string; message_id: string }
  | { type: "publish_message"; channel_id: string; message_id: string }
  | { type: "bulk_delete_messages"; channel_id: string; count: number }
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
  | { type: "modal_field"; id: string }
  | { type: "target"; field: TargetField }
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

export interface BotChannelResource {
  id: string;
  name: string;
  kind: string;
  parent_id: string | null;
}

export interface BotRoleResource {
  id: string;
  name: string;
  color: number;
  managed: boolean;
}

export interface BotMemberResource {
  id: string;
  name: string;
  username: string;
}

export interface BotGuildResources {
  id: string;
  name: string;
  icon: string | null;
  channels: BotChannelResource[];
  roles: BotRoleResource[];
  members: BotMemberResource[];
}

export interface BotResources {
  guilds: BotGuildResources[];
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
