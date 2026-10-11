# Oteryn Platform Information Architecture

## Status

Target information architecture for the production-ready Oteryn frontend.

This document distinguishes:

- `CURRENT` — a route/screen/interaction currently delivered by the repository;
- `FUTURE-READY` — an intentional navigation/content slot that must not appear as a working destination until a real route and authoritative data exist;
- `DEPENDENCY` — a target surface that needs a separately bounded backend/read-model task before UI implementation can truthfully expose it.

Visual / UX Acceptance is not reclassified by this document. The currently delivered UI remains subject to the failing Visual / UX Acceptance gate until implementation and screenshot evidence are rerun.

## Source-of-truth inputs

The IA is derived from:

- `routes/web.php`;
- `docs/testing/FUNCTIONAL_ACCEPTANCE_MATRIX.md`;
- `docs/acceptance/VISUAL_UX_ACCEPTANCE_MATRIX.md`;
- `docs/architecture/SYSTEM_ARCHITECTURE.md`;
- `docs/architecture/MODULE_CATALOG.md`;
- current public/Admin Blade shells and `public/css/app.css`.

No navigation item in this document proves that a route exists. `CURRENT` is the only delivered-status label.

## Product model

Oteryn has three related interface systems:

1. **Public Game Portal** — the MMORPG world portal and public information surface.
2. **Account Center** — part of the same Oteryn portal ecosystem, with focused Identity/security and character-management flows.
3. **Admin Console** — a separate professional application shell using shared design tokens but higher information density and reduced fantasy decoration.

The public portal and Account Center should feel like one product. The Admin Console should feel like an Oteryn-operated tool, not like a public game page.

## Current delivered surface inventory

### Public Game Portal — CURRENT

- Home.
- Embedded character-search interaction on Home and the `/characters` search endpoint behavior.
- News list.
- News detail.
- Online characters.
- Highscores.
- Servers/runtime status.
- Character detail.
- Guild detail.
- Managed public page.

### Public Game Portal — not currently delivered as index/screens

- Guilds index: `FUTURE-READY`.
- Dedicated Character Search results/list screen: not currently delivered as a separate screen; current search is an interaction that resolves toward Character detail/not-found.
- Server Information page distinct from `/servers`: `FUTURE-READY`.
- Download/Game Client surface: `FUTURE-READY`.
- Knowledge/Wiki surface: `FUTURE-READY`.
- Houses index/detail: `FUTURE-READY`.
- Events/boosted-content system: `FUTURE-READY`.

### Identity and account — CURRENT

- Registration.
- Login.
- Logout action.
- Password recovery request.
- Password reset.
- Authenticated password change.
- MFA challenge.
- MFA settings — not enabled state.
- MFA enrollment and confirmation.
- MFA recovery-code reveal after successful confirmation.
- MFA settings — confirmed state.
- MFA disable action.
- Character creation.

### Identity and account — missing current screens

- General Account Center/Account Overview: `DEPENDENCY` / not delivered.
- User-visible legacy engine provisioning-status surface: `DEPENDENCY` / not delivered.
- Character list owned by the authenticated user: `DEPENDENCY` unless an existing authoritative read path is exposed for the account context.
- Session-management surface: `FUTURE-READY`; no current route should be implied.
- Account settings surface beyond existing password/MFA actions: `FUTURE-READY`.

The Accounts module already owns durable pending/ready/conflict provisioning/binding state, but the current UI does not expose a dedicated account-status screen. The implementation agent must not fabricate a read model or state transition inside Blade/CSS work.

### Admin Console — CURRENT

- Admin dashboard.
- News CMS list.
- News create form.
- News edit form.
- Managed pages list.
- Managed page create form.
- Managed page edit form.
- Role management.
- Audit log.

### Admin Console — not currently separate screens

- Permissions management page: `FUTURE-READY`; permissions exist as authorization concepts, but no dedicated current route is delivered.
- Operational dashboards beyond current admin dashboard: `FUTURE-READY`.

## Target global IA

