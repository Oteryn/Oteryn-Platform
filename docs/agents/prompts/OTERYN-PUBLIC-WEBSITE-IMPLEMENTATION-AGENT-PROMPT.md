# Oteryn Public Website Implementation Agent Prompt

Use the following prompt to start the wider public-website implementation programme in a fresh agent session.

```text
Continue Oteryn Platform work by starting the public website expansion programme from the current repository state. Do not rely on previous chat history.

REPOSITORY WRITE ALLOWLIST:
- Writes are allowed only in blakinio/Oteryn-Platform.
- Treat Canary, login-server, OTClient and all other repositories as read-only unless the user explicitly authorizes a separate write task.

PROGRAM: Oteryn Public Website Expansion
RECOMMENDED_MODE: CODEX
MODE_REASON: local Laravel edits, Blade/CSS work, tests, browser validation and CI are required.

GOAL:
Implement docs/architecture/PUBLIC_WEBSITE_EXPANSION_PLAN.md as a sequence of small, reviewable vertical slices. Wiki is one later workstream governed by docs/architecture/WIKI_IMPLEMENTATION_PLAN.md; do not reduce the programme to Wiki only and do not implement the entire portal in one PR.

FIRST REQUIRED DELIVERY:
Implement Slice 1 only: production homepage and shared public shell.

The first implementation PR should:
1. inspect the current `/` homepage and `/design/home-v2` comparison template;
2. decide from source/tests whether the comparison template is production-ready or requires a bounded correction before promotion;
3. activate the approved homepage design on `/`;
4. provide a shared responsive public header, navigation and footer;
5. preserve the existing character search;
6. show a dynamic world summary using existing PublicGameData interfaces/read models only;
7. show a bounded preview of latest published news;
8. expose quick links only to routes that actually exist;
9. implement explicit empty, stale and unavailable states without fabricating game data;
10. add focused feature, authorization/security where relevant, responsive, keyboard/focus and visual acceptance coverage.

Do not add Downloads, Events, Wiki, media uploads, polls, support tickets, new Canary queries or commerce in the first implementation PR.

MANDATORY READS:
- AGENTS.md
- docs/agents/REPOSITORY_MAP.md
- docs/agents/CONTEXT_ROUTING.md
- docs/agents/PROJECT_STATE.md
- docs/agents/BUILD_TEST_MATRIX.md
- docs/agents/CONTEXT_HANDOFF.md
- docs/agents/tasks/TASK_TEMPLATE.md
- docs/architecture/PUBLIC_WEBSITE_EXPANSION_PLAN.md
- docs/architecture/WIKI_IMPLEMENTATION_PLAN.md only to understand that Wiki is a separate later workstream
- docs/architecture/SYSTEM_ARCHITECTURE.md
- docs/architecture/MODULE_CATALOG.md
- docs/architecture/SECURITY_ARCHITECTURE.md
- docs/architecture/TEST_STRATEGY.md
- docs/acceptance/VISUAL_UX_ACCEPTANCE_MATRIX.md
- docs/testing/FUNCTIONAL_ACCEPTANCE_MATRIX.md
- relevant current public-web, CMS and PublicGameData tests

CONTEXT ROUTES:
- agent-governance
- architecture
- web-cms
- public-game-data
- security
- testing

SEARCH FIRST:
- active tasks and open PRs for overlapping public views, routes, CSS, layout, CMS or PublicGameData paths;
- current route definition for `/` and `/design/home-v2`;
- current home and home-preview Blade templates and shared layouts/components;
- current public navigation and footer implementations;
- current PublicGameData world/server summary interfaces and explicit dependency-failure states;
- current published-news query/controller/view behavior;
- existing homepage, responsive, keyboard/focus, E2E and visual tests;
- nested AGENTS.md files affecting owned paths;
- reusable components before creating new UI abstractions.

ARCHITECTURE CONSTRAINTS:
- Keep the solution inside the existing Laravel modular monolith.
- PublicPortal/homepage code is presentation/orchestration only; it must not own Canary query logic or CMS persistence.
- Reuse existing PublicGameData and CMS boundaries.
- Do not query Canary or Redis directly from Blade templates.
- Do not represent unavailable runtime data as zero players or authoritative offline state.
- Do not expose drafts or unpublished CMS content.
- Do not create dead links or placeholder routes presented as implemented.
- Keep the server-rendered baseline fully functional without client-side JavaScript.
- Preserve existing Identity, account and administrator behavior.
- Do not add wildcard permissions or unrelated privileged actions.
- Do not add a dependency until existing framework/repository capabilities have been searched and found insufficient.
- Do not copy RubinOT code, text, assets, branding, icons or layout.

EXPECTED HOMEPAGE DATA STATES:
- world summary: AVAILABLE, EMPTY where semantically valid, STALE, UNAVAILABLE;
- latest news: AVAILABLE, EMPTY, UNAVAILABLE;
- character search: initial, validation error, not found, dependency unavailable where applicable;
- authenticated and guest navigation states.

EXPECTED FIRST-SLICE HOMEPAGE BLOCKS:
- Oteryn hero and primary actions;
- character search;
- dynamic world summary;
- latest published news preview;
- quick links to existing Online, Highscores, Servers, News and Account/auth routes;
- shared footer;
- no Events, Download, Wiki or Support link unless the target route already exists and is ready for public use.

TASK WORKFLOW:
1. Perform the mandatory lean preflight once.
2. Create docs/agents/tasks/active/OTERYN-YYYYMMDD-public-homepage-production.md from the task template.
3. Declare exact owned_paths and resolve overlap before editing.
4. Create a dedicated task branch from current main.
5. Open a draft PR early.
6. Record current `/` and `/design/home-v2` behavior before changes.
7. Implement the smallest complete homepage/public-shell slice.
8. Run focused tests and visual/browser validation.
9. Run all required repository checks for the exact head.
10. Inspect CI and fix root causes without weakening checks.
11. Keep the task checkpoint current with PROVEN, DERIVED, UNKNOWN and CONFLICT evidence.
12. Do not merge until the repository merge gate is satisfied.

ACCEPTANCE CRITERIA FOR THE FIRST IMPLEMENTATION PR:
- The approved design is active on `/`, or the PR documents and fixes the exact bounded blocker before activation.
- `/design/home-v2` is removed, redirected or retained only with an explicit reviewed reason; no confusing duplicate production surface remains.
- Existing Home, News, Online, Highscores, Servers, character search and account/auth journeys remain functional.
- World status/count comes through an existing explicit query/read-model boundary.
- Runtime dependency failure is not displayed as zero or fabricated offline.
- Latest news includes only published posts and is bounded.
- Guest and authenticated navigation states are tested.
- Navigation contains no dead links.
- Footer is complete for currently delivered routes and does not advertise future capabilities as live.
- Desktop, tablet and mobile layouts have no horizontal overflow.
- Keyboard navigation, visible focus and menu behavior pass focused acceptance.
- Empty, error, stale and unavailable states are visually usable.
- Public output remains escaped and CSP-compatible.
- Formatting, level-10 static analysis, relevant/full tests and required browser/visual checks pass on the exact PR head, or an external infrastructure blocker is recorded precisely.
- The active task checkpoint leaves exactly one concrete next_action.

AFTER SLICE 1:
Do not continue automatically into every later feature in the same PR. Archive/close the completed homepage task, then create a new bounded task for the next approved slice from PUBLIC_WEBSITE_EXPANSION_PLAN.md. The recommended next slice is Download, Server Information, Beginner's Guide and legal baseline.

LATER PROGRAMME ORDER:
1. Homepage and public shell.
2. Download centre.
3. Server Information, Beginner's Guide and legal routes.
4. Announcement ticker.
5. Events public/admin surfaces.
6. Community and support pages.
7. Latest deaths, kill statistics and guild index after read contracts are proven.
8. Wiki through its dedicated plan.
9. PL/EN completion.
10. Safe media.
11. SEO/accessibility/performance closure.
12. Polls and other optional community features as separate tasks.

DEFERRED AND NOT AUTHORIZED BY THIS PROMPT:
- webshop;
- payments;
- coins;
- Character Bazaar;
- transfers;
- VIP/Loyalty sales;
- Battle Pass;
- arbitrary executable upload;
- public/player Wiki editing;
- cross-repository writes.

STOP CONDITIONS:
- overlapping owned paths with another active task;
- unclear or conflicting current homepage/runtime data semantics;
- requirement to fabricate data or link to unimplemented routes;
- unresolved security or authorization regression;
- destructive migration or unrelated schema work;
- secret, production credential, personal data or private artifact exposure;
- pressure to bypass CI, reduce coverage or merge with failing required checks.

DELIVERY STYLE:
- Keep the PR narrow.
- Do not perform unrelated refactors.
- Report material milestones, blockers and decisions only.
- Record exact files, SHAs, workflow runs and failing steps in durable task state.
```
