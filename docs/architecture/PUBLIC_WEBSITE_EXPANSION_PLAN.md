# Oteryn Public Website Expansion Plan

## Status

**PROPOSED — architecture and phased delivery plan only.**

This document defines the planned expansion of the Oteryn public website and its administrator-facing content workflows. It does not claim that the described capabilities are implemented.

RubinOT is used only as product inspiration for information architecture, content discoverability and portal completeness. Do not copy its code, text, branding, icons, artwork, layouts or proprietary data.

The dedicated Wiki architecture is defined separately in `docs/architecture/WIKI_IMPLEMENTATION_PLAN.md`. Wiki is one workstream inside the wider public-site programme, not the entire programme.

## Goal

Turn the current Oteryn website into a complete MMORPG portal that helps a visitor:

- understand what Oteryn is;
- see whether the world is available;
- create and secure an account;
- download and configure the client;
- learn how to begin playing;
- discover news and events;
- inspect public game information and statistics;
- find rules, legal information and support;
- enter the Oteryn Wiki;
- join the community.

The programme must preserve the existing Laravel modular monolith, Platform-owned Identity, deny-by-default RBAC, confirmed-MFA administrator access, administrator audit and read-only legacy engine integration boundaries.

## Current reusable capabilities

The current Platform already provides reusable foundations for:

- public Home, News, Online, Highscores and Servers surfaces;
- exact-name character search and character profiles;
- guild detail;
- Platform registration, login, password recovery and MFA;
- account overview and character creation;
- published news and managed pages;
- CMS administration;
- explicit administrator RBAC and audit;
- read-only public game-data queries;
- a dark-fantasy homepage comparison template under the existing design-preview route.

The expansion should extend these capabilities rather than build a second website or duplicate authentication and authorization policy.

## Product scope

### Launch-priority public surfaces

1. Production homepage.
2. Dynamic world status and online count.
3. Latest news preview.
4. Important-announcement ticker.
5. Client download centre.
6. Installation and first-login guide.
7. Server information and rates.
8. Events schedule and event detail.
9. Wiki entry point.
10. Discord/community links.
11. Support and bug-report guidance.
12. Game rules and legal documents.
13. Complete responsive header and footer.
14. English and Polish content architecture.

### Post-launch public surfaces

1. Latest deaths.
2. Kill statistics.
3. Guild index.
4. Team page.
5. Polls.
6. Public service-status and maintenance history.
7. Boosted creature and boosted boss, only if an authoritative game/runtime contract exists.
8. Catalogues of custom game systems.
9. Structured quest, monster, boss and item catalogues.
10. Interactive map, only as a separate later programme.

### Explicitly deferred commercial surfaces

The following are not part of the initial public-site programme:

- webshop;
- premium coins;
- payments;
- character bazaar;
- account or world transfers;
- VIP or loyalty sales;
- Battle Pass;
- promotions, affiliate or influencer systems.

These require the separately deferred Payments programme, transaction ledger, refund/chargeback handling, fraud controls and explicit legacy engine fulfilment contracts.

## Information architecture

### Primary navigation

Recommended desktop navigation:

```text
Home
News
Game
  Online
  Highscores
  Characters
  Guilds
  Servers
  Latest Deaths          [later]
  Kill Statistics        [later]
Learn
  Beginner's Guide
  Server Information
  Wiki
  Rules
Community
  Events
  Discord
  Team                    [later]
Support
Download
Account / Sign in
```

Mobile navigation must use an accessible disclosure menu and preserve account, Download and Play/Start actions without horizontal overflow.

### Footer

The footer should expose durable routes rather than repeat the entire main menu:

```text
Game
- Online
- Highscores
- Servers
- Characters

Learn
- Download
- Beginner's Guide
- Server Information
- Wiki

Community
- News
- Events
- Discord

Support and legal
- Support
- Report a Bug
- Game Rules
- Terms of Service
- Privacy Policy
- Cookie Policy
```

The footer should also show copyright, current site language and an explicit statement that game and service status data can become temporarily unavailable rather than displaying fabricated values.

## Architectural decision

Build the expanded portal inside the existing Oteryn Platform Laravel modular monolith.

Do not deploy a separate public frontend, WordPress installation or second CMS for this scope.

### Module boundaries

#### PublicPortal presentation module

Owns:

