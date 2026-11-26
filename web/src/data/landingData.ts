export const DEFAULT_LOCALE = "en" as const;
export const SUPPORTED_LOCALES = ["en", "fr"] as const;

export type Locale = (typeof SUPPORTED_LOCALES)[number];
export type PublicPageKind = "landing" | "privacy" | "terms";

export const PUBLIC_PAGE_KINDS: readonly PublicPageKind[] = ["landing", "privacy", "terms"];

export interface LocaleMeta {
  code: Locale;
  language: string;
  region?: string;
  label: string;
  shortLabel: string;
  htmlLang: string;
  hreflang: string;
  ogLocale: string;
  /** Route prefix inside React Router ("" for default locale, "/fr" for French). */
  routePrefix: string;
  /** Landing route path inside React Router (without trailing slash except root). */
  routePath: string;
  /** Canonical pathname on disk / CDN (trailing slash on locale directories). */
  canonicalPath: string;
}

export const LOCALE_META: Record<Locale, LocaleMeta> = {
  en: {
    code: "en",
    language: "en",
    label: "English",
    shortLabel: "EN",
    htmlLang: "en",
    hreflang: "en",
    ogLocale: "en_US",
    routePrefix: "",
    routePath: "/",
    canonicalPath: "/",
  },
  fr: {
    code: "fr",
    language: "fr",
    region: "FR",
    label: "Français",
    shortLabel: "FR",
    htmlLang: "fr",
    hreflang: "fr",
    ogLocale: "fr_FR",
    routePrefix: "/fr",
    routePath: "/fr",
    canonicalPath: "/fr/",
  },
};

export function isLocale(value: string): value is Locale {
  return (SUPPORTED_LOCALES as readonly string[]).includes(value);
}

export function publicRoutePath(locale: Locale, page: PublicPageKind): string {
  const prefix = LOCALE_META[locale].routePrefix;
  if (page === "landing") {
    return prefix || "/";
  }
  return `${prefix}/${page}`;
}

export function publicCanonicalPath(locale: Locale, page: PublicPageKind): string {
  const prefix = LOCALE_META[locale].routePrefix;
  if (page === "landing") {
    return prefix ? `${prefix}/` : "/";
  }
  return `${prefix}/${page}/`;
}

export function publicRouteFromPath(
  pathname: string,
): { page: PublicPageKind; locale: Locale } | null {
  const segments = pathname.split("/").filter(Boolean);
  if (segments.length === 0) {
    return { page: "landing", locale: DEFAULT_LOCALE };
  }

  const first = segments[0] ?? "";
  if (isLocale(first) && first !== DEFAULT_LOCALE) {
    if (segments.length === 1) {
      return { page: "landing", locale: first };
    }
    if (segments.length === 2 && (segments[1] === "privacy" || segments[1] === "terms")) {
      return { page: segments[1], locale: first };
    }
    return null;
  }

  if (segments.length === 1 && (first === "privacy" || first === "terms")) {
    return { page: first, locale: DEFAULT_LOCALE };
  }

  return null;
}

export function localeFromPath(pathname: string): Locale {
  const parsed = publicRouteFromPath(pathname);
  if (parsed) {
    return parsed.locale;
  }
  const segment = pathname.split("/").filter(Boolean)[0];
  if (segment && isLocale(segment) && segment !== DEFAULT_LOCALE) {
    return segment;
  }
  return DEFAULT_LOCALE;
}

export function localeUrl(
  siteUrl: string,
  locale: Locale,
  page: PublicPageKind = "landing",
): string {
  const base = siteUrl.replace(/\/+$/, "");
  return `${base}${publicCanonicalPath(locale, page)}`;
}

export interface ShowcaseBlock {
  kind: "trigger" | "option" | "logic" | "variable" | "action" | "role" | "flow";
  label: string;
  detail: string;
  indent?: number;
}

export interface ShowcaseExample {
  id: string;
  tabLabel: string;
  summary: string;
  blocks: ShowcaseBlock[];
  discordUser: string;
  discordCommandOrEvent: string;
  discordBotName: string;
  discordReply: string;
  discordNote: string;
}

export interface CapabilityItem {
  category: string;
  color: string;
  title: string;
  description: string;
  tags: string[];
}

export interface FaqItem {
  question: string;
  answer: string;
}

export interface LegalDocumentCopy {
  seoTitle: string;
  seoDescription: string;
  title: string;
  updatedLabel: string;
  intro: string;
  sections: Array<{
    heading: string;
    paragraphs: string[];
    bullets?: string[];
  }>;
}

export interface LegalCopy {
  backToHome: string;
  privacy: LegalDocumentCopy;
  terms: LegalDocumentCopy;
}

