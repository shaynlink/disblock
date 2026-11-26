import { useEffect, useId } from "react";
import { ArrowLeft, Globe } from "lucide-react";
import { Link, useNavigate } from "react-router";
import { Logo } from "../components/Logo.tsx";
import { Card } from "../components/ui.tsx";
import {
  LANDING_COPY,
  LEGAL_COPY,
  LOCALE_META,
  type Locale,
  SUPPORTED_LOCALES,
  isLocale,
  publicRoutePath,
} from "../data/landingData.ts";
import { syncDocumentSeo } from "../lib/seo.ts";
import { buttonClass, cn } from "../lib/styles.ts";
import { LEGAL_ENTITY, LEGAL_HOSTS } from "../data/legalEntity.ts";

export interface LegalPageProps {
  locale: Locale;
  kind: "privacy" | "terms" | "legal";
}

export function LegalPage({ locale, kind }: LegalPageProps) {
  const navCopy = LANDING_COPY[locale];
  const legal = LEGAL_COPY[locale];
  const doc = legal[kind];
  const entityLabels =
    locale === "fr"
      ? {
          title: "Informations sur l'éditeur",
          publisher: "Éditeur",
          legalForm: "Forme juridique",
          address: "Adresse du siège",
          registration: "Immatriculation / TVA",
          contact: "Contact",
          director: "Directeur de publication",
          mediator: "Médiateur de la consommation",
          hosts: "Hébergeurs techniques",
        }
      : {
          title: "Publisher information",
          publisher: "Publisher",
          legalForm: "Legal form",
          address: "Registered address",
          registration: "Registration / VAT",
          contact: "Contact",
          director: "Publication director",
          mediator: "Consumer mediator",
          hosts: "Technical hosts",
        };
  const navigate = useNavigate();
  const langSelectId = useId();

  useEffect(() => {
    syncDocumentSeo(locale, kind);
  }, [locale, kind]);

  const onLocaleChange = (nextValue: string) => {
    if (!isLocale(nextValue) || nextValue === locale) {
      return;
    }
    void navigate(publicRoutePath(nextValue, kind));
  };

  return (
    <div className="flex min-h-screen flex-col bg-background text-foreground">
      <header className="sticky top-0 z-40 border-b bg-background/95 backdrop-blur-sm">
        <a href="#main-heading" className="skip-link visually-hidden">
          {navCopy.a11y.skipToContent}
        </a>

        <div className="mx-auto flex max-w-4xl items-center justify-between gap-4 px-4 py-3 sm:px-6">
          <Link to={publicRoutePath(locale, "landing")} className="rounded-lg">
            <Logo />
          </Link>

          <nav aria-label={navCopy.a11y.primaryNav} className="flex items-center gap-3">
            <div className="relative flex items-center">
              <label htmlFor={langSelectId} className="visually-hidden">
                {navCopy.a11y.languageLabel}
              </label>
              <Globe
                className="pointer-events-none absolute left-2.5 size-3.5 text-muted-foreground"
                aria-hidden="true"
              />
              <select
                id={langSelectId}
                value={locale}
                onChange={(event) => onLocaleChange(event.target.value)}
                className="min-h-8 cursor-pointer rounded-md border border-input bg-background py-1.5 pr-3 pl-8 text-xs leading-normal font-medium text-foreground shadow-xs transition-colors hover:bg-accent focus-visible:ring-2 focus-visible:ring-ring focus-visible:outline-none"
              >
                {SUPPORTED_LOCALES.map((code) => (
                  <option key={code} value={code}>
                    {LOCALE_META[code].label}
                  </option>
                ))}
              </select>
            </div>

            <Link to={publicRoutePath(locale, "landing")} className={buttonClass("outline", "sm")}>
              <ArrowLeft className="size-4" aria-hidden="true" />
              <span className="hidden sm:inline">{legal.backToHome}</span>
            </Link>
          </nav>
        </div>
      </header>

      <main className="mx-auto w-full max-w-4xl flex-1 px-4 py-12 sm:px-6 sm:py-16">
        <h1
          id="main-heading"
          tabIndex={-1}
          className="text-3xl font-bold tracking-tight sm:text-4xl"
        >
          {doc.title}
        </h1>
        <p className="mt-2 text-sm text-muted-foreground">{doc.updatedLabel}</p>
        <p className="mt-6 text-base leading-relaxed text-muted-foreground">{doc.intro}</p>

        {kind === "legal" && (
          <Card className="mt-8 border-amber-500/40 bg-amber-500/5 p-6">
            <h2 className="text-lg font-semibold">{entityLabels.title}</h2>
            <dl className="mt-4 grid gap-x-6 gap-y-3 text-sm sm:grid-cols-[11rem_1fr]">
              <dt className="font-medium">{entityLabels.publisher}</dt>
              <dd>{LEGAL_ENTITY.publisher}</dd>
              <dt className="font-medium">{entityLabels.legalForm}</dt>
              <dd>{LEGAL_ENTITY.legalForm}</dd>
              <dt className="font-medium">{entityLabels.address}</dt>
              <dd>{LEGAL_ENTITY.registeredAddress}</dd>
              <dt className="font-medium">{entityLabels.registration}</dt>
              <dd>
                {LEGAL_ENTITY.registration} · {LEGAL_ENTITY.vatNumber}
              </dd>
              <dt className="font-medium">{entityLabels.contact}</dt>
              <dd>
                {LEGAL_ENTITY.contactEmail} · {LEGAL_ENTITY.phone}
              </dd>
              <dt className="font-medium">{entityLabels.director}</dt>
              <dd>{LEGAL_ENTITY.publicationDirector}</dd>
              <dt className="font-medium">{entityLabels.mediator}</dt>
              <dd>{LEGAL_ENTITY.consumerMediator}</dd>
              <dt className="font-medium">{entityLabels.hosts}</dt>
              <dd>{LEGAL_HOSTS.join("; ")}</dd>
            </dl>
          </Card>
        )}

        <div className="mt-10 space-y-6">
          {doc.sections.map((section) => (
            <Card key={section.heading} className="p-6">
              <h2 className="text-lg font-semibold">{section.heading}</h2>
              <div className="mt-3 space-y-3 text-sm leading-relaxed text-muted-foreground">
                {section.paragraphs.map((paragraph) => (
                  <p key={paragraph}>{paragraph}</p>
                ))}
                {section.bullets && (
                  <ul className="list-disc space-y-1.5 pl-5">
                    {section.bullets.map((bullet) => (
                      <li key={bullet}>{bullet}</li>
                    ))}
                  </ul>
                )}
              </div>
            </Card>
          ))}
        </div>
      </main>

      <footer className="border-t bg-background py-8">
        <div className="mx-auto flex max-w-4xl flex-col items-start justify-between gap-4 px-4 sm:flex-row sm:items-center sm:px-6">
          <p className="text-sm text-muted-foreground">{navCopy.footer.tagline}</p>

          <nav aria-label={navCopy.a11y.footerNav}>
            <ul className="flex flex-wrap items-center gap-x-5 gap-y-2 text-sm text-muted-foreground">
              <li>
                <Link to={publicRoutePath(locale, "landing")} className="hover:text-foreground">
                  Disblock
                </Link>
              </li>
              <li>
                <Link
                  to={publicRoutePath(locale, "legal")}
                  aria-current={kind === "legal" ? "page" : undefined}
                  className={cn(
                    "hover:text-foreground",
                    kind === "legal" &&
                      "font-semibold text-foreground underline underline-offset-4",
                  )}
                >
                  {navCopy.nav.legal}
                </Link>
              </li>
              <li>
                <Link
                  to={publicRoutePath(locale, "privacy")}
                  aria-current={kind === "privacy" ? "page" : undefined}
                  className={cn(
                    "hover:text-foreground",
                    kind === "privacy" &&
                      "font-semibold text-foreground underline underline-offset-4",
                  )}
                >
                  {navCopy.nav.privacy}
                </Link>
              </li>
              <li>
                <Link
                  to={publicRoutePath(locale, "terms")}
                  aria-current={kind === "terms" ? "page" : undefined}
                  className={cn(
                    "hover:text-foreground",
                    kind === "terms" &&
                      "font-semibold text-foreground underline underline-offset-4",
                  )}
                >
                  {navCopy.nav.terms}
                </Link>
              </li>
              {SUPPORTED_LOCALES.map((code) => (
                <li key={code}>
                  <Link
                    to={publicRoutePath(code, kind)}
                    aria-current={code === locale ? "page" : undefined}
                    className={cn(
                      "hover:text-foreground",
                      code === locale &&
                        "font-semibold text-foreground underline underline-offset-4",
                    )}
                  >
                    {LOCALE_META[code].shortLabel}
                  </Link>
                </li>
              ))}
            </ul>
          </nav>
        </div>
      </footer>
    </div>
  );
}