- homepage composition;
- shared public navigation and footer;
- public view models that combine already-authorized data from other modules;
- public-site feature flags and route discoverability;
- presentation of explicit unavailable/empty/error states.

Must not own:

- authentication policy;
- Legacy engine queries;
- CMS persistence;
- payment state;
- game-runtime mutations.

PublicPortal is an orchestration/presentation boundary. It should not become a generic domain dumping ground.

#### CMS extension

Existing CMS continues to own:

- news;
- managed informational pages;
- publication state;
- content editing behind exact permissions;
- administrator audit.

CMS may be extended with explicit typed content for:

- important announcements;
- legal documents;
- Beginner's Guide pages;
- Server Information pages;
- support and team informational content.

Do not encode event schedules, client binaries or runtime game statistics as arbitrary managed-page text when dedicated models and validation are required.

#### PublicGameData

Existing PublicGameData continues to own read-only game/runtime projections:

- world/server status;
- online count and online list;
- characters;
- guilds;
- highscores;
- later latest-death and kill-statistic projections after schema and privacy contracts are verified.

Homepage and other portal pages consume this module through explicit query/view-model interfaces. They must not perform new raw legacy engine queries directly from Blade templates or homepage controllers.

#### Downloads module

A new Downloads module should own:

- supported client release records;
- operating-system variants;
- version and release notes;
- immutable artifact URL or approved storage reference;
- file size;
- SHA-256 checksum;
- minimum/supported OS details;
- publication state;
- optional mandatory-update flag;
- installation instructions association.

The Platform should not accept arbitrary public executable uploads in the first slice. Initial releases should reference immutable, operator-approved artifacts. Direct binary upload requires a separate threat model and storage/signing plan.

#### Events module

A new Events module should own:

- event title and slug;
- localized summary and body;
- start and end times;
- timezone policy;
- status: draft, scheduled, active, completed, cancelled;
- optional registration or external link;
- optional associated news post;
- optional image reference after safe media support exists;
- featured state;
- publication and audit lifecycle.

Events must not be inferred from free-form news text.

#### Support content boundary

Initial Support should be content-first:

- troubleshooting;
- known issues;
- account-security guidance;
- bug-report instructions;
- contact and Discord escalation routes.

A stored support-ticket system is not required for the first release. If later introduced, it must become a dedicated module with privacy, retention, abuse, attachment and staff-authorization rules.

#### Wiki

Wiki is a dedicated module defined by `WIKI_IMPLEMENTATION_PLAN.md` and integrates into the shared navigation and homepage when its public release criteria are satisfied.

#### Polls

Polls are a later bounded module. They require authenticated or anonymous voting policy, rate limiting, duplicate-vote controls, result-visibility rules and administrator audit. Do not implement them as an unprotected generic form.

## Homepage architecture

### Homepage composition

The production homepage should be composed from independent blocks so one failed dependency does not blank the entire page:

```text
PublicHomeController
  -> HomePageQuery
       -> CMS latest published news
       -> CMS active announcements
       -> PublicGameData world summary
       -> Events active/upcoming summary
       -> configured community/support links
       -> enabled feature/route registry
  -> HomePageViewModel
  -> Blade view
```

Each dependency result should carry an explicit state:

```text
AVAILABLE
EMPTY
UNAVAILABLE
STALE
```

The UI must distinguish:

- zero players online;
- runtime status temporarily unavailable;
- no scheduled event;
- event service unavailable;
- no published news.

Do not present dependency failure as `0 online` or `offline` unless that state is authoritative.

### Homepage blocks

Recommended launch layout:

1. Hero with Oteryn identity and primary actions.
2. Character search.
3. World status card.
4. Latest news preview.
5. Active announcement ticker.
6. Upcoming event card.
7. Quick-start cards: Download, Beginner's Guide, Wiki, Discord.
8. Public-world links: Online, Highscores, Servers.
9. Support and rules callout.
10. Complete footer.

### World status card

Display only verified fields:

- world name;
- Online, Maintenance, Offline or Data unavailable;
- current online count;
- world/region;
- PvP type;
- server-save time;
- optional online record if a trustworthy source exists;
- data freshness timestamp.

Do not invent uptime, record or region data that is not represented by an explicit configuration or read model.

### Boosted creature/boss

Keep this component behind a feature flag until all are proven:

- authoritative source;
- refresh semantics;
- icon rights/storage;
- unavailable behavior;
- exact bonus description.