export interface LandingCopy {
  seo: {
    title: string;
    description: string;
    ogImageAlt: string;
  };
  a11y: {
    skipToContent: string;
    primaryNav: string;
    menuButton: string;
    languageLabel: string;
    footerNav: string;
  };
  nav: {
    howItWorks: string;
    blocks: string;
    pricing: string;
    faq: string;
    privacy: string;
    terms: string;
    signIn: string;
    openDashboard: string;
  };
  hero: {
    badge: string;
    title: string;
    subtitle: string;
    primaryCta: string;
    dashboardCta: string;
    secondaryCta: string;
    cancelledNotice: string;
    trustPoints: string[];
  };
  showcase: {
    eyebrow: string;
    editorTitle: string;
    previewTitle: string;
    tabListLabel: string;
    examples: ShowcaseExample[];
  };
  howItWorks: {
    eyebrow: string;
    title: string;
    subtitle: string;
    steps: Array<{
      number: string;
      title: string;
      description: string;
    }>;
  };
  capabilities: {
    eyebrow: string;
    title: string;
    subtitle: string;
    items: CapabilityItem[];
  };
  security: {
    eyebrow: string;
    title: string;
    subtitle: string;
    items: Array<{
      title: string;
      description: string;
    }>;
  };
  pricing: {
    eyebrow: string;
    title: string;
    subtitle: string;
    freePlan: {
      name: string;
      badge: string;
      price: string;
      period: string;
      description: string;
      features: string[];
      cta: string;
    };
    proPlan: {
      name: string;
      badge: string;
      price: string;
      period: string;
      description: string;
      features: string[];
      cta: string;
    };
  };
  faq: {
    eyebrow: string;
    title: string;
    items: FaqItem[];
  };
  finalCta: {
    title: string;
    subtitle: string;
    cta: string;
  };
  footer: {
    tagline: string;
    copyright: string;
  };
}

