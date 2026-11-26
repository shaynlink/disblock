import { type FormEvent, useState } from "react";
import { Link, useNavigate } from "react-router";
import { api, avatarUrl } from "../api.ts";
import { StatusBadge } from "../components/StatusBadge.tsx";
import type { Bot } from "../types.ts";
import { usePolling } from "../usePolling.ts";

export function Dashboard() {
  const [bots, setBots] = useState<Bot[] | null>(null);

  usePolling(async () => setBots(await api.bots()), 5000);

  return (
    <main className="page">
      <h1>Your bots</h1>
      {bots === null ? (
        <p className="muted">Loading…</p>
      ) : (
        <>
          {bots.length > 0 && (
            <ul className="bot-list">
              {bots.map((bot) => (
                <li key={bot.id}>
                  <Link to={`/bots/${bot.id}`} className="bot-card">
                    <img
                      className="avatar"
                      src={avatarUrl(bot.application_id, bot.avatar)}
                      alt=""
                      width={48}
                      height={48}
                    />
                    <span className="bot-card-name">{bot.name}</span>
                    <StatusBadge bot={bot} />
                  </Link>
                </li>
              ))}
            </ul>
          )}
          <AddBot first={bots.length === 0} />
        </>
      )}
    </main>
  );
}

function AddBot({ first }: { first: boolean }) {
  const navigate = useNavigate();
  const [token, setToken] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  const submit = async (event: FormEvent) => {
    event.preventDefault();
    setBusy(true);
    setError(null);
    try {
      const bot = await api.addBot(token);
      await navigate(`/bots/${bot.id}`);
    } catch (failure) {
      setError(failure instanceof Error ? failure.message : "Could not add the bot");
      setBusy(false);
    }
  };

  return (
    <section className="panel">
      <h2>{first ? "Connect your first bot" : "Connect another bot"}</h2>
      <ol className="help">
        <li>
          Open the{" "}
          <a href="https://discord.com/developers/applications" target="_blank" rel="noreferrer">
            Discord developer portal
          </a>{" "}
          and create an application, or pick an existing one.
        </li>
        <li>
          In <strong>Bot</strong>, click <strong>Reset Token</strong> and copy the token.
        </li>
        <li>Paste it below. Disblock will manage this bot's slash commands from now on.</li>
      </ol>
      <form className="inline-form" onSubmit={submit}>
        <input
          type="password"
          autoComplete="off"
          placeholder="Bot token"
          value={token}
          onChange={(event) => setToken(event.target.value)}
          required
        />
        <button type="submit" className="button primary" disabled={busy || token.trim() === ""}>
          {busy ? "Connecting…" : "Connect"}
        </button>
      </form>
      {error && <p className="error">{error}</p>}
    </section>
  );
}
