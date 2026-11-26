import { useCallback, useState } from "react";
import { api } from "./api.ts";
import type { ExecutionEntry } from "./types.ts";
import { usePolling } from "./usePolling.ts";

/** A bot's execution history, refreshed every few seconds. */
export function useExecutions(botId: string) {
  const [entries, setEntries] = useState<ExecutionEntry[] | null>(null);
  const refresh = useCallback(async () => setEntries(await api.executions(botId)), [botId]);
  usePolling(refresh, 5000, botId);
  return { entries, refresh };
}