export const LANDING_COPY: Record<Locale, LandingCopy> = {
  en: {
    seo: {
      title: "Disblock — Build Discord Bots with Visual Blocks (24/7 Hosted)",
      description:
        "Create custom Discord bots without writing code or managing servers. Snap triggers, conditions, roles, and persistent variables in visual blocks — Disblock hosts your bot 24/7.",
      ogImageAlt: "Disblock visual Discord bot builder and 24/7 hosting",
    },
    a11y: {
      skipToContent: "Skip to main content",
      primaryNav: "Primary",
      menuButton: "Menu",
      languageLabel: "Language",
      footerNav: "Footer",
    },
    nav: {
      howItWorks: "How it works",
      blocks: "Blocks",
      pricing: "Pricing",
      faq: "FAQ",
      privacy: "Privacy",
      terms: "Terms",
      signIn: "Sign in with Discord",
      openDashboard: "Open dashboard",
    },
    hero: {
      badge: "Free during beta • 24/7 managed hosting",
      title: "Build Discord bots with blocks. We host them 24/7.",
      subtitle:
        "No code, no VPS, no setup headaches. Connect your Discord bot, snap slash commands, conditions, roles, and persistent member variables together — and go live in seconds.",
      primaryCta: "Start building with Discord",
      dashboardCta: "Go to your bots",
      secondaryCta: "See how it works",
      cancelledNotice: "Discord sign-in was cancelled. You can try again whenever you are ready.",
      trustPoints: [
        "AES-256-GCM encrypted bot tokens",
        "Instant hot-swap updates without reconnecting",
        "Minimal Discord gateway intents requested",
      ],
    },
    showcase: {
      eyebrow: "Visual editor preview",
      editorTitle: "Block workspace",
      previewTitle: "Live Discord result",
      tabListLabel: "Example bot workflows",
      examples: [
        {
          id: "welcome",
          tabLabel: "Welcome & Role",
          summary:
            "Greet new members in a welcome channel and assign a starter role automatically.",
          blocks: [
            { kind: "trigger", label: "when a member joins", detail: "Server Members intent" },
            {
              kind: "action",
              label: "send",
              detail: '"Welcome {user} to the server!" in #welcome',
              indent: 1,
            },
            { kind: "role", label: "give role ID", detail: "Member", indent: 1 },
          ],
          discordUser: "@alex",
          discordCommandOrEvent: "just joined the server",
          discordBotName: "CommunityBot",
          discordReply: "Welcome @alex to the server! Role @Member added.",
          discordNote: "Runs automatically on every member join",
        },
        {
          id: "points",
          tabLabel: "Points & Levels",
          summary:
            "Reward active members with persistent per-member points and automatic milestones.",
          blocks: [
            { kind: "trigger", label: "when a message contains", detail: '"gg" (ignore case)' },
            {
              kind: "variable",
              label: "change member's points by",
              detail: "random 5 to 10",
              indent: 1,
            },
            {
              kind: "logic",
              label: "if",
              detail: "member's points ≥ 100",
              indent: 1,
            },
            {
              kind: "action",
              label: "reply",
              detail: '"Level up {user}! Total: {member:points} pts"',
              indent: 2,
            },
          ],
          discordUser: "@sam",
          discordCommandOrEvent: "gg team, great match!",
          discordBotName: "CommunityBot",
          discordReply: "Level up @sam! Total: 105 pts",
          discordNote: "Saved in Postgres per member across restarts",
        },
        {
          id: "slash",
          tabLabel: "Slash Commands",
          summary:
            "Register native Discord /commands with typed options and private ephemeral replies.",
          blocks: [
            { kind: "trigger", label: "when /hello is used", detail: '"Greet a member"' },
            {
              kind: "option",
              label: "option member",
              detail: "type: member · required",
              indent: 1,
            },
            {
              kind: "action",
              label: "reply",
              detail: '"Hey {option.member}, welcome from {user.name}!" (ephemeral)',
              indent: 1,
            },
          ],
          discordUser: "@maya",
          discordCommandOrEvent: "/hello member:@alex",
          discordBotName: "CommunityBot",
          discordReply: "Hey @alex, welcome from maya!",
          discordNote: "Slash commands sync to Discord on Save",
        },
      ],
    },
    howItWorks: {
      eyebrow: "How it works",
      title: "From zero to a live Discord bot in 3 steps",
      subtitle:
        "Everything runs in your browser and on Disblock's always-on Rust runtime. No terminal, no Node.js process to babysit.",
      steps: [
        {
          number: "01",
          title: "Connect your Discord bot",
          description:
            "Sign in with Discord, create an application in the Discord Developer Portal, and paste its bot token. Your token is encrypted with AES-256-GCM before storage.",
        },
        {
          number: "02",
          title: "Snap blocks together",
          description:
            "Combine triggers, command options, if/else conditions, role checks, math, delays, and saved per-member or per-server variables.",
        },
        {
          number: "03",
          title: "Click Save — it's live",
          description:
            "Saving pushes your slash commands straight to Discord and hot-swaps your program into the running bot in seconds without disconnecting it.",
        },
      ],
    },
    capabilities: {
      eyebrow: "What you can build",
      title: "Real logic, persistent memory, zero boilerplate",
      subtitle:
        "Unlike rigid preset bots, Disblock lets you shape exact rules for your server using visual blocks backed by a deterministic interpreter.",
      items: [
        {
          category: "Triggers",
          color: "#6366f1",
          title: "Slash commands & events",
          description:
            "React when members run /commands with typed options, join the server, or send messages matching text patterns.",
          tags: ["/slash_command", "member_join", "message_contains"],
        },
        {
          category: "Logic & Roles",
          color: "#0ea5e9",
          title: "Conditions & permission checks",
          description:
            "Branch flows with if/else, number and text comparisons, boolean operators (and/or/not), and role membership checks.",
          tags: ["if / else", "has role", "compare", "and / or / not"],
        },
        {
          category: "Variables",
          color: "#a855f7",
          title: "Persistent server & member state",
          description:
            "Store points, counters, or custom values scoped to a single run, a specific member, or the entire server.",
          tags: ["local", "member:points", "server:counter"],
        },
        {
          category: "Actions & Flow",
          color: "#22c55e",
          title: "Messages, reactions, roles & timing",
          description:
            "Send channel messages or ephemeral replies, add emoji reactions, grant or remove roles, roll random numbers, or wait between steps.",
          tags: ["reply", "send_message", "react", "add/remove role", "wait"],
        },
      ],
    },
    security: {
      eyebrow: "Built for safety & uptime",
      title: "Designed to keep your bot token and server safe",
      subtitle:
        "Running custom bots shouldn't mean risking leaked tokens or accidental @everyone pings.",
      items: [
        {
          title: "AES-256-GCM encrypted tokens",
          description:
            "Every bot token is sealed with authenticated encryption bound to its unique bot ID and never exposed back to the browser.",
        },
        {
          title: "Least-privilege Discord intents",
          description:
            "Disblock inspects your blocks and requests only the Discord gateway intents your bot actually uses, skipping unused events.",
        },
        {
          title: "Mention & validation guardrails",
          description:
            "User-supplied placeholders can never trigger @everyone or role pings, and incomplete blocks are flagged before saving.",
        },
        {
          title: "Live execution logs",
          description:
            "Inspect trigger runs, durations, and any runtime error directly from your bot's Activity dashboard.",
        },
      ],
    },
    pricing: {
      eyebrow: "Pricing",
      title: "Free while we build. Transparent as you grow.",
      subtitle:
        "Start hosting your community bots today for free during the beta — no credit card required.",
      freePlan: {
        name: "Beta Free",
        badge: "Available now",
        price: "€0",
        period: "/ month during beta",
        description: "Everything you need to build, host, and iterate on custom Discord bots.",
        features: [
          "Up to 5 hosted Discord bots per account",
          "24/7 managed runtime with hot-swap saves",
          "Slash commands, member joins & message triggers",
          "Logic blocks & persistent member/server variables",
          "Real-time execution activity log",
        ],
        cta: "Start free with Discord",
      },
      proPlan: {
        name: "Pro (per bot)",
        badge: "Coming soon",
        price: "€5–8",
        period: "/ bot / month (planned)",
        description:
          "Built for active communities that need advanced Discord components and scale.",
        features: [
          "Higher execution limits & priority reliability",
          "Rich embeds, buttons & select menus",
          "Scheduled tasks & custom bot status",
          "Ready-made community templates",
          'No "Made with Disblock" watermark',
        ],
        cta: "Join free beta first",
      },
    },
    faq: {
      eyebrow: "FAQ",
      title: "Frequently asked questions",
      items: [
        {
          question: "Do I need to know how to code or rent a VPS?",
          answer:
            "No. You assemble your bot visually using Disblock's block editor in your browser, and Disblock runs and hosts the bot 24/7 for you.",
        },
        {
          question: "Is my Discord bot token safe with Disblock?",
          answer:
            "Yes. Your bot token is encrypted at rest using AES-256-GCM bound to your bot's ID, used solely to connect your bot to Discord, and never sent back to the frontend.",
        },
        {
          question: "How quickly do changes go live after editing blocks?",
          answer:
            "Within seconds. When you click Save, Disblock validates your blocks, updates your slash commands on Discord, and swaps the new program into the running bot without disconnecting it.",
        },
        {
          question: "What happens if my bot already had slash commands?",
          answer:
            "Once connected, Disblock manages the bot's slash commands based on the triggers defined in your block workspace, keeping Discord in sync whenever you save.",
        },
        {
          question: "Can my bot remember points, levels, or counters per member?",
          answer:
            "Yes. Variables support three scopes: this run only (local), per member, and per server. Member and server variables persist across runs and can be used inside messages with {member:NAME} or {server:NAME}.",
        },
        {
          question: "How much does Disblock cost?",
          answer:
            "Hosting is currently free during the beta for up to 5 bots per account. A per-bot Pro tier is planned later for higher limits, rich embeds/buttons, scheduled tasks, and custom status.",
        },
      ],
    },
    finalCta: {
      title: "Ready to launch your Discord bot?",
      subtitle:
        "Sign in with Discord, connect your bot token, and ship your first command in minutes.",
      cta: "Sign in with Discord",
    },
    footer: {
      tagline: "Visual Discord bot builder & 24/7 managed hosting.",
      copyright: "Disblock",
    },
  },
  fr: {
    seo: {
      title: "Disblock — Créez des bots Discord en blocs visuels (Hébergés 24/7)",
      description:
        "Créez des bots Discord sur mesure sans coder ni gérer de serveur. Assemblez déclencheurs, conditions, rôles et variables persistantes en blocs visuels — Disblock héberge votre bot 24/7.",
      ogImageAlt: "Disblock — éditeur visuel de bots Discord et hébergement 24/7",
    },
    a11y: {
      skipToContent: "Aller au contenu principal",
      primaryNav: "Navigation principale",
      menuButton: "Menu",
      languageLabel: "Langue",
      footerNav: "Pied de page",
    },
    nav: {
      howItWorks: "Comment ça marche",
      blocks: "Blocs",
      pricing: "Tarifs",
      faq: "FAQ",
      privacy: "Confidentialité",
      terms: "Conditions",
      signIn: "Connexion avec Discord",
      openDashboard: "Ouvrir le tableau de bord",
    },
    hero: {
      badge: "Gratuit pendant la bêta • Hébergement géré 24/7",
      title: "Créez vos bots Discord en blocs. On les héberge 24/7.",
      subtitle:
        "Sans code, sans VPS et sans configuration serveur. Connectez votre bot Discord, assemblez commandes slash, conditions, rôles et variables persistantes — et publiez en quelques secondes.",
      primaryCta: "Commencer avec Discord",
      dashboardCta: "Accéder à mes bots",
      secondaryCta: "Voir comment ça marche",
      cancelledNotice:
        "La connexion Discord a été annulée. Vous pouvez réessayer quand vous voulez.",
      trustPoints: [
        "Tokens chiffrés en AES-256-GCM",
        "Mises à jour instantanées sans reconnexion",
        "Intents Discord minimaux demandés",
      ],
    },
    showcase: {
      eyebrow: "Aperçu de l'éditeur visuel",
      editorTitle: "Espace de blocs",
      previewTitle: "Rendu en direct sur Discord",
      tabListLabel: "Exemples de bots réalisables",
      examples: [
        {
          id: "welcome",
          tabLabel: "Bienvenue & Rôle",
          summary:
            "Accueillez les nouveaux membres dans un salon et attribuez un rôle automatiquement.",
          blocks: [
            { kind: "trigger", label: "quand un membre rejoint", detail: "Intent Server Members" },
            {
              kind: "action",
              label: "envoyer",
              detail: '"Bienvenue {user} sur le serveur !" dans #bienvenue',
              indent: 1,
            },
            { kind: "role", label: "donner le rôle ID", detail: "Membre", indent: 1 },
          ],
          discordUser: "@alex",
          discordCommandOrEvent: "vient de rejoindre le serveur",
          discordBotName: "CommunityBot",
          discordReply: "Bienvenue @alex sur le serveur ! Rôle @Membre ajouté.",
          discordNote: "S'exécute automatiquement à chaque arrivée",
        },
        {
          id: "points",
          tabLabel: "Points & Niveaux",
          summary:
            "Récompensez les membres actifs avec des points persistants et des paliers automatiques.",
          blocks: [
            { kind: "trigger", label: "quand un message contient", detail: '"gg" (sans casse)' },
            {
              kind: "variable",
              label: "modifier member's points de",
              detail: "aléatoire de 5 à 10",
              indent: 1,
            },
            {
              kind: "logic",
              label: "si",
              detail: "member's points ≥ 100",
              indent: 1,
            },
            {
              kind: "action",
              label: "répondre",
              detail: '"Bravo {user} ! Total : {member:points} pts"',
              indent: 2,
            },
          ],
          discordUser: "@sam",
          discordCommandOrEvent: "gg la team, belle partie !",
          discordBotName: "CommunityBot",
          discordReply: "Bravo @sam ! Total : 105 pts",
          discordNote: "Sauvegardé en base Postgres par membre",
        },
        {
          id: "slash",
          tabLabel: "Commandes Slash",
          summary:
            "Enregistrez des commandes /slash natives avec options typées et réponses privées.",
          blocks: [
            { kind: "trigger", label: "quand /hello est utilisé", detail: '"Saluer un membre"' },
            {
              kind: "option",
              label: "option member",
              detail: "type : membre · requis",
              indent: 1,
            },
            {
              kind: "action",
              label: "répondre",
              detail: '"Salut {option.member}, de la part de {user.name} !" (éphémère)',
              indent: 1,
            },
          ],
          discordUser: "@maya",
          discordCommandOrEvent: "/hello member:@alex",
          discordBotName: "CommunityBot",
          discordReply: "Salut @alex, de la part de maya !",
          discordNote: "Commandes slash synchronisées à chaque sauvegarde",
        },
      ],
    },
    howItWorks: {
      eyebrow: "Comment ça marche",
      title: "De zéro à un bot Discord en ligne en 3 étapes",
      subtitle:
        "Tout se configure dans votre navigateur et tourne sur le runtime Rust de Disblock. Aucun terminal ni serveur à administrer.",
      steps: [
        {
          number: "01",
          title: "Connectez votre bot Discord",
          description:
            "Connectez-vous avec Discord, créez une application sur le Discord Developer Portal et collez son token. Il est immédiatement chiffré en AES-256-GCM.",
        },
        {
          number: "02",
          title: "Assemblez vos blocs",
          description:
            "Combinez déclencheurs, options de commandes, conditions si/sinon, vérifications de rôles, calculs, pauses et variables par membre ou par serveur.",
        },
        {
          number: "03",
          title: "Sauvegardez — c'est en ligne",
          description:
            "La sauvegarde envoie vos commandes slash à Discord et met à jour votre bot à chaud en quelques secondes, sans déconnexion.",
        },
      ],
    },
    capabilities: {
      eyebrow: "Capacités",
      title: "Une vraie logique métier, avec mémoire persistante",
      subtitle:
        "Contrairement aux bots figés à options prédéfinies, Disblock vous laisse composer les règles exactes de votre communauté en blocs visuels.",
      items: [
        {
          category: "Déclencheurs",
          color: "#6366f1",
          title: "Commandes slash & événements",
          description:
            "Déclenchez des actions sur les /commandes avec options typées, l'arrivée d'un membre ou un mot-clé dans un message.",
          tags: ["/slash_command", "member_join", "message_contains"],
        },
        {
          category: "Logique & Rôles",
          color: "#0ea5e9",
          title: "Conditions & contrôles d'accès",
          description:
            "Branchez vos flux avec si/sinon, comparaisons numériques ou textuelles, opérateurs (et/ou/non) et vérification de rôle.",
          tags: ["si / sinon", "possède le rôle", "comparer", "et / ou / non"],
        },
        {
          category: "Variables",
          color: "#a855f7",
          title: "Données persistantes par membre ou serveur",
          description:
            "Mémorisez des points, compteurs ou états sur l'exécution courante, par membre ou pour tout le serveur.",
          tags: ["local", "member:points", "server:counter"],
        },
        {
          category: "Actions & Flux",
          color: "#22c55e",
          title: "Messages, réactions, rôles & délais",
          description:
            "Répondez en public ou en message éphémère, envoyez dans un salon, ajoutez des réactions, gérez les rôles et temporisez l'exécution.",
          tags: ["répondre", "envoyer", "réagir", "rôles", "attendre"],
        },
      ],
    },
    security: {
      eyebrow: "Sécurité & Fiabilité",
      title: "Conçu pour protéger votre token et votre serveur",
      subtitle:
        "Créer un bot personnalisé ne doit jamais exposer votre token ni risquer des mentions @everyone involontaires.",
      items: [
        {
          title: "Tokens chiffrés en AES-256-GCM",
          description:
            "Chaque token de bot est scellé avec un chiffrement authentifié lié à son identifiant unique et n'est jamais renvoyé au navigateur.",
        },
        {
          title: "Intents Discord au strict nécessaire",
          description:
            "Disblock analyse vos blocs pour ne demander que les intents Gateway indispensables et ignorer les événements inutilisés.",
        },
        {
          title: "Garde-fous & validation intégrée",
          description:
            "Les variables et textes saisis ne peuvent pas déclencher @everyone ou mentionner des rôles, et les blocs incomplets sont signalés avant sauvegarde.",
        },
        {
          title: "Historique d'exécution en direct",
          description:
            "Consultez chaque déclenchement, son temps d'exécution et les erreurs éventuelles depuis l'onglet Activité de votre bot.",
        },
      ],
    },
    pricing: {
      eyebrow: "Tarifs",
      title: "Gratuit pendant la bêta. Transparent pour la suite.",
      subtitle:
        "Lancez et hébergez vos bots communautaires gratuitement dès aujourd'hui — sans carte bancaire.",
      freePlan: {
        name: "Bêta Gratuite",
        badge: "Disponible",
        price: "0 €",
        period: "/ mois pendant la bêta",
        description: "Tout le nécessaire pour créer, héberger et ajuster vos bots Discord.",
        features: [
          "Jusqu'à 5 bots Discord hébergés par compte",
          "Hébergement 24/7 avec mise à jour à chaud",
          "Commandes slash, arrivées de membres et mots-clés",
          "Blocs logiques et variables persistantes membre/serveur",
          "Historique d'exécution en temps réel",
        ],
        cta: "Démarrer gratuitement avec Discord",
      },
      proPlan: {
        name: "Pro (par bot)",
        badge: "Bientôt disponible",
        price: "5–8 €",
        period: "/ bot / mois (prévu)",
        description:
          "Pensé pour les communautés actives qui veulent plus de composants et de volume.",
        features: [
          "Limites d'exécutions étendues & priorité",
          "Embeds riches, boutons et menus de sélection",
          "Tâches planifiées & statut personnalisé du bot",
          "Modèles prêts à l'emploi (tickets, modération…)",
          "Suppression de la mention « Made with Disblock »",
        ],
        cta: "Tester d'abord la bêta gratuite",
      },
    },
    faq: {
      eyebrow: "FAQ",
      title: "Questions fréquentes",
      items: [
        {
          question: "Faut-il savoir coder ou louer un VPS ?",
          answer:
            "Non. Vous assemblez le comportement de votre bot dans l'éditeur visuel depuis votre navigateur, et Disblock l'héberge en continu 24/7.",
        },
        {
          question: "Mon token de bot Discord est-il en sécurité ?",
          answer:
            "Oui. Votre token est chiffré au repos en AES-256-GCM avec liaison à l'ID du bot, utilisé uniquement pour connecter le bot à Discord et jamais renvoyé au navigateur.",
        },
        {
          question: "Combien de temps faut-il pour appliquer une modification ?",
          answer:
            "Quelques secondes. À la sauvegarde, Disblock valide vos blocs, met à jour les commandes slash sur Discord et remplace le programme en cours d'exécution sans déconnecter le bot.",
        },
        {
          question: "Que deviennent les commandes slash existantes de mon bot ?",
          answer:
            "Une fois connecté, Disblock gère les commandes slash de votre bot à partir des déclencheurs présents dans votre espace de blocs afin de garder Discord synchronisé.",
        },
        {
          question: "Mon bot peut-il mémoriser des points, niveaux ou compteurs ?",
          answer:
            "Oui. Les variables proposent trois portées : exécution courante (local), par membre et par serveur. Les valeurs par membre et par serveur sont conservées entre les exécutions et utilisables via {member:NOM} ou {server:NOM}.",
        },
        {
          question: "Combien coûte Disblock ?",
          answer:
            "L'hébergement est gratuit pendant toute la bêta (jusqu'à 5 bots par compte). Un abonnement Pro par bot est prévu par la suite pour les embeds, boutons, tâches planifiées et plafonds étendus.",
        },
      ],
    },
    finalCta: {
      title: "Prêt à lancer votre bot Discord ?",
      subtitle:
        "Connectez-vous avec Discord, liez votre token et publiez votre première commande en quelques minutes.",
      cta: "Connexion avec Discord",
    },
    footer: {
      tagline: "Éditeur visuel de bots Discord et hébergement géré 24/7.",
      copyright: "Disblock",
    },
  },
};

