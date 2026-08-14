# Oteryn Portal Completion Delivery Plan

## Status and authority

**CURRENT DELIVERY PLAN — subordinate to accepted ADRs, operation-specific contracts, repository governance and `docs/architecture/PORTAL_COMPLETENESS_ARCHITECTURE.md`.**

This plan converts the portal review into an implementation order. It does not itself authorize production, protected-environment, payment, live-data or external-repository mutation. Live Git/task/Issue/PR/CI state always overrides dated examples.

Companion records:

- review: `docs/agents/reports/OTERYN-20260810-portal-architecture-product-review.md`;
- programme: `docs/agents/programs/OTERYN_PORTAL_COMPLETION.md`;
- work allocation: `docs/agents/programs/OTERYN_PORTAL_COMPLETION_WORK_ALLOCATION.md`;
- worker prompt: `docs/agents/prompts/OTERYN-PORTAL-COMPLETION-EXECUTION-PROMPT.md`;
- short alias: `PORTAL-CLOSEOUT`.

## Guiding decision

Do **not** rewrite the portal framework or split it into default microservices. Retain the Laravel modular monolith and complete it through bounded vertical slices.

Every slice must preserve:

- one clear domain owner and dependency direction;
- explicit authn/authz/privacy and failure semantics;
- version/freshness/revision semantics where facts can become stale;
- least privilege and fail-closed handling of authority uncertainty;
- additive/reversible data changes when persistence changes;
- real applicable frontend/integration/E2E for user-facing work;
- structured observability and rollback/operational boundaries;
- exact-head validation and terminal task/PR lifecycle.

## Execution allocation

The delivery order in this document answers **what should happen next** at a product/architecture level. The canonical live scheduler is `docs/agents/programs/OTERYN_PORTAL_COMPLETION.md`; it classifies every selection-order entry from live evidence and selects the first unowned `READY` candidate. `docs/agents/programs/OTERYN_PORTAL_COMPLETION_WORK_ALLOCATION.md` answers **who should own the already-selected work** through model-agnostic roles and whether a bounded implementation is technically suitable for optional Codex execution.

The allocation is subordinate to this delivery order, the canonical programme's live selection, task/PR ownership and repository authorization. `IMPLEMENTATION_OWNER` is an ownership role, not a model choice. Codex suitability never grants permission to consume owner-funded Codex/OpenAI quota; `AGENTS.md` requires explicit owner permission for each exact use/task. Without that permission, the same bounded task uses another permitted execution mode.

## Programme priorities

### P0 — reliable control plane and source of truth

Goal: make future autonomous task selection trustworthy.

Required outcomes:

1. live task files, convenience indexes, Issues and PR state are reconciled where drift materially affects routing;
2. no completed task remains falsely active;
3. no open related PR lacks an intentional disposition;
4. historical state is labelled historical rather than silently overwritten;
5. GitHub connector capability is checked before any local-CLI access conclusion;
6. prompt/governance changes follow the prompt-evaluation standard;
7. `ACTIVE_WORK.md` and `PROJECT_STATE.md` never override newer active-task, PR/Issue or protected-`main` evidence.

The exact current routing snapshot belongs in the selected task/selection proof rather than in this durable plan. A stale convenience index is evidence to reconcile, not a queue.

Exit gate: an agent can determine current owner, exact next action and blockers from live durable state without relying on chat history or dated queue prose.

### P1 — close proven high-risk security/authority findings

Use the existing Issue-owned remediation programme; do not duplicate repair ownership.

Historical repairs #948 (immutable Download Center artifact reference), #944 (bounded entitlement stale-authority lease) and #941 (private Today cache isolation) are terminal completed examples. They must not be treated as current priorities simply because an older report listed them. Each invocation queries live open remediation Issues and routes only an implementation-authorized current high-risk candidate through `OTERYN_PLATFORM_REMEDIATION`.

Exit gate: every launch-critical repair has exact-head validation, applicable E2E/N/A, terminal PR/Issue/task closeout and no material finding remains.

### P1 — production/public-edge proof

Repository/staging evidence is not production proof. Issue #490 and the production-readiness gate control this work.

Required evidence before public-go-live claims can become true:

- exact deployed Platform/Gateway release identities;
- DNS/TLS/redirect/HSTS/WAF/origin/private-ingress state;
- production database/cache/mail/queue/scheduler topology and effective privileges;
- structured logs, metrics, alerting and on-call ownership;
- backup/restore evidence and rollback mechanism;
- controlled production smoke on the authorized release.

Do not perform these operations without explicit protected-environment/production authority.

Exit gate: every production claim is tied to exact deployed identity and direct authorized evidence; remaining facts stay `UNKNOWN`.

### P1/P2 — core Account Center and Character Portfolio

Finish the user-critical account/character lifecycle before optional social polish when its authoritative dependencies exist.

Revalidate live product gaps and deliver bounded slices for:

