import { useId, useState } from "react";
import { AlertTriangle, Check, Layers, Plus, Replace } from "lucide-react";
import { BOT_TEMPLATES, type BotTemplate, type TemplateId } from "../blocks/templates.ts";
import type { BotIntents, BotResources } from "../types.ts";
import { Badge, Button, Dialog, Input } from "./ui.tsx";

const INTENT_LABELS: Record<"members" | "message_content", string> = {
  members: "Server Members Intent",
  message_content: "Message Content Intent",
};

function initialValues(template: BotTemplate): Record<string, string> {
  const next: Record<string, string> = {};
  for (const field of template.fields) {
    next[field.key] = field.defaultValue ?? "";
  }
  return next;
}

export function TemplateDialog({
  open,
  onClose,
  onApply,
  hasCustomBlocks,
  intents,
  resources,
}: {
  open: boolean;
  onClose: () => void;
  onApply: (
    template: BotTemplate,
    values: Record<string, string>,
    mode: "append" | "replace",
  ) => void;
  hasCustomBlocks: boolean;
  intents: BotIntents | null;
  resources: BotResources | null;
}) {
  const [selectedId, setSelectedId] = useState<TemplateId>("welcome");
  const [valuesByTemplate, setValuesByTemplate] = useState<
    Record<TemplateId, Record<string, string>>
  >(() => ({
    welcome: initialValues(BOT_TEMPLATES[0]),
    levels: initialValues(BOT_TEMPLATES[1]),
    reaction_roles: initialValues(BOT_TEMPLATES[2]),
    moderation: initialValues(BOT_TEMPLATES[3]),
    tickets: initialValues(BOT_TEMPLATES[4]),
  }));
  const [confirmingReplace, setConfirmingReplace] = useState(false);
  const idPrefix = useId();
  const replaceWarningId = useId();

  const close = () => {
    setConfirmingReplace(false);
    onClose();
  };

  const selected = BOT_TEMPLATES.find((template) => template.id === selectedId) ?? BOT_TEMPLATES[0];
  const values = valuesByTemplate[selected.id];

  const updateField = (key: string, value: string) => {
    setValuesByTemplate((current) => ({
      ...current,
      [selected.id]: {
        ...current[selected.id],
        [key]: value,
      },
    }));
  };

  const missingIntents = intents ? selected.intents.filter((intent) => !intents[intent]) : [];

  const apply = (mode: "append" | "replace") => {
    onApply(selected, values, mode);
    close();
  };

  return (
    <Dialog
      open={open}
      onClose={close}
      size="xl"
      title="Ready-made templates"
      description="Start with a complete bot setup or append blocks below your existing triggers."
    >
      <div className="grid gap-6 md:grid-cols-[240px_1fr]">
        <ul className="space-y-1.5" aria-label="Available templates">
          {BOT_TEMPLATES.map((template) => {
            const active = template.id === selected.id;
            return (
              <li key={template.id}>
                <button
                  type="button"
                  aria-pressed={active}
                  onClick={() => {
                    setSelectedId(template.id);
                    setConfirmingReplace(false);
                  }}
                  className={`flex w-full cursor-pointer flex-col items-start gap-1 rounded-lg border px-3 py-2.5 text-left transition-colors focus-visible:ring-[3px] focus-visible:ring-ring/50 focus-visible:outline-none ${
                    active
                      ? "border-primary bg-primary/5 text-foreground"
                      : "border-transparent hover:bg-muted/70"
                  }`}
                >
                  <div className="flex w-full items-center justify-between gap-2">
                    <span className="text-sm font-semibold">{template.name}</span>
                    <Badge className="bg-secondary text-secondary-foreground">
                      {template.category}
                    </Badge>
                  </div>
                  {template.intents.length > 0 && (
                    <span className="text-xs text-muted-foreground">
                      Needs {template.intents.map((intent) => INTENT_LABELS[intent]).join(", ")}
                    </span>
                  )}
                </button>
              </li>
            );
          })}
        </ul>

        <div className="flex flex-col justify-between gap-5 rounded-xl border bg-muted/20 p-5">
          <div className="space-y-4">
            <div>
              <div className="flex flex-wrap items-center gap-2">
                <h2 className="text-lg font-semibold">{selected.name}</h2>
                <Badge className="bg-secondary text-secondary-foreground">
                  {selected.category}
                </Badge>
              </div>
              <p className="mt-1.5 text-sm text-muted-foreground">{selected.summary}</p>
            </div>

            <ul className="space-y-1.5 text-sm">
              {selected.highlights.map((item) => (
                <li key={item} className="flex items-center gap-2">
                  <Check className="size-4 shrink-0 text-green-600" aria-hidden="true" />
                  <span>{item}</span>
                </li>
              ))}
            </ul>

            {selected.intents.length > 0 && (
              <div
                className={`flex items-start gap-2 rounded-lg border px-3 py-2.5 text-xs ${
                  missingIntents.length > 0
                    ? "border-amber-500/40 bg-amber-500/10 text-amber-950"
                    : "border-border bg-background text-muted-foreground"
                }`}
              >
                <AlertTriangle
                  className={`mt-0.5 size-4 shrink-0 ${
                    missingIntents.length > 0 ? "text-amber-600" : "text-muted-foreground"
                  }`}
                  aria-hidden="true"
                />
                <p>
                  {missingIntents.length > 0 ? (
                    <>
                      <span className="font-medium">Disabled on Discord:</span> enable{" "}
                      {missingIntents.map((intent) => INTENT_LABELS[intent]).join(" and ")} in the
                      Discord developer portal before saving.
                    </>
                  ) : (
                    <>
                      Requires{" "}
                      {selected.intents.map((intent) => INTENT_LABELS[intent]).join(" and ")} in
                      Discord.
                    </>
                  )}
                </p>
              </div>
            )}

            <div className="space-y-3 border-t pt-4">
              <div>
                <h3 className="text-sm font-medium">Quick setup (optional)</h3>
                <p className="text-xs text-muted-foreground">
                  Pick from your bot&apos;s Discord servers or paste IDs now — you can edit them on
                  the blocks anytime.
                </p>
              </div>
              <div className="grid gap-3 sm:grid-cols-2">
                {selected.fields.map((field) => {
                  const inputId = `${idPrefix}-${selected.id}-${field.key}`;
                  const hintId = `${inputId}-hint`;
                  const hasPicker =
                    field.kind &&
                    resources &&
                    resources.guilds.some((guild) =>
                      field.kind === "channel" ? guild.channels.length > 0 : guild.roles.length > 0,
                    );
                  return (
                    <div key={field.key} className="grid gap-1">
                      <label htmlFor={inputId} className="text-xs font-medium text-foreground">
                        {field.label}
                      </label>
                      {hasPicker && (
                        <select
                          aria-label={`Pick ${field.label}`}
                          value={values[field.key] ?? ""}
                          onChange={(event) => {
                            if (event.target.value) {
                              updateField(field.key, event.target.value);
                            }
                          }}
                          className="h-9 w-full rounded-md border border-input bg-background px-2.5 text-sm shadow-xs focus-visible:border-ring focus-visible:ring-[3px] focus-visible:ring-ring/50 focus-visible:outline-none"
                        >
                          <option value="">
                            Choose a {field.kind === "channel" ? "channel" : "role"}…
                          </option>
                          {resources.guilds.map((guild) => {
                            const items = field.kind === "channel" ? guild.channels : guild.roles;
                            if (items.length === 0) {
                              return null;
                            }
                            return (
                              <optgroup key={guild.id} label={guild.name}>
                                {items.map((item) => (
                                  <option key={item.id} value={item.id}>
                                    {field.kind === "channel" ? `#${item.name}` : `@${item.name}`}
                                  </option>
                                ))}
                              </optgroup>
                            );
                          })}
                        </select>
                      )}
                      <Input
                        id={inputId}
                        value={values[field.key] ?? ""}
                        onChange={(event) => updateField(field.key, event.target.value)}
                        placeholder={field.placeholder}
                        aria-describedby={hintId}
                      />
                      <p id={hintId} className="text-xs text-muted-foreground">
                        {field.hint}
                      </p>
                    </div>
                  );
                })}
              </div>
            </div>
          </div>

          <div className="space-y-2 border-t pt-4">
            <p
              id={replaceWarningId}
              className={
                confirmingReplace
                  ? "text-xs font-medium text-destructive"
                  : "invisible text-xs font-medium text-destructive"
              }
              aria-hidden={!confirmingReplace}
            >
              This will replace your current blocks in the editor. Click Confirm replace or Cancel.
            </p>

            <div className="flex flex-wrap items-center justify-end gap-2">
              {confirmingReplace ? (
                <>
                  <Button
                    variant="destructive"
                    onClick={() => apply("replace")}
                    aria-describedby={replaceWarningId}
                  >
                    <Replace />
                    Confirm replace
                  </Button>
                  <Button variant="outline" onClick={() => setConfirmingReplace(false)} autoFocus>
                    Cancel
                  </Button>
                </>
              ) : hasCustomBlocks ? (
                <>
                  <Button variant="outline" onClick={() => setConfirmingReplace(true)}>
                    <Replace />
                    Replace current blocks
                  </Button>
                  <Button onClick={() => apply("append")}>
                    <Plus />
                    Add to workspace
                  </Button>
                </>
              ) : (
                <>
                  <Button variant="outline" onClick={() => apply("append")}>
                    <Plus />
                    Add below starter
                  </Button>
                  <Button onClick={() => apply("replace")}>
                    <Layers />
                    Use template
                  </Button>
                </>
              )}
            </div>
          </div>
        </div>
      </div>
    </Dialog>
  );
}
