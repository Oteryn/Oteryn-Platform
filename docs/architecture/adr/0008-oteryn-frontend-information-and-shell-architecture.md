# ADR 0008 — Oteryn frontend information and shell architecture

## Status

Accepted — 2026-07-21

## Context

The delivered Oteryn Platform frontend contains a functional public shell and a functional administrator shell, while Identity, password, MFA and character-creation pages are largely standalone presentation surfaces. Current Visual / UX Acceptance evidence classifies the product as a functional technical MVP rather than a public-launch-ready UI.

The durable functional surface already spans:

- public News, Online, Highscores, Servers, Character lookup/detail, Guild detail and managed pages;
- Platform Identity registration/login/logout, password recovery/change and MFA;
- greenfield game-account provisioning/binding behavior;
- character creation;
- administrator dashboard, CMS, RBAC and audit.

The current route set does not deliver a general Account Overview/Account Center screen, dedicated provisioning-status screen, Guilds index or standalone Character Search results screen. The frontend architecture must therefore distinguish delivered routes from future navigation capacity instead of presenting roadmap concepts as working features.

The project needs a durable model that:

1. communicates Oteryn as an MMORPG world portal rather than generic SaaS;
2. keeps Account/Identity flows visually connected to the portal;
3. keeps privileged administration operationally separate;
4. supports responsive desktop/tablet/mobile behavior;
5. can expand toward future Community/Game services without a navigation rewrite;
6. does not copy proprietary branding/assets from reference products;
7. does not weaken backend security, authorization or legacy engine integration boundaries.

## Decision

### 1. Use three visually related but structurally distinct interface systems

#### Public Portal

The Public Portal uses a modern dark-fantasy MMORPG branded shell.

It owns:

- world/brand presentation;
- public navigation;
- public content;
- public game-data presentation;
- Character Search access;
- world/server status utilities;
- state-aware Account entry.

#### Account Center

The Account Center is part of the same Oteryn portal ecosystem rather than a separate SaaS application.

Identity/security flows use a focused Oteryn shell. The target Account Center groups Overview, Characters and Security, with Settings added only when real settings exist.

A general Account Overview, provisioning-status surface and authoritative character list are not currently delivered. Any required route/controller/read-model work is a separate bounded dependency and must not be invented in presentation code.

#### Admin Console

The Admin Console uses a separate professional application shell with shared Oteryn design tokens.

It prioritizes:

- navigation clarity;
- dense data presentation;
- tables/forms;
- permission-aware current destinations;
- responsive operational use.

Heavy fantasy artwork is excluded from normal Admin work areas.

### 2. Use grouped scalable public information architecture

The target primary public hierarchy is:

- Home through the Oteryn brand;
- News;
- Community;
- Game;
- Account.

Community currently exposes real Character Search interaction, Online and Highscores. Guild detail remains a valid contextual route, while a Guilds index stays future-ready until implemented.

Game currently exposes Servers. Server Information, Downloads and Knowledge/Wiki remain future-ready and must not be rendered as working destinations before implementation.

This grouped model is chosen over a permanently flat navigation bar containing every current/future feature.

### 3. Character Search is a utility-level capability

Character Search remains highly discoverable through the homepage and a suitable portal utility/navigation entry.

It is not duplicated as a full form in every possible shell region. Wide desktop may use a utility rail; tablet/mobile recomposes it into the content/navigation flow.

### 4. A permanent three-column public layout is not required

Wide desktop may use:

- optional context navigation;
- dominant main content;
- optional World Status/Character Search utility rail.

The right rail and then context sidebar collapse/reflow before main content becomes narrow. Table-heavy pages may use wide main content without a utility rail.

Tablet has an intentional one-dominant-column composition. Mobile uses a compact header, Account action and accessible navigation drawer.

### 5. Use one semantic design-token foundation

Public, Account and Admin share semantic tokens for:

- background/surfaces;
- typography;
- spacing;
- borders/elevation;
- accent and semantic states;
- focus;
- buttons/forms/alerts/statuses/tables/pagination.

