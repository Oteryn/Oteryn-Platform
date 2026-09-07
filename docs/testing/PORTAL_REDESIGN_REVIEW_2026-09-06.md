# Oteryn portal redesign — route and visual review

Issue #1297 · PR #1298 · `feat/20260906-premium-portal-redesign`.

## Candidate and delivery boundary

This is an implementation ledger, not a merge, deployment or final acceptance claim. The final PR attestation and `portal-review/manifest.json` must identify the same tested commit.

Initial full candidate: `4910e9dd4d54139847e317a5521688f556831942`. Resume main: `2c3934fece8e072f23ea33ad82f4b4399fc3b45e`, preserving its policy 3.0.0 binding and independent governance changes. Production, staging, live identities/payments, protected environments, workflow changes and PR merge are excluded.

## Implemented design

Public and identity layouts use standalone `portal-system.css` and `portal-pages.css`; homepage composition is in `home-production.css`. Admin keeps its existing style system. Grouped registered navigation retains native details, keyboard dismissal/focus return, locale and current-route semantics and a no-JavaScript path.

The homepage combines original citadel artwork, guest/player actions, truthful world states, character search, an editorial journal, realm ledger and player journey. The registered classic variant inherits the redesigned homepage. Editorial, event, world/community, knowledge, downloads, Bazaar and account families have distinct compositions on shared controls. The account area has a character roster, security sections, owner-only history, support and player tools.

`public/images/oteryn-citadel.webp` is original decorative artwork, not a gameplay screenshot. Concept images are not application evidence. No new frontend dependency, external asset host, third-party game artwork or font is introduced.

## Route coverage

The initial application inventory has 102 non-admin GET/HEAD entries: 98 HTML/navigation entries below and four non-HTML streams (`robots.txt`, `sitemap.xml`, two Wiki media routes). Aliases share their controller-selected views deliberately. Existing POST/PUT/PATCH/DELETE workflows keep their backend contracts.

The vendor `/oauth/authorize` route (`passport.authorizations.authorize`) additionally uses `game-auth/oauth/authorize` and the identity shell. Native OAuth feature tests remain unchanged. Canonical inventories remain `docs/testing/ROUTE_VIEW_NAVIGATION_INVENTORY.json` and `scripts/acceptance/coverage/portal-coverage-manifest.json`; this ledger does not replace a gate.

Paths below `resources/views/` omit `.blade.php`.

