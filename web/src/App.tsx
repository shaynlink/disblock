import { useCallback, useEffect, useState } from "react";
import { Loader2 } from "lucide-react";
import { ApiError, api } from "./api.ts";
import { Shell } from "./components/Shell.tsx";
import { LoginPage } from "./pages/LoginPage.tsx";
import type { User } from "./types.ts";

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
      return (
        <div className="flex min-h-screen items-center justify-center">
          <Loader2 className="size-8 animate-spin text-primary" />
        </div>
      );
    case "error":
      return (
        <div className="flex min-h-screen items-center justify-center p-6 text-center text-muted-foreground">
          {session.message}. Refresh the page to try again.
        </div>
      );
    case "signed-out":
      return <LoginPage />;
    case "signed-in":
      return <Shell user={session.user} onLogout={logout} />;
  }
}