```text
OTERYN

PUBLIC PORTAL
├── Home                                      [CURRENT]
├── News                                      [CURRENT]
│   ├── News List                             [CURRENT]
│   └── News Detail                           [CURRENT]
├── Community
│   ├── Character Search                      [CURRENT interaction]
│   ├── Character Detail                      [CURRENT contextual]
│   ├── Online                                [CURRENT]
│   ├── Highscores                            [CURRENT]
│   ├── Guild Detail                          [CURRENT contextual]
│   └── Guilds                                [FUTURE-READY index]
├── Game
│   ├── Servers                               [CURRENT]
│   ├── Server Information                    [FUTURE-READY]
│   ├── Downloads                             [FUTURE-READY]
│   └── Knowledge / Wiki                      [FUTURE-READY]
├── Managed Public Pages                      [CURRENT contextual]
└── Account
    ├── Login                                 [CURRENT]
    ├── Register                              [CURRENT]
    ├── Password Recovery                     [CURRENT]
    └── Account Center                        [DEPENDENCY target shell]
        ├── Overview                          [DEPENDENCY]
        ├── Characters                        [DEPENDENCY target section]
        │   └── Create Character              [CURRENT focused operation]
        ├── Security
        │   ├── Change Password               [CURRENT]
        │   └── MFA                           [CURRENT]
        └── Settings                          [FUTURE-READY]

ADMIN CONSOLE
├── Dashboard                                 [CURRENT]
├── Content
│   ├── News                                  [CURRENT]
│   └── Managed Pages                         [CURRENT]
├── Access
│   ├── Roles                                 [CURRENT]
│   └── Permissions                           [FUTURE-READY separate screen]
└── Operations
    └── Audit Log                             [CURRENT]
```

## Public primary navigation

### Desktop target

Use the Oteryn brand/home link plus four primary destinations:

```text
[ OTERYN ]   News   Community   Game                         Account
```

- **Home** is reached through the Oteryn brand/logo rather than consuming another top-level slot.
- **News** is direct because it is a primary portal function.
- **Community** groups Character Search, Online and Highscores. Guilds must not appear as an active index destination until a Guilds index exists.
- **Game** contains Servers. Future Server Information/Downloads/Knowledge entries remain hidden or explicitly non-clickable roadmap documentation until implemented.
- **Account** is a state-aware entry point:
  - guest: `Login` primary, `Create Account` secondary;
  - authenticated: target Account Center entry after that route exists; until then implementation must use only real current destinations and avoid pretending an overview exists.

This grouping is preferred over permanently exposing every feature in a flat bar because it leaves room for future Community/Game capabilities without repeatedly redesigning the header.

## Context navigation

A persistent three-column layout is **not** the universal portal rule.

Use contextual navigation when it adds orientation:

- News list/detail: News context can expose latest/list/back navigation.
- Community pages: Character Search, Online and Highscores are siblings.
- Game: Servers is the only current destination; do not render an empty Game sidebar solely for symmetry.
- Account Center: persistent section navigation is appropriate after Account Center exists.

On content-heavy or table-heavy pages, contextual navigation must collapse before the main content becomes too narrow.

## Utility model

Character Search should be easy to reach without appearing in three competing places.

Target placement:

- desktop wide portal: utility rail or header utility trigger;
- homepage: dedicated Character Search block;
- tablet/mobile: a compact search action/card after the main page heading or inside the Community drawer.

Do not duplicate a full search form simultaneously in header, left navigation and right rail.

World/server status is a utility, not a primary navigation branch. It may appear as:

- a compact wide-desktop utility card;
- homepage status block;
- tablet/mobile status card below primary content heading.

The full authoritative details remain on Servers.

## Homepage IA

The homepage serves both a new player and a returning player.

Target content order:

1. **Oteryn hero / world identity**.
2. **Primary action group**:
   - `Create Account` — `CURRENT` route;
   - `Play / Account` — state-aware wording using real available destinations;
   - `Download` — only when a real current download route/content exists; otherwise omit, do not render a fake CTA.
3. **World snapshot**:
   - server/runtime status from authoritative current data;
   - players online where authoritative;
   - server-save or other values only when an authoritative source exists.
