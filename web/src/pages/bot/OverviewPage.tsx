import type { ReactNode } from "react";
import { ArrowRight, Blocks, Loader2 } from "lucide-react";
import { Link } from "react-router";
import { ActivityList } from "../../components/ActivityList.tsx";
import { useExecutions } from "../../useExecutions.ts";
import { Badge, Card, CardContent, CardHeader } from "../../components/ui.tsx";
import { buttonClass } from "../../lib/styles.ts";
import type { BotStatus, Trigger } from "../../types.ts";
import { useBot } from "./context.ts";

const dateFormat = new Intl.DateTimeFormat(undefined, { dateStyle: "medium" });

const STATUS_TEXT: Record<BotStatus, string> = {
  online: "Connected to Discord",
  starting: "Connecting to Discord…",
  stopped: "Stopped, it does not answer",
  error: "Stopped because of an error",
};

function Field({ label, children }: { label: string; children: ReactNode }) {
  return (
    <div>
      <p className="text-sm text-muted-foreground">{label}</p>
      <div className="mt-0.5 text-sm font-medium">{children}</div>
    </div>
  );
}

function count(triggers: Trigger[], type: Trigger["type"]): number {
  return triggers.filter((trigger) => trigger.type === type).length;
}

export function OverviewPage() {
  const { bot, detail } = useBot();
  const { entries } = useExecutions(bot.id);
  const triggers = detail.program.handlers.map((handler) => handler.trigger);

  return (
    <div className="grid gap-6 md:grid-cols-2">
      <Card>
        <CardHeader title="Information" />
        <CardContent className="space-y-4">
          <Field label="Application ID">
            <code>{bot.application_id}</code>
          </Field>
          <Field label="Connected on">{dateFormat.format(new Date(bot.created_at))}</Field>
          <Field label="Hosting">{STATUS_TEXT[bot.status]}</Field>
        </CardContent>
      </Card>

      <Card>
        <CardHeader
          title="Program"
          description="What your bot reacts to."
          action={
            <Link to="blocks" className={buttonClass("default", "sm")}>
              <Blocks />
              Open the editor
            </Link>
          }
        />
        <CardContent className="grid grid-cols-3 gap-4">
          <Field label="Slash commands">
            <span className="text-2xl font-bold">{count(triggers, "slash_command")}</span>
          </Field>
          <Field label="Member joins">
            <span className="text-2xl font-bold">{count(triggers, "member_join")}</span>
          </Field>
          <Field label="Message triggers">
            <span className="text-2xl font-bold">{count(triggers, "message_contains")}</span>
          </Field>
          <div className="col-span-3 flex flex-wrap gap-2">
            {triggers
              .filter((trigger) => trigger.type === "slash_command")
              .map((trigger) => (
                <Badge key={trigger.name} className="bg-secondary text-secondary-foreground">
                  /{trigger.name}
                </Badge>
              ))}
          </div>
        </CardContent>
      </Card>

      <Card className="md:col-span-2">
        <CardHeader
          title="Recent activity"
          description="The last 5 runs of your triggers."
          action={
            <Link to="activity" className={buttonClass("outline", "sm")}>
              View all
              <ArrowRight />
            </Link>
          }
        />
        <CardContent>
          {entries ? (
            <ActivityList entries={entries.slice(0, 5)} />
          ) : (
            <div className="flex justify-center py-8">
              <Loader2 className="size-6 animate-spin text-primary" />
            </div>
          )}
        </CardContent>
      </Card>
    </div>
  );
}
