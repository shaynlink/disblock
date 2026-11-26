/**
 * Serves the web app and forwards /api to the Rust server on Fly.io, so the
 * browser sees a single origin: session cookies stay first-party and no CORS
 * is needed.
 *
 * With the `ORIGIN_SECRET` secret set (`wrangler secret put ORIGIN_SECRET`),
 * it also tells the server the browser's IP, for per-client rate limits.
 */

declare global {
  interface Env {
    ORIGIN_SECRET?: string;
  }
}
export default {
  async fetch(request, env): Promise<Response> {
    const url = new URL(request.url);
    if (!url.pathname.startsWith("/api/")) {
      return env.ASSETS.fetch(request);
    }

    const headers = new Headers(request.headers);
    headers.delete("host");
    // Never let a browser choose these: only the Worker vouches for an IP.
    headers.delete("x-disblock-origin-secret");
    headers.delete("x-disblock-client-ip");
    const clientIp = request.headers.get("cf-connecting-ip");
    if (env.ORIGIN_SECRET && clientIp) {
      headers.set("x-disblock-origin-secret", env.ORIGIN_SECRET);
      headers.set("x-disblock-client-ip", clientIp);
    }
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
