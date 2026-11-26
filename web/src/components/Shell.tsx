import { type ReactNode, useEffect } from "react";
import {
  AlertCircle,
  ArrowLeft,
  Blocks,
  Bot,
  History,
  LayoutDashboard,
  LogOut,
  Settings,
} from "lucide-react";
import { Link, NavLink, Outlet, useMatch } from "react-router";
import { avatarUrl } from "../api.ts";
import { cn } from "../lib/styles.ts";
import type { User } from "../types.ts";
import { Logo } from "./Logo.tsx";
import { Button } from "./ui.tsx";

export { Logo };

interface NavItem {
  to: string;
  label: string;
  icon: ReactNode;
  end?: boolean;
}

function Sidebar({ user }: { user: User }) {
  const bot = useMatch("/bots/:id/*");
  const botId = bot?.params.id;

  const items: NavItem[] = botId
    ? [
        { to: `/bots/${botId}`, label: "Overview", icon: <LayoutDashboard />, end: true },
        { to: `/bots/${botId}/blocks`, label: "Blocks", icon: <Blocks /> },
        { to: `/bots/${botId}/activity`, label: "Activity", icon: <History /> },
        { to: `/bots/${botId}/settings`, label: "Settings", icon: <Settings /> },
      ]
    : [{ to: "/", label: "Bots", icon: <Bot />, end: true }];

  return (
    <aside className="flex shrink-0 flex-col rounded-xl border bg-card shadow-sm md:sticky md:top-0 md:h-full md:w-64">
      <div className="flex items-center justify-between border-b p-4">
        <span className="text-lg font-bold">{botId ? "Bot Management" : "Dashboard"}</span>
        {botId && (
          <Link
            to="/"
            className="rounded-lg p-2 transition-colors hover:bg-accent"
            aria-label="Back to all bots"
          >
            <ArrowLeft className="size-5" />
          </Link>
        )}
      </div>
      <nav className="flex-1 p-3">
        <h3 className="mb-2 hidden px-3 text-xs font-semibold tracking-wider text-muted-foreground uppercase md:block">
          {botId ? "Bot" : "Main"}
        </h3>
        <ul className="flex gap-1 overflow-x-auto md:flex-col">
          {items.map((item) => (
            <li key={item.to}>
              <NavLink
                to={item.to}
                end={item.end}
                className={({ isActive }) =>
                  cn(
                    "flex items-center gap-3 rounded-lg px-3 py-2 text-sm font-medium whitespace-nowrap transition-colors [&_svg]:size-4",
                    isActive ? "bg-primary text-primary-foreground" : "hover:bg-accent",
                  )
                }
              >
                {item.icon}
                {item.label}
              </NavLink>
            </li>
          ))}
        </ul>
      </nav>
      <div className="hidden border-t p-3 md:block">
        <div className="flex items-center gap-3 rounded-lg bg-muted px-3 py-2">
          <img
            className="size-8 rounded-full bg-background"
            src={avatarUrl(user.discord_id, user.avatar)}
            alt=""
          />
          <span className="truncate text-sm font-medium">{user.username}</span>
        </div>
      </div>
    </aside>
  );
}

export function Shell({ user, onLogout }: { user: User; onLogout: () => void }) {
  const isBotRoute = Boolean(useMatch("/bots/:id/*"));

  useEffect(() => {
    document.documentElement.lang = "en";
    if (!isBotRoute) {
      document.title = "Your bots — Disblock";
    }
  }, [isBotRoute]);

  return (
    <div className="flex min-h-screen flex-col gap-2.5 p-2.5 md:h-screen">
      <header className="sticky top-0 z-40 rounded-xl border bg-white">
        <div className="flex items-center justify-between px-6 py-4">
          <Link to="/">
            <Logo />
          </Link>
          <Button
            variant="ghost"
            size="icon"
            onClick={onLogout}
            aria-label="Sign out"
            title="Sign out"
          >
            <LogOut />
          </Button>
        </div>
      </header>

      <div className="flex min-h-0 flex-1 flex-col gap-4 md:flex-row">
        <Sidebar user={user} />
        <div className="flex min-h-0 flex-1 flex-col overflow-y-auto rounded-xl p-1">
          <div className="mb-2 flex items-start gap-2 rounded-xl border bg-yellow-100 px-6 py-3 text-sm shadow-sm">
            <AlertCircle className="mt-0.5 size-4 shrink-0 text-amber-600" />
            <p>
              Disblock is in <span className="font-semibold">beta</span>. Hosting is free while we
              build, and a lot is coming.
            </p>
          </div>
          <main className="mx-auto flex w-full max-w-6xl min-h-0 flex-1 flex-col px-4 py-6">
            <Outlet />
          </main>
        </div>
      </div>
    </div>
  );
}
