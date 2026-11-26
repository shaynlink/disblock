import { type KeyboardEvent, useId, useState } from "react";
import { Blocks, Bot, Sparkles } from "lucide-react";
import type { LandingCopy, ShowcaseBlock } from "../data/landingData.ts";
import { cn } from "../lib/styles.ts";

const BLOCK_STYLES: Record<ShowcaseBlock["kind"], { bg: string; badge: string }> = {
  trigger: { bg: "bg-[#4338ca] text-white", badge: "bg-white/20 text-white" },
  option: { bg: "bg-[#be185d] text-white", badge: "bg-white/20 text-white" },
  action: { bg: "bg-[#15803d] text-white", badge: "bg-white/20 text-white" },
  role: { bg: "bg-[#b45309] text-white", badge: "bg-white/20 text-white" },
  logic: { bg: "bg-[#0369a1] text-white", badge: "bg-white/20 text-white" },
  variable: { bg: "bg-[#7e22ce] text-white", badge: "bg-white/20 text-white" },
  flow: { bg: "bg-[#334155] text-white", badge: "bg-white/20 text-white" },
};

const INDENT_CLASSES = ["ml-0", "ml-4 sm:ml-6", "ml-8 sm:ml-12"];

export function LandingShowcase({ copy }: { copy: LandingCopy["showcase"] }) {
  const [selectedIndex, setSelectedIndex] = useState(0);
  const baseId = useId();
  const examples = copy.examples;
  const active = examples[selectedIndex] ?? examples[0];

  const onTabKeyDown = (event: KeyboardEvent<HTMLButtonElement>, index: number) => {
    let next: number | null = null;
    if (event.key === "ArrowRight") {
      next = (index + 1) % examples.length;
    } else if (event.key === "ArrowLeft") {
      next = (index - 1 + examples.length) % examples.length;
    } else if (event.key === "Home") {
      next = 0;
    } else if (event.key === "End") {
      next = examples.length - 1;
    }

    if (next === null) {
      return;
    }
    event.preventDefault();
    setSelectedIndex(next);
    const targetId = `${baseId}-tab-${examples[next]?.id}`;
    document.getElementById(targetId)?.focus();
  };

  if (!active) {
    return null;
  }

  return (
    <div className="rounded-2xl border bg-card p-4 shadow-sm sm:p-6">
      <div className="mb-4 flex flex-wrap items-center justify-between gap-3">
        <span className="inline-flex items-center gap-1.5 text-xs font-semibold tracking-wider text-muted-foreground uppercase">
          <Sparkles className="size-3.5" aria-hidden="true" />
          {copy.eyebrow}
        </span>

        <div
          role="tablist"
          aria-label={copy.tabListLabel}
          className="inline-flex flex-wrap gap-1 rounded-lg bg-muted p-1"
        >
          {examples.map((example, index) => {
            const selected = index === selectedIndex;
            return (
              <button
                key={example.id}
                type="button"
                role="tab"
                id={`${baseId}-tab-${example.id}`}
                aria-selected={selected}
                aria-controls={`${baseId}-panel-${example.id}`}
                tabIndex={selected ? 0 : -1}
                onClick={() => setSelectedIndex(index)}
                onKeyDown={(event) => onTabKeyDown(event, index)}
                className={cn(
                  "cursor-pointer rounded-md px-3 py-1.5 text-xs font-medium transition-colors focus-visible:ring-2 focus-visible:ring-ring focus-visible:outline-none sm:text-sm",
                  selected
                    ? "bg-background text-foreground shadow-xs"
                    : "text-muted-foreground hover:text-foreground",
                )}
              >
                {example.tabLabel}
              </button>
            );
          })}
        </div>
      </div>

      <div
        role="tabpanel"
        id={`${baseId}-panel-${active.id}`}
        aria-labelledby={`${baseId}-tab-${active.id}`}
        tabIndex={0}
        className="space-y-4 focus-visible:ring-2 focus-visible:ring-ring focus-visible:outline-none"
      >
        <p className="text-sm text-muted-foreground">{active.summary}</p>

        <div className="grid gap-4 lg:grid-cols-12">
          <div className="rounded-xl border bg-muted/40 p-4 lg:col-span-7">
            <div className="mb-3 flex items-center gap-2 text-xs font-semibold tracking-wider text-muted-foreground uppercase">
              <Blocks className="size-3.5" aria-hidden="true" />
              <span>{copy.editorTitle}</span>
            </div>
            <ul className="space-y-2">
              {active.blocks.map((block, index) => {
                const style = BLOCK_STYLES[block.kind];
                const indent = INDENT_CLASSES[block.indent ?? 0] ?? INDENT_CLASSES[0];
                return (
                  <li
                    key={`${active.id}-${index}`}
                    className={cn(
                      "flex flex-wrap items-center justify-between gap-2 rounded-lg px-3.5 py-2.5 text-xs font-semibold shadow-xs sm:text-sm",
                      style.bg,
                      indent,
                    )}
                  >
                    <span>{block.label}</span>
                    <span className={cn("rounded px-2 py-0.5 font-mono text-xs", style.badge)}>
                      {block.detail}
                    </span>
                  </li>
                );
              })}
            </ul>
          </div>

          <div className="flex flex-col justify-between rounded-xl border bg-[#1e1f22] p-4 text-zinc-100 lg:col-span-5">
            <div>
              <div className="mb-3 flex items-center justify-between text-xs font-semibold tracking-wider text-zinc-400 uppercase">
                <span>{copy.previewTitle}</span>
                <span className="inline-flex items-center gap-1.5 text-emerald-400">
                  <span className="size-2 rounded-full bg-emerald-400" aria-hidden="true" />
                  Online
                </span>
              </div>

              <div className="space-y-3 text-sm">
                <div className="rounded-lg bg-white/5 px-3 py-2">
                  <div className="text-xs font-semibold text-zinc-300">{active.discordUser}</div>
                  <div className="mt-0.5 font-mono text-xs text-zinc-400">
                    {active.discordCommandOrEvent}
                  </div>
                </div>

                <div className="flex items-start gap-2.5">
                  <span
                    className="mt-0.5 flex size-7 shrink-0 items-center justify-center rounded-full bg-[#5865f2] text-white"
                    aria-hidden="true"
                  >
                    <Bot className="size-4" />
                  </span>
                  <div className="min-w-0">
                    <div className="flex items-center gap-1.5">
                      <span className="text-sm font-semibold text-white">
                        {active.discordBotName}
                      </span>
                      <span className="rounded bg-[#5865f2] px-1.5 py-0.5 text-[10px] font-bold tracking-wide text-white uppercase">
                        APP
                      </span>
                    </div>
                    <p className="mt-1 text-sm leading-snug text-zinc-200">{active.discordReply}</p>
                  </div>
                </div>
              </div>
            </div>

            <p className="mt-4 border-t border-white/10 pt-3 text-xs text-zinc-400">
              {active.discordNote}
            </p>
          </div>
        </div>
      </div>
    </div>
  );
}
