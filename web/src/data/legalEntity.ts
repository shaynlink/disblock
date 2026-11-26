/**
 * Replaced at build time with the publisher's real legal identity. The
 * fallbacks deliberately make missing production information visible instead
 * of inventing an address, registration number, or consumer mediator.
 */
function required(value: string | undefined): string {
  return value?.trim() || "À compléter avant mise en production";
}

export const LEGAL_ENTITY = {
  publisher: required(import.meta.env.VITE_LEGAL_PUBLISHER),
  legalForm: required(import.meta.env.VITE_LEGAL_FORM),
  registeredAddress: required(import.meta.env.VITE_LEGAL_ADDRESS),
  registration: required(import.meta.env.VITE_LEGAL_REGISTRATION),
  vatNumber: required(import.meta.env.VITE_LEGAL_VAT),
  contactEmail: required(import.meta.env.VITE_LEGAL_EMAIL),
  phone: required(import.meta.env.VITE_LEGAL_PHONE),
  publicationDirector: required(import.meta.env.VITE_LEGAL_PUBLICATION_DIRECTOR),
  consumerMediator: required(import.meta.env.VITE_LEGAL_CONSUMER_MEDIATOR),
} as const;

export const LEGAL_HOSTS = [
  "Cloudflare, Inc. — web delivery",
  "Fly.io, Inc. — API and bot runtime",
  "Neon, Inc. — PostgreSQL database",
] as const;
