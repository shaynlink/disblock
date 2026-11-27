import * as Sentry from "@sentry/react";
import { useEffect } from "react";
import {
  createRoutesFromChildren,
  matchRoutes,
  useLocation,
  useNavigationType,
} from "react-router";

const dsn = import.meta.env.VITE_SENTRY_DSN;
/** `production` and `staging` come from the deploy; local builds report `development`. */
const environment =
  import.meta.env.VITE_SENTRY_ENVIRONMENT ?? (import.meta.env.DEV ? "development" : "production");
/** Production samples; development and staging keep everything to debug releases. */
const sampleRate = environment === "production" ? 0.1 : 1.0;

/**
 * Error tracking, tracing, logs and Session Replay. Disabled when no DSN is
 * configured, so local development sends nothing by default.
 */
if (dsn) {
  Sentry.init({
    dsn,
    environment,
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
    tracesSampleRate: sampleRate,
    // Only our own API, which is same-origin behind the Cloudflare Worker.
    tracePropagationTargets: [/^\/api\//],
    replaysSessionSampleRate: sampleRate,
    replaysOnErrorSampleRate: 1.0,
  });
}
