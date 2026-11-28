import * as Blockly from "blockly";
import type { BotResources } from "../types.ts";

export type DiscordResourceKind = "channel" | "role" | "member";

let currentResources: BotResources | null = null;

export function setDiscordPickerResources(resources: BotResources | null): void {
  currentResources = resources;
}

export function getDiscordPickerResources(): BotResources | null {
  return currentResources;
}

const QUICK_TEMPLATES: Record<DiscordResourceKind, Array<{ label: string; value: string }>> = {
  channel: [
    { label: "Current channel", value: "{channel.id}" },
    { label: "Option: channel", value: "{option.channel}" },
    { label: "Selected channel", value: "{selected}" },
  ],
  role: [
    { label: "Option: role", value: "{option.role}" },
    { label: "Selected role", value: "{selected}" },
  ],
  member: [
    { label: "Triggering user", value: "{user.id}" },
    { label: "Right-clicked target", value: "{target.id}" },
    { label: "Option: member", value: "{option.member}" },
    { label: "Selected user", value: "{selected}" },
  ],
};

function channelPrefix(kind: string): string {
  switch (kind) {
    case "voice":
    case "stage":
      return "🔊";
    case "forum":
      return "💬";
    case "announcement":
      return "📢";
    default:
      return "#";
  }
}

function roleColorHex(color: number): string {
  return color > 0 ? `#${color.toString(16).padStart(6, "0")}` : "#99aab5";
}

interface FieldDiscordResourceConfig extends Blockly.FieldTextInputFromJsonConfig {
  resource?: DiscordResourceKind;
}

/**
 * Text input field augmented with a live Discord server/channel/role/member picker
 * dropdown. Always preserves manual snowflake IDs and `{template}` placeholders.
 */
export class FieldDiscordResource extends Blockly.FieldTextInput {
  readonly resourceKind: DiscordResourceKind;
  private pickerListEl: HTMLDivElement | null = null;

  constructor(
    value?: string,
    validator?: Blockly.FieldTextInputValidator,
    config?: FieldDiscordResourceConfig,
  ) {
    super(value, validator, config);
    this.resourceKind = config?.resource ?? "channel";
  }

  static override fromJson(options: FieldDiscordResourceConfig): FieldDiscordResource {
    return new FieldDiscordResource(options.text, undefined, options);
  }

  protected override getText_(): string {
    const raw = String(this.getValue() ?? "");
    const trimmed = raw.trim();
    if (!trimmed || !currentResources) {
      return raw;
    }
    for (const guild of currentResources.guilds) {
      if (this.resourceKind === "channel") {
        const ch = guild.channels.find((item) => item.id === trimmed);
        if (ch) {
          return `${channelPrefix(ch.kind)}${ch.name}`;
        }
      } else if (this.resourceKind === "role") {
        const role = guild.roles.find((item) => item.id === trimmed);
        if (role) {
          return `@${role.name}`;
        }
      } else if (this.resourceKind === "member") {
        const member = guild.members.find((item) => item.id === trimmed);
        if (member) {
          return `@${member.name}`;
        }
      }
    }
    return raw;
  }

  protected override showEditor_(e?: Event, quietInput?: boolean): void {
    super.showEditor_(e, quietInput);
    if (typeof document === "undefined" || !this.htmlInput_) {
      return;
    }

    const content = Blockly.DropDownDiv.getContentDiv();
    const wrapper = document.createElement("div");
    wrapper.className = "disblock-discord-picker";
    wrapper.style.cssText =
      "min-width: 240px; max-width: 320px; max-height: 260px; overflow-y: auto; padding: 6px; font-family: Inter Variable, system-ui, sans-serif; font-size: 12px; color: #171717; background: #ffffff;";

    const templatesRow = document.createElement("div");
    templatesRow.style.cssText =
      "display: flex; flex-wrap: wrap; gap: 4px; padding-bottom: 6px; margin-bottom: 6px; border-bottom: 1px solid #e5e5e5;";
    for (const tpl of QUICK_TEMPLATES[this.resourceKind]) {
      const chip = document.createElement("button");
      chip.type = "button";
      chip.textContent = tpl.value;
      chip.title = tpl.label;
      chip.style.cssText =
        "padding: 2px 6px; border-radius: 4px; border: 1px solid #d4d4d8; background: #f4f4f5; color: #18181b; cursor: pointer; font-size: 11px;";
      chip.addEventListener("pointerdown", (event) => {
        event.preventDefault();
        event.stopPropagation();
        this.applyPickedValue(tpl.value);
      });
      templatesRow.appendChild(chip);
    }
    wrapper.appendChild(templatesRow);

    const list = document.createElement("div");
    this.pickerListEl = list;
    wrapper.appendChild(list);

    content.appendChild(wrapper);
    this.renderPickerOptions("");

    this.htmlInput_.addEventListener("input", () => {
      this.renderPickerOptions(this.htmlInput_?.value ?? "");
    });

    Blockly.DropDownDiv.setColour("#ffffff", "#e5e5e5");
    Blockly.DropDownDiv.showPositionedByField(this, () => {
      this.pickerListEl = null;
    });
  }

