import { Suspense, lazy, useCallback, useEffect, useState } from "react";
import { Navigate, Route, Routes } from "react-router";
import { ApiError, api } from "./api.ts";
import { Header } from "./components/Header.tsx";
import { Dashboard } from "./pages/Dashboard.tsx";
import { Landing } from "./pages/Landing.tsx";
import type { User } from "./types.ts";

// The block editor pulls in Blockly, so it only loads on a bot's page.
const BotPage = lazy(() =>
  import("./pages/BotPage.tsx").then((module) => ({ default: module.BotPage })),
);

type Session =
  | { state: "loading" }
  | { state: "signed-out" }
  | { state: "signed-in"; user: User }
  | { state: "error"; message: string };

export default function App() {
  const [session, setSession] = useState<Session>({ state: "loading" });

  useEffect(() => {
    let active = true;
    api
      .me()
      .then((user) => active && setSession({ state: "signed-in", user }))
      .catch((error: unknown) => {
        if (!active) {
          return;
        }
        if (error instanceof ApiError && error.status === 401) {
          setSession({ state: "signed-out" });
        } else {
          setSession({
            state: "error",
            message: error instanceof Error ? error.message : "Disblock is unreachable",
          });
        }
      });
    return () => {
      active = false;
    };
  }, []);

  const logout = useCallback(async () => {
    await api.logout();
    setSession({ state: "signed-out" });
  }, []);

  switch (session.state) {
    case "loading":
      return <div className="splash">Loading…</div>;
    case "error":
      return <div className="splash">{session.message}. Refresh the page to try again.</div>;
    case "signed-out":
      return <Landing />;
    case "signed-in":
      return (
        <div className="app">
          <Header user={session.user} onLogout={logout} />
          <Routes>
            <Route path="/" element={<Dashboard />} />
            <Route
              path="/bots/:id"
              element={
                <Suspense fallback={<main className="page muted">Loading the editor…</main>}>
                  <BotPage />
                </Suspense>
              }
            />
            <Route path="*" element={<Navigate to="/" replace />} />
          </Routes>
        </div>
      );
  }
}
