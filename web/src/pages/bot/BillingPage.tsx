import { useEffect, useState } from "react";
import { CreditCard, ExternalLink, Loader2, Sparkles } from "lucide-react";
import { api, type BillingInterval, type BillingStatus } from "../../api.ts";
import { cn } from "../../lib/styles.ts";
import { Button, Card, CardContent, CardHeader, Badge } from "../../components/ui.tsx";
import { useBot } from "./context.ts";

/** Shown before Checkout, which displays the authoritative price and taxes. */
const PLANS: { interval: BillingInterval; price: string; period: string; note?: string }[] = [
  { interval: "month", price: "€6", period: "per month" },
  { interval: "year", price: "€50", period: "per year", note: "2 months free" },
];

export function BillingPage() {
  const { bot } = useBot();
  const [billing, setBilling] = useState<BillingStatus | null>(null);
  const [busy, setBusy] = useState<"checkout" | "portal" | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [billingInterval, setBillingInterval] = useState<BillingInterval>("year");

  useEffect(() => {
    let active = true;
    api
      .billingStatus(bot.id)
      .then((status) => active && setBilling(status))
      .catch((failure: unknown) => {
        if (active) {
          setError(failure instanceof Error ? failure.message : "Could not load billing");
        }
      });
    return () => {
      active = false;
    };
  }, [bot.id]);

  const redirect = async (kind: "checkout" | "portal") => {
    setBusy(kind);
    setError(null);
    try {
      const result =
        kind === "checkout"
          ? await api.checkout(bot.id, billingInterval)
          : await api.billingPortal();
      window.location.assign(result.url);
    } catch (failure) {
      setError(failure instanceof Error ? failure.message : "Could not open billing");
      setBusy(null);
    }
  };

  if (!billing && !error) {
    return (
      <div className="flex justify-center py-12">
        <Loader2 className="size-8 animate-spin text-primary" />
      </div>
    );
  }

  const pro = billing?.plan === "pro";
  const cancelled = billing?.cancel_at_period_end;
  return (
    <div className="grid max-w-2xl gap-6">
      <Card className={pro ? "border-primary/50" : undefined}>
        <CardHeader
          title={
            <span className="flex items-center gap-2">
              {pro ? (
                <Sparkles className="size-5 text-amber-500" />
              ) : (
                <CreditCard className="size-5" />
              )}
              {pro ? "Pro" : "Free"}
              <Badge className={pro ? "bg-primary" : "bg-secondary text-secondary-foreground"}>
                {pro ? "Active" : "Current plan"}
              </Badge>
            </span>
          }
          description={
            pro
              ? "This bot can use scheduled tasks and other Pro capabilities."
              : "Upgrade this bot to unlock scheduled tasks. The price and renewal terms are shown securely by Stripe."
          }
        />
        <CardContent className="space-y-4">
          {cancelled && (
            <p className="rounded-lg border border-amber-500/30 bg-amber-500/10 px-3 py-2 text-sm">
              Pro is set to cancel at the end of the current billing period. You can resume it in
              the billing portal.
            </p>
          )}
          {billing?.status === "past_due" && (
            <p className="rounded-lg border border-destructive/30 bg-destructive/5 px-3 py-2 text-sm text-destructive">
              A payment is required before Pro capabilities can be used again.
            </p>
          )}
          {!pro && (
            <div
              role="radiogroup"
              aria-label="Billing period"
              className="grid gap-3 sm:grid-cols-2"
            >
              {PLANS.map((plan) => (
                <button
                  key={plan.interval}
                  type="button"
                  role="radio"
                  aria-checked={billingInterval === plan.interval}
                  onClick={() => setBillingInterval(plan.interval)}
                  className={cn(
                    "cursor-pointer rounded-lg border p-4 text-left transition-colors",
                    billingInterval === plan.interval
                      ? "border-primary ring-2 ring-primary/20"
                      : "hover:bg-accent",
                  )}
                >
                  <span className="flex items-baseline gap-1.5">
                    <span className="text-2xl font-bold">{plan.price}</span>
                    <span className="text-sm text-muted-foreground">{plan.period}</span>
                  </span>
                  <span className="mt-1 block text-xs text-muted-foreground">
                    {plan.note ?? "Cancel anytime"}
                  </span>
                </button>
              ))}
            </div>
          )}
          {error && <p className="text-sm text-destructive">{error}</p>}
          <div className="flex flex-wrap gap-3">
            {!pro && (
              <Button onClick={() => void redirect("checkout")} disabled={busy !== null}>
                {busy === "checkout" ? <Loader2 className="animate-spin" /> : <Sparkles />}
                Upgrade this bot
              </Button>
            )}
            {billing?.status && (
              <Button
                variant="outline"
                onClick={() => void redirect("portal")}
                disabled={busy !== null}
              >
                {busy === "portal" ? <Loader2 className="animate-spin" /> : <ExternalLink />}
                Manage subscription
              </Button>
            )}
          </div>
        </CardContent>
      </Card>

      <Card>
        <CardHeader
          title="Server-enforced limits"
          description="The dashboard is only a control surface. Disblock verifies this bot's subscription on the server before it enables paid functionality."
        />
        <CardContent>
          <ul className="list-disc space-y-1.5 pl-5 text-sm text-muted-foreground">
            <li>Free accounts can host up to five bots.</li>
            <li>
              Scheduled tasks are enabled only while this bot has an active or trialing Pro
              subscription.
            </li>
            <li>
              Subscription changes are reconciled from signed Stripe webhooks, not the Checkout
              return page.
            </li>
          </ul>
        </CardContent>
      </Card>
    </div>
  );
}
