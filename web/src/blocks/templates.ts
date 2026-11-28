import * as Blockly from "blockly";
import { starterWorkspace } from "./blocks.ts";

export type TemplateId = "welcome" | "levels" | "reaction_roles" | "moderation" | "tickets";

export interface TemplateField {
  key: string;
  label: string;
  placeholder: string;
  hint: string;
  defaultValue?: string;
  kind?: "channel" | "role";
}

export interface TemplateBlockState {
  type: string;
  x?: number;
  y?: number;
  fields?: Record<string, unknown>;
  inputs?: Record<string, { block: TemplateBlockState }>;
  next?: { block: TemplateBlockState };
}

export interface WorkspaceState {
  blocks: {
    languageVersion: number;
    blocks: TemplateBlockState[];
  };
}

export interface BotTemplate {
  id: TemplateId;
  name: string;
  category: string;
  summary: string;
  highlights: string[];
  intents: Array<"members" | "message_content">;
  fields: TemplateField[];
  build: (values: Record<string, string>) => WorkspaceState;
}

function fieldValue(values: Record<string, string>, key: string, fallback = ""): string {
  const raw = values[key]?.trim();
  return raw ? raw : fallback;
}

function richEmbed(spec: {
  title: string;
  description: string;
  color: string;
  timestamp?: boolean;
  footer?: string;
  fields?: Array<{ name: string; value: string; inline?: boolean }>;
}): TemplateBlockState {
  let parts: TemplateBlockState | undefined;
  const partList: TemplateBlockState[] = [];

  for (const field of spec.fields ?? []) {
    partList.push({
      type: "embed_field",
      fields: { NAME: field.name, VALUE: field.value, INLINE: Boolean(field.inline) },
    });
  }
  if (spec.footer) {
    partList.push({
      type: "embed_footer",
      fields: { TEXT: spec.footer, ICON_URL: "" },
    });
  }
  for (let i = partList.length - 1; i >= 0; i -= 1) {
    parts = {
      ...partList[i],
      ...(parts ? { next: { block: parts } } : {}),
    };
  }

  return {
    type: "embed",
    fields: {
      TITLE: spec.title,
      DESCRIPTION: spec.description,
      COLOR: spec.color,
      TIMESTAMP: Boolean(spec.timestamp),
    },
    ...(parts ? { inputs: { PARTS: { block: parts } } } : {}),
  };
}

function chain(blocks: TemplateBlockState[]): TemplateBlockState {
  let current = blocks[blocks.length - 1];
  for (let i = blocks.length - 2; i >= 0; i -= 1) {
    current = {
      ...blocks[i],
      next: { block: current },
    };
  }
  return current;
}

function slashOption(spec: {
  name: string;
  description: string;
  kind:
    | "string"
    | "integer"
    | "number"
    | "boolean"
    | "user"
    | "channel"
    | "role"
    | "mentionable"
    | "attachment";
  required: boolean;
}): TemplateBlockState {
  return {
    type: "command_option",
    fields: {
      NAME: spec.name,
      DESCRIPTION: spec.description,
      KIND: spec.kind,
      REQUIRED: spec.required,
    },
  };
}

