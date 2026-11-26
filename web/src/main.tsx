import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { Navigate, createBrowserRouter } from "react-router";
import { RouterProvider } from "react-router/dom";
import App from "./App.tsx";
import { DEFAULT_LOCALE, SUPPORTED_LOCALES, publicRoutePath } from "./data/landingData.ts";
import { BotsPage } from "./pages/BotsPage.tsx";
import { ActivityPage } from "./pages/bot/ActivityPage.tsx";
import { BotLayout } from "./pages/bot/BotLayout.tsx";
import { OverviewPage } from "./pages/bot/OverviewPage.tsx";
import { SettingsPage } from "./pages/bot/SettingsPage.tsx";
import "./index.css";

const publicRoutes = SUPPORTED_LOCALES.flatMap((locale) => {
  const routes: Array<{ path: string; element: null }> = [];
  if (locale !== DEFAULT_LOCALE) {
    routes.push({ path: publicRoutePath(locale, "landing").replace(/^\//, ""), element: null });
  }
  routes.push({ path: publicRoutePath(locale, "privacy").replace(/^\//, ""), element: null });
  routes.push({ path: publicRoutePath(locale, "terms").replace(/^\//, ""), element: null });
  return routes;
});

const router = createBrowserRouter([
  {
    path: "/",
    element: <App />,
    children: [
      { index: true, element: <BotsPage /> },
      ...publicRoutes,
      {
        path: "bots/:id",
        element: <BotLayout />,
        children: [
          { index: true, element: <OverviewPage /> },
          {
            path: "blocks",
            // The block editor pulls in Blockly, so it only loads when opened.
            lazy: async () => ({
              Component: (await import("./pages/bot/BlocksPage.tsx")).BlocksPage,
            }),
          },
          { path: "activity", element: <ActivityPage /> },
          { path: "settings", element: <SettingsPage /> },
        ],
      },
      { path: "*", element: <Navigate to="/" replace /> },
    ],
  },
]);

const root = document.getElementById("root");
if (!root) {
  throw new Error("index.html is missing the #root element");
}

createRoot(root).render(
  <StrictMode>
    <RouterProvider router={router} />
  </StrictMode>,
);
