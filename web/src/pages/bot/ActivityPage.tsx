import { useState } from "react";
import { Loader2, RefreshCw } from "lucide-react";
import { ActivityList } from "../../components/ActivityList.tsx";
import { useExecutions } from "../../useExecutions.ts";
import { Button, Card, CardContent, CardHeader } from "../../components/ui.tsx";
import { useBot } from "./context.ts";

export function ActivityPage() {
  const { bot } = useBot();
  const { entries, refresh } = useExecutions(bot.id);
  const [refreshing, setRefreshing] = useState(false);

  const reload = async () => {
    setRefreshing(true);
    try {
      await refresh();
    } finally {
      setRefreshing(false);
    }
  };

  return (
    <Card>
      <CardHeader
        title="Activity"
        description={
          entries ? `Last ${entries.length} runs of your triggers, newest first.` : "Loading…"
        }
        action={
          <Button variant="outline" onClick={reload} disabled={refreshing}>
            <RefreshCw className={refreshing ? "animate-spin" : undefined} />
            Refresh
          </Button>
        }
      />
      <CardContent>
        {entries ? (
          <ActivityList entries={entries} />
        ) : (
          <div className="flex justify-center py-8">
            <Loader2 className="size-6 animate-spin text-primary" />
          </div>
        )}
      </CardContent>
    </Card>
  );
}