A static manually edited card must not impersonate live game state.

## Proposed public routes

### Existing routes retained

```text
/
/news
/news/{slug}
/online
/highscores
/servers
/characters
/characters/{name}
/guilds/{name}
/account
```

### Launch additions

```text
/download
/download/{platform?}
/getting-started
/server-information
/events
/events/{slug}
/wiki
/support
/support/report-a-bug
/rules
/legal/terms
/legal/privacy
/legal/cookies
```

Where a page is purely editorial in the first version, a stable named route may resolve to a published managed page through a typed route binding rather than exposing `/pages/{slug}` as the primary user-facing URL.

### Later additions

```text
/deaths
/kill-statistics
/guilds
/team
/polls
/status
```

## Administrator routes and permissions

### Proposed exact permissions

```text
portal.access
portal.announcements.manage
portal.settings.manage
downloads.manage
events.manage
events.publish
support.content.manage
```

Wiki permissions remain defined by the dedicated Wiki plan.

Do not add wildcard permissions. Existing role bundles receive new permissions only through explicit reviewed changes.

### Proposed administrator surfaces

```text
/admin/portal
/admin/announcements
/admin/downloads
/admin/events
/admin/support-content
/admin/site-settings
```

All privileged routes require:

```text
auth
+ mfa.confirmed
+ exact admin.permission
```

Every state-changing operation must be CSRF protected, server-side validated and audited with bounded non-secret metadata.

## Proposed persistence

### `site_announcements`

Suggested fields:

- `id`;
- `title`;
- `body`;
- `severity`: info, maintenance, warning;
- `starts_at`;
- `ends_at`;
- `is_published`;
- optional internal-link route or approved external URL;
- `created_by`, `updated_by`, `published_by`;
- timestamps and optimistic version column if concurrent editing is supported.

Only currently active, published announcements are shown publicly.

### `client_releases`

Suggested fields:

- `id`;
- `version`;
- `channel`: stable, beta;
- `published_at`;
- `is_current`;
- `minimum_supported_version` when required;
- localized release notes reference;
- author/publisher references;
- timestamps.

### `client_release_artifacts`

Suggested fields:

- `client_release_id`;
- `platform`: windows, linux, macos if supported;
- `architecture`;
- approved immutable artifact URL/storage reference;
- `filename`;
- `size_bytes`;
- `sha256`;
- optional signature reference;
- `is_enabled`.

Public download output must show version, platform, size and SHA-256.

### `events`

Suggested fields:

- `id`;
- `status`;
- `starts_at`, `ends_at`;
- timezone normalized to UTC in persistence;
- `featured`;
- `news_post_id` optional;
- approved internal/external action URL optional;
- author/publisher references;
- timestamps and lock version.

### `event_translations`

Suggested fields:

- `event_id`;
- `locale`;
- `title`;
- `slug`;
- `summary`;
- safe content body;
- SEO fields.

Localized slugs are unique per locale.

### Site links and settings

Security-sensitive and deployment-specific values should remain configuration, not editor-controlled data:

- Discord invite allowlist/domain;
- client artifact storage domain;
- support mailbox destination;
- canonical hostname;
- default locale;
- server-save timezone.

A small audited `site_settings` model may be introduced only for explicitly safe presentation values. Do not create an unrestricted key/value settings table that can alter security, URLs or executable behaviour.

## Content strategy

### Managed editorial pages needed for launch

1. About Oteryn.
2. Beginner's Guide.
3. Installation.
4. First Login.
5. Server Information.
6. Rates.
7. PvP Rules.
8. Game Rules.
9. Naming Rules.
10. Prohibited Software.
11. Account Security.
12. MFA Guide.
13. FAQ.
14. Known Issues.
15. Report a Bug.
16. Terms of Service.
17. Privacy Policy.
18. Cookie Policy.
19. Contact and Support.

Published legal documents should record an effective date and version. Policy changes should not silently overwrite historical meaning without revision or archival evidence.

## Localization

Initial languages:

- English: `en`;
- Polish: `pl`.

Architecture rules:

- stable locale-aware URLs;
- explicit fallback policy;
- no automatic publication of a missing or outdated translation;
- `hreflang` output;
- language switcher preserves the equivalent route where possible;
- dates and numbers formatted for the selected locale;
- editorial status indicates missing and stale translations.

