import { useCallback, useEffect, useState } from "react";
import { Loader2 } from "lucide-react";
import { useLocation, useNavigate } from "react-router";
import { ApiError, api } from "./api.ts";
import { Shell } from "./components/Shell.tsx";
import { DEFAULT_LOCALE, publicRouteFromPath } from "./data/landingData.ts";
import { LandingPage } from "./pages/LandingPage.tsx";
import { LegalPage } from "./pages/LegalPage.tsx";
import type { User } from "./types.ts";

const SESSION_HINT_KEY = "disblock_signed_in";

type Session =
  | { state: "loading" }
  | { state: "signed-out" }
  | { state: "signed-in"; user: User }
  | { state: "error"; message: string };

function readSessionHint(): boolean {
  try {
    return window.localStorage.getItem(SESSION_HINT_KEY) === "1";
  } catch {
    return false;
  }
}

function writeSessionHint(signedIn: boolean): void {
  try {
    if (signedIn) {
      window.localStorage.setItem(SESSION_HINT_KEY, "1");
    } else {
      window.localStorage.removeItem(SESSION_HINT_KEY);
    }
  } catch {
    // Ignore storage restrictions (private mode / blocked storage).
  }
}

export default function App() {
  const location = useLocation();
  const navigate = useNavigate();
  const publicRoute = publicRouteFromPath(location.pathname);
  const isPublicOnlyView =
    publicRoute !== null &&
    (publicRoute.page !== "landing" || publicRoute.locale !== DEFAULT_LOCALE);
  const isRootLanding = location.pathname === "/";

  const [session, setSession] = useState<Session>(() => {
    // Keep public routes immediately paintable (matching prerendered HTML)
    // unless a returning signed-in session hint is present on "/".
    if (isPublicOnlyView || (isRootLanding && !readSessionHint())) {
      return { state: "signed-out" };
    }
    return { state: "loading" };
  });

  useEffect(() => {
    if (!location.hash) {
      window.scrollTo({ top: 0, behavior: "instant" });
    }
  }, [location.pathname, location.hash]);

  useEffect(() => {
    let active = true;
    api
      .me()
      .then((user) => {
        if (!active) {
          return;
        }
        writeSessionHint(true);
        setSession({ state: "signed-in", user });
      })
      .catch((error: unknown) => {
        if (!active) {
          return;
        }
        if (error instanceof ApiError && error.status === 401) {
          writeSessionHint(false);
          setSession({ state: "signed-out" });
        } else if (isPublicOnlyView || (isRootLanding && !readSessionHint())) {
          // Keep public marketing and legal pages accessible even if the API is unreachable.
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
  }, [isPublicOnlyView, isRootLanding]);

  const logout = useCallback(async () => {
    await api.logout();
    writeSessionHint(false);
    setSession({ state: "signed-out" });
  }, []);

  if (publicRoute && publicRoute.page !== "landing") {
    return <LegalPage locale={publicRoute.locale} kind={publicRoute.page} />;
  }

  if (publicRoute?.page === "landing" && publicRoute.locale !== DEFAULT_LOCALE) {
    return (
      <LandingPage
        locale={publicRoute.locale}
        signedIn={session.state === "signed-in"}
        onOpenDashboard={() => void navigate("/")}
      />
    );
  }

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
      return <LandingPage locale={DEFAULT_LOCALE} />;
    case "signed-in":
      return <Shell user={session.user} onLogout={logout} />;
  }
}
