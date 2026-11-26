import { useState } from "react";
import { Download, Loader2, ShieldAlert, Trash2 } from "lucide-react";
import { ApiError, api } from "../api.ts";
import {
  Button,
  Card,
  CardContent,
  CardHeader,
  Dialog,
  Input,
  PageHeader,
} from "../components/ui.tsx";

export function AccountPage() {
  const [exporting, setExporting] = useState(false);
  const [deleting, setDeleting] = useState(false);
  const [confirmOpen, setConfirmOpen] = useState(false);
  const [confirmation, setConfirmation] = useState("");
  const [error, setError] = useState<string | null>(null);

  const exportData = async () => {
    setExporting(true);
    setError(null);
    try {
      const data = await api.accountExport();
      const url = URL.createObjectURL(data);
      const link = document.createElement("a");
      link.href = url;
      link.download = "disblock-account-export.json";
      link.click();
      URL.revokeObjectURL(url);
    } catch (failure) {
      setError(failure instanceof Error ? failure.message : "Could not export data");
    } finally {
      setExporting(false);
    }
  };

  const deleteAccount = async () => {
    setDeleting(true);
    setError(null);
    try {
      await api.deleteAccount();
      window.location.assign("/");
    } catch (failure) {
      const message = failure instanceof Error ? failure.message : "Could not delete account";
      setError(
        failure instanceof ApiError && failure.status === 503
          ? "Billing must be available to safely close an account with a Stripe subscription."
          : message,
      );
      setDeleting(false);
    }
  };

  return (
    <div className="grid max-w-2xl gap-6">
      <PageHeader
        title="Account & data"
        description="Download your account data or permanently remove your Disblock account."
      />
      {error && (
        <p className="rounded-lg border border-destructive/30 bg-destructive/5 p-3 text-sm text-destructive">
          {error}
        </p>
      )}

      <Card>
        <CardHeader
          title="Export your data"
          description="Download a machine-readable JSON copy of your account, bot configuration, execution history, and saved variables. Bot tokens are never included."
          action={
            <Button variant="outline" onClick={() => void exportData()} disabled={exporting}>
              {exporting ? <Loader2 className="animate-spin" /> : <Download />}
              Download export
            </Button>
          }
        />
      </Card>

      <Card className="border-destructive/30">
        <CardHeader
          title="Delete account"
          description="This permanently removes your Disblock account and all hosted bots, programs, logs, and variables. Active Stripe subscriptions are cancelled before deletion."
          action={
            <Button variant="destructive" onClick={() => setConfirmOpen(true)}>
              <Trash2 />
              Delete account
            </Button>
          }
        />
        <CardContent>
          <p className="flex gap-2 text-sm text-muted-foreground">
            <ShieldAlert className="mt-0.5 size-4 shrink-0 text-destructive" />
            Download an export first. Payment records held by Stripe can remain subject to its legal
            retention duties.
          </p>
        </CardContent>
      </Card>

      <Dialog
        open={confirmOpen}
        onClose={() => !deleting && setConfirmOpen(false)}
        title="Permanently delete your account?"
        description="This cannot be undone. Your bots will stop and their Discord slash commands will be removed."
      >
        <label className="grid gap-2 text-sm font-medium">
          Type <code>DELETE</code> to confirm
          <Input
            value={confirmation}
            onChange={(event) => setConfirmation(event.target.value)}
            autoFocus
          />
        </label>
        <div className="flex justify-end gap-3">
          <Button variant="outline" onClick={() => setConfirmOpen(false)} disabled={deleting}>
            Cancel
          </Button>
          <Button
            variant="destructive"
            onClick={() => void deleteAccount()}
            disabled={confirmation !== "DELETE" || deleting}
          >
            {deleting ? <Loader2 className="animate-spin" /> : <Trash2 />}
            Delete permanently
          </Button>
        </div>
      </Dialog>
    </div>
  );
}
