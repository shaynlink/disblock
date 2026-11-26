import type {
  Bot,
  BotDetail,
  BotIntents,
  ErrorLocation,
  ExecutionEntry,
  Program,
  User,
} from "./types.ts";

export class ApiError extends Error {
  readonly status: number;
  readonly location?: ErrorLocation;

  constructor(status: number, message: string, location?: ErrorLocation) {
    super(message);
    this.status = status;
    this.location = location;
  }
}

async function request<T>(method: string, path: string, body?: unknown): Promise<T> {
  const response = await fetch(`/api${path}`, {
    method,
    credentials: "same-origin",
    headers: body === undefined ? undefined : { "Content-Type": "application/json" },
    body: body === undefined ? undefined : JSON.stringify(body),
  });

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

export const api = {
  me: () => request<User>("GET", "/me"),
  logout: () => request<void>("POST", "/auth/logout"),
  bots: () => request<Bot[]>("GET", "/bots"),
  bot: (id: string) => request<BotDetail>("GET", `/bots/${id}`),
  addBot: (token: string) => request<Bot>("POST", "/bots", { token }),
  setEnabled: (id: string, enabled: boolean) => request<void>("PATCH", `/bots/${id}`, { enabled }),
  deleteBot: (id: string) => request<void>("DELETE", `/bots/${id}`),
  botIntents: (id: string) => request<BotIntents>("GET", `/bots/${id}/intents`),
  saveProgram: (id: string, workspace: object, program: Program) =>
    request<void>("PUT", `/bots/${id}/program`, { workspace, program }),
  executions: (id: string) => request<ExecutionEntry[]>("GET", `/bots/${id}/executions`),
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
