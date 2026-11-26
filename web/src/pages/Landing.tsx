import { LOGIN_URL } from "../api.ts";
import { Logo } from "../components/Header.tsx";

const STEPS = [
  {
    title: "Connect your bot",
    text: "Paste your bot token. It is encrypted, and only used to run your bot.",
  },
  {
    title: "Snap blocks together",
    text: "Slash commands, welcome messages, auto-replies and roles, without code.",
  },
  {
    title: "Save, it is live",
    text: "Disblock hosts your bot around the clock. Changes apply in seconds.",
  },
];

export function Landing() {
  const cancelled = new URLSearchParams(window.location.search).get("login") === "cancelled";

  return (
    <main className="landing">
      <Logo />
      <h1>Build Discord bots with blocks.</h1>
      <p className="landing-lead">We host them for you. No code, no server.</p>
      <a className="button primary large" href={LOGIN_URL}>
        Sign in with Discord
      </a>
      {cancelled && <p className="muted">Sign in was cancelled.</p>}
      <ol className="steps">
        {STEPS.map((step, index) => (
          <li key={step.title}>
            <span className="step-number">{index + 1}</span>
            <h2>{step.title}</h2>
            <p>{step.text}</p>
          </li>
        ))}
      </ol>
    </main>
  );
}
