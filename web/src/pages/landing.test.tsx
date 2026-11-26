import { describe, expect, it } from "vite-plus/test";
import {
  LANDING_COPY,
  LEGAL_COPY,
  LOCALE_META,
  PUBLIC_PAGE_KINDS,
  SUPPORTED_LOCALES,
  buildJsonLd,
  localeFromPath,
  localeUrl,
  publicRouteFromPath,
} from "../data/landingData.ts";
import { renderLandingHtml, renderPublicPageHtml } from "../prerender.ts";

describe("landing page i18n and SSR", () => {
  it("resolves locales and public routes from URL paths cleanly", () => {
    expect(localeFromPath("/")).toBe("en");
    expect(localeFromPath("/fr")).toBe("fr");
    expect(localeFromPath("/fr/")).toBe("fr");
    expect(localeFromPath("/bots/123")).toBe("en");
    expect(publicRouteFromPath("/legal")).toEqual({ page: "legal", locale: "en" });

    expect(publicRouteFromPath("/")).toEqual({ page: "landing", locale: "en" });
    expect(publicRouteFromPath("/privacy")).toEqual({ page: "privacy", locale: "en" });
    expect(publicRouteFromPath("/terms")).toEqual({ page: "terms", locale: "en" });
    expect(publicRouteFromPath("/fr/privacy/")).toEqual({ page: "privacy", locale: "fr" });
    expect(publicRouteFromPath("/fr/terms")).toEqual({ page: "terms", locale: "fr" });
    expect(publicRouteFromPath("/bots/123")).toBeNull();
  });

  it("renders valid semantic landing and legal HTML for every supported locale", () => {
    for (const locale of SUPPORTED_LOCALES) {
      const html = renderLandingHtml(locale);
      const copy = LANDING_COPY[locale];

      expect(html.match(/<h1\b/g)?.length).toBe(1);
      expect(html).toContain("<header");
      expect(html).toContain("<nav");
      expect(html).toContain("<main");
      expect(html).toContain("<footer");
      expect(html).toContain(copy.hero.title);
      expect(html).toContain('popover="auto"');
      expect(html).toContain("/api/auth/login");

      for (const item of copy.faq.items) {
        expect(html.replaceAll("&#x27;", "'")).toContain(item.question);
      }

      for (const kind of ["privacy", "terms", "legal"] as const) {
        const legalHtml = renderPublicPageHtml(kind, locale);
        expect(legalHtml.match(/<h1\b/g)?.length).toBe(1);
        expect(legalHtml.replaceAll("&#x27;", "'")).toContain(LEGAL_COPY[locale][kind].title);
      }
    }
  });

  it("builds JSON-LD matching FAQ items and canonical URLs for every locale and page", () => {
    for (const locale of SUPPORTED_LOCALES) {
      const jsonLd = buildJsonLd("https://disblock.org/", locale, "landing") as {
        "@graph": Array<Record<string, unknown>>;
      };
      const [softwareApp, faqPage] = jsonLd["@graph"];

      expect(softwareApp?.["@type"]).toBe("SoftwareApplication");
      expect(softwareApp?.url).toBe(localeUrl("https://disblock.org", locale, "landing"));
      expect(softwareApp?.inLanguage).toBe(LOCALE_META[locale].htmlLang);

      expect(faqPage?.["@type"]).toBe("FAQPage");
      const entities = faqPage?.mainEntity as Array<{ name: string }>;
      expect(entities).toHaveLength(LANDING_COPY[locale].faq.items.length);

      for (const page of PUBLIC_PAGE_KINDS) {
        if (page === "landing") {
          continue;
        }
        const legalLd = buildJsonLd("https://disblock.org", locale, page);
        expect(legalLd["@type"]).toBe("WebPage");
        expect(legalLd.url).toBe(localeUrl("https://disblock.org", locale, page));
      }
    }
  });
});