Implementation may begin single-language only if the schema and URL design remain migration-safe for PL/EN and the temporary limitation is stated truthfully.

## Search

The main public-site search may eventually cover:

- news;
- managed pages;
- events;
- Wiki.

Character search remains a separate exact-name game-data function.

Do not merge both into one ambiguous field in the first release. The homepage character search should remain clearly labeled. Wiki provides its own content search until a later federated-search programme is approved.

## Security requirements

### Public content

- escape output by default;
- no arbitrary raw HTML;
- sanitize any later maintained rich-text format with an allowlist;
- validate internal and external links;
- do not expose draft or scheduled content before publication time;
- rate-limit abuse-prone search and support endpoints;
- preserve explicit dependency-failure semantics.

### Downloads

- show SHA-256 for each artifact;
- reference immutable approved artifacts;
- do not proxy arbitrary user-supplied URLs;
- restrict allowed artifact hosts;
- add `Content-Disposition` and content-type controls if Platform serves files;
- do not allow editor upload of executable files without a separately approved release-signing and storage threat model.

### Administrator actions

- exact permission;
- confirmed MFA;
- CSRF protection;
- validated state transition;
- bounded audit metadata;
- no article bodies, credentials or secrets in audit events;
- optimistic locking for concurrent content edits where needed.

### External links

Discord, social-media and download links must come from trusted configuration or tightly validated records. Prevent `javascript:`, data URLs and open redirects.

## UX and visual requirements

### Shared design

- use the Oteryn dark-fantasy visual language;
- prioritize readable body content and clear hierarchy;
- use decorative art as framing, not as a contrast-reducing background behind long text;
- use reusable cards, status badges, buttons, breadcrumbs, empty states and notices;
- preserve visible keyboard focus;
- support reduced motion;
- meet responsive requirements at desktop, tablet and mobile widths.

### Homepage visual acceptance

Verify at minimum:

- authenticated and guest headers;
- world available, empty, stale and unavailable states;
- no-news state;
- no-event state;
- ticker with short and long text;
- character-search validation and not-found state;
- mobile menu;
- footer wrapping;
- loading-free server-rendered baseline;
- no horizontal overflow.

### Content-page visual acceptance

Verify:

- short and very long pages;
- tables;
- lists;
- callouts;
- breadcrumbs;
- translated titles;
- 404 and unpublished states;
- footer and navigation at all supported widths.

## SEO and discoverability

- canonical URL for every public page;
- localized `hreflang`;
- unique title and description;
- Open Graph metadata;
- sitemap coverage for published public routes;
- robots exclusion for admin, drafts, previews and account/security pages;
- structured data only when semantically valid;
- permanent redirects for intentionally changed public slugs.

## Performance and caching

- cache published editorial content with deterministic invalidation;
- do not extend game/runtime freshness TTLs through page caching;
- cache homepage editorial blocks separately from runtime status;
- use bounded pagination;
- avoid per-item N+1 queries;
- preload only required relationships;
- use responsive images when media support exists;
- keep the public baseline functional without client-side JavaScript.

## Observability

Record bounded operational information for:

- public route status and latency;
- explicit dependency-unavailable classifications;
- failed download-artifact resolution;
- administrator mutation outcomes;
- event publication jobs if queues are used;
- cache invalidation failures.

Do not place personal data, credentials, full submitted support text or secrets into logs.

## Delivery programme

The programme must be implemented as multiple small vertical slices. Do not deliver the entire portal in one PR.

### Slice 0 — architecture and inventory

Deliver:

- this plan;
- public-surface inventory;
- route and module ownership decisions;
- explicit list of current versus planned capabilities;
- separate implementation task records per later slice.

Exit gate:

- no feature is described as implemented without source/runtime evidence;
- no overlapping active task owns the same paths.

### Slice 1 — production homepage and public shell

Deliver:

- review the existing homepage comparison template;
- promote the approved design to `/` or document why it is not ready;
- shared header/navigation/footer;
- character search;
- dynamic world summary using existing PublicGameData only;
- latest published news preview;
- quick links only to routes that really exist;
- explicit empty/unavailable states;
- desktop/tablet/mobile and keyboard acceptance.

Do not add new persistence in this slice unless a smaller prerequisite is proven necessary.

Exit gate:

- homepage is not a static mock;
- no fabricated game data;
- no dead navigation links;
- old public routes remain functional;
- relevant CI and browser acceptance pass on the exact head.

