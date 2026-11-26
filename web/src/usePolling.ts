import { useEffect, useRef } from "react";

/** Call `task` now and then every `intervalMs` while the tab is visible. */
export function usePolling(
  task: () => Promise<void>,
  intervalMs: number,
  key: unknown = null,
): void {
  const latest = useRef(task);
  useEffect(() => {
    latest.current = task;
  });

  useEffect(() => {
    let timer: ReturnType<typeof setTimeout> | undefined;
    let stopped = false;

    const tick = async () => {
      if (document.visibilityState === "visible") {
        try {
          await latest.current();
        } catch {
          // A failed refresh is retried on the next tick.
        }
      }
      if (!stopped) {
        timer = setTimeout(tick, intervalMs);
      }
    };
    void tick();

    return () => {
      stopped = true;
      clearTimeout(timer);
    };
  }, [intervalMs, key]);
}
