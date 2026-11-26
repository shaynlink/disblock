import fs from "node:fs/promises";
import path from "node:path";
import tailwindcss from "@tailwindcss/vite";
import react from "@vitejs/plugin-react";
import { type Plugin, createServer, defineConfig, lazyPlugins } from "vite-plus";

interface PrerenderModule {
  DEFAULT_LOCALE: string;
  PUBLIC_PAGE_KINDS: readonly string[];
  SUPPORTED_LOCALES: readonly string[];
  LOCALE_META: Record<
    string,
    {
      code: string;
      htmlLang: string;
      hreflang: string;
      ogLocale: string;
      canonicalPath: string;
    }
  >;
  LANDING_COPY: Record<
    string,
    {
      seo: {
        title: string;
        description: string;
        ogImageAlt: string;
      };
    }
  >;
  LEGAL_COPY: Record<
    string,
    {
      privacy: { seoTitle: string; seoDescription: string };
      terms: { seoTitle: string; seoDescription: string };
    }
  >;
  localeUrl: (siteUrl: string, locale: string, page?: string) => string;
  publicCanonicalPath: (locale: string, page: string) => string;
  buildJsonLd: (siteUrl: string, locale: string, page?: string) => Record<string, unknown>;
  renderPublicPageHtml: (page: string, locale: string) => string;
}

function escapeHtml(value: string): string {
  return value
    .replaceAll("&", "&amp;")
    .replaceAll("<", "&lt;")
    .replaceAll(">", "&gt;")
    .replaceAll('"', "&quot;");
}

function pageSeoCopy(
  mod: PrerenderModule,
  locale: string,
  page: string,
): { title: string; description: string; ogImageAlt: string } {
  const landing = mod.LANDING_COPY[locale];
  const legal = mod.LEGAL_COPY[locale];
  if (!landing || !legal) {
    throw new Error(`Missing locale copy for "${locale}"`);
  }
  if (page === "privacy") {
    return {
      title: legal.privacy.seoTitle,
      description: legal.privacy.seoDescription,
      ogImageAlt: landing.seo.ogImageAlt,
    };
  }
  if (page === "terms") {
    return {
      title: legal.terms.seoTitle,
      description: legal.terms.seoDescription,
      ogImageAlt: landing.seo.ogImageAlt,
    };
  }
  return landing.seo;
}

function buildSeoHead(siteUrl: string, locale: string, page: string, mod: PrerenderModule): string {
  const meta = mod.LOCALE_META[locale];
  if (!meta) {
    throw new Error(`Missing locale metadata for "${locale}"`);
  }

  const seo = pageSeoCopy(mod, locale, page);
  const canonical = mod.localeUrl(siteUrl, locale, page);
  const defaultUrl = mod.localeUrl(siteUrl, mod.DEFAULT_LOCALE, page);
  const ogImageUrl = `${siteUrl}/og-image.png`;
  const jsonLd = JSON.stringify(mod.buildJsonLd(siteUrl, locale, page)).replaceAll("<", "\\u003c");

  const alternateLinks = mod.SUPPORTED_LOCALES.map((loc) => {
    const altMeta = mod.LOCALE_META[loc];
    return `    <link rel="alternate" hreflang="${escapeHtml(altMeta?.hreflang ?? loc)}" href="${escapeHtml(mod.localeUrl(siteUrl, loc, page))}" />`;
  });
  alternateLinks.push(
    `    <link rel="alternate" hreflang="x-default" href="${escapeHtml(defaultUrl)}" />`,
  );

  const alternateOgLocales = mod.SUPPORTED_LOCALES.filter((loc) => loc !== locale).map((loc) => {
    const altMeta = mod.LOCALE_META[loc];
    return `    <meta property="og:locale:alternate" content="${escapeHtml(altMeta?.ogLocale ?? loc)}" />`;
  });

  return [
    `    <link rel="canonical" href="${escapeHtml(canonical)}" />`,
    ...alternateLinks,
    `    <meta property="og:type" content="website" />`,
    `    <meta property="og:site_name" content="Disblock" />`,
    `    <meta property="og:locale" content="${escapeHtml(meta.ogLocale)}" />`,
    ...alternateOgLocales,
    `    <meta property="og:title" content="${escapeHtml(seo.title)}" />`,
    `    <meta property="og:description" content="${escapeHtml(seo.description)}" />`,
    `    <meta property="og:url" content="${escapeHtml(canonical)}" />`,
    `    <meta property="og:image" content="${escapeHtml(ogImageUrl)}" />`,
    `    <meta property="og:image:width" content="1200" />`,
    `    <meta property="og:image:height" content="630" />`,
    `    <meta property="og:image:alt" content="${escapeHtml(seo.ogImageAlt)}" />`,
    `    <meta name="twitter:card" content="summary_large_image" />`,
    `    <meta name="twitter:title" content="${escapeHtml(seo.title)}" />`,
    `    <meta name="twitter:description" content="${escapeHtml(seo.description)}" />`,
    `    <meta name="twitter:image" content="${escapeHtml(ogImageUrl)}" />`,
    `    <script type="application/ld+json">${jsonLd}</script>`,
  ].join("\n");
}

