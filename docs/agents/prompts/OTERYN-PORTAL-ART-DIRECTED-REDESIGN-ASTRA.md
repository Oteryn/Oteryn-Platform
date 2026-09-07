# GPT-6 ASTRA — OTERYN PORTAL ART-DIRECTED PRODUCT REDESIGN

Repository: `Oteryn/Oteryn-Platform`

Primary product authority: Issue `#1297`.

Existing implementation PR: `#1298` on branch `feat/20260906-premium-portal-redesign`.

Visual target: the owner-provided reference screenshot that must be attached to the Astra invocation together with this prompt.

Route/view ledger:
`docs/testing/PORTAL_REDESIGN_REVIEW_2026-09-06.md`

## MISSION

Transform the **entire player-facing Oteryn web portal** into a genuinely premium, cinematic MMORPG website whose visual quality and composition match the intent of the supplied visual target.

The current implementation in PR #1298 is a functional baseline, **not the accepted visual destination**. It is too close to a technical web application/dashboard in important page families.

You may substantially rewrite the player-facing Blade markup, CSS, presentation JavaScript, original visual assets and translations when necessary.

Preserve the real application contract.

Do not redesign by merely changing colors, radii, borders or card styles.

The result must look like a finished game product, not an admin panel, SaaS dashboard, documentation site or generic Laravel frontend.

## LIVE AUTHORITY FIRST

Before material work, refresh and obey:

- protected/current `main`;
- root and applicable nearer `AGENTS.md` / `AGENTS.override.md`;
- Issue `#1297`;
- PR `#1298`;
- the active task record:
  `docs/agents/tasks/active/OTERYN-20260906-premium-portal-redesign.md`;
- the route/view ledger;
- the owner-provided visual target screenshot;
- current checks, reviews, unresolved threads and path ownership.

GitHub live state is authoritative.

If any instruction conflicts with this prompt, follow repository precedence and record the conflict rather than guessing.

Keep all work in the existing owned redesign PR unless live authority requires otherwise.

Do not merge, deploy, change production state or access live user/payment data unless the owner explicitly authorizes that separately.

## HOW TO READ THE VISUAL TARGET

The supplied screenshot contains surrounding chat UI because it records the owner's original reference.

Ignore the chat interface, archive chips and editor overlays.

Study the **portal composition itself**.

Do not pixel-copy the screenshot. Use it as the required art-direction and quality bar.

The important qualities are:

- cinematic world art integrated into the layout rather than placed in a small card;
- a strong, unmistakable fantasy-game hero;
- dark blue/charcoal/black depth with restrained warm gold/ivory highlights;
- premium serif/display typography paired with highly readable UI text;
- layered scenery, atmospheric gradients, vignettes, subtle texture and depth;
- compact game navigation embedded into the product identity;
- clear realm/server status integrated with the hero;
- asymmetric editorial composition instead of uniform card grids;
- ornamental dividers and small fantasy details used with restraint;
- large image-led news and discovery areas;
- strong player calls to action;
- dense information presented like a game portal, not a spreadsheet;
- a coherent desktop composition that translates intentionally to tablet and phone.

The target is **premium MMORPG editorial/game UI**, not “dark mode with cards”.

## NON-NEGOTIABLE PRODUCT CONTRACTS

Preserve existing:

- routes and route names;
- controllers;
- models and domain behavior;
- authentication, authorization and ownership rules;
- CSRF/session/security behavior;
- account and MFA workflows;
- payment semantics;
- support/report/enforcement semantics;
- truthful server and character data;
- feature availability states;
- localization behavior;
- forms, field names, HTTP methods and validation contracts;
- accessibility semantics;
- current admin/control-plane surfaces unless explicitly in player-facing scope.

Do not invent fake live data.

Do not hard-code successful/online/available states when the application says otherwise.

Do not weaken tests, diagnostic guards, CSP, auth boundaries or branch protection to make the redesign pass.

## DESIGN SYSTEM

Build one recognizably Oteryn visual language, but do **not** force every page into the same card template.

Use shared tokens and primitives for:

- color;
- typography;
- spacing;
- borders and ornaments;
- focus states;
- buttons;
- form controls;
- status indicators;
- navigation;
- tables/ledgers;
- media treatment;
- responsive behavior.

Then give each page family its own composition.

### Avoid

