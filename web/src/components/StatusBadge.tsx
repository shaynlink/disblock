import type { Bot } from "../types.ts";

const LABELS: Record<Bot["status"], string> = {
  online: "Online",
  starting: "Starting",
  stopped: "Stopped",
  error: "Error",
};

export function StatusBadge({ bot }: { bot: Pick<Bot, "status" | "status_detail"> }) {
  return (
    <span className={`badge badge-${bot.status}`} title={bot.status_detail ?? undefined}>
      <span className="badge-dot" />
      {LABELS[bot.status]}
    </span>
  );
}
