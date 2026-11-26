import { useCallback, useEffect, useState } from "react";
import { AlertCircle, ExternalLink, Loader2 } from "lucide-react";
import { Outlet, useNavigate, useParams } from "react-router";
import { ApiError, api, avatarUrl, inviteUrl } from "../../api.ts";
import { StatusIndicator, Switch } from "../../components/ui.tsx";
import { buttonClass } from "../../lib/styles.ts";
import type { Bot, BotDetail, Program } from "../../types.ts";
import { usePolling } from "../../usePolling.ts";
import type { BotOutlet } from "./context.ts";

export function BotLayout() {
  const { id = "" } = useParams();
  const navigate = useNavigate();
  // Keyed by bot id, so switching bots never shows the previous one.
  const [loaded, setLoaded] = useState<BotDetail | null>(null);
  const detail = loaded?.id === id ? loaded : null;
  const [bot, setBot] = useState<Bot | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  useEffect(() => {
    let active = true;
    api
      .bot(id)
      .then((bot) => {
        if (active) {
          setLoaded(bot);
          setBot(bot);
        }
      })
      .catch((failure: unknown) => {
        if (!active) {
          return;
        }
        if (failure instanceof ApiError && failure.status === 404) {
          void navigate("/", { replace: true });
        } else {
          setError(failure instanceof Error ? failure.message : "Could not load the bot");
        }
      });
    return () => {
      active = false;
    };
  }, [id, navigate]);

  usePolling(
    async () => {
      const current = (await api.bots()).find((item) => item.id === id);
      if (current) {
        setBot(current);
      }
    },
    4000,
    id,
  );

  const setEnabled = useCallback(
    async (enabled: boolean) => {
      setBusy(true);
      setError(null);
      setBot(
        (current) => current && { ...current, enabled, status: enabled ? "starting" : "stopped" },
      );
      try {
        await api.setEnabled(id, enabled);
      } catch (failure) {
        setError(failure instanceof Error ? failure.message : "Could not change the bot");
        setBot((current) => current && { ...current, enabled: !enabled });
      } finally {
        setBusy(false);
      }
    },
    [id],
  );

  const onProgramSaved = useCallback((workspace: object, program: Program) => {
    setLoaded((current) => current && { ...current, workspace, program });
  }, []);

  useEffect(() => {
    if (bot) {
      document.title = `${bot.name} — Disblock`;
    }
  }, [bot]);

  if (!detail || !bot) {
    return error ? (
      <p className="text-sm text-destructive">{error}</p>
    ) : (
      <div className="flex justify-center py-12">
        <Loader2 className="size-8 animate-spin text-primary" />
      </div>
    );
  }

  const outlet: BotOutlet = { bot, detail, setEnabled, onProgramSaved };

  return (
    <div className="flex min-h-0 flex-1 flex-col">
      <div className="mb-6 flex flex-wrap items-center justify-between gap-4">
        <div className="flex min-w-0 items-center gap-4">
          <img
            className="size-12 shrink-0 rounded-full bg-muted"
            src={avatarUrl(bot.application_id, bot.avatar)}
            alt=""
          />
          <div className="min-w-0">
            <div className="flex items-center gap-3">
              <h2 className="truncate text-2xl font-bold tracking-tight">{bot.name}</h2>
              <Switch
                checked={bot.enabled}
                onChange={setEnabled}
                disabled={busy}
                label={bot.enabled ? "Stop the bot" : "Start the bot"}
              />
            </div>
            <StatusIndicator status={bot.status} detail={bot.status_detail} />
          </div>
        </div>
        <a
          href={inviteUrl(bot.application_id)}
          target="_blank"
          rel="noreferrer"
          className={buttonClass("outline")}
        >
          <ExternalLink />
          Add to a server
        </a>
      </div>

      {bot.status === "error" && bot.status_detail && (
        <div className="mb-6 flex items-start gap-2 rounded-xl border border-destructive/30 bg-destructive/5 px-4 py-3 text-sm text-destructive">
          <AlertCircle className="mt-0.5 size-4 shrink-0" />
          <p>{bot.status_detail}</p>
        </div>
      )}
      {error && (
        <p className="mb-6 rounded-xl border border-destructive/30 bg-destructive/5 px-4 py-3 text-sm text-destructive">
          {error}
        </p>
      )}

      <Outlet context={outlet} />
    </div>
  );
}