### Slice 2 — Download, Server Information, Guide and legal baseline

Deliver:

- stable `/download` route;
- client release metadata and immutable artifact references;
- checksum display;
- installation and first-login guidance;
- Server Information and rates;
- Game Rules, Terms, Privacy and Cookies;
- typed public routes for these pages;
- administrator management through existing or explicitly extended CMS permissions.

Exit gate:

- visitor can create an account, obtain a verified client and understand how to connect;
- no executable upload surface is introduced without separate approval;
- legal/publication states are tested.

### Slice 3 — announcements and events

Deliver:

- active announcement ticker;
- event list/detail;
- upcoming, active and archived states;
- administrator event workflow;
- exact permissions and audit;
- homepage event preview.

Exit gate:

- time boundaries and timezone behavior are deterministic;
- unpublished events are not public;
- no-event and cancelled-event states are covered.

### Slice 4 — community and support

Deliver:

- Discord and official social links;
- support landing page;
- bug-report instructions;
- known issues and FAQ;
- optional team page;
- safe external-link configuration.

A stored ticket workflow remains a separate decision.

### Slice 5 — additional public game statistics

Candidate capabilities:

- latest deaths;
- kill statistics;
- guild index;
- public service status.

Before implementation:

- verify legacy engine schema and semantics;
- define field allowlists;
- verify privacy and moderation policy;
- use read-only integration;
- define pagination and dependency-failure behavior.

### Slice 6 — Wiki

Execute the dedicated programme in `WIKI_IMPLEMENTATION_PLAN.md` through separately reviewed slices.

Wiki public navigation activates only after published-only reads, RBAC, revisions, search and required visual/security acceptance are proven.

### Slice 7 — Polish and English completion

Deliver:

- localized URLs and navigation;
- translation workflow;
- `hreflang`;
- locale-aware metadata and dates;
- translation completeness indicators;
- browser acceptance in both languages.

### Slice 8 — safe media and richer editorial presentation

Deliver only after an explicit upload-security task:

- approved image formats;
- MIME/content validation;
- decode and re-encode;
- metadata removal;
- size/dimension limits;
- immutable storage names;
- alt text;
- reference tracking;
- audit.

Do not enable arbitrary files, SVG, scripts or executable uploads.

### Slice 9 — SEO, accessibility, performance and resilience closure

Deliver:

- complete sitemap and metadata;
- keyboard/focus acceptance;
- responsive acceptance;
- error/empty/unavailable states;
- cache invalidation tests;
- bounded performance and public soak evidence;
- visual acceptance matrix updates.

### Slice 10 — later community features

Possible separate tasks:

- polls;
- team profiles;
- maintenance history;
- boosted creature/boss;
- structured game-system catalogues;
- interactive map discovery.

Each capability requires its own contract and must not be bundled into unrelated content work.

### Deferred commerce programme

Webshop, coins, payments, Character Bazaar, transfers, VIP/Loyalty sales and Battle Pass remain outside this plan's implementation authority.

## Recommended PR sequence

1. Public website architecture/inventory documentation.
2. Homepage production activation and shared public shell.
3. Download centre foundation.
4. Server Information, Beginner's Guide and legal routes.
5. Announcement ticker.
6. Events model and public/admin surfaces.
7. Community and support pages.
8. Latest deaths read model.
9. Kill statistics and guild index.
10. Wiki foundation and later Wiki slices.
11. PL/EN architecture and translation workflow.
12. Safe media.
13. SEO/accessibility/performance closure.

## Programme completion criteria

The expanded public website can be considered delivered for its approved non-commercial scope only when:

- the approved homepage is active on `/`;
- server status distinguishes real zero/offline/unavailable states;
- latest news and announcements are dynamic;
- Download provides approved version, platform, size and checksum information;
- Beginner's Guide and Server Information are published;
- event schedule works;
- support and required legal documents exist;
- Wiki is linked only after its own release gate passes;
- navigation and footer expose no dead links;
- administrator mutations require authentication, confirmed MFA and exact permissions;
- privileged mutations are audited;
- all public content follows publication rules;
- desktop, tablet and mobile acceptance pass;
- keyboard, focus, contrast and error-state acceptance pass;
- English and Polish behavior matches the approved launch scope;
- relevant tests and required CI pass on the exact release candidate;
- production verification remains separately classified and is not inferred from repository/staging evidence.
