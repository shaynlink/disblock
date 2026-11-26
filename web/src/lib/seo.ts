import {
  LANDING_COPY,
  LEGAL_COPY,
  LOCALE_META,
  type Locale,
  type PublicPageKind,
  localeUrl,
} from "../data/landingData.ts";

/** Synchronizes document language, title, description, and canonical link on SPA route changes. */
export function syncDocumentSeo(locale: Locale, page: PublicPageKind): void {
  const htmlLang = LOCALE_META[locale].htmlLang;
  const title =
    page === "landing" ? LANDING_COPY[locale].seo.title : LEGAL_COPY[locale][page].seoTitle;
  const description =
    page === "landing"
      ? LANDING_COPY[locale].seo.description
      : LEGAL_COPY[locale][page].seoDescription;

  document.documentElement.lang = htmlLang;
  document.title = title;

  const descriptionMeta = document.querySelector<HTMLMetaElement>('meta[name="description"]');
  if (descriptionMeta) {
    descriptionMeta.content = description;
  }

  const canonicalLink = document.querySelector<HTMLLinkElement>('link[rel="canonical"]');
  if (canonicalLink) {
    let origin = "https://disblock.org";
    try {
      origin = new URL(canonicalLink.href).origin;
    } catch {
      // Keep default origin fallback if canonical href is relative or malformed.
    }
    canonicalLink.href = localeUrl(origin, locale, page);
  }
}
