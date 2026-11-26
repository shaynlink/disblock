import { useEffect, useState } from "react";
import { useNavigate, useParams } from "react-router";
import { ApiError, api, avatarUrl, inviteUrl } from "../api.ts";
import { Activity } from "../components/Activity.tsx";
import { Editor } from "../components/Editor.tsx";
import { StatusBadge } from "../components/StatusBadge.tsx";
import type { Bot, BotDetail } from "../types.ts";
import { usePolling } from "../usePolling.ts";

type Tab = "editor" | "activity";

export function BotPage() {
  const { id = "" } = useParams();
  const navigate = useNavigate();
  const [detail, setDetail] = useState<BotDetail | null>(null);
  const [status, setStatus] = useState<Bot | null>(null);
  const [tab, setTab] = useState<Tab>("editor");
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  useEffect(() => {
    let active = true;
    api
      .bot(id)
      .then((bot) => {
        if (active) {
          setDetail(bot);
          setStatus(bot);
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
      const bots = await api.bots();
      const current = bots.find((bot) => bot.id === id);
      if (current) {
        setStatus(current);
      }
    },
    4000,
    id,
  );

  if (!detail || !status) {
    return (
      <main className="page">
        {error ? <p className="error">{error}</p> : <p className="muted">Loading…</p>}
      </main>
    );
  }

  const toggle = async () => {
    setBusy(true);
    setError(null);
    try {
      await api.setEnabled(id, !status.enabled);
      setStatus({
        ...status,
        enabled: !status.enabled,
        status: status.enabled ? "stopped" : "starting",
      });
    } catch (failure) {
      setError(failure instanceof Error ? failure.message : "Could not change the bot");
    } finally {
      setBusy(false);
    }
  };

  const remove = async () => {
    if (
      !window.confirm(
        `Remove ${status.name} from Disblock? Its slash commands are deleted from Discord.`,
      )
    ) {
      return;
    }
    setBusy(true);
    try {
      await api.deleteBot(id);
      await navigate("/", { replace: true });
    } catch (failure) {
      setError(failure instanceof Error ? failure.message : "Could not remove the bot");
      setBusy(false);
    }
  };

  return (
    <main className="bot-page">
      <section className="bot-header">
        <img
          className="avatar"
          src={avatarUrl(status.application_id, status.avatar)}
          alt=""
          width={56}
          height={56}
        />
        <div className="bot-header-title">
          <h1>{status.name}</h1>
          <StatusBadge bot={status} />
        </div>
        <div className="bot-header-actions">
          <a
            className="button"
            href={inviteUrl(status.application_id)}
            target="_blank"
            rel="noreferrer"
          >
            Add to a server
          </a>
          <button type="button" className="button" onClick={toggle} disabled={busy}>
            {status.enabled ? "Stop" : "Start"}
          </button>
          <button type="button" className="button danger" onClick={remove} disabled={busy}>
            Remove
          </button>
        </div>
      </section>
      {status.status === "error" && status.status_detail && (
        <p className="error banner">{status.status_detail}</p>
      )}
      {error && <p className="error banner">{error}</p>}
      <nav className="tabs">
        <button
          type="button"
          className={tab === "editor" ? "active" : ""}
          onClick={() => setTab("editor")}
        >
          Blocks
        </button>
        <button
          type="button"
          className={tab === "activity" ? "active" : ""}
          onClick={() => setTab("activity")}
        >
          Activity
        </button>
      </nav>
      <div className={tab === "editor" ? "tab-panel editor-panel" : "hidden"}>
        <Editor bot={detail} />
      </div>
      {tab === "activity" && (
        <div className="tab-panel">
          <Activity botId={id} />
        </div>
      )}
    </main>
  );
}
