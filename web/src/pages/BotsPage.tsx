import { useState } from "react";
import { Bot as BotIcon, Loader2, Pencil, Plus, Settings } from "lucide-react";
import { Link } from "react-router";
import { api, avatarUrl } from "../api.ts";
import { AddBotDialog } from "../components/AddBotDialog.tsx";
import { Button, Card, PageHeader, StatusIndicator, Switch } from "../components/ui.tsx";
import { buttonClass } from "../lib/styles.ts";
import type { Bot } from "../types.ts";
import { usePolling } from "../usePolling.ts";

export function BotsPage() {
  const [bots, setBots] = useState<Bot[] | null>(null);
  const [adding, setAdding] = useState(false);
  const [error, setError] = useState<string | null>(null);

  usePolling(async () => setBots(await api.bots()), 5000);

  const setEnabled = async (bot: Bot, enabled: boolean) => {
    setBots(
      (current) =>
        current?.map((item) =>
          item.id === bot.id
            ? { ...item, enabled, status: enabled ? "starting" : "stopped" }
            : item,
        ) ?? null,
    );
    setError(null);
    try {
      await api.setEnabled(bot.id, enabled);
    } catch (failure) {
      setError(failure instanceof Error ? failure.message : "Could not change the bot");
    }
    setBots(await api.bots());
  };

  return (
    <>
      <PageHeader
        title="Your bots"
        description="Manage your Discord bots and their status."
        action={
          bots && bots.length > 0 ? (
            <Button onClick={() => setAdding(true)}>
              <Plus />
              New bot
            </Button>
          ) : undefined
        }
      />

      {error && (
        <p className="mb-6 rounded-lg border border-destructive/30 bg-destructive/5 px-4 py-3 text-sm text-destructive">
          {error}
        </p>
      )}

      {bots === null ? (
        <div className="flex justify-center py-12">
          <Loader2 className="size-8 animate-spin text-primary" />
        </div>
      ) : bots.length === 0 ? (
        <Card className="flex flex-col items-center p-12 text-center">
          <span className="flex size-12 items-center justify-center rounded-full bg-muted">
            <BotIcon className="size-6" />
          </span>
          <h3 className="mt-4 text-lg font-semibold">No bot yet</h3>
          <p className="mt-1 mb-6 text-muted-foreground">
            Connect a Discord bot to start building it with blocks.
          </p>
          <Button onClick={() => setAdding(true)}>
            <Plus />
            Connect your first bot
          </Button>
        </Card>
      ) : (
        <div className="grid grid-cols-1 gap-6 md:grid-cols-2 lg:grid-cols-3">
          {bots.map((bot) => (
            <BotCard key={bot.id} bot={bot} onToggle={(enabled) => setEnabled(bot, enabled)} />
          ))}
        </div>
      )}

      <AddBotDialog open={adding} onClose={() => setAdding(false)} />
    </>
  );
}

function BotCard({ bot, onToggle }: { bot: Bot; onToggle: (enabled: boolean) => void }) {
  return (
    <Card className="overflow-hidden transition-shadow hover:shadow-lg">
      <div className="flex items-start justify-between p-6 pb-3">
        <Link to={`/bots/${bot.id}`} className="flex min-w-0 items-center gap-3">
          <img
            className="size-12 shrink-0 rounded-full bg-muted"
            src={avatarUrl(bot.application_id, bot.avatar)}
            alt=""
          />
          <div className="min-w-0">
            <h3 className="truncate text-lg leading-tight font-semibold">{bot.name}</h3>
            <StatusIndicator status={bot.status} detail={bot.status_detail} />
          </div>
        </Link>
        <Link
          to={`/bots/${bot.id}/settings`}
          className={buttonClass("ghost", "icon", "size-8")}
          aria-label={`${bot.name} settings`}
        >
          <Settings />
        </Link>
      </div>
      <div className="space-y-4 px-6 pb-6">
        <p className="truncate text-sm text-muted-foreground">
          Discord bot • ID: {bot.application_id}
        </p>
        <div className="flex items-center justify-between border-t pt-4">
          <label className="flex items-center gap-2 text-sm font-medium">
            Status
            <Switch checked={bot.enabled} onChange={onToggle} label={`Run ${bot.name}`} />
          </label>
          <Link to={`/bots/${bot.id}/blocks`} className={buttonClass("outline", "sm")}>
            <Pencil />
            Edit blocks
          </Link>
        </div>
      </div>
    </Card>
  );
}