export const BOT_TEMPLATES: BotTemplate[] = [
  {
    id: "welcome",
    name: "Welcome & Goodbye",
    category: "Community",
    summary:
      "Greet new members with an embed card, count arrivals, grant a starter role, and announce departures.",
    highlights: [
      "Join & leave embeds",
      "Auto starter role",
      "Member counter ({server:member_joins})",
    ],
    intents: ["members"],
    fields: [
      {
        key: "channelId",
        label: "Welcome channel ID",
        placeholder: "Right-click a channel in Discord → Copy Channel ID",
        hint: "Used for welcome and goodbye messages.",
        kind: "channel",
      },
      {
        key: "roleId",
        label: "Starter role ID",
        placeholder: "Server Settings → Roles → Copy Role ID",
        hint: "Automatically assigned when someone joins.",
        kind: "role",
      },
    ],
    build(values) {
      const channelId = fieldValue(values, "channelId");
      const roleId = fieldValue(values, "roleId");
      return {
        blocks: {
          languageVersion: 0,
          blocks: [
            {
              type: "member_join",
              x: 40,
              y: 40,
              inputs: {
                ACTIONS: {
                  block: chain([
                    { type: "add_role", fields: { ROLE_ID: roleId } },
                    {
                      type: "variable_change",
                      fields: { SCOPE: "server", NAME: "member_joins" },
                      inputs: { BY: { block: { type: "value_number", fields: { NUM: 1 } } } },
                    },
                    {
                      type: "send_rich_message",
                      fields: { CONTENT: "Welcome {user}!", CHANNEL_ID: channelId },
                      inputs: {
                        EMBEDS: {
                          block: richEmbed({
                            title: "Welcome {user.name}!",
                            description:
                              "Glad to have you here, {user}. You are join #{server:member_joins}.",
                            color: "#57F287",
                            footer: "User ID: {user.id}",
                            timestamp: true,
                          }),
                        },
                      },
                    },
                  ]),
                },
              },
            },
            {
              type: "member_leave",
              x: 40,
              y: 420,
              inputs: {
                ACTIONS: {
                  block: {
                    type: "send_rich_message",
                    fields: { CONTENT: "", CHANNEL_ID: channelId },
                    inputs: {
                      EMBEDS: {
                        block: richEmbed({
                          title: "{user.name} left the server",
                          description: "See you next time!",
                          color: "#99AAB5",
                          footer: "User ID: {user.id}",
                          timestamp: true,
                        }),
                      },
                    },
                  },
                },
              },
            },
          ],
        },
      };
    },
  },
  {
    id: "levels",
    name: "Levels & XP",
    category: "Engagement",
    summary:
      "Reward community reactions and `/claim-xp` rolls with persistent XP, automatic level-ups, and a `/rank` card.",
    highlights: [
      "Reaction & `/claim-xp` XP",
      "Automatic level-ups at 100 XP",
      "`/rank` embed card",
    ],
    intents: [],
    fields: [
      {
        key: "channelId",
        label: "Level-up announcements channel ID",
        placeholder: "Channel ID for level-up shoutouts",
        hint: "Receives a message whenever a member levels up.",
        kind: "channel",
      },
    ],
    build(values) {
      const channelId = fieldValue(values, "channelId");
      const levelUpCheck: TemplateBlockState = {
        type: "logic_if",
        inputs: {
          CONDITION: {
            block: {
              type: "logic_compare",
              fields: { OP: "greater_or_equal" },
              inputs: {
                LEFT: { block: { type: "variable_get", fields: { SCOPE: "member", NAME: "xp" } } },
                RIGHT: { block: { type: "value_number", fields: { NUM: 100 } } },
              },
            },
          },
          THEN: {
            block: chain([
              {
                type: "variable_set",
                fields: { SCOPE: "member", NAME: "xp" },
                inputs: {
                  VALUE: {
                    block: {
                      type: "value_math",
                      fields: { OP: "subtract" },
                      inputs: {
                        LEFT: {
                          block: { type: "variable_get", fields: { SCOPE: "member", NAME: "xp" } },
                        },
                        RIGHT: { block: { type: "value_number", fields: { NUM: 100 } } },
                      },
                    },
                  },
                },
              },
              {
                type: "variable_change",
                fields: { SCOPE: "member", NAME: "level" },
                inputs: { BY: { block: { type: "value_number", fields: { NUM: 1 } } } },
              },
              {
                type: "send_message",
                fields: {
                  CONTENT: "🎉 GG {user}! You just reached **Level {member:level}**!",
                  CHANNEL_ID: channelId,
                },
              },
            ]),
          },
        },
      };

      return {
        blocks: {
          languageVersion: 0,
          blocks: [
            {
              type: "reaction_add",
              x: 40,
              y: 40,
              fields: { EMOJI: "", MESSAGE_ID: "" },
              inputs: {
                ACTIONS: {
                  block: chain([
                    {
                      type: "variable_change",
                      fields: { SCOPE: "member", NAME: "xp" },
                      inputs: { BY: { block: { type: "value_number", fields: { NUM: 5 } } } },
                    },
                    levelUpCheck,
                  ]),
                },
              },
            },
            {
              type: "slash_command",
              x: 40,
              y: 480,
              fields: { NAME: "claim-xp", DESCRIPTION: "Earn bonus XP points" },
              inputs: {
                ACTIONS: {
                  block: chain([
                    {
                      type: "variable_set",
                      fields: { SCOPE: "local", NAME: "gain" },
                      inputs: {
                        VALUE: {
                          block: {
                            type: "value_random",
                            inputs: {
                              MIN: { block: { type: "value_number", fields: { NUM: 15 } } },
                              MAX: { block: { type: "value_number", fields: { NUM: 30 } } },
                            },
                          },
                        },
                      },
                    },
                    {
                      type: "variable_change",
                      fields: { SCOPE: "member", NAME: "xp" },
                      inputs: {
                        BY: {
                          block: { type: "variable_get", fields: { SCOPE: "local", NAME: "gain" } },
                        },
                      },
                    },
                    {
                      type: "logic_if",
                      inputs: {
                        CONDITION: {
                          block: {
                            type: "logic_compare",
                            fields: { OP: "greater_or_equal" },
                            inputs: {
                              LEFT: {
                                block: {
                                  type: "variable_get",
                                  fields: { SCOPE: "member", NAME: "xp" },
                                },
                              },
                              RIGHT: { block: { type: "value_number", fields: { NUM: 100 } } },
                            },
                          },
                        },
                        THEN: {
                          block: chain([
                            {
                              type: "variable_set",
                              fields: { SCOPE: "member", NAME: "xp" },
                              inputs: {
                                VALUE: {
                                  block: {
                                    type: "value_math",
                                    fields: { OP: "subtract" },
                                    inputs: {
                                      LEFT: {
                                        block: {
                                          type: "variable_get",
                                          fields: { SCOPE: "member", NAME: "xp" },
                                        },
                                      },
                                      RIGHT: {
                                        block: { type: "value_number", fields: { NUM: 100 } },
                                      },
                                    },
                                  },
                                },
                              },
                            },
                            {
                              type: "variable_change",
                              fields: { SCOPE: "member", NAME: "level" },
                              inputs: {
                                BY: { block: { type: "value_number", fields: { NUM: 1 } } },
                              },
                            },
                            {
                              type: "reply_rich",
                              fields: { CONTENT: "", EPHEMERAL: false },
                              inputs: {
                                EMBEDS: {
                                  block: richEmbed({
                                    title: "Level up! 🎉",
                                    description:
                                      "{user} gained **+{var:gain} XP** and reached **Level {member:level}** ({member:xp}/100 XP).",
                                    color: "#57F287",
                                  }),
                                },
                              },
                            },
                          ]),
                        },
                        ELSE: {
                          block: {
                            type: "reply_rich",
                            fields: { CONTENT: "", EPHEMERAL: false },
                            inputs: {
                              EMBEDS: {
                                block: richEmbed({
                                  title: "+{var:gain} XP",
                                  description:
                                    "You are at **Level {member:level}** with **{member:xp}/100 XP**.",
                                  color: "#5865F2",
                                }),
                              },
                            },
                          },
                        },
                      },
                    },
                  ]),
                },
              },
            },
            {
              type: "slash_command",
              x: 40,
              y: 1120,
              fields: { NAME: "rank", DESCRIPTION: "See your current level and XP" },
              inputs: {
                ACTIONS: {
                  block: {
                    type: "reply_rich",
                    fields: { CONTENT: "", EPHEMERAL: false },
                    inputs: {
                      EMBEDS: {
                        block: richEmbed({
                          title: "{user.name}'s rank",
                          description: "Keep reacting and claiming XP to climb higher!",
                          color: "#5865F2",
                          fields: [
                            { name: "Level", value: "{member:level}", inline: true },
                            { name: "XP", value: "{member:xp} / 100", inline: true },
                          ],
                        }),
                      },
                    },
                  },
                },
              },
            },
          ],
        },
      };
    },
  },
  {
    id: "reaction_roles",
    name: "Reaction Roles",
    category: "Roles",
    summary:
      "Post a self-assignable role embed via `/roles-panel` and toggle a role when members add or remove the reaction.",
    highlights: [
      "`/roles-panel` embed post",
      "Grant role on reaction added",
      "Remove role on reaction removed",
    ],
    intents: [],
    fields: [
      {
        key: "roleId",
        label: "Role ID to toggle",
        placeholder: "Role ID given/removed when reacting",
        hint: "Make sure the bot's role sits above this role in Server Settings.",
        kind: "role",
      },
      {
        key: "emoji",
        label: "Reaction emoji",
        placeholder: "🔔",
        defaultValue: "🔔",
        hint: "Unicode emoji (like 🔔) or custom emoji written name:id.",
      },
      {
        key: "messageId",
        label: "Panel message ID (optional)",
        placeholder: "Leave blank to match any message",
        hint: "Paste the message ID after running `/roles-panel` to lock triggers to that message.",
      },
    ],
    build(values) {
      const roleId = fieldValue(values, "roleId");
      const emoji = fieldValue(values, "emoji", "🔔");
      const messageId = fieldValue(values, "messageId");

      return {
        blocks: {
          languageVersion: 0,
          blocks: [
            {
              type: "slash_command",
              x: 40,
              y: 40,
              fields: { NAME: "roles-panel", DESCRIPTION: "Post the reaction roles panel" },
              inputs: {
                ACTIONS: {
                  block: chain([
                    {
                      type: "reply_rich",
                      fields: { CONTENT: "", EPHEMERAL: false },
                      inputs: {
                        EMBEDS: {
                          block: richEmbed({
                            title: "Choose your roles",
                            description: `React with ${emoji} below to receive the role, and remove your reaction to opt out.`,
                            color: "#5865F2",
                          }),
                        },
                      },
                    },
                  ]),
                },
              },
            },
            {
              type: "reaction_add",
              x: 40,
              y: 340,
              fields: { EMOJI: emoji, MESSAGE_ID: messageId },
              inputs: {
                ACTIONS: {
                  block: { type: "add_role", fields: { ROLE_ID: roleId } },
                },
              },
            },
            {
              type: "reaction_remove",
              x: 40,
              y: 540,
              fields: { EMOJI: emoji, MESSAGE_ID: messageId },
              inputs: {
                ACTIONS: {
                  block: { type: "remove_role", fields: { ROLE_ID: roleId } },
                },
              },
            },
          ],
        },
      };
    },
  },
  {
    id: "moderation",
    name: "Moderation",
    category: "Safety",
    summary:
      "Role-gated `/warn`, `/mute`, `/kick` slash commands with audit logs plus a keyword auto-moderator.",
    highlights: [
      "`/warn`, `/mute` (10m) & `/kick`",
      "Rich mod-log channel embed",
      "Auto-delete & DM on filtered words",
    ],
    intents: ["message_content"],
    fields: [
      {
        key: "modRoleId",
        label: "Moderator role ID",
        placeholder: "Only members with this role can run mod commands",
        hint: "Checked with `has role` on every moderation command.",
        kind: "role",
      },
      {
        key: "logChannelId",
        label: "Mod log channel ID",
        placeholder: "Channel ID for moderation audit logs",
        hint: "Receives rich embed logs for every warning, mute, kick, and auto-delete.",
        kind: "channel",
      },
      {
        key: "blockedWord",
        label: "Auto-mod blocked word",
        placeholder: "spamlink",
        defaultValue: "spamlink",
        hint: "Messages containing this word are deleted automatically.",
      },
    ],
    build(values) {
      const modRoleId = fieldValue(values, "modRoleId");
      const logChannelId = fieldValue(values, "logChannelId");
      const blockedWord = fieldValue(values, "blockedWord", "spamlink");

      const denyReply: TemplateBlockState = {
        type: "reply",
        fields: {
          CONTENT: "You need the moderator role to run this command.",
          EPHEMERAL: true,
        },
      };

      const modGuard = (thenBlock: TemplateBlockState): TemplateBlockState => ({
        type: "logic_if",
        inputs: {
          CONDITION: { block: { type: "has_role", fields: { ROLE_ID: modRoleId } } },
          THEN: { block: thenBlock },
          ELSE: { block: denyReply },
        },
      });

      const modOptions = chain([
        slashOption({
          name: "member",
          description: "Target member",
          kind: "user",
          required: true,
        }),
        slashOption({
          name: "reason",
          description: "Reason for action",
          kind: "string",
          required: true,
        }),
      ]);

      const blocks: TemplateBlockState[] = [
        {
          type: "slash_command",
          x: 40,
          y: 40,
          fields: { NAME: "warn", DESCRIPTION: "Warn a member via DM and log it" },
          inputs: {
            OPTIONS: { block: modOptions },
            ACTIONS: {
              block: modGuard(
                chain([
                  {
                    type: "direct_message_member",
                    fields: {
                      MEMBER_ID: "{option.member}",
                      CONTENT: "You received a warning: {option.reason}",
                    },
                  },
                  {
                    type: "send_rich_message",
                    fields: { CONTENT: "", CHANNEL_ID: logChannelId },
                    inputs: {
                      EMBEDS: {
                        block: richEmbed({
                          title: "Member warned",
                          description: "A moderator issued a warning.",
                          color: "#FEE75C",
                          timestamp: true,
                          fields: [
                            { name: "Member", value: "{option.member}", inline: true },
                            { name: "Moderator", value: "{user}", inline: true },
                            { name: "Reason", value: "{option.reason}", inline: false },
                          ],
                        }),
                      },
                    },
                  },
                  {
                    type: "reply",
                    fields: { CONTENT: "⚠️ Warned {option.member}.", EPHEMERAL: true },
                  },
                ]),
              ),
            },
          },
        },
        {
          type: "slash_command",
          x: 40,
          y: 660,
          fields: { NAME: "mute", DESCRIPTION: "Timeout a member for 10 minutes" },
          inputs: {
            OPTIONS: {
              block: chain([
                slashOption({
                  name: "member",
                  description: "Target member",
                  kind: "user",
                  required: true,
                }),
                slashOption({
                  name: "reason",
                  description: "Reason for mute",
                  kind: "string",
                  required: true,
                }),
              ]),
            },
            ACTIONS: {
              block: modGuard(
                chain([
                  {
                    type: "timeout_member",
                    fields: {
                      MEMBER_ID: "{option.member}",
                      MINUTES: 10,
                      REASON: "{option.reason}",
                    },
                  },
                  {
                    type: "send_rich_message",
                    fields: { CONTENT: "", CHANNEL_ID: logChannelId },
                    inputs: {
                      EMBEDS: {
                        block: richEmbed({
                          title: "Member timed out (10 min)",
                          description: "Communication disabled temporarily.",
                          color: "#F97316",
                          timestamp: true,
                          fields: [
                            { name: "Member", value: "{option.member}", inline: true },
                            { name: "Moderator", value: "{user}", inline: true },
                            { name: "Reason", value: "{option.reason}", inline: false },
                          ],
                        }),
                      },
                    },
                  },
                  {
                    type: "reply",
                    fields: {
                      CONTENT: "🔇 Timed out {option.member} for 10 minutes.",
                      EPHEMERAL: true,
                    },
                  },
                ]),
              ),
            },
          },
        },
        {
          type: "slash_command",
          x: 40,
          y: 1280,
          fields: { NAME: "kick", DESCRIPTION: "Kick a member from the server" },
          inputs: {
            OPTIONS: {
              block: chain([
                slashOption({
                  name: "member",
                  description: "Target member",
                  kind: "user",
                  required: true,
                }),
                slashOption({
                  name: "reason",
                  description: "Reason for kick",
                  kind: "string",
                  required: true,
                }),
              ]),
            },
            ACTIONS: {
              block: modGuard(
                chain([
                  {
                    type: "kick_member",
                    fields: {
                      MEMBER_ID: "{option.member}",
                      REASON: "{option.reason}",
                    },
                  },
                  {
                    type: "send_rich_message",
                    fields: { CONTENT: "", CHANNEL_ID: logChannelId },
                    inputs: {
                      EMBEDS: {
                        block: richEmbed({
                          title: "Member kicked",
                          description: "Removed from the server.",
                          color: "#ED4245",
                          timestamp: true,
                          fields: [
                            { name: "Member", value: "{option.member}", inline: true },
                            { name: "Moderator", value: "{user}", inline: true },
                            { name: "Reason", value: "{option.reason}", inline: false },
                          ],
                        }),
                      },
                    },
                  },
                  {
                    type: "reply",
                    fields: { CONTENT: "👢 Kicked {option.member}.", EPHEMERAL: true },
                  },
                ]),
              ),
            },
          },
        },
        {
          type: "message_contains",
          x: 40,
          y: 1900,
          fields: { PATTERN: blockedWord, CASE_SENSITIVE: false },
          inputs: {
            ACTIONS: {
              block: {
                type: "logic_if",
                inputs: {
                  CONDITION: {
                    block: {
                      type: "logic_not",
                      inputs: {
                        VALUE: {
                          block: { type: "has_role", fields: { ROLE_ID: modRoleId } },
                        },
                      },
                    },
                  },
                  THEN: {
                    block: chain([
                      { type: "delete_triggering_message" },
                      {
                        type: "direct_message_member",
                        fields: {
                          MEMBER_ID: "{user.id}",
                          CONTENT: "Your message in {channel} was removed by auto-moderation.",
                        },
                      },
                      {
                        type: "send_rich_message",
                        fields: { CONTENT: "", CHANNEL_ID: logChannelId },
                        inputs: {
                          EMBEDS: {
                            block: richEmbed({
                              title: "Auto-mod deleted message",
                              description: "Filtered phrase detected in {channel}.",
                              color: "#ED4245",
                              timestamp: true,
                              fields: [{ name: "Author", value: "{user}", inline: true }],
                            }),
                          },
                        },
                      },
                    ]),
                  },
                },
              },
            },
          },
        },
      ];

      return {
        blocks: {
          languageVersion: 0,
          blocks,
        },
      };
    },
  },
  {
    id: "tickets",
    name: "Support Tickets",
    category: "Support",
    summary:
      "Post an interactive `/ticket-panel` dropdown, log numbered tickets to a staff queue, and manage Claim / Close buttons.",
    highlights: [
      "Category select menu panel",
      "Numbered staff queue (`{server:ticket_count}`)",
      "Live Claim & Close button updates",
    ],
    intents: [],
    fields: [
      {
        key: "staffChannelId",
        label: "Staff queue channel ID",
        placeholder: "Private staff channel ID where new tickets appear",
        hint: "Receives ticket cards with Claim and Close buttons.",
        kind: "channel",
      },
      {
        key: "staffRoleId",
        label: "Staff role ID",
        placeholder: "Role ID allowed to claim/close tickets",
        hint: "Checked when clicking Claim or Close, and when posting `/ticket-panel`.",
        kind: "role",
      },
    ],
    build(values) {
      const staffChannelId = fieldValue(values, "staffChannelId");
      const staffRoleId = fieldValue(values, "staffRoleId");

      const closeButton: TemplateBlockState = {
        type: "component_button",
        fields: {
          LABEL: "Close",
          STYLE: "danger",
          ID: "ticket_close",
          EMOJI: "🔒",
          DISABLED: false,
        },
      };

      const claimButton: TemplateBlockState = {
        type: "component_button",
        fields: {
          LABEL: "Claim",
          STYLE: "success",
          ID: "ticket_claim",
          EMOJI: "🙋",
          DISABLED: false,
        },
        next: { block: closeButton },
      };

      const staffOnly = (thenBlock: TemplateBlockState, denied: string): TemplateBlockState => ({
        type: "logic_if",
        inputs: {
          CONDITION: { block: { type: "has_role", fields: { ROLE_ID: staffRoleId } } },
          THEN: { block: thenBlock },
          ELSE: {
            block: { type: "reply", fields: { CONTENT: denied, EPHEMERAL: true } },
          },
        },
      });

      return {
        blocks: {
          languageVersion: 0,
          blocks: [
            {
              type: "slash_command",
              x: 40,
              y: 40,
              fields: { NAME: "ticket-panel", DESCRIPTION: "Post the support ticket menu" },
              inputs: {
                ACTIONS: {
                  block: staffOnly(
                    {
                      type: "reply_rich",
                      fields: { CONTENT: "", EPHEMERAL: false },
                      inputs: {
                        EMBEDS: {
                          block: richEmbed({
                            title: "Need help? Open a support ticket",
                            description:
                              "Pick the topic that best matches your issue below. A staff member will follow up with you.",
                            color: "#5865F2",
                          }),
                        },
                        COMPONENTS: {
                          block: {
                            type: "component_select_menu",
                            fields: {
                              ID: "ticket_topic",
                              PLACEHOLDER: "Select a ticket topic…",
                              MIN: 1,
                              MAX: 1,
                              DISABLED: false,
                            },
                            inputs: {
                              OPTIONS: {
                                block: chain([
                                  {
                                    type: "select_option",
                                    fields: {
                                      LABEL: "General help",
                                      VALUE: "general",
                                      DESCRIPTION: "Questions about the server",
                                      EMOJI: "💬",
                                      DEFAULT: false,
                                    },
                                  },
                                  {
                                    type: "select_option",
                                    fields: {
                                      LABEL: "Account & roles",
                                      VALUE: "account",
                                      DESCRIPTION: "Permissions or verification issues",
                                      EMOJI: "🛡️",
                                      DEFAULT: false,
                                    },
                                  },
                                  {
                                    type: "select_option",
                                    fields: {
                                      LABEL: "Report an issue",
                                      VALUE: "report",
                                      DESCRIPTION: "Report a bug or member problem",
                                      EMOJI: "🚨",
                                      DEFAULT: false,
                                    },
                                  },
                                ]),
                              },
                            },
                          },
                        },
                      },
                    },
                    "Only staff members can post the ticket panel.",
                  ),
                },
              },
            },
            {
              type: "select_menu_trigger",
              x: 40,
              y: 620,
              fields: { ID: "ticket_topic" },
              inputs: {
                ACTIONS: {
                  block: chain([
                    {
                      type: "variable_change",
                      fields: { SCOPE: "server", NAME: "ticket_count" },
                      inputs: { BY: { block: { type: "value_number", fields: { NUM: 1 } } } },
                    },
                    {
                      type: "send_rich_message",
                      fields: { CONTENT: "", CHANNEL_ID: staffChannelId },
                      inputs: {
                        EMBEDS: {
                          block: richEmbed({
                            title: "Ticket #{server:ticket_count} — {selected}",
                            description: "Submitted by {user} (`{user.name}`).",
                            color: "#5865F2",
                            footer: "User ID: {user.id}",
                            timestamp: true,
                          }),
                        },
                        COMPONENTS: { block: claimButton },
                      },
                    },
                    {
                      type: "reply",
                      fields: {
                        CONTENT:
                          "🎫 Ticket #{server:ticket_count} ({selected}) opened! Our staff team has been notified.",
                        EPHEMERAL: true,
                      },
                    },
                  ]),
                },
              },
            },
            {
              type: "button_click",
              x: 40,
              y: 1120,
              fields: { ID: "ticket_claim" },
              inputs: {
                ACTIONS: {
                  block: staffOnly(
                    {
                      type: "update_message",
                      fields: { CONTENT: "" },
                      inputs: {
                        EMBEDS: {
                          block: richEmbed({
                            title: "Ticket claimed",
                            description: "Being handled by {user}.",
                            color: "#FEE75C",
                            timestamp: true,
                          }),
                        },
                        COMPONENTS: { block: closeButton },
                      },
                    },
                    "Only staff members can claim tickets.",
                  ),
                },
              },
            },
            {
              type: "button_click",
              x: 40,
              y: 1520,
              fields: { ID: "ticket_close" },
              inputs: {
                ACTIONS: {
                  block: staffOnly(
                    {
                      type: "update_message",
                      fields: { CONTENT: "" },
                      inputs: {
                        EMBEDS: {
                          block: richEmbed({
                            title: "Ticket closed",
                            description: "Marked resolved and closed by {user}.",
                            color: "#57F287",
                            timestamp: true,
                          }),
                        },
                      },
                    },
                    "Only staff members can close tickets.",
                  ),
                },
              },
            },
          ],
        },
      };
    },
  },
];