4. **Latest News** — current published content.
5. **Character Search** — current interaction.
6. **Community snapshot** — Highscores/Online summaries only if the implementation has authoritative data available without introducing new unbounded queries.
7. **Game/world feature content** — only from current managed content or explicitly implemented product features.
8. **Community/external links** — only configured real links.

The homepage must not market Downloads, rates, events, boosted content, client version or game systems as facts without an authoritative current source.

## Account Center IA

### Target shell

```text
ACCOUNT CENTER

Overview
Characters
Security
Settings
```

Status by section:

- `Overview` — `DEPENDENCY`.
- `Characters` — target section; creation is `CURRENT`, list/ownership overview requires authoritative read support.
- `Security` — current password and MFA operations can be linked here.
- `Settings` — `FUTURE-READY`; hide until real settings exist.

### Overview target content

Only expose state that is actually available:

```text
Welcome back

ACCOUNT STATUS
Platform Identity: <authoritative state if exposed>
MFA: Enabled / Not enabled
Game Account: Pending / Ready / Conflict only from authoritative binding state

YOUR CHARACTERS
<authoritative character list if a bounded read model exists>
[ Create Character ]

SECURITY SUMMARY
Password
MFA
```

The target hierarchy is valid, but the implementation must split out a backend/read-model dependency if the current controller layer cannot safely supply the data.

### Focused account operations

The following remain focused screens rather than being embedded into one oversized settings page:

- password change;
- password recovery request;
- password reset;
- MFA challenge;
- MFA enrollment/confirmation;
- MFA disable;
- character creation.

Focused screens use the Account/Identity visual shell and provide a clear return path to the nearest real parent destination.

## Admin IA

Target navigation:

```text
OTERYN ADMIN

Dashboard
Content
├── News
└── Managed Pages
Access
├── Roles
└── Permissions        [FUTURE-READY]
Operations
└── Audit Log
```

Admin navigation rules:

- render only permitted destinations;
- no UI visibility replaces server-side authorization;
- active section is explicit;
- `Public site` and an authenticated account/logout control are available without mixing public portal navigation into the admin sidebar;
- future Permissions must not be shown as a working link until a real route exists.

## Deep-link and contextual surfaces

Some current routes are valid contextual destinations without requiring an index page:

- Character detail.
- Guild detail.
- News detail.
- Managed public page.

The absence of a Guilds index does not invalidate Guild detail. Navigation should provide contextual recovery actions such as `Back to Community`, Character Search or Home rather than inventing a missing index.

## Error and state IA

Errors and dependency states are part of the product architecture, not framework leftovers.

Target product-owned error destinations/states:

- `403` authorization denied;
- `404` not found;
- `419` expired page/session/CSRF state;
- `429` too many requests;
- `500` unexpected application failure;
- `503` dependency/service unavailable.

Each state keeps the safest applicable shell, uses non-technical language, and offers one or two real recovery actions.

Important current empty/dependency states to preserve and improve:

- no published news;
- no online players;
- no highscores result;
- character not found;
- guild not found;
- guild with no members;
- runtime unknown/unavailable;
- public game-data dependency unavailable;
- no characters once an Account Center character list exists;
- provisioning pending/ready/conflict only after a real account-status surface exists;
- authorization denied.

## Navigation state rules

- Active location must be communicated by text/structure, not color alone.
- Breadcrumbs are useful on deep News/Character/Guild/managed-page contexts, but should stay shallow.
- Mobile drawer groups Public, Community, Game and Account destinations.
- Admin uses its own drawer/sidebar model.
- Account security flows never expose privileged Admin links unless authorization permits them.
- Logout is always an explicit POST action, not a GET navigation shortcut.

## Future expansion rules

A new navigation item may become `CURRENT` only when:

1. a real route exists;
2. authorization rules are defined where needed;
3. authoritative data/content exists;
4. empty/error states are defined;
5. responsive behavior is covered;
6. acceptance evidence is updated.

This rule lets the IA scale toward Guilds, Downloads, Knowledge and additional Game Services without presenting roadmap concepts as delivered product functionality.