Composition and decoration differ by shell, but core component behavior remains consistent.

### 6. Use modern dark-fantasy visual direction with placeholder-safe art

The public visual direction is atmospheric dark fantasy with restrained bronze/gold accents and readable modern content surfaces.

Artwork is optional enhancement. Core layout must remain production-usable before final logo/hero/world art exists.

No proprietary branding, artwork, textures, code or pixel-for-pixel layout from Tibia, RubinOT, TibiaScape, Medivia, MyAAC or other products is copied.

### 7. Responsive tables use explicit local strategies

Each data surface chooses one or more of:

- full table;
- reduced columns;
- contained local horizontal scroll;
- responsive row/card transformation;
- summary/detail disclosure.

Document-level horizontal scrolling is not an accepted table strategy.

### 8. Product-owned errors and dependency states are part of the UI architecture

The target frontend includes branded CSP-compatible 403, 404, 419, 429, 500 and 503 surfaces with safe messaging and real recovery actions.

Partial dependency failures such as runtime unknown/unavailable remain inside the normal shell where possible and are not misrepresented as authoritative empty/offline data.

### 9. Visual acceptance remains a separate post-implementation gate

This ADR and the design documents define direction only.

They do not reclassify Visual / UX Acceptance. The failing acceptance result remains authoritative until the bounded implementation slices are complete and exact-final-SHA browser evidence is rerun across required desktop/tablet/mobile surfaces.

## Consequences

- The UI implementation agent has a durable source of truth for IA, shells, visual direction, responsive behavior and components.
- Public Portal and Account Center can become visually coherent without merging their security/domain responsibilities.
- Admin remains efficient for privileged work without looking like an unrelated generic template.
- Future Guilds, Downloads, Knowledge and Game services have clear IA slots without being falsely advertised as current.
- Account Overview/provisioning visibility is explicitly identified as a dependency rather than a CSS/Blade shortcut.
- The current UI can be migrated incrementally in bounded slices.
- Table and long-content responsiveness become component requirements rather than ad hoc page fixes.
- Art production can proceed independently from core frontend implementation.

## Rejected alternatives

### Keep every current feature as a flat top-level public link

Rejected. It does not scale cleanly toward future Community/Game services and increases header clutter.

### Use one universal three-column shell on every public page

Rejected. It unnecessarily narrows table-heavy/main content and performs poorly on smaller desktops/tablets.

### Make Account Center a visually separate SaaS dashboard

Rejected. Oteryn Account management is part of the player portal ecosystem and should preserve world/product continuity.

### Reuse the fantasy public shell unchanged for Admin

Rejected. Admin needs higher density and reduced decorative overhead while still sharing design tokens.

### Copy a classic Tibia/MyAAC theme for familiarity

Rejected. Interaction/IA familiarity may inspire structure, but proprietary branding/assets and obsolete fixed-width/responsive patterns are not acceptable.

### Mark missing Account Overview/Guilds/Downloads as delivered navigation now

Rejected. Current routes and acceptance evidence do not prove those screens exist.

### Solve provisioning status entirely in frontend presentation work

Rejected. The UI may only render authoritative state supplied through approved application boundaries; new read/controller/retry behavior requires a separate bounded task.

## Follow-up

Implementation proceeds in bounded slices defined by `docs/design/UI_ARCHITECTURE.md`:

1. design foundation and Public shell;
2. Public portal surfaces;
3. Identity/Account presentation shell;
4. Account Overview/provisioning/character list only after required dependencies;
5. Admin shell and privileged surfaces;
6. error/empty/responsive polish;
7. final Visual / UX Acceptance rerun and matrix reconciliation.

Supporting durable design documents:

- `docs/design/INFORMATION_ARCHITECTURE.md`;
- `docs/design/VISUAL_DIRECTION.md`;
- `docs/design/UI_ARCHITECTURE.md`;
- `docs/design/DESIGN_SYSTEM.md`;
- `docs/design/RESPONSIVE_STRATEGY.md`.