| URI | Route name | View or navigation result | Redesigned family |
|---|---|---|---|
| `/` | `home` | `home` | Realm entry |
| `account` | `account.overview` | `identity/account/overview` | Player workspace |
| `account/bazaar` | `marketplace.account` | `marketplace/account` | Player workspace |
| `account/bazaar/sell` | `marketplace.listing.create` | `marketplace/create` | Player workspace |
| `account/characters/create` | `account.characters.create` | `characters/create` | Player workspace |
| `account/characters/{name}/profile` | `account.characters.profile.edit` | `identity/account/character-profile-preferences` | Player workspace |
| `account/payments` | `payments.account.index` | `payments/index` | Player workspace |
| `account/payments/return/{orderPublicId}` | `payments.account.return` | `payments/return` | Player workspace |
| `account/security` | `identity.account-security.show` | `identity/account/security` | Player workspace |
| `account/tools/session-analyzer` | `player-companion.session-analyses.index` | `player-companion/session-analyses/index` | Player workspace |
| `account/tools/session-analyzer/{analysis}` | `player-companion.session-analyses.show` | `player-companion/session-analyses/show` | Player workspace |
| `bazaar` | `legacy.marketplace.index` | `marketplace/index` | Bazaar |
| `bazaar/{auction}` | `legacy.marketplace.show` | `marketplace/show` | Bazaar |
| `characters` | `legacy.game.characters.search` | `REDIRECT:character-or-home` | Search journey |
| `characters/{name}` | `legacy.game.characters.show` | `game/character` | World and community |
| `deaths` | `legacy.game.deaths.index` | `game/deaths` | World and community |
| `download/{platform?}` | `legacy.downloads.index` | `downloads/index` | Guides and support |
| `email-change/confirm/{token}` | `identity.email-change.confirm.create` | `identity/email-change/confirm` | Identity |
| `email-change/recover/{token}` | `identity.email-change.recover.create` | `identity/email-change/recover` | Identity |
| `events` | `legacy.events.index` | `events/index` | Journal and timeline |
| `events/{slug}` | `legacy.events.show` | `events/show` | Journal and timeline |
| `forgot-password` | `password.request` | `identity/forgot-password` | Identity |
| `getting-started` | `legacy.editorial.getting-started` | `support/editorial/show` | Guides and support |
| `guilds` | `legacy.game.guilds.index` | `game/guilds/index` | World and community |
| `guilds/{name}` | `legacy.game.guilds.show` | `game/guild` | World and community |
| `highscores` | `legacy.game.highscores.index` | `game/highscores` | World and community |
| `legal/cookies` | `legacy.legal.cookies` | `support/editorial/show` | Guides and support |
| `legal/privacy` | `legacy.legal.privacy` | `support/editorial/show` | Guides and support |
| `legal/terms` | `legacy.legal.terms` | `support/editorial/show` | Guides and support |
| `login` | `identity.login.create` | `identity/login` | Identity |
| `mfa` | `identity.mfa.settings` | `identity/mfa/settings` | Identity |
| `mfa/challenge` | `identity.mfa.challenge.create` | `identity/mfa/challenge` | Identity |
| `news` | `legacy.news.index` | `news/index` | Journal and timeline |
| `news/{slug}` | `legacy.news.show` | `news/show` | Journal and timeline |
| `online` | `legacy.game.online.index` | `game/online` | World and community |
| `pages/{slug}` | `legacy.pages.show` | `pages/show` | Guides and support |
| `password/change` | `identity.password.change.create` | `identity/change-password` | Identity |
| `recovery-key` | `identity.recovery-key.recover.create` | `identity/recovery-key/recover` | Identity |
| `register` | `identity.register.create` | `identity/register` | Identity |
| `reset-password/{token}` | `password.reset` | `identity/reset-password` | Identity |
| `rules` | `legacy.editorial.rules` | `support/editorial/show` | Guides and support |
| `server-information` | `legacy.editorial.server-information` | `support/editorial/show` | Guides and support |
| `servers` | `legacy.game.servers.index` | `game/servers` | World and community |
| `support` | `legacy.support.index` | `support/editorial/show` | Guides and support |
| `support/enforcement` | `support.enforcement.index` | `support/enforcement/index` | Player workspace |
| `support/enforcement/{enforcementRecord}` | `support.enforcement.show` | `support/enforcement/show` | Player workspace |
| `support/report-a-bug` | `legacy.support.report-a-bug` | `support/editorial/show` | Guides and support |
| `support/reports` | `support.reports.index` | `support/reports/index` | Player workspace |
| `support/reports/create` | `support.reports.create` | `support/reports/create` | Player workspace |
| `support/reports/{playerReport}` | `support.reports.show` | `support/reports/show` | Player workspace |
| `support/tickets` | `support.tickets.index` | `support/tickets/index` | Player workspace |
| `support/tickets/create` | `support.tickets.create` | `support/tickets/create` | Player workspace |
| `support/tickets/{supportTicket}` | `support.tickets.show` | `support/tickets/show` | Player workspace |
| `today` | `legacy.today.index` | `public/today/index` | Journal and timeline |
| `wiki` | `legacy.wiki.index` | `wiki/index` | Knowledge |
| `wiki/catalog` | `legacy.game-catalog.index` | `game-catalog/index` | Knowledge |
| `wiki/category/{slug}` | `legacy.wiki.category` | `wiki/category` | Knowledge |
| `wiki/creatures` | `legacy.game-catalog.creatures.index` | `game-catalog/creatures/index` | Knowledge |
| `wiki/creatures/{slug}` | `legacy.game-catalog.creatures.show` | `game-catalog/creatures/show` | Knowledge |
| `wiki/items` | `legacy.game-catalog.items.index` | `game-catalog/items/index` | Knowledge |
| `wiki/items/{slug}` | `legacy.game-catalog.items.show` | `game-catalog/items/show` | Knowledge |
| `wiki/search` | `legacy.wiki.search` | `wiki/search` | Knowledge |
| `wiki/{slug}` | `legacy.wiki.article` | `wiki/article` | Knowledge |
| `{locale}` | `localized.home` | `home` | Realm entry |
| `{locale}/bazaar` | `marketplace.index` | `marketplace/index` | Bazaar |
| `{locale}/bazaar/{auction}` | `marketplace.show` | `marketplace/show` | Bazaar |
| `{locale}/characters` | `game.characters.search` | `REDIRECT:character-or-home` | Search journey |
| `{locale}/characters/{name}` | `game.characters.show` | `game/character` | World and community |
| `{locale}/deaths` | `game.deaths.index` | `game/deaths` | World and community |
| `{locale}/download/{platform?}` | `downloads.index` | `downloads/index` | Guides and support |
| `{locale}/events` | `events.index` | `events/index` | Journal and timeline |
| `{locale}/events/{slug}` | `events.show` | `events/show` | Journal and timeline |
| `{locale}/getting-started` | `editorial.getting-started` | `support/editorial/show` | Guides and support |
| `{locale}/guilds` | `game.guilds.index` | `game/guilds/index` | World and community |
| `{locale}/guilds/{name}` | `game.guilds.show` | `game/guild` | World and community |
| `{locale}/highscores` | `game.highscores.index` | `game/highscores` | World and community |
| `{locale}/legal/cookies` | `legal.cookies` | `support/editorial/show` | Guides and support |
| `{locale}/legal/privacy` | `legal.privacy` | `support/editorial/show` | Guides and support |
| `{locale}/legal/terms` | `legal.terms` | `support/editorial/show` | Guides and support |
| `{locale}/news` | `news.index` | `news/index` | Journal and timeline |
| `{locale}/news/{slug}` | `news.show` | `news/show` | Journal and timeline |
| `{locale}/online` | `game.online.index` | `game/online` | World and community |
| `{locale}/pages/{slug}` | `pages.show` | `pages/show` | Guides and support |
| `{locale}/rules` | `editorial.rules` | `support/editorial/show` | Guides and support |
| `{locale}/server-information` | `editorial.server-information` | `support/editorial/show` | Guides and support |
| `{locale}/servers` | `game.servers.index` | `game/servers` | World and community |
| `{locale}/support` | `support.index` | `support/editorial/show` | Guides and support |
| `{locale}/support/report-a-bug` | `support.report-a-bug` | `support/editorial/show` | Guides and support |
| `{locale}/today` | `today.index` | `public/today/index` | Journal and timeline |
| `{locale}/wiki` | `wiki.index` | `wiki/index` | Knowledge |
| `{locale}/wiki/catalog` | `game-catalog.index` | `game-catalog/index` | Knowledge |
| `{locale}/wiki/category/{slug}` | `wiki.category` | `wiki/category` | Knowledge |
| `{locale}/wiki/creatures` | `game-catalog.creatures.index` | `game-catalog/creatures/index` | Knowledge |
| `{locale}/wiki/creatures/{slug}` | `game-catalog.creatures.show` | `game-catalog/creatures/show` | Knowledge |
| `{locale}/wiki/items` | `game-catalog.items.index` | `game-catalog/items/index` | Knowledge |
| `{locale}/wiki/items/{slug}` | `game-catalog.items.show` | `game-catalog/items/show` | Knowledge |
| `{locale}/wiki/search` | `wiki.search` | `wiki/search` | Knowledge |
| `{locale}/wiki/{slug}` | `wiki.article` | `wiki/article` | Knowledge |

