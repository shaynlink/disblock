import { useState } from "react";
import { api } from "../api.ts";
import type { ExecutionEntry } from "../types.ts";
import { usePolling } from "../usePolling.ts";

const timeFormat = new Intl.DateTimeFormat(undefined, { dateStyle: "short", timeStyle: "medium" });

export function Activity({ botId }: { botId: string }) {
  const [entries, setEntries] = useState<ExecutionEntry[] | null>(null);

  usePolling(async () => setEntries(await api.executions(botId)), 5000, botId);

  if (entries === null) {
    return <p className="muted">Loading…</p>;
  }
  if (entries.length === 0) {
    return (
      <p className="muted">
        Nothing ran yet. Use one of your bot's triggers in Discord and it shows up here.
      </p>
    );
  }

  return (
    <table className="activity">
      <thead>
        <tr>
          <th>When</th>
          <th>Trigger</th>
          <th>Result</th>
          <th className="numeric">Duration</th>
        </tr>
      </thead>
      <tbody>
        {entries.map((entry) => (
          <tr key={entry.id}>
            <td>{timeFormat.format(new Date(entry.created_at))}</td>
            <td>
              <code>{entry.trigger}</code>
            </td>
            <td className={entry.success ? "success" : "error"}>
              {entry.success ? "Success" : entry.error}
            </td>
            <td className="numeric">{entry.duration_ms} ms</td>
          </tr>
        ))}
      </tbody>
    </table>
  );
}