/** True when the workspace differs from an empty or untouched starter workspace. */
export function hasCustomWorkspaceBlocks(workspace: Blockly.Workspace): boolean {
  const topBlocks = workspace.getTopBlocks(false);
  if (topBlocks.length === 0) {
    return false;
  }
  const current = Blockly.serialization.workspaces.save(workspace) as {
    blocks?: { blocks?: Array<{ id?: string } & Record<string, unknown>> };
  };
  const starter = JSON.stringify(starterWorkspace.blocks.blocks.map(stripBlockPositionAndId));
  const actual = JSON.stringify((current.blocks?.blocks ?? []).map(stripBlockPositionAndId));
  return starter !== actual;
}

function stripBlockPositionAndId(block: Record<string, unknown>): unknown {
  const clone: Record<string, unknown> = {};
  for (const [key, val] of Object.entries(block)) {
    if (key === "id" || key === "x" || key === "y") {
      continue;
    }
    if (Array.isArray(val)) {
      clone[key] = val.map((item) =>
        item && typeof item === "object"
          ? stripBlockPositionAndId(item as Record<string, unknown>)
          : item,
      );
    } else if (val && typeof val === "object") {
      clone[key] = stripBlockPositionAndId(val as Record<string, unknown>);
    } else {
      clone[key] = val;
    }
  }
  return clone;
}

