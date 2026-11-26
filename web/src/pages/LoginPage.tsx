import { Blocks, Rocket, ShieldCheck } from "lucide-react";
import { LOGIN_URL } from "../api.ts";
import { DiscordIcon } from "../components/DiscordIcon.tsx";
import { Logo } from "../components/Shell.tsx";
import { Card } from "../components/ui.tsx";
import { buttonClass } from "../lib/styles.ts";

const FEATURES = [
  {
    icon: <ShieldCheck />,
    title: "Connect your bot",
    text: "Your token is encrypted and only used to run your bot.",
  },
  {
    icon: <Blocks />,
    title: "Snap blocks together",
    text: "Slash commands, welcome messages, auto-replies and roles.",
  },
  {
    icon: <Rocket />,
    title: "Save, it is live",
    text: "We host your bot around the clock. Changes apply in seconds.",
  },
];

export function LoginPage() {
  const cancelled = new URLSearchParams(window.location.search).get("login") === "cancelled";

  return (
    <div className="flex min-h-screen items-center justify-center p-6">
      <Card className="w-full max-w-md p-8">
        <div className="flex justify-center">
          <Logo />
        </div>
        <div className="mt-8 space-y-2 text-center">
          <h1 className="text-2xl font-bold">Build Discord bots with blocks</h1>
          <p className="text-muted-foreground">No code, no server. We host them for you.</p>
        </div>

        <a href={LOGIN_URL} className={buttonClass("default", "lg", "mt-8 w-full")}>
          <DiscordIcon />
          Sign in with Discord
        </a>
        {cancelled && (
          <p className="mt-3 text-center text-sm text-muted-foreground">Sign in was cancelled.</p>
        )}

        <ul className="mt-8 space-y-4 border-t pt-6">
          {FEATURES.map((feature) => (
            <li key={feature.title} className="flex gap-3">
              <span className="flex size-8 shrink-0 items-center justify-center rounded-lg bg-muted [&_svg]:size-4">
                {feature.icon}
              </span>
              <div>
                <p className="text-sm font-medium">{feature.title}</p>
                <p className="text-sm text-muted-foreground">{feature.text}</p>
              </div>
            </li>
          ))}
        </ul>
      </Card>
    </div>
  );
}
