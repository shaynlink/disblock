import { type FormEvent, useState } from "react";
import { ExternalLink, Loader2 } from "lucide-react";
import { useNavigate } from "react-router";
import { api } from "../api.ts";
import { Button, Dialog, Input } from "./ui.tsx";

export function AddBotDialog({ open, onClose }: { open: boolean; onClose: () => void }) {
  const navigate = useNavigate();
  const [token, setToken] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  const close = () => {
    setToken("");
    setError(null);
    onClose();
  };

  const submit = async (event: FormEvent) => {
    event.preventDefault();
    setBusy(true);
    setError(null);
    try {
      const bot = await api.addBot(token);
      close();
      await navigate(`/bots/${bot.id}/blocks`);
    } catch (failure) {
      setError(failure instanceof Error ? failure.message : "Could not add the bot");
    } finally {
      setBusy(false);
    }
  };

  return (
    <Dialog
      open={open}
      onClose={close}
      title="Connect a bot"
      description="Disblock hosts the bot and manages its slash commands from now on."
    >
      <ol className="list-decimal space-y-1.5 pl-5 text-sm text-muted-foreground">
        <li>
          Open the{" "}
          <a
            href="https://discord.com/developers/applications"
            target="_blank"
            rel="noreferrer"
            className="inline-flex items-center gap-1 font-medium text-foreground underline-offset-4 hover:underline"
          >
            Discord developer portal
            <ExternalLink className="size-3" />
          </a>{" "}
          and create an application, or pick one.
        </li>
        <li>
          In <span className="font-medium text-foreground">Bot</span>, click{" "}
          <span className="font-medium text-foreground">Reset Token</span> and copy it.
        </li>
        <li>Paste the token below.</li>
      </ol>
      <form className="grid gap-3" onSubmit={submit}>
        <label className="grid gap-2 text-sm font-medium">
          Bot token
          <Input
            type="password"
            autoComplete="off"
            placeholder="MTEw…"
            value={token}
            onChange={(event) => setToken(event.target.value)}
            required
          />
        </label>
        {error && <p className="text-sm text-destructive">{error}</p>}
        <div className="flex justify-end gap-2">
          <Button variant="outline" onClick={close}>
            Cancel
          </Button>
          <Button type="submit" disabled={busy || token.trim() === ""}>
            {busy && <Loader2 className="animate-spin" />}
            {busy ? "Connecting…" : "Connect"}
          </Button>
        </div>
      </form>
    </Dialog>
  );
}