function nextAppendY(workspace: Blockly.Workspace): number {
  const topBlocks = workspace.getTopBlocks(false);
  if (topBlocks.length === 0) {
    return 40;
  }
  let maxBottom = 0;
  for (const block of topBlocks) {
    if ("getBoundingRectangle" in block && typeof block.getBoundingRectangle === "function") {
      const bounds = block.getBoundingRectangle() as { bottom: number };
      maxBottom = Math.max(maxBottom, bounds.bottom);
    } else {
      const pos = block.getRelativeToSurfaceXY();
      maxBottom = Math.max(maxBottom, pos.y + 240);
    }
  }
  return Math.round(maxBottom + 60);
}

/** Insert a ready-made template into the workspace by adding to it or replacing it. */
export function insertTemplate(
  workspace: Blockly.Workspace,
  template: BotTemplate,
  values: Record<string, string>,
  mode: "append" | "replace",
): Blockly.Block | null {
  const state = template.build(values);
  if (mode === "replace") {
    Blockly.serialization.workspaces.load(state, workspace);
    return workspace.getTopBlocks(true)[0] ?? null;
  }

  const baseY = nextAppendY(workspace);
  const templateBlocks = state.blocks.blocks;
  const minY = Math.min(...templateBlocks.map((block) => block.y ?? 40));

  let firstInserted: Blockly.Block | null = null;
  Blockly.Events.setGroup(true);
  try {
    for (const block of templateBlocks) {
      const positioned: Blockly.serialization.blocks.State = {
        ...block,
        x: block.x ?? 40,
        y: (block.y ?? 40) - minY + baseY,
      };
      const created = Blockly.serialization.blocks.append(positioned, workspace, {
        recordUndo: true,
      });
      if (!firstInserted) {
        firstInserted = created;
      }
    }
  } finally {
    Blockly.Events.setGroup(false);
  }
  return firstInserted;
}
