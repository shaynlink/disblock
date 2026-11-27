import * as Sentry from "@sentry/react";
import type {
  Bot,
  BotDetail,
  BotIntents,
  ErrorLocation,
  ExecutionEntry,
  Program,
  User,
} from "./types.ts";

export interface BillingStatus {
  plan: "free" | "pro";
  status: string | null;
  cancel_at_period_end: boolean;
}

export class ApiError extends Error {
  readonly status: number;
  readonly location?: ErrorLocation;

  constructor(status: number, message: string, location?: ErrorLocation) {
    super(message);
    this.status = status;
    this.location = location;
  }
}

/** `/bots/<uuid>/program` → `/bots/:id/program`, so metrics stay low cardinality. */
function routeOf(path: string): string {
  return path.split("?")[0]!.replace(/\/[0-9a-f-]{36}(?=\/|$)/gi, "/:id");
}

async function request<T>(method: string, path: string, body?: unknown): Promise<T> {
  const started = performance.now();
  const response = await fetch(`/api${path}`, {
    method,
    credentials: "same-origin",
    headers: body === undefined ? undefined : { "Content-Type": "application/json" },
    body: body === undefined ? undefined : JSON.stringify(body),
  });
  const attributes = { route: `${method} ${routeOf(path)}`, status: response.status };
  Sentry.metrics.distribution("api.request.duration", performance.now() - started, {
    unit: "millisecond",
    attributes,
  });
  if (response.status >= 500) {
    Sentry.logger.error("API request failed", attributes);
  }

  if (!response.ok) {
    let message = `Request failed (${response.status})`;
    let location: ErrorLocation | undefined;
    try {
      const payload: unknown = await response.json();
      if (
        payload &&
        typeof payload === "object" &&
        "error" in payload &&
        typeof payload.error === "string"
      ) {
        message = payload.error;
      }
      if (
        payload &&
        typeof payload === "object" &&
        "location" in payload &&
        payload.location &&
        typeof payload.location === "object"
      ) {
        location = payload.location as ErrorLocation;
      }
    } catch {
      // The body was not JSON, keep the generic message.
    }
    throw new ApiError(response.status, message, location);
  }

  if (response.status === 204) {
    return undefined as T;
  }
  return (await response.json()) as T;
}

async function accountExport(): Promise<Blob> {
  const response = await fetch("/api/account/export", { credentials: "same-origin" });
  if (!response.ok) {
    throw new ApiError(response.status, `Could not export data (${response.status})`);
  }
  return response.blob();
}

export const api = {
  me: () => request<User>("GET", "/me"),
  logout: () => request<void>("POST", "/auth/logout"),
  accountExport,
  deleteAccount: () => request<void>("DELETE", "/account", { confirmation: "DELETE" }),
  bots: () => request<Bot[]>("GET", "/bots"),
  bot: (id: string) => request<BotDetail>("GET", `/bots/${id}`),
  addBot: (token: string) => request<Bot>("POST", "/bots", { token }),
  setEnabled: (id: string, enabled: boolean) => request<void>("PATCH", `/bots/${id}`, { enabled }),
  deleteBot: (id: string) => request<void>("DELETE", `/bots/${id}`),
  botIntents: (id: string) => request<BotIntents>("GET", `/bots/${id}/intents`),
  saveProgram: (id: string, workspace: object, program: Program) =>
    request<void>("PUT", `/bots/${id}/program`, { workspace, program }),
  executions: (id: string) => request<ExecutionEntry[]>("GET", `/bots/${id}/executions`),
  billingStatus: (id: string) => request<BillingStatus>("GET", `/billing/bots/${id}`),
  checkout: (botId: string) =>
    request<{ url: string }>("POST", "/billing/checkout", { bot_id: botId }),
  billingPortal: () => request<{ url: string }>("POST", "/billing/portal"),
};

export const LOGIN_URL = "/api/auth/login";

/** Send Messages, Add Reactions, Manage Roles, Read Message History, View Channels. */
const BOT_PERMISSIONS = "268504128";

export function inviteUrl(applicationId: string): string {
  const params = new URLSearchParams({
    client_id: applicationId,
    scope: "bot applications.commands",
    permissions: BOT_PERMISSIONS,
  });
  return `https://discord.com/oauth2/authorize?${params.toString()}`;
}

/** Discord avatar of a user or bot, or the default one. */
export function avatarUrl(userId: string, hash: string | null): string {
  if (!hash) {
    return "https://cdn.discordapp.com/embed/avatars/0.png";
  }
  return `https://cdn.discordapp.com/avatars/${userId}/${hash}.png?size=128`;
}
