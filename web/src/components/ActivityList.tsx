import { cn } from "../lib/styles.ts";
import type { ExecutionEntry } from "../types.ts";

const timeFormat = new Intl.DateTimeFormat(undefined, { dateStyle: "short", timeStyle: "medium" });

export function ActivityList({ entries }: { entries: ExecutionEntry[] }) {
  if (entries.length === 0) {
    return (
      <p className="rounded-lg border border-dashed p-6 text-center text-sm text-muted-foreground">
        Nothing ran yet. Use one of your bot's triggers in Discord and it shows up here.
      </p>
    );
  }

  return (
    <ul className="space-y-2">
      {entries.map((entry) => (
        <li key={entry.id} className="flex items-start gap-3 rounded-lg border p-3">
          <span
            className={cn(
              "mt-1.5 size-2 shrink-0 rounded-full",
              entry.success ? "bg-green-500" : "bg-red-500",
            )}
          />
          <div className="min-w-0 flex-1">
            <p className="flex flex-wrap items-baseline gap-x-2 text-sm">
              <span className="font-medium">{entry.success ? "Success" : "Failed"}</span>
              <code className="text-xs text-muted-foreground">{entry.trigger}</code>
            </p>
            {entry.error && <p className="mt-0.5 text-sm text-destructive">{entry.error}</p>}
            <p className="mt-0.5 text-xs text-muted-foreground">
              {timeFormat.format(new Date(entry.created_at))}
            </p>
          </div>
          <span className="text-sm font-semibold whitespace-nowrap">{entry.duration_ms}ms</span>
        </li>
      ))}
    </ul>
  );
}