function buildSitemapXml(siteUrl: string, mod: PrerenderModule): string {
  const urls: string[] = [];

  for (const page of mod.PUBLIC_PAGE_KINDS) {
    const defaultUrl = mod.localeUrl(siteUrl, mod.DEFAULT_LOCALE, page);
    for (const locale of mod.SUPPORTED_LOCALES) {
      const locUrl = mod.localeUrl(siteUrl, locale, page);
      const priority = page === "landing" ? (locale === mod.DEFAULT_LOCALE ? "1.0" : "0.9") : "0.4";
      const changefreq = page === "landing" ? "weekly" : "monthly";
      const alternates = mod.SUPPORTED_LOCALES.map((alt) => {
        const altMeta = mod.LOCALE_META[alt];
        return `    <xhtml:link rel="alternate" hreflang="${escapeHtml(altMeta?.hreflang ?? alt)}" href="${escapeHtml(mod.localeUrl(siteUrl, alt, page))}" />`;
      });
      alternates.push(
        `    <xhtml:link rel="alternate" hreflang="x-default" href="${escapeHtml(defaultUrl)}" />`,
      );

      urls.push(
        [
          `  <url>`,
          `    <loc>${escapeHtml(locUrl)}</loc>`,
          ...alternates,
          `    <changefreq>${changefreq}</changefreq>`,
          `    <priority>${priority}</priority>`,
          `  </url>`,
        ].join("\n"),
      );
    }
  }

  return [
    `<?xml version="1.0" encoding="UTF-8"?>`,
    `<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9"`,
    `        xmlns:xhtml="http://www.w3.org/1999/xhtml">`,
    ...urls,
    `</urlset>`,
    ``,
  ].join("\n");
}

function buildRobotsTxt(siteUrl: string): string {
  return [
    `User-agent: *`,
    `Allow: /`,
    `Disallow: /api/`,
    `Disallow: /bots/`,
    ``,
    `Sitemap: ${siteUrl}/sitemap.xml`,
    ``,
  ].join("\n");
}

function disblockSeoPlugin(): Plugin {
  let isBuild = false;
  let rootDir = process.cwd();
  let outDir = "dist";
  const siteUrl = (process.env.VITE_SITE_URL || "https://disblock.org").replace(/\/+$/, "");

  return {
    name: "disblock-seo-prerender",
    configResolved(config) {
      isBuild = config.command === "build";
      rootDir = config.root;
      outDir = path.resolve(config.root, config.build.outDir);
    },
    transformIndexHtml(html) {
      return html.replaceAll("https://disblock.org", siteUrl);
    },
    async closeBundle() {
      if (!isBuild) {
        return;
      }

      const templatePath = path.join(outDir, "index.html");
      const template = await fs.readFile(templatePath, "utf8");

      const server = await createServer({
        root: rootDir,
        configFile: false,
        plugins: [react()],
        appType: "custom",
        logLevel: "silent",
        optimizeDeps: { noDiscovery: true },
      });

      try {
        const mod = (await server.ssrLoadModule("/src/prerender.ts")) as PrerenderModule;

        for (const page of mod.PUBLIC_PAGE_KINDS) {
          for (const locale of mod.SUPPORTED_LOCALES) {
            const meta = mod.LOCALE_META[locale];
            if (!meta) {
              continue;
            }

            const seo = pageSeoCopy(mod, locale, page);
            const appHtml = mod.renderPublicPageHtml(page, locale);
            const seoHead = buildSeoHead(siteUrl, locale, page, mod);

            const localizedHtml = template
              .replace(/<html lang="[^"]*">/, `<html lang="${escapeHtml(meta.htmlLang)}">`)
              .replace(/<title>[\s\S]*?<\/title>/, `<title>${escapeHtml(seo.title)}</title>`)
              .replace(
                /<meta\s+name="description"\s+content="[^"]*"\s*\/>/,
                `<meta name="description" content="${escapeHtml(seo.description)}" />`,
              )
              .replace(/[ \t]*<!--SEO_HEAD_START-->[\s\S]*?<!--SEO_HEAD_END-->/, seoHead)
              .replace('<div id="root"></div>', `<div id="root">${appHtml}</div>`);

            const relDir = mod.publicCanonicalPath(locale, page).replace(/^\/+/, "");
            const targetPath = path.join(outDir, relDir, "index.html");
            await fs.mkdir(path.dirname(targetPath), { recursive: true });
            await fs.writeFile(targetPath, localizedHtml, "utf8");
          }
        }

        await fs.writeFile(path.join(outDir, "robots.txt"), buildRobotsTxt(siteUrl), "utf8");
        await fs.writeFile(path.join(outDir, "sitemap.xml"), buildSitemapXml(siteUrl, mod), "utf8");
      } finally {
        await server.close();
      }
    },
  };
}

// https://vite.dev/config/
export default defineConfig({
  fmt: {
    ignorePatterns: ["worker/worker-configuration.d.ts"],
  },
  lint: {
    // Generated by `wrangler types`.
    ignorePatterns: ["worker/worker-configuration.d.ts"],
    plugins: ["react", "typescript", "oxc"],
    rules: {
      "react/rules-of-hooks": "error",
      "react/only-export-components": [
        "warn",
        {
          allowConstantExport: true,
        },
      ],
      "vite-plus/prefer-vite-plus-imports": "error",
    },
    options: {
      typeAware: true,
      typeCheck: true,
    },
    jsPlugins: [
      {
        name: "vite-plus",
        specifier: "vite-plus/oxlint-plugin",
      },
    ],
  },
  plugins: lazyPlugins(() => [react(), tailwindcss(), disblockSeoPlugin()]),
  build: {
    // Blockly alone weighs ~900 kB and is only loaded by the editor page.
    chunkSizeWarningLimit: 1000,
  },
  server: {
    // The Rust API, as Cloudflare routes /api in production.
    proxy: { "/api": "http://localhost:8080" },
  },
});