export const LEGAL_COPY: Record<Locale, LegalCopy> = {
  en: {
    backToHome: "Back to Disblock",
    privacy: {
      seoTitle: "Privacy Policy — Disblock",
      seoDescription:
        "How Disblock handles Discord sign-in data, encrypted bot tokens, execution logs, and bot variables.",
      title: "Privacy Policy",
      updatedLabel: "Beta notice — Last updated November 26, 2026",
      intro:
        "Disblock stores only the minimum data needed to authenticate your Discord account, host your Discord bots, and execute the block programs you configure.",
      sections: [
        {
          heading: "1. Data we collect and store",
          paragraphs: [
            "When you use Disblock, we process only the data required to run the service:",
          ],
          bullets: [
            "Discord account identity (identify scope only): your Discord user ID, username, and avatar hash.",
            "Session authentication: an HTTP-only session cookie (disblock_session) valid for 30 days. Only a SHA-256 hash of the session token is stored in our database.",
            "Bot configuration: your bot's Discord application ID, name, avatar, block editor workspace, compiled program, and your bot token encrypted with AES-256-GCM bound to the bot ID.",
            "Bot runtime state: execution history entries (trigger name, success/error status, duration) and any persistent member or server variables written by your bot's blocks.",
          ],
        },
        {
          heading: "2. How bot tokens and Discord events are handled",
          paragraphs: [
            "Bot tokens are encrypted at rest with AES-256-GCM and are never returned to the browser after submission. Disblock connects to Discord using only the gateway intents required by your saved blocks and ignores events your bot does not handle.",
          ],
        },
        {
          heading: "3. Retention and deletion",
          paragraphs: [
            "Expired sessions and bot execution history older than 30 days are automatically deleted. Removing a bot from its Settings page immediately stops its runtime connection, removes its slash commands from Discord, and deletes the bot record, its execution history, and its stored variables from our database.",
          ],
        },
        {
          heading: "4. Hosting & third-party processors",
          paragraphs: [
            "Disblock runs its web interface on Cloudflare, its API and bot gateway runtime on Fly.io (EU region), and its PostgreSQL database on Neon, exchanging data with the Discord API strictly to authenticate you and run your bots.",
          ],
        },
      ],
    },
    terms: {
      seoTitle: "Terms of Service — Disblock",
      seoDescription:
        "Terms governing your use of Disblock's visual Discord bot builder and 24/7 beta hosting.",
      title: "Terms of Service",
      updatedLabel: "Beta notice — Last updated November 26, 2026",
      intro:
        "By signing in to Disblock and connecting a Discord bot, you agree to the terms below covering the beta service.",
      sections: [
        {
          heading: "1. Service scope and beta status",
          paragraphs: [
            "Disblock is currently provided in beta. Hosting is free while we build, subject to per-account beta limits (such as the current limit of up to 5 bots per user). Features, limits, and paid plans may evolve before commercial launch.",
          ],
        },
        {
          heading: "2. Your responsibilities and Discord compliance",
          paragraphs: [
            "You are solely responsible for the bots you connect and the block programs you publish. You must comply with Discord's Terms of Service and Developer Policy, and only connect bot applications you own or are authorized to manage.",
          ],
          bullets: [
            "Do not use Disblock for spam, automated abuse, harassment, or unauthorized data collection.",
            "Do not attempt to bypass rate limits, security boundaries, or runtime resource protections.",
          ],
        },
        {
          heading: "3. Slash command management",
          paragraphs: [
            "When you connect a bot token and save blocks, Disblock manages that bot's registered slash commands on Discord to match your block workspace. Removing the bot from Disblock clears those Disblock-managed slash commands.",
          ],
        },
        {
          heading: "4. Availability and termination",
          paragraphs: [
            "We aim for continuous 24/7 operation, but beta hosting is provided as-is without a formal SLA. We may suspend bots or accounts that violate these terms or threaten platform stability, and you can disconnect and remove your bots at any time.",
          ],
        },
      ],
    },
  },
  fr: {
    backToHome: "Retour à Disblock",
    privacy: {
      seoTitle: "Politique de confidentialité — Disblock",
      seoDescription:
        "Comment Disblock traite les données de connexion Discord, les tokens chiffrés, les journaux d'exécution et les variables des bots.",
      title: "Politique de confidentialité",
      updatedLabel: "Version bêta — Dernière mise à jour le 26 novembre 2026",
      intro:
        "Disblock ne conserve que les données strictement nécessaires pour authentifier votre compte Discord, héberger vos bots et exécuter les programmes que vous assemblez en blocs.",
      sections: [
        {
          heading: "1. Données collectées et stockées",
          paragraphs: [
            "Lors de l'utilisation de Disblock, seules les informations indispensables au fonctionnement du service sont traitées :",
          ],
          bullets: [
            "Identité Discord (scope identify uniquement) : identifiant Discord, nom d'utilisateur et hash d'avatar.",
            "Session d'authentification : cookie HTTP-only (disblock_session) valable 30 jours. Seule l'empreinte SHA-256 du jeton de session est enregistrée en base.",
            "Configuration du bot : identifiant d'application Discord, nom, avatar, espace de blocs, programme compilé et token du bot chiffré en AES-256-GCM lié à l'identifiant du bot.",
            "État d'exécution : journaux d'activité (déclencheur, succès/erreur, durée) et variables persistantes par membre ou par serveur enregistrées par vos blocs.",
          ],
        },
        {
          heading: "2. Protection des tokens et événements Discord",
          paragraphs: [
            "Votre token de bot est chiffré au repos en AES-256-GCM et n'est jamais renvoyé au navigateur après saisie. Disblock ne demande à Discord que les intents Gateway nécessaires à vos blocs et ignore les événements inutilisés.",
          ],
        },
        {
          heading: "3. Durée de conservation et suppression",
          paragraphs: [
            "Les sessions expirées et l'historique d'exécution de plus de 30 jours sont supprimés automatiquement. La suppression d'un bot depuis ses paramètres déconnecte immédiatement le bot, supprime ses commandes slash sur Discord et efface le bot, ses journaux et ses variables de notre base.",
          ],
        },
        {
          heading: "4. Hébergement et sous-traitants techniques",
          paragraphs: [
            "L'interface web est servie via Cloudflare, l'API et le runtime des bots tournent sur Fly.io (région EU) et la base PostgreSQL est hébergée sur Neon, en interaction directe avec l'API Discord pour l'authentification et l'exécution de vos bots.",
          ],
        },
      ],
    },
    terms: {
      seoTitle: "Conditions d'utilisation — Disblock",
      seoDescription:
        "Conditions applicables à l'utilisation de l'éditeur visuel de bots Discord Disblock et de l'hébergement bêta 24/7.",
      title: "Conditions d'utilisation",
      updatedLabel: "Version bêta — Dernière mise à jour le 26 novembre 2026",
      intro:
        "En vous connectant à Disblock et en reliant un bot Discord, vous acceptez les présentes conditions liées au service en bêta.",
      sections: [
        {
          heading: "1. Périmètre du service et phase bêta",
          paragraphs: [
            "Disblock est actuellement proposé en version bêta. L'hébergement est gratuit pendant cette phase dans la limite des plafonds bêta (actuellement jusqu'à 5 bots par compte). Les fonctionnalités, limites et offres payantes pourront évoluer avant le lancement commercial.",
          ],
        },
        {
          heading: "2. Responsabilités et respect des règles Discord",
          paragraphs: [
            "Vous restez seul responsable des bots connectés et des programmes que vous configurez. Vous vous engagez à respecter les Conditions d'utilisation et la Politique développeur de Discord, et à ne connecter que des applications dont vous êtes propriétaire ou administrateur autorisé.",
          ],
          bullets: [
            "N'utilisez pas Disblock pour du spam, du harcèlement, des abus automatisés ou une collecte de données non autorisée.",
            "Ne tentez pas de contourner les limites de débit, les protections de sécurité ou l'isolation du runtime.",
          ],
        },
        {
          heading: "3. Gestion des commandes slash",
          paragraphs: [
            "Dès qu'un token est connecté et sauvegardé, Disblock synchronise les commandes slash du bot sur Discord selon les déclencheurs de votre espace de blocs. Supprimer le bot depuis Disblock efface ces commandes slash sur Discord.",
          ],
        },
        {
          heading: "4. Disponibilité et résiliation",
          paragraphs: [
            "Nous visons une disponibilité 24/7, mais l'hébergement bêta est fourni en l'état sans engagement SLA formel. Nous pouvons suspendre tout bot ou compte en infraction avec ces conditions, et vous pouvez déconnecter et supprimer vos bots à tout moment.",
          ],
        },
      ],
    },
  },
};