Additional selected/response views: `home-classic`, `game/unavailable`, `game-catalog/unavailable`, `wiki/unavailable`, `identity/mfa/recovery-codes`, `identity/recovery-key/generated`, `game-auth/oauth/authorize` and `errors/layout`. Sensitive generated values are not screenshot targets. Admin, API/transport, media bytes and inactive `home-preview` are excluded.

## Verification and unresolved work

Historical head `4910e9d` produced 148 actual Laravel renders in run `34104987932`, artifact `10012202828`. That run passed 14 smoke and 27 portability cases and found narrow populated Polish payment overflow. Those results do not certify the repaired head.

Prepared repairs preserve native download-table semantics and add keyboard-accessible payment scrolling. They retain locale in player-tool navigation, remove duplicate account-empty messaging, share the classic homepage, update the task checkpoint and remove the temporary tracked-source recovery test. Event tests declare exact expected negative HTTP responses through the existing diagnostic API without changing authorization or the guard.

The extended visual test covers public and account families, synthetic character/profile settings, ticket/report detail, MFA challenge, grouped navigation, search, no-JavaScript fallback, EN/PL, 320/390/820/1440/1920 widths, 200% text and reduced motion. It captures actual Laravel HTTP output with synthetic fixtures, masks sensitive enrollment/recovery values before rasterization and disables raw authentication traces. Its exact-head results are pending.

**Publication limitation:** tool safeguards rejected writes to `scripts/acceptance/tests/payment-foundation-acceptance.spec.mjs` and `scripts/acceptance/tests/support-legal-acceptance.spec.mjs`. Both files retain their published `4910e9d` bytes, assertions and enabled execution. Their proposed repair-package changes are not included or certified. No alternate write route, test suppression or guard weakening was used. Observe their original tests on the new candidate; a failure is not a pass and may still require a separately authorized repair.

Local PHP/JavaScript syntax and checkpoint checks are supporting evidence only. The sandbox has PHP 8.4, no Composer/application dependencies and no outbound DNS; required PHP 8.5 Laravel, static and browser acceptance must run in existing isolated GitHub Actions. No repaired exact-head PASS is claimed here.

## Compatibility and visual acceptance

Controllers, routes, models, schemas, production configuration, admin UI, security/payment behavior and gate implementations are unchanged. Presentation retains escaping, CSRF, input names, methods and route contracts. Rollback is the coherent presentation diff, with no migration.

Empty, unavailable and inactive capability screens must remain truthful; fixtures are not proof of populated production data. Token-bearing confirmation/generated-value views use the shared system and their existing feature tests without publishing secret-bearing renders. Browser smoke and keyboard/viewport review are not a full assistive-technology certification.

Required closeout: inspect the complete published diff; pass relevant feature/static/browser and required exact-head checks; inspect the new renders and fix findings; attach matching evidence and leave PR #1298 for human visual review. No merge, activation or deployment is authorized.