- delete/grace/restore;
- conflict-safe rename;
- world/profile transfer only behind an accepted product decision and operation contract;
- native Character Portfolio composition under Accounts;
- canonical `CharacterId` integration and additive migration away from permanent Canary numeric identifiers where authorized;
- typed unavailable/stale/conflict/incompatible portfolio states;
- idempotent command receipts/results for game-owned mutations;
- EN/PL, accessibility, responsive and negative-path E2E.

Current native delete/restore Issue #317 and rename Issue #319 require accepted Oteryn-v2 Character Authority command/result semantics before runtime implementation. World transfer #320 additionally requires an explicit product decision. Server-owned evidence is a dependency, not implied Platform authority. If the required evidence lives only in a server repository, record the exact cross-repository blocker and do not inspect that repository without separate owner authorization.

Exit gate: the Account Center gives a player one authoritative, safe view of owned characters and supported lifecycle actions without hidden legacy identity coupling.

### P2 — LiveOps and Today

Build a first-party “what matters now?” surface rather than more disconnected pages.

#### LiveOps owner responsibilities

`docs/architecture/LIVEOPS_ARCHITECTURE.md` is the focused canonical architecture. Architecture Issue #1046 and its closeout are terminal; that is architecture evidence only and does not mean LiveOps runtime exists. `MODULE_CATALOG.md` correctly remains `LiveOps | PLANNED` until an executable capability is merged and validated.

Typed current-state projections may include:

- maintenance/server save;
- world/service status with observation age;
- status/maintenance history;
- authoritative raid/boss schedules when a producer exists;
- rotations/boosted systems when their authoritative contract exists.

The first intended runtime package is `WorldStatus + configured Maintenance`. It is not promoted to canonical selector `READY` until the exact authoritative runtime-status source required by the delivered WorldStatus capability is proven from permitted evidence. Platform configured maintenance remains independent policy authority and cannot manufacture observed runtime health. `ServerSave` is a separate blocked capability until its authoritative producer, applicability, time-base, recurrence/effective revision and freshness semantics are proven.

Never convert stale/unavailable data into `0`, `offline`, `none`, success or another fabricated fact.

#### PublicPortal Today composition

Compose from existing owners:

```text
Announcements / Events / CMS editorial state
+ LiveOps current state
+ PublicGameData community/world projections
+ authenticated PlayerCompanion private signals when permitted
```

Rules:

- source freshness/applicability/confidence is preserved;
- a focused public Today architecture/documentation package may proceed independently of a blocked runtime producer when it does not claim runtime delivery;
- public Today implementation may launch before private personalization if its exact public source dependencies are ready;
- private Today waits for the accepted owner-private cache isolation/security gate;
- public subfragments may be cached only when private state cannot influence them.

Exit gate: Today is useful under normal, stale, partial and dependency-failure states and never leaks owner-private content.

### P2 — dependency cleanup and federated public search

Before onboarding Announcements/Events as search providers, remove their reverse dependency on `PublicPortal\PublicContentState` so provider direction remains acyclic.

Then implement the accepted PublicPortal federated-search capability over source-owned queries:

- CMS/news/pages;
- Announcements/Events after reverse-edge cleanup;
- Wiki published localized search;
- GameCatalog active/verified/public-safe entities;
- other sources only after explicit public-indexability contracts.

Rules:

- source modules own eligibility/publication/localization/source-local relevance/canonical URL;
- PublicPortal owns orchestration, grouping/interleaving, normalized result envelope and partial failure;
- raw relevance scores are not assumed globally comparable;
- provider outage differs from zero results;
- private modules/data are excluded;
- raw query text is not an ordinary log field/metric label;
- future search index is rebuildable derived infrastructure, never source truth.

Before implementation starts, create or reuse one exact bounded Issue/package and prove dependency cleanup scope and ownership. Architecture maturity alone is not selector `READY`.

Exit gate: search is useful, privacy-safe, resilient to partial provider failure and has no reverse module cycle.

### P2 — first-party Client Distribution

ADR 0035 and `docs/architecture/CLIENT_DISTRIBUTION_ARCHITECTURE.md` accept the Platform-side TUF role-separated updater trust boundary. Issue #1039 is the explicit Platform implementation handoff and is reachable directly from the canonical programme after earlier entries are terminal, owned, blocked or decision-gated.

Platform implementation must preserve:

- updater-enabled immutable release/target identity and channel-scoped monotonic sequencing;
- policy revision, minimum-supported release, optional/recommended/required mode, withdrawal/revocation and explicit rollback state;
- browser publication truth separate from signed-generation/updater activation truth;
- exact signed-generation public metadata verification without private signing-key custody in Laravel;
- fail-closed channel/target/replay/generation mismatch behavior;
- idempotent ambiguity-safe reconciliation across Platform approval, protected signing/publication and Platform activation.

