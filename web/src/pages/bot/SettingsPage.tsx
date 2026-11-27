import { useState } from "react";
import { ExternalLink, Loader2, Trash2 } from "lucide-react";
import { useNavigate } from "react-router";
import { api, inviteUrl } from "../../api.ts";
import { Button, Card, CardContent, CardHeader, Switch } from "../../components/ui.tsx";
import { buttonClass } from "../../lib/styles.ts";
import { useBot } from "./context.ts";

export function SettingsPage() {
  const { bot, setEnabled } = useBot();
  const navigate = useNavigate();
  const [removing, setRemoving] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const remove = async () => {
    if (
      !window.confirm(
        `Remove ${bot.name} from Disblock? Its slash commands are deleted from Discord.`,
      )
    ) {
      return;
    }
    setRemoving(true);
    setError(null);
    try {
      await api.deleteBot(bot.id);
      await navigate("/", { replace: true });
    } catch (failure) {
      setError(failure instanceof Error ? failure.message : "Could not remove the bot");
      setRemoving(false);
    }
  };

  return (
    <div className="grid gap-6">
      <Card>
        <CardHeader
          title="Hosting"
          description="When it is off, the bot disconnects from Discord and stops answering."
          action={
            <Switch
              checked={bot.enabled}
              onChange={setEnabled}
              label={bot.enabled ? "Stop the bot" : "Start the bot"}
            />
          }
        />
      </Card>

      <Card>
        <CardHeader
          title="Servers"
          description="Invite the bot with the permissions its blocks need: send messages, react and manage roles."
          action={
            <a
              href={inviteUrl(bot.application_id)}
              target="_blank"
              rel="noreferrer"
              className={buttonClass("outline")}
            >
              <ExternalLink />
              Add to a server
            </a>
          }
        />
      </Card>

      <Card>
        <CardHeader
          title="Privileged intents"
          description="Some triggers need Discord's permission to receive their events."
        />
        <CardContent>
          <ul className="list-disc space-y-1.5 pl-5 text-sm text-muted-foreground">
            <li>
              <span className="font-medium text-foreground">When a member joins / leaves</span>{" "}
              needs the <span className="font-medium text-foreground">Server Members Intent</span>.
            </li>
            <li>
              <span className="font-medium text-foreground">When a message contains</span> needs the{" "}
              <span className="font-medium text-foreground">Message Content Intent</span>.
            </li>
          </ul>
          <a
            href={`https://discord.com/developers/applications/${bot.application_id}/bot`}
            target="_blank"
            rel="noreferrer"
            className={buttonClass("outline", "sm", "mt-4")}
          >
            <ExternalLink />
            Open the bot's intents
          </a>
        </CardContent>
      </Card>

      <Card className="border-destructive/30">
        <CardHeader
          title="Remove the bot"
          description="Disblock stops hosting it and deletes its slash commands from Discord. The bot itself stays in the developer portal."
          action={
            <Button variant="destructive" onClick={remove} disabled={removing}>
              {removing ? <Loader2 className="animate-spin" /> : <Trash2 />}
              Remove
            </Button>
          }
        />
        {error && (
          <CardContent>
            <p className="text-sm text-destructive">{error}</p>
          </CardContent>
        )}
      </Card>
    </div>
  );
}