  private applyPickedValue(value: string): void {
    this.setValue(value);
    if (this.htmlInput_) {
      this.htmlInput_.value = value;
    }
    Blockly.DropDownDiv.hideIfOwner(this);
    Blockly.WidgetDiv.hide();
  }

  private renderPickerOptions(filter: string): void {
    const list = this.pickerListEl;
    if (!list) {
      return;
    }
    list.innerHTML = "";
    const resources = currentResources;
    if (!resources || resources.guilds.length === 0) {
      const empty = document.createElement("div");
      empty.style.cssText = "padding: 6px 4px; color: #71717a; font-size: 11px;";
      empty.textContent =
        "No Discord servers loaded yet — invite your bot to a server or paste an ID / {template}.";
      list.appendChild(empty);
      return;
    }

    const q = filter.trim().toLowerCase();
    let count = 0;

    for (const guild of resources.guilds) {
      const items: Array<{ id: string; label: string; badge?: string; dotColor?: string }> = [];
      if (this.resourceKind === "channel") {
        for (const ch of guild.channels) {
          if (!q || ch.name.toLowerCase().includes(q) || ch.id.includes(q)) {
            items.push({
              id: ch.id,
              label: `${channelPrefix(ch.kind)} ${ch.name}`,
              badge: ch.kind,
            });
          }
        }
      } else if (this.resourceKind === "role") {
        for (const role of guild.roles) {
          if (!q || role.name.toLowerCase().includes(q) || role.id.includes(q)) {
            items.push({
              id: role.id,
              label: `@${role.name}`,
              dotColor: roleColorHex(role.color),
            });
          }
        }
      } else {
        for (const member of guild.members) {
          if (
            !q ||
            member.name.toLowerCase().includes(q) ||
            member.username.toLowerCase().includes(q) ||
            member.id.includes(q)
          ) {
            items.push({
              id: member.id,
              label: `${member.name} (@${member.username})`,
            });
          }
        }
      }

      if (items.length === 0) {
        continue;
      }

      const header = document.createElement("div");
      header.style.cssText =
        "padding: 4px 4px 2px; font-weight: 600; font-size: 10px; text-transform: uppercase; letter-spacing: 0.04em; color: #71717a;";
      header.textContent = guild.name;
      list.appendChild(header);

      for (const item of items.slice(0, 40)) {
        count += 1;
        const btn = document.createElement("button");
        btn.type = "button";
        btn.style.cssText =
          "display: flex; width: 100%; align-items: center; justify-content: space-between; gap: 8px; padding: 5px 6px; border: none; border-radius: 4px; background: transparent; cursor: pointer; text-align: left; color: #18181b;";
        btn.addEventListener("mouseenter", () => {
          btn.style.background = "#f4f4f5";
        });
        btn.addEventListener("mouseleave", () => {
          btn.style.background = "transparent";
        });
        btn.addEventListener("pointerdown", (event) => {
          event.preventDefault();
          event.stopPropagation();
          this.applyPickedValue(item.id);
        });

        const left = document.createElement("span");
        left.style.cssText = "display: flex; align-items: center; gap: 6px; min-width: 0;";
        if (item.dotColor) {
          const dot = document.createElement("span");
          dot.style.cssText = `width: 8px; height: 8px; border-radius: 9999px; flex-shrink: 0; background: ${item.dotColor};`;
          left.appendChild(dot);
        }
        const text = document.createElement("span");
        text.style.cssText = "overflow: hidden; text-overflow: ellipsis; white-space: nowrap;";
        text.textContent = item.label;
        left.appendChild(text);
        btn.appendChild(left);

        if (item.badge) {
          const badge = document.createElement("span");
          badge.style.cssText = "font-size: 10px; color: #71717a; flex-shrink: 0;";
          badge.textContent = item.badge;
          btn.appendChild(badge);
        }

        list.appendChild(btn);
      }
    }

    if (count === 0) {
      const none = document.createElement("div");
      none.style.cssText = "padding: 6px 4px; color: #71717a; font-size: 11px;";
      none.textContent =
        this.resourceKind === "member" &&
        resources.guilds.every((guild) => guild.members.length === 0)
          ? "Member list needs Server Members Intent, or use {user.id} / {option.member}."
          : "No matching items. You can still type an ID or {template}.";
      list.appendChild(none);
    }
  }
}

let pickerFieldRegistered = false;

export function registerDiscordPickerField(): void {
  if (!pickerFieldRegistered) {
    pickerFieldRegistered = true;
    Blockly.fieldRegistry.register("field_discord_resource", FieldDiscordResource);
  }
}
