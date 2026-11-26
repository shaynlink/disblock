/**
 * Serves the web app and forwards /api to the Rust server on Fly.io, so the
 * browser sees a single origin: session cookies stay first-party and no CORS
 * is needed.
 */
export default {
  async fetch(request, env): Promise<Response> {
    const url = new URL(request.url);
    if (!url.pathname.startsWith("/api/")) {
      return env.ASSETS.fetch(request);
    }

    const headers = new Headers(request.headers);
    headers.delete("host");
    headers.set("x-forwarded-host", url.host);
    headers.set("x-forwarded-proto", url.protocol.replace(":", ""));

    return fetch(new URL(url.pathname + url.search, env.API_ORIGIN), {
      method: request.method,
      headers,
      body: request.body,
      // OAuth redirects and their cookies must reach the browser untouched.
      redirect: "manual",
    });
  },
} satisfies ExportedHandler<Env>;