- generic SaaS cards everywhere;
- admin-dashboard sidebars for public pages;
- excessive bordered rectangles;
- endless equal-width grids;
- plain technical tables without game framing;
- brown/orange medieval template clichés;
- imitating Tibia or another existing game's website;
- decorative clutter that hurts readability;
- fake game screenshots;
- copyrighted third-party artwork copied into the repo;
- sacrificing accessibility for atmosphere.

## REQUIRED PAGE FAMILIES

Use the live route ledger. Cover every applicable player-facing route, not only the homepage.

At minimum redesign and visually review:

### 1. Global shell

- desktop header/navigation;
- mobile navigation;
- footer;
- locale controls;
- contextual navigation;
- logged-out and logged-in states;
- global search/entry points where present;
- consistent fantasy framing without turning every page into the homepage.

### 2. Homepage

This is the strongest art-directed surface.

Required qualities:

- cinematic Oteryn world hero;
- clear title/brand proposition;
- primary play/download CTA;
- secondary exploration CTA;
- truthful realm/server status;
- useful character search;
- latest news/editorial content;
- event/community discovery;
- world/lore/product discovery;
- visual depth from first viewport through footer.

Do not reduce the homepage to a hero followed by generic cards.

### 3. News and events

Treat them as an MMORPG chronicle/editorial publication.

Use image-led hierarchy, featured stories, dates/categories/statuses, readable article pages and event-state treatment.

### 4. Servers, online list and highscores

Present these as living-world/realm ledgers.

Keep information dense and scannable, but add strong hierarchy, world status identity and polished responsive behavior.

Do not turn them into an admin table.

### 5. Character search, character profile and deaths

Make character identity the focus.

Use clear hero/profile hierarchy, world/vocation/level/status information, history and relevant actions.

Respect privacy and unavailable states exactly.

### 6. Guilds and guild detail

Create a community/guild identity surface rather than a generic directory.

### 7. Wiki and game catalog

Use an explorer/knowledge-book composition with strong search, taxonomy, article readability and item/creature presentation.

It must remain fast and practical for actual gameplay reference.

### 8. Downloads and getting started

Treat this as the path into the game.

Use platform/download hierarchy, requirements, onboarding steps and trust/status information.

### 9. Support, reports and enforcement

Keep clarity and seriousness while retaining the Oteryn identity.

Forms must remain highly usable.

Do not make safety/legal content visually theatrical at the expense of comprehension.

### 10. Bazaar / marketplace

Make it feel like an in-world market while retaining truthful listing, account and transaction semantics.

Dense commerce information must remain easy to compare.

### 11. Login, registration and account recovery

Use an immersive identity entrance with Oteryn scenery and a focused, trustworthy form.

Do not use a generic centered authentication card on an empty background.

### 12. Account center

This may be more functional than the public site, but must still look like part of the same game.

Use a player/account workspace, not an internal admin dashboard.

Cover:

- overview;
- character management;
- profile preferences;
- security;
- MFA;
- sessions;
- recovery flows;
- payments/history where applicable;
- player companion/tools.

### 13. Error, empty, unavailable and maintenance states

Create designed states for:

- 404;
- 403/authorization failures where rendered;
- empty lists;
- unavailable runtime data;
- maintenance;
- feature disabled/unavailable.

These must be truthful and visually intentional.

## ORIGINAL VISUAL ASSETS

Prefer original Oteryn-owned art already in the branch when suitable.

You may create additional original decorative artwork or compositions if the execution surface supports it.

Otherwise use authored CSS/SVG/vector/background treatments and repository-owned assets.

For every new substantial asset:

- keep provenance clear;
- do not copy protected third-party art;
- optimize file size;
- use responsive loading appropriately;
- provide meaningful alt text when content-bearing;
- use empty alt/decorative semantics when purely decorative.

## RESPONSIVE TARGETS

Explicitly design and inspect around:

- 320 px;
- 390 px;
- 768–900 px;
- 1366–1440 px;
- 1920 px.

Responsive behavior must be intentional.

Do not simply stack every desktop card vertically.

Preserve hierarchy and atmosphere on phone.

Test:

- no unintended horizontal document overflow;
- long Polish labels/content;
- long names;
- empty/unavailable states;
- forms and validation errors;
- dense tables/ledgers;
- mobile navigation;
- large text.

## ACCESSIBILITY

Preserve or improve:

