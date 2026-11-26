import { useEffect, useId, useRef, useState } from "react";
import { ArrowRight, Check, Globe, Menu, ShieldCheck, X, Zap } from "lucide-react";
import { Link, useLocation, useNavigate } from "react-router";
import { LOGIN_URL } from "../api.ts";
import { DiscordIcon } from "../components/DiscordIcon.tsx";
import { LandingShowcase } from "../components/LandingVisuals.tsx";
import { Logo } from "../components/Logo.tsx";
import { Badge, Card } from "../components/ui.tsx";
import {
  LANDING_COPY,
  LOCALE_META,
  type Locale,
  SUPPORTED_LOCALES,
  isLocale,
  publicRoutePath,
} from "../data/landingData.ts";
import { syncDocumentSeo } from "../lib/seo.ts";
import { buttonClass, cn } from "../lib/styles.ts";

export interface LandingPageProps {
  locale: Locale;
  signedIn?: boolean;
  onOpenDashboard?: () => void;
}

export function LandingPage({ locale, signedIn = false, onOpenDashboard }: LandingPageProps) {
  const copy = LANDING_COPY[locale];
  const navigate = useNavigate();
  const location = useLocation();
  const menuId = useId();
  const langSelectId = useId();

  const headerRef = useRef<HTMLElement>(null);
  const innerRef = useRef<HTMLDivElement>(null);
  const menuRef = useRef<HTMLUListElement>(null);
  const [isWide, setIsWide] = useState(false);
  const cancelled = new URLSearchParams(location.search).get("login") === "cancelled";

  useEffect(() => {
    syncDocumentSeo(locale, "landing");
  }, [locale]);

  useEffect(() => {
    const header = headerRef.current;
    const inner = innerRef.current;
    const menu = menuRef.current;
    if (!header || !inner || !menu) {
      return;
    }

    let rafId = 0;
    let lastWidth = header.clientWidth;

    const measureLayout = (force: boolean) => {
      const currentWidth = header.clientWidth;
      const currentlyWide = header.classList.contains("is-wide");
      if (!force && !currentlyWide && currentWidth <= lastWidth) {
        lastWidth = currentWidth;
        return;
      }
      lastWidth = currentWidth;

      const wasOpen = !currentlyWide && menu.matches(":popover-open");
      if (wasOpen) {
        menu.hidePopover();
      }

      header.classList.add("is-measuring", "is-wide");
      menu.removeAttribute("popover");
      const fits = inner.scrollWidth <= inner.clientWidth;

      if (fits) {
        setIsWide(true);
      } else {
        header.classList.remove("is-wide");
        menu.setAttribute("popover", "auto");
        setIsWide(false);
        if (wasOpen) {
          try {
            menu.showPopover();
          } catch {
            // Ignore if browser cannot reopen popover during rapid resize.
          }
        }
      }

      cancelAnimationFrame(rafId);
      rafId = requestAnimationFrame(() => {
        header.classList.remove("is-measuring");
      });
    };

    measureLayout(true);

    const observer = new ResizeObserver(() => measureLayout(false));
    observer.observe(header);
    observer.observe(menu);

    return () => {
      cancelAnimationFrame(rafId);
      observer.disconnect();
    };
  }, [locale, signedIn]);

  const closePopoverIfOpen = () => {
    const menu = menuRef.current;
    if (!isWide && menu && menu.matches(":popover-open")) {
      menu.hidePopover();
    }
  };

  const onLocaleChange = (nextValue: string) => {
    if (!isLocale(nextValue) || nextValue === locale) {
      return;
    }
    closePopoverIfOpen();
    void navigate(LOCALE_META[nextValue].routePath);
  };

  const renderPrimaryAuthAction = (className: string, label: string) => {
    if (signedIn) {
      return onOpenDashboard ? (
        <button type="button" onClick={onOpenDashboard} className={className}>
          {copy.nav.openDashboard}
          <ArrowRight aria-hidden="true" />
        </button>
      ) : (
        <Link to="/" className={className}>
          {copy.nav.openDashboard}
          <ArrowRight aria-hidden="true" />
        </Link>
      );
    }

    return (
      <a href={LOGIN_URL} className={className}>
        <DiscordIcon />
        {label}
      </a>
    );
  };

  return (
    <div className="min-h-screen bg-background text-foreground">
      <header
        ref={headerRef}
        className={cn(
          "site-header sticky top-0 z-40 border-b bg-background/95 backdrop-blur-sm",
          isWide && "is-wide",
        )}
      >
        <a href="#main-heading" className="skip-link visually-hidden">
          {copy.a11y.skipToContent}
        </a>

        <div
          ref={innerRef}
          className="header-inner mx-auto flex max-w-6xl items-center justify-between gap-4 px-4 py-3 sm:px-6"
        >
          <Link to={LOCALE_META[locale].routePath} className="rounded-lg focus-visible:outline-2">
            <Logo />
          </Link>

          <nav className="site-nav flex items-center" aria-label={copy.a11y.primaryNav}>
            <button
              type="button"
              className={buttonClass("outline", "sm", "menu-button")}
              popoverTarget={menuId}
              popoverTargetAction="toggle"
              aria-controls={menuId}
            >
              <Menu className="size-4" aria-hidden="true" />
              <span>{copy.a11y.menuButton}</span>
            </button>

            <ul
              id={menuId}
              ref={menuRef}
              {...(isWide ? {} : ({ popover: "auto" } as Record<string, string>))}
              className="site-menu"
            >
              <li>
                <a
                  href="#how-it-works"
                  onClick={closePopoverIfOpen}
                  className="nav-link rounded-md px-3 py-2 text-sm font-medium text-muted-foreground transition-colors hover:bg-accent hover:text-foreground"
                >
                  {copy.nav.howItWorks}
                </a>
              </li>
              <li>
                <a
                  href="#blocks"
                  onClick={closePopoverIfOpen}
                  className="nav-link rounded-md px-3 py-2 text-sm font-medium text-muted-foreground transition-colors hover:bg-accent hover:text-foreground"
                >
                  {copy.nav.blocks}
                </a>
              </li>
              <li>
                <a
                  href="#pricing"
                  onClick={closePopoverIfOpen}
                  className="nav-link rounded-md px-3 py-2 text-sm font-medium text-muted-foreground transition-colors hover:bg-accent hover:text-foreground"
                >
                  {copy.nav.pricing}
                </a>
              </li>
              <li>
                <a
                  href="#faq"
                  onClick={closePopoverIfOpen}
                  className="nav-link rounded-md px-3 py-2 text-sm font-medium text-muted-foreground transition-colors hover:bg-accent hover:text-foreground"
                >
                  {copy.nav.faq}
                </a>
              </li>

              <li className="flex items-center">
                <label htmlFor={langSelectId} className="visually-hidden">
                  {copy.a11y.languageLabel}
                </label>
                <div className="relative flex items-center">
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
              </li>

              <li>
                {renderPrimaryAuthAction(
                  buttonClass("default", "sm", "w-full md:w-auto"),
                  copy.nav.signIn,
                )}
              </li>
            </ul>
          </nav>
        </div>
      </header>

      <main>
        {/* Hero */}
        <section className="border-b bg-muted/40">
          <div className="mx-auto max-w-6xl px-4 py-14 sm:px-6 sm:py-20">
            <div className="mx-auto max-w-3xl text-center">
              <Badge className="mb-5 border-border bg-background px-3 py-1 text-xs font-medium text-foreground shadow-xs">
                <span
                  className="mr-2 inline-block size-2 rounded-full bg-green-500"
                  aria-hidden="true"
                />
                {copy.hero.badge}
              </Badge>

              <h1
                id="main-heading"
                tabIndex={-1}
                className="text-4xl font-bold tracking-tight text-balance sm:text-5xl"
              >
                {copy.hero.title}
              </h1>

              <p className="mt-5 text-base leading-relaxed text-pretty text-muted-foreground sm:text-lg">
                {copy.hero.subtitle}
              </p>

              <div className="mt-8 flex flex-wrap items-center justify-center gap-3">
                {renderPrimaryAuthAction(buttonClass("default", "lg"), copy.hero.primaryCta)}
                <a href="#how-it-works" className={buttonClass("outline", "lg")}>
                  {copy.hero.secondaryCta}
                  <ArrowRight className="size-4" aria-hidden="true" />
                </a>
              </div>

              {cancelled && (
                <p
                  role="status"
                  className="mx-auto mt-4 inline-flex items-center gap-2 rounded-lg border border-amber-300 bg-amber-50 px-3.5 py-2 text-sm text-amber-900"
                >
                  <X className="size-4 shrink-0" aria-hidden="true" />
                  <span>{copy.hero.cancelledNotice}</span>
                </p>
              )}

              <ul className="mt-8 flex flex-wrap items-center justify-center gap-x-6 gap-y-2 text-sm text-muted-foreground">
                {copy.hero.trustPoints.map((point) => (
                  <li key={point} className="inline-flex items-center gap-1.5">
                    <Check className="size-4 text-green-600" aria-hidden="true" />
                    <span>{point}</span>
                  </li>
                ))}
              </ul>
            </div>

            <div className="mt-12">
              <LandingShowcase copy={copy.showcase} />
            </div>
          </div>
        </section>

        {/* How it works */}
        <section id="how-it-works" className="scroll-mt-20 py-16 sm:py-20">
          <div className="mx-auto max-w-6xl px-4 sm:px-6">
            <div className="max-w-2xl">
              <p className="text-sm font-semibold tracking-wider text-muted-foreground uppercase">
                {copy.howItWorks.eyebrow}
              </p>
              <h2 className="mt-2 text-3xl font-bold tracking-tight">{copy.howItWorks.title}</h2>
              <p className="mt-3 text-muted-foreground">{copy.howItWorks.subtitle}</p>
            </div>

            <ol className="mt-10 grid gap-6 md:grid-cols-3">
              {copy.howItWorks.steps.map((step) => (
                <li key={step.number}>
                  <Card className="h-full p-6">
                    <span className="inline-flex size-9 items-center justify-center rounded-lg bg-primary font-mono text-sm font-bold text-primary-foreground">
                      {step.number}
                    </span>
                    <h3 className="mt-4 text-lg font-semibold">{step.title}</h3>
                    <p className="mt-2 text-sm leading-relaxed text-muted-foreground">
                      {step.description}
                    </p>
                  </Card>
                </li>
              ))}
            </ol>
          </div>
        </section>

        {/* Capabilities */}
        <section id="blocks" className="scroll-mt-20 border-t bg-muted/40 py-16 sm:py-20">
          <div className="mx-auto max-w-6xl px-4 sm:px-6">
            <div className="max-w-2xl">
              <p className="text-sm font-semibold tracking-wider text-muted-foreground uppercase">
                {copy.capabilities.eyebrow}
              </p>
              <h2 className="mt-2 text-3xl font-bold tracking-tight">{copy.capabilities.title}</h2>
              <p className="mt-3 text-muted-foreground">{copy.capabilities.subtitle}</p>
            </div>

            <div className="mt-10 grid gap-6 md:grid-cols-2">
              {copy.capabilities.items.map((item) => (
                <Card key={item.category} className="flex flex-col justify-between p-6">
                  <div>
                    <div className="flex items-center gap-2.5">
                      <span
                        className="size-3 rounded-full"
                        style={{ backgroundColor: item.color }}
                        aria-hidden="true"
                      />
                      <span className="text-xs font-semibold tracking-wider text-muted-foreground uppercase">
                        {item.category}
                      </span>
                    </div>
                    <h3 className="mt-3 text-xl font-semibold">{item.title}</h3>
                    <p className="mt-2 text-sm leading-relaxed text-muted-foreground">
                      {item.description}
                    </p>
                  </div>
                  <ul className="mt-5 flex flex-wrap gap-2">
                    {item.tags.map((tag) => (
                      <li
                        key={tag}
                        className="rounded-md bg-secondary px-2.5 py-1 font-mono text-xs text-secondary-foreground"
                      >
                        {tag}
                      </li>
                    ))}
                  </ul>
                </Card>
              ))}
            </div>
          </div>
        </section>

        {/* Security & Reliability */}
        <section className="border-t py-16 sm:py-20">
          <div className="mx-auto max-w-6xl px-4 sm:px-6">
            <div className="max-w-2xl">
              <p className="inline-flex items-center gap-1.5 text-sm font-semibold tracking-wider text-muted-foreground uppercase">
                <ShieldCheck className="size-4" aria-hidden="true" />
                {copy.security.eyebrow}
              </p>
              <h2 className="mt-2 text-3xl font-bold tracking-tight">{copy.security.title}</h2>
              <p className="mt-3 text-muted-foreground">{copy.security.subtitle}</p>
            </div>

            <div className="mt-10 grid gap-6 sm:grid-cols-2">
              {copy.security.items.map((item) => (
                <Card key={item.title} className="p-6">
                  <h3 className="text-base font-semibold">{item.title}</h3>
                  <p className="mt-2 text-sm leading-relaxed text-muted-foreground">
                    {item.description}
                  </p>
                </Card>
              ))}
            </div>
          </div>
        </section>

        {/* Pricing */}
        <section id="pricing" className="scroll-mt-20 border-t bg-muted/40 py-16 sm:py-20">
          <div className="mx-auto max-w-6xl px-4 sm:px-6">
            <div className="mx-auto max-w-2xl text-center">
              <p className="text-sm font-semibold tracking-wider text-muted-foreground uppercase">
                {copy.pricing.eyebrow}
              </p>
              <h2 className="mt-2 text-3xl font-bold tracking-tight">{copy.pricing.title}</h2>
              <p className="mt-3 text-muted-foreground">{copy.pricing.subtitle}</p>
            </div>

            <div className="mx-auto mt-10 grid max-w-4xl gap-6 md:grid-cols-2">
              <Card className="flex flex-col justify-between border-2 border-primary p-6 sm:p-8">
                <div>
                  <div className="flex items-center justify-between gap-2">
                    <h3 className="text-xl font-bold">{copy.pricing.freePlan.name}</h3>
                    <Badge>{copy.pricing.freePlan.badge}</Badge>
                  </div>
                  <div className="mt-4 flex items-baseline gap-1.5">
                    <span className="text-4xl font-bold">{copy.pricing.freePlan.price}</span>
                    <span className="text-sm text-muted-foreground">
                      {copy.pricing.freePlan.period}
                    </span>
                  </div>
                  <p className="mt-3 text-sm text-muted-foreground">
                    {copy.pricing.freePlan.description}
                  </p>
                  <ul className="mt-6 space-y-2.5 text-sm">
                    {copy.pricing.freePlan.features.map((feature) => (
                      <li key={feature} className="flex items-start gap-2.5">
                        <Check
                          className="mt-0.5 size-4 shrink-0 text-green-600"
                          aria-hidden="true"
                        />
                        <span>{feature}</span>
                      </li>
                    ))}
                  </ul>
                </div>

                {renderPrimaryAuthAction(
                  buttonClass("default", "lg", "mt-8 w-full"),
                  copy.pricing.freePlan.cta,
                )}
              </Card>

              <Card className="flex flex-col justify-between p-6 sm:p-8">
                <div>
                  <div className="flex items-center justify-between gap-2">
                    <h3 className="text-xl font-bold">{copy.pricing.proPlan.name}</h3>
                    <Badge className="bg-secondary text-secondary-foreground">
                      {copy.pricing.proPlan.badge}
                    </Badge>
                  </div>
                  <div className="mt-4 flex items-baseline gap-1.5">
                    <span className="text-4xl font-bold">{copy.pricing.proPlan.price}</span>
                    <span className="text-sm text-muted-foreground">
                      {copy.pricing.proPlan.period}
                    </span>
                  </div>
                  <p className="mt-3 text-sm text-muted-foreground">
                    {copy.pricing.proPlan.description}
                  </p>
                  <ul className="mt-6 space-y-2.5 text-sm text-muted-foreground">
                    {copy.pricing.proPlan.features.map((feature) => (
                      <li key={feature} className="flex items-start gap-2.5">
                        <Zap className="mt-0.5 size-4 shrink-0 text-amber-600" aria-hidden="true" />
                        <span>{feature}</span>
                      </li>
                    ))}
                  </ul>
                </div>

                {renderPrimaryAuthAction(
                  buttonClass("outline", "lg", "mt-8 w-full"),
                  copy.pricing.proPlan.cta,
                )}
              </Card>
            </div>
          </div>
        </section>

        {/* FAQ */}
        <section
          id="faq"
          aria-labelledby="faq-heading"
          className="scroll-mt-20 border-t py-16 sm:py-20"
        >
          <div className="mx-auto max-w-3xl px-4 sm:px-6">
            <div className="text-center">
              <p className="text-sm font-semibold tracking-wider text-muted-foreground uppercase">
                {copy.faq.eyebrow}
              </p>
              <h2 id="faq-heading" className="mt-2 text-3xl font-bold tracking-tight">
                {copy.faq.title}
              </h2>
            </div>

            <div className="faq-accordion mt-10 space-y-3">
              {copy.faq.items.map((item) => (
                <details key={item.question} className="faq-item rounded-xl border bg-card">
                  <summary className="faq-summary px-5 py-4 text-left text-sm font-semibold sm:text-base">
                    <span>{item.question}</span>
                  </summary>
                  <div className="faq-content px-5 pb-4 text-sm leading-relaxed text-muted-foreground">
                    <p>{item.answer}</p>
                  </div>
                </details>
              ))}
            </div>
          </div>
        </section>

        {/* Final CTA */}
        <section className="border-t bg-muted/40 py-16">
          <div className="mx-auto max-w-3xl px-4 text-center sm:px-6">
            <h2 className="text-3xl font-bold tracking-tight">{copy.finalCta.title}</h2>
            <p className="mt-3 text-muted-foreground">{copy.finalCta.subtitle}</p>
            <div className="mt-7 flex justify-center">
              {renderPrimaryAuthAction(buttonClass("default", "lg"), copy.finalCta.cta)}
            </div>
          </div>
        </section>
      </main>

      <footer className="border-t bg-background py-10">
        <div className="mx-auto flex max-w-6xl flex-col items-start justify-between gap-6 px-4 sm:flex-row sm:items-center sm:px-6">
          <div>
            <Logo />
            <p className="mt-2 text-sm text-muted-foreground">{copy.footer.tagline}</p>
          </div>

          <nav aria-label={copy.a11y.footerNav}>
            <ul className="flex flex-wrap items-center gap-x-6 gap-y-2 text-sm text-muted-foreground">
              <li>
                <a href="#how-it-works" className="hover:text-foreground">
                  {copy.nav.howItWorks}
                </a>
              </li>
              <li>
                <a href="#blocks" className="hover:text-foreground">
                  {copy.nav.blocks}
                </a>
              </li>
              <li>
                <a href="#pricing" className="hover:text-foreground">
                  {copy.nav.pricing}
                </a>
              </li>
              <li>
                <a href="#faq" className="hover:text-foreground">
                  {copy.nav.faq}
                </a>
              </li>
              <li>
                <Link to={publicRoutePath(locale, "privacy")} className="hover:text-foreground">
                  {copy.nav.privacy}
                </Link>
              </li>
              <li>
                <Link to={publicRoutePath(locale, "terms")} className="hover:text-foreground">
                  {copy.nav.terms}
                </Link>
              </li>
              {SUPPORTED_LOCALES.map((code) => (
                <li key={code}>
                  <Link
                    to={publicRoutePath(code, "landing")}
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