export function buildJsonLd(
  siteUrl: string,
  locale: Locale,
  page: PublicPageKind = "landing",
): Record<string, unknown> {
  const url = localeUrl(siteUrl, locale, page);

  if (page !== "landing") {
    const doc = LEGAL_COPY[locale][page];
    return {
      "@context": "https://schema.org",
      "@type": "WebPage",
      name: doc.seoTitle,
      description: doc.seoDescription,
      inLanguage: LOCALE_META[locale].htmlLang,
      url,
    };
  }

  const copy = LANDING_COPY[locale];

  return {
    "@context": "https://schema.org",
    "@graph": [
      {
        "@type": "SoftwareApplication",
        name: "Disblock",
        applicationCategory: "DeveloperApplication",
        operatingSystem: "Web",
        inLanguage: LOCALE_META[locale].htmlLang,
        url,
        description: copy.seo.description,
        offers: {
          "@type": "Offer",
          price: "0",
          priceCurrency: "EUR",
          availability: "https://schema.org/InStock",
        },
      },
      {
        "@type": "FAQPage",
        inLanguage: LOCALE_META[locale].htmlLang,
        mainEntity: copy.faq.items.map((item) => ({
          "@type": "Question",
          name: item.question,
          acceptedAnswer: {
            "@type": "Answer",
            text: item.answer,
          },
        })),
      },
    ],
  };
}
