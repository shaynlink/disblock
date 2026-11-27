import * as Sentry from "@sentry/react";
import { useEffect } from "react";
import {
  createRoutesFromChildren,
  matchRoutes,
  useLocation,
  useNavigationType,
} from "react-router";

const dsn = import.meta.env.VITE_SENTRY_DSN;

/**
 * Error tracking, tracing, logs and Session Replay. Disabled when no DSN is
 * configured, so local development sends nothing by default.
 */
if (dsn) {
  Sentry.init({
    dsn,
    environment: import.meta.env.VITE_SENTRY_ENVIRONMENT ?? import.meta.env.MODE,
    release: import.meta.env.VITE_RELEASE_VERSION,
    integrations: [
      Sentry.reactRouterV7BrowserTracingIntegration({
        useEffect,
        useLocation,
        useNavigationType,
        createRoutesFromChildren,
        matchRoutes,
      }),
      // Text and media stay masked: bot tokens and member data never leave the page.
      Sentry.replayIntegration({ maskAllText: true, blockAllMedia: true }),
      // Forward warnings and errors written to the console as Sentry logs.
      Sentry.consoleLoggingIntegration({ levels: ["warn", "error"] }),
    ],
    // Every trace locally, a sample in production.
    tracesSampleRate: import.meta.env.DEV ? 1.0 : 0.1,
    // Only our own API, which is same-origin behind the Cloudflare Worker.
    tracePropagationTargets: [/^\/api\//],
    replaysSessionSampleRate: import.meta.env.DEV ? 1.0 : 0.1,
    replaysOnErrorSampleRate: 1.0,
  });
}