External updater implementation, protected signer infrastructure/key operations and production activation remain separate authority/evidence gates. They must not make the bounded Platform implementation unreachable, and Platform-only tests must not be mislabeled as real updater E2E.

Exit gate: the Platform side truthfully implements the accepted updater-distribution contract with exact-head focused/integration/security evidence and no private signer custody.

### P2 — Wiki and GameCatalog completeness

Framework existence is not content completeness. Create/maintain machine-readable expected inventories covering applicable:

- localized categories/articles/slugs/internal links;
- items, creatures, bosses, NPCs, spells, quests, achievements;
- fields, relations, media/fallbacks;
- expected counts and profile/version applicability;
- provenance and effective dates.

Close known media/failure paths rather than hiding HTTP 500s or broken references. Route Game Catalog detail through `GAME_CATALOG_PRODUCTION_COMPLETION_PROGRAM.md`; do not inspect an external producer repository without separate authorization.

Exit gate: content-completeness claims are machine-checkable against expected inventories and version/profile boundaries.

### P2/P3 — Player Companion P0

Deliver player utility as narrow vertical slices backed by authoritative/versioned data.

The Hunt Session Analyzer v1 is the terminal first complete PlayerCompanion workflow. Further tools are independent follow-up slices and must be selected/decomposed separately rather than keeping the P0 foundation perpetually non-terminal.

Recommended follow-up candidates, subject to live dependency proof:

1. hunt finder/guidance;
2. equipment explorer;
3. build/proficiency/perk planner;
4. quest/access tracker;
5. EXP/training calculators.

For every tool:

- formulas/rules carry ruleset/catalog/profile version;
- deterministic facts are distinguished from recommendations;
- private session/history/tracking data is owner-private by default;
- imports and large inputs are bounded/validated;
- shareable artifacts are explicit and revocable where applicable;
- UI explains assumptions and unavailable/incompatible inputs.

Exit gate: every promoted workflow is complete end-to-end, not merely a calculator stub.

### P3 — World Hub and community expansion

Create a World Hub only when multiple worlds/profiles or meaningful status/history signals justify it. It is a composition, not routing authority.

May combine world identity/policy, PublicGameData population/community projections, LiveOps status/history and evidence-backed GameAnalytics trends. Preserve world/profile/ruleset/season dimensions and observation age.

Do not derive current admission/routing authority from cached portal data.

### P3 — commerce activation

Wallet/Bazaar foundations do not make the portal a complete payment system.

Activate customer payments/products/entitlements only after separate gates cover:

- provider selection and signed/idempotent webhook contract;
- ledger/reconciliation and failure recovery;
- refunds/chargebacks;
- product/entitlement lifecycle and finite stale authority;
- legal/consumer/tax/invoicing decisions for launch markets;
- sandbox/real applicable E2E;
- observability, kill switch, rollback and support procedures;
- explicit owner/protected-environment authorization.

Exit gate: commercial authority cannot outlive its trusted validity and every money/value mutation is auditable/recoverable.

## Cross-cutting acceptance gates

Every task must classify scope and prove applicable rows:

| Layer | Gate |
|---|---|
| Evidence | Exact gap, owner, duplicate/overlap search. |
| Architecture | Ownership, dependency direction, contract/versioning/migration/rollback. |
| Security/privacy | Threat/abuse paths, authn/authz, validation, cache/log/privacy boundaries. |
| Persistence | Correct owner, transaction/idempotency, additive/reversible migration where needed. |
| Backend | Domain/application/adapters, failure/freshness semantics, telemetry. |
| Frontend | Real integrated UI and required states, EN/PL, accessibility, responsive behaviour. |
| Integration | Real producer/consumer path; no completion claim from mocks. |
| Tests | Risk-proportional unit/property/feature/integration/security/contract tests. |
| E2E | Real browser/runtime path or concrete non-executable `NOT_APPLICABLE` reason. |
| Operations | Rollout, feature flag when needed, observability, rollback/runbook. |
| Closeout | Exact-head self-review/CI, terminal related PRs/reviews, merge, task/Issue archive and released ownership. |

## PR policy

Before beginning a slice, inspect overlapping open/recent PRs and classify them `KEEP | FIX | REBASE | SUPERSEDED | CLOSE | NEEDS_DECISION`.

Never close because a PR is merely old or red. Close automatically only with concrete duplicate/obsolete/superseded evidence and after preserving unique work/evidence.

Do not preserve a dated list of “currently open” PR numbers here as routing authority. The exact live PR inventory and overlap disposition belongs in the selected task/selection proof and must be refreshed before mutation and before closeout.

## Completion definition

The portal programme is not globally complete while required product capabilities, production evidence or launch-critical security/authority findings remain unresolved.

A single task is complete only when its observable outcome exists and the repository's exact-head validation, E2E/N/A, review/PR hygiene, merge and task/Issue closeout rules are terminal.

`PORTAL-CLOSEOUT` is the durable short alias for selecting/resuming this programme from live state.