- semantic heading hierarchy;
- keyboard navigation;
- visible focus;
- accessible names;
- form labels/errors;
- landmark structure;
- contrast;
- reduced motion;
- 200% text resizing;
- forced-colors usability;
- touch target usability.

Atmosphere is not a reason to hide focus or lower legibility.

## EN / PL

The visual system must work in both English and Polish.

Do not leave mixed-language player-facing copy.

Polish text expansion is part of layout acceptance.

## IMPLEMENTATION METHOD

Work autonomously through the whole portal.

Do not stop after producing a homepage.

A recommended execution sequence is:

1. inventory live player-facing routes/views and compare them with the ledger;
2. inspect the visual target and current real screenshots;
3. define the revised art direction and reusable design primitives;
4. rebuild global shell;
5. rebuild homepage to the target quality bar;
6. rebuild each page family;
7. render the real Laravel application using synthetic fixtures;
8. capture representative EN/PL screenshots at required viewport classes;
9. visually inspect them;
10. fix anything that still looks like a technical/dashboard template;
11. run relevant feature/static/browser tests;
12. fix regressions without weakening invariants;
13. run exact-head required checks;
14. update the route/visual evidence and PR handoff.

Use implementation, render, inspect, fix loops.

Do not perform one pass and declare success.

## VISUAL ACCEPTANCE BAR

This is the key correction to the previous implementation.

A page is **not visually accepted** merely because:

- it renders;
- tests pass;
- it uses the new colors;
- it has no overflow;
- it shares design tokens;
- it contains all expected data.

For each major page family, inspect a real rendered screenshot and ask:

> Would a player see this as a polished commercial MMORPG portal, or as a technical Laravel application wearing a dark theme?

If the answer is the second one, continue redesigning.

The original visual target is the quality-direction benchmark.

The homepage should be recognizably close in **art direction, hierarchy, richness and atmosphere**, while still being an original Oteryn implementation.

Other page families do not need to copy the homepage layout, but must feel authored by the same game/product team.

## REAL APPLICATION EVIDENCE

Visual acceptance must use the real Laravel/Blade application and truthful synthetic fixtures.

Do not use standalone HTML mockups as proof.

Create a final screenshot matrix that includes representative:

- homepage guest desktop/phone/wide;
- open desktop/mobile navigation;
- news index/article;
- event index/detail;
- server/world status;
- online list;
- highscores;
- character search/profile;
- guild list/detail;
- Wiki index/article/search;
- catalog item/creature;
- downloads/getting started;
- support index/form/detail;
- login/register/recovery;
- account overview/characters;
- security/MFA;
- Bazaar;
- payments/history if player-facing;
- player tools;
- 404;
- unavailable/maintenance/empty;
- Polish equivalents for representative high-density screens;
- 320 px;
- 200% text.

Inspect the screenshots rather than only generating them.

## TEST REPAIRS

PR #1298 currently contains known browser-acceptance fixture/diagnostic failures.

Treat them separately from visual design.

Repair genuine test-fixture/test-declaration defects narrowly when authorized:

- preserve the diagnostic guard;
- declare only exact expected negative HTTP responses;
- make synthetic identities/fixtures isolated and repeatable;
- do not suppress tests;
- do not convert failures into skips;
- do not loosen security assertions;
- do not change product behavior just to satisfy a broken fixture.

Then rerun the full applicable matrix on the exact final head.

## COMPLETION CONDITIONS

Do not stop because “most pages are done”.

Complete only when all are true:

- every player-facing route family in the live ledger has an intentional design;
- no important surface still looks like the rejected technical/dashboard baseline;
- EN/PL representative screens are reviewed;
- required responsive widths are reviewed;
- 200% text and accessibility-critical behaviors are validated;
- real Laravel screenshots have been inspected;
- relevant tests pass on the exact final head;
- required repository checks pass;
- the PR contains truthful final evidence;
- no unresolved visual blocker remains.

Keep PR #1298 in review workflow; do not merge or deploy unless separately authorized by the owner after final visual review.

## FINAL RESPONSE

Report concisely:

- branch;
- final commit SHA;
- PR;
- major page families rebuilt;
- route coverage;
- real screenshot/evidence location;
- EN/PL and responsive coverage;
- validation/check results;
- any remaining blocker.

Do not claim production deployment unless it actually occurred under separate authorization.

If no blocker remains, end with:

`The complete art-directed Oteryn portal redesign is ready for human visual review.`
