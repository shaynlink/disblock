import { createElement } from "react";
import { renderToString } from "react-dom/server";
import { StaticRouter } from "react-router";
import {
  DEFAULT_LOCALE,
  LANDING_COPY,
  LEGAL_COPY,
  LOCALE_META,
  type Locale,
  PUBLIC_PAGE_KINDS,
  type PublicPageKind,
  SUPPORTED_LOCALES,
  buildJsonLd,
  localeUrl,
  publicCanonicalPath,
  publicRoutePath,
} from "./data/landingData.ts";
import { LandingPage } from "./pages/LandingPage.tsx";
import { LegalPage } from "./pages/LegalPage.tsx";

export {
  DEFAULT_LOCALE,
  LANDING_COPY,
  LEGAL_COPY,
  LOCALE_META,
  PUBLIC_PAGE_KINDS,
  SUPPORTED_LOCALES,
  buildJsonLd,
  localeUrl,
  publicCanonicalPath,
};
export type { Locale, PublicPageKind };

export function renderPublicPageHtml(page: PublicPageKind, locale: Locale): string {
  const child =
    page === "landing"
      ? createElement(LandingPage, { locale })
      : createElement(LegalPage, { locale, kind: page });

  return renderToString(
    createElement(StaticRouter, { location: publicRoutePath(locale, page) }, child),
  );
}

export function renderLandingHtml(locale: Locale): string {
  return renderPublicPageHtml("landing", locale);
}
