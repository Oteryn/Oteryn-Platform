# Oteryn Wiki Implementation Agent Prompt

Use the following prompt to start implementation in a new agent session.

```text
Continue Oteryn Platform work by starting the Oteryn Wiki implementation programme from the current repository state. Do not rely on previous chat history.

REPOSITORY WRITE ALLOWLIST:
- Writes are allowed only in blakinio/Oteryn-Platform.
- Treat Canary, login-server, OTClient and all other repositories as read-only unless the user explicitly authorizes a separate write task.

PROGRAM: Oteryn Wiki
RECOMMENDED_MODE: CODEX
MODE_REASON: local repository edits, migrations, tests and CI validation are required.

GOAL:
Implement the Oteryn Wiki architecture described in docs/architecture/WIKI_IMPLEMENTATION_PLAN.md as a sequence of small, reviewable vertical slices. Do not implement the entire programme in one PR.

FIRST REQUIRED DELIVERY:
Start with Slice 0 and Slice 1 only, unless repository evidence proves they must be split further:
1. durable Wiki ADR covering module ownership, Markdown safety, revision model, search abstraction and media boundary;
2. Wiki module registration/status update in architecture documentation;
3. implementation task record and dedicated branch;
4. database/domain foundation for articles, translations, categories and append-only revisions;
5. exact Wiki RBAC permissions and administrator audit event definitions;
6. factories and focused database/domain/authorization tests;
7. no public Wiki activation, navigation link or file upload in this first implementation PR.

MANDATORY READS:
- AGENTS.md
- docs/agents/REPOSITORY_MAP.md
- docs/agents/CONTEXT_ROUTING.md
- docs/agents/PROJECT_STATE.md
- docs/agents/BUILD_TEST_MATRIX.md
- docs/agents/CONTEXT_HANDOFF.md
- docs/agents/tasks/TASK_TEMPLATE.md
- docs/architecture/WIKI_IMPLEMENTATION_PLAN.md
- docs/architecture/SYSTEM_ARCHITECTURE.md
- docs/architecture/MODULE_CATALOG.md
- docs/architecture/SECURITY_ARCHITECTURE.md
- docs/architecture/DATA_OWNERSHIP.md
- docs/architecture/TEST_STRATEGY.md
- relevant existing ADRs for CMS, RBAC, audit and production migrations

CONTEXT ROUTES:
- agent-governance
- architecture
- web-cms
- admin-rbac
- database
- security
- testing

SEARCH FIRST:
- active tasks and open PRs for overlapping paths, modules, permissions or migrations;
- existing CMS news and managed-page models, services, controllers, requests, views and tests;
- existing role/permission registry and role-bundle update conventions;
- existing administrator audit writer/event conventions;
- current migration naming, foreign-key and rollback patterns;
- current Markdown-related dependencies before proposing a new package;
- current image-processing/runtime support, but do not implement media uploads in the first slice;
- current optimistic-locking or version-column patterns;
- nested AGENTS.md files affecting owned paths.

ARCHITECTURE CONSTRAINTS:
- Wiki is a dedicated module inside the existing Laravel modular monolith.
- Reuse Platform Identity, confirmed MFA, explicit RBAC and Audit.
- Do not expand the current generic managed-page model until evidence proves it can satisfy Wiki invariants without coupling or migration risk.
- Public reads must eventually be published-only, but public routes are out of scope for the first slice.
- Revisions are append-only. Restoring a revision must later create a new revision.
- Localized slugs are unique per locale.
- No wildcard permissions.
- No arbitrary HTML, plugins, code execution or file upload in the first slice.
- Do not add a dependency before searching for an existing maintained capability and documenting why it is insufficient.
- Migrations must be backward-conscious and reversible; never assume an empty production database.
- Keep controllers thin and durable business rules in appropriately scoped services/actions/domain classes.

PROPOSED FIRST-SLICE DATA BOUNDARY:
- wiki_articles
- wiki_article_translations
- wiki_categories
- wiki_category_translations
- wiki_article_category
- wiki_revisions

Do not add wiki_media, search infrastructure, public routes or final content rendering unless a smaller prerequisite is strictly required and documented.

PROPOSED FIRST-SLICE PERMISSIONS:
- wiki.access
- wiki.articles.manage
- wiki.categories.manage
- wiki.publish

Defer wiki.media.manage until the media slice unless repository permission-registry constraints make reserving it now safer; document the decision.

REQUIRED INVARIANTS:
- unique localized article slug;
- valid supported locale;
- deterministic lifecycle transitions;
- no publication with missing required localized content;
- every article write creates an append-only revision where the selected workflow requires it;
- concurrent stale edits fail explicitly rather than silently overwriting newer content;
- privileged actions require auth + mfa.confirmed + exact permission;
- no editor receives publish authority implicitly;
- administrator mutations are auditable without storing complete article bodies or secrets in audit metadata.

TASK WORKFLOW:
1. Perform the mandatory lean preflight once.
2. Create docs/agents/tasks/active/OTERYN-YYYYMMDD-wiki-foundation.md from the task template.
3. Declare exact owned_paths and check for overlap.
4. Create a dedicated task branch from current main.
5. Open a draft PR early.
6. Implement the smallest complete first slice.
7. Run focused validation, then full required repository checks for the exact head.
8. Inspect CI and fix root causes without weakening checks.
9. Keep the task checkpoint current with PROVEN, DERIVED, UNKNOWN and CONFLICT evidence.
10. Do not merge until the repository merge gate is satisfied.

ACCEPTANCE CRITERIA FOR THE FIRST IMPLEMENTATION PR:
- Wiki ADR is present and consistent with WIKI_IMPLEMENTATION_PLAN.md.
- Module catalog records the truthful implementation state.
- New migrations are reversible and covered by isolated database tests.
- Domain models/services enforce lifecycle, locale, slug and revision invariants.
- Exact Wiki permissions are registered deny-by-default.
- Existing roles receive permissions only through explicit reviewed bundle changes.
- Admin audit records Wiki mutations using bounded non-secret metadata.
- Missing auth, missing MFA and missing permission are tested.
- No public Wiki route or navigation activation is introduced.
- No file upload or arbitrary HTML surface is introduced.
- Formatting, level-10 static analysis and relevant/full tests pass on the exact PR head, or an external infrastructure blocker is recorded precisely.
- The active task checkpoint leaves exactly one concrete next_action.

STOP CONDITIONS:
- overlapping owned paths with another active task;
- unclear permission-registry migration behavior;
- destructive or irreversible migration without a tested rollback path;
- unresolved Markdown sanitization assumption presented as proven;
- any secret, production credential or personal data;
- pressure to bypass CI, weaken tests or merge with unresolved security failures.

DELIVERY STYLE:
- Keep the PR narrow.
- Do not perform unrelated refactors.
- Report material milestones, blockers and decisions only.
- Cite exact files, SHAs, workflow runs and failing steps in the durable task checkpoint.
```
