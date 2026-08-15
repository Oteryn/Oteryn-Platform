# Oteryn Portal Completion Delivery Plan

## Status and authority

**CURRENT PORTAL CAPABILITY/DEPENDENCY DELIVERY PLAN — subordinate to accepted ADRs, operation-specific contracts, repository governance and `docs/architecture/PORTAL_COMPLETENESS_ARCHITECTURE.md`.**

This plan converts the portal review into a portal-specific capability/dependency order. It is not the global Platform Roadmap, live task scheduler, ownership source or production authority. Live Git/task/Issue/PR/CI state always overrides dated examples.

Companion records:

- review: `docs/agents/reports/OTERYN-20260810-portal-architecture-product-review.md`;
- architecture owner: `docs/architecture/PORTAL_COMPLETENESS_ARCHITECTURE.md`;
- programme / sole live selector: `docs/agents/programs/OTERYN_PORTAL_COMPLETION.md`;
- non-scheduling completion scope: `docs/agents/programs/OTERYN_PORTAL_COMPLETION_SCOPE.json`;
- post-selection work allocation: `docs/agents/programs/OTERYN_PORTAL_COMPLETION_WORK_ALLOCATION.md`;
- worker prompt: `docs/agents/prompts/OTERYN-PORTAL-COMPLETION-EXECUTION-PROMPT.md`;
- short alias: `PORTAL-CLOSEOUT`.

## Delivery-control hierarchy

Use this order without collapsing the layers:

1. `ROADMAP.md` — global Platform phase/risk order and phase exit gates.
2. `PORTAL_COMPLETENESS_ARCHITECTURE.md` — durable portal completion/release boundary and implement/defer/reject architecture.
3. **This plan** — portal capability/dependency order.
4. `OTERYN_PORTAL_COMPLETION.md` — sole live selector that resolves current candidates and ownership.
5. `OTERYN_PORTAL_COMPLETION_WORK_ALLOCATION.md` — post-selection role/maturity mapping only.
6. Exact Issue/task/branch/PR — one bounded execution owner.

`OTERYN_PORTAL_COMPLETION_SCOPE.json` is a non-scheduling projection of accepted launch/completion dispositions. It cannot reorder this plan, select work, claim ownership, prove live state or promote a candidate to `READY`.

A historical Roadmap phase marked complete does not imply that every later product capability in this portal plan is exhausted. Conversely, an accepted portal design does not prove runtime implementation or production readiness.

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
- explicit world/profile/ruleset/catalog/season applicability where relevant;
- exact-head validation and terminal task/PR/source-branch lifecycle.

## Architecture decision boundary

This delivery plan applies accepted architecture. A new durable module boundary, trust boundary, dependency direction or other ADR-level decision is not invented inside a delivery slice.

When a capability exposes a genuinely unresolved durable architecture question:

- search accepted ADRs/focused architecture and the architecture decision backlog;
- reuse an existing architecture owner when present;
- otherwise route the exact decision through `OTERYN_PLATFORM_ARCHITECTURE_REVIEW`;
- keep affected runtime implementation `DECISION_REQUIRED` or `BLOCKED` until accepted authority exists;
- return implementation to the normal Portal Completion owner after the decision is accepted.

Bounded documentation/decomposition work that merely applies an accepted boundary may remain inside Portal Completion and must not claim runtime delivery.

## Programme priorities

### P0 — reliable control plane and source of truth

Goal: make future autonomous **portal** task selection trustworthy.

Required outcomes:

1. live portal task files, convenience indexes, Issues and PR state are reconciled where drift materially affects routing;
2. no completed portal task remains falsely active;
3. no related portal PR lacks an intentional disposition;
4. historical state is labelled historical rather than silently overwritten;
5. GitHub connector capability is checked before any local-CLI access conclusion;
6. prompt/governance changes follow the prompt-evaluation standard;
7. `ACTIVE_WORK.md` and `PROJECT_STATE.md` never override newer active-task, PR/Issue or protected-`main` evidence.

**Repository-history boundary:** historical `RETAIN`/`RECOVERY`, historical ref preservation/deletion and steady-state branch hygiene are governed by ADR 0037/0039 and the repository Branch Lifecycle / Historical Branch Audit controls. They are not Portal Completion P0 feature work. Portal selection only consumes the resulting live ownership/ref truth.

Exit gate: an agent can determine current portal owner, exact next action and blockers from live durable state without relying on chat history or dated queue prose.

### P1 — close proven high-risk security/authority findings

Use the existing Issue-owned remediation programme; do not duplicate repair ownership.

Historical repairs such as #948, #944 and #941 are terminal examples only. Each invocation queries live open remediation Issues and routes only a currently implementation-authorized candidate through `OTERYN_PLATFORM_REMEDIATION`.

Exit gate: every launch-critical repair has exact-head validation, applicable E2E/N/A, terminal PR/Issue/task/source-branch closeout and no material finding remains.

### P1 — production/public-edge proof

Repository/staging evidence is not production proof. The production-readiness gate and current PublicEdge/OperationsObservability evidence owners control this work.

Required evidence before public-go-live claims can become true includes:

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
- canonical `CharacterId` integration and additive migration away from permanent legacy numeric identifiers where authorized;
- typed unavailable/stale/conflict/incompatible portfolio states;
- idempotent command receipts/results for game-owned mutations;
- EN/PL, accessibility, responsive and negative-path E2E.

Server-owned evidence is a dependency, not implied Platform authority. If required evidence lives only in a server/game repository, record the exact blocker and do not inspect that repository without separate owner authorization.

Exit gate: Account Center gives a player one authoritative, safe view of owned characters and supported lifecycle actions without hidden legacy identity coupling.

### P2 — LiveOps and Today

Build a first-party “what matters now?” surface rather than more disconnected pages.

#### LiveOps owner responsibilities

`docs/architecture/LIVEOPS_ARCHITECTURE.md` is the focused canonical architecture. Architecture completion does not mean LiveOps runtime exists; `MODULE_CATALOG.md` remains truthful until executable capability is merged and validated.

Typed current-state projections may include:

- maintenance/server save;
- world/service status with observation age;
- status/maintenance history;
- authoritative raid/boss schedules when a producer exists;
- rotations/boosted systems when their authoritative contract exists.

The first intended runtime package is `WorldStatus + configured Maintenance`. It is not promoted to live selector `READY` until the exact authoritative runtime-status source required by the capability is proven from permitted evidence. Platform configured maintenance remains independent policy authority and cannot manufacture observed runtime health. `ServerSave` remains separately gated until its authoritative producer, applicability, time-base, recurrence/effective revision and freshness semantics are proven.

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
- focused public Today architecture/documentation may proceed independently when it does not claim blocked runtime delivery;
- public Today implementation may launch before private personalization when its exact public source dependencies are ready;
- private Today participates only when its accepted completion-scope trigger and ADR 0032 identity/privacy/cache gates are satisfied;
- public subfragments may be cached only when private state cannot influence them.

Exit gate: Today is useful under normal, stale, partial and dependency-failure states and never leaks owner-private content.

### P2 — dependency cleanup and federated public search

Before onboarding Announcements/Events as search providers, remove their reverse dependency on `PublicPortal\PublicContentState` so provider direction remains acyclic.

Then implement accepted PublicPortal federated search over source-owned queries:

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

ADR 0035 and `docs/architecture/CLIENT_DISTRIBUTION_ARCHITECTURE.md` accept the Platform-side TUF role-separated updater trust boundary. A live implementation handoff is selected only when its completion-scope trigger and current ownership/eligibility are proven.

Platform implementation must preserve:

- updater-enabled immutable release/target identity and channel-scoped monotonic sequencing;
- policy revision, minimum-supported release, optional/recommended/required mode, withdrawal/revocation and explicit rollback state;
- browser publication truth separate from signed-generation/updater activation truth;
- exact signed-generation public metadata verification without private signing-key custody in Laravel;
- fail-closed channel/target/replay/generation mismatch behavior;
- idempotent ambiguity-safe reconciliation across Platform approval, protected signing/publication and Platform activation.

External updater implementation, protected signer infrastructure/key operations and production activation remain separate authority/evidence gates. Platform-only tests must never be mislabeled as real updater E2E.

Exit gate: the Platform side truthfully implements the accepted updater-distribution contract with exact-head focused/integration/security evidence and no private signer custody.

### P2 — Wiki and GameCatalog completeness

Framework existence is not content completeness. Create/maintain machine-readable expected inventories covering applicable:

- localized categories/articles/slugs/internal links;
- items, creatures, bosses, NPCs, spells, quests, achievements;
- fields, relations, media/fallbacks;
- expected counts and profile/version applicability;
- provenance and effective dates.

Close known media/failure paths rather than hiding HTTP 500s or broken references. Route specialized Game Catalog detail through `GAME_CATALOG_PRODUCTION_COMPLETION_PROGRAM.md`, but current repository governance still controls whether any server/game repository may be accessed.

Exit gate: content-completeness claims are machine-checkable against expected inventories and version/profile boundaries.

### P2/P3 — Player Companion

The Hunt Session Analyzer v1 is the terminal first complete PlayerCompanion foundation workflow. It is not a perpetual open P0 item.

Further tools are independent **conditional** vertical slices and participate only when individually promoted by accepted product scope and their authoritative/versioned dependencies are proven:

1. hunt finder/guidance;
2. equipment explorer;
3. character build planner;
4. charm/perk/proficiency planner;
5. quest/access tracker;
6. EXP/training calculators;
7. validated shareable builds.

For every promoted tool:

- formulas/rules carry ruleset/catalog/profile version;
- deterministic facts are distinguished from recommendations;
- private session/history/tracking data is owner-private by default;
- imports and large inputs are bounded/validated;
- shareable artifacts are explicit and revocable where applicable;
- UI explains assumptions and unavailable/incompatible inputs.

Exit gate: every promoted workflow is complete end-to-end, not merely a calculator stub.

### Cross-cutting — multi-world, profiles, rulesets and seasons

This is a conditional invariant, not an independent standing queue item.

Every selected capability must preserve applicable `world`, `profile`, `ruleset`, `catalog snapshot`, `season` and effective-period dimensions in URLs, cache keys, projections, events, formulas and persistence where relevant. A single-world launch must not create an irreversible global assumption.

If accepted architecture already defines the dimensions, implement them inside the selected slice. If the slice would require a new durable identity/dimension/compatibility decision, route that exact question through Architecture Review before runtime work. Do not create speculative multi-world infrastructure solely to satisfy the invariant.

### P3 — World Hub and community expansion

Create World Hub only after its deferred/conditional scope is explicitly promoted and multiple worlds/profiles or meaningful authoritative status/history signals justify it. It remains composition, never routing/admission authority.

Optional community read surfaces are product inventory inputs, not automatic launch requirements. Forum remains deferred unless durable owned discussion/moderation need is accepted.

### P3 — commerce activation

Wallet/Bazaar foundations do not make the portal a complete payment system.

Commerce capability participates only after an explicit product/business disposition. Production activation remains deferred/blocked until separate gates cover:

- provider selection and signed/idempotent webhook contract;
- ledger/reconciliation and failure recovery;
- refunds/chargebacks;
- product/entitlement lifecycle and finite stale authority;
- legal/consumer/tax/invoicing decisions for launch markets;
- sandbox/real applicable E2E;
- observability, kill switch, rollback and support procedures;
- explicit owner/protected-environment authorization.

Exit gate: commercial authority cannot outlive trusted validity and every money/value mutation is auditable/recoverable.

## Cross-cutting acceptance gates

Every selected task must classify scope and prove applicable rows:

| Layer | Gate |
|---|---|
| Evidence | Exact gap, owner, completion-scope disposition/trigger and duplicate/overlap search. |
| Architecture | Ownership, dependency direction, accepted decision/contract/versioning/migration/rollback. |
| Security/privacy | Threat/abuse paths, authn/authz, validation, cache/log/privacy boundaries. |
| Persistence | Correct owner, transaction/idempotency, additive/reversible migration where needed. |
| Backend | Domain/application/adapters, failure/freshness semantics, telemetry. |
| Frontend | Real integrated UI and required states, EN/PL, accessibility, responsive behaviour. |
| Integration | Real producer/consumer path; no completion claim from mocks. |
| Multi-world applicability | Preserve applicable world/profile/ruleset/catalog/season/effective-period dimensions or route the exact unresolved durable decision. |
| Tests | Risk-proportional unit/property/feature/integration/security/contract tests. |
| E2E | Real browser/runtime path or concrete non-executable `NOT_APPLICABLE` reason. |
| Operations | Rollout, feature flag when needed, observability, rollback/runbook. |
| Closeout | Exact-head self-review/CI, terminal related PRs/reviews, merge, task/Issue archive, source-branch/resource closeout and released ownership. |

## PR policy

Before beginning a slice, inspect overlapping open/recent PRs and classify them `KEEP | FIX | REBASE | SUPERSEDED | CLOSE | NEEDS_DECISION`.

Never close because a PR is merely old or red. Close automatically only with concrete duplicate/obsolete/superseded evidence and after preserving unique work/evidence.

Do not preserve a dated list of “currently open” PR numbers here as routing authority. Exact live PR inventory and overlap disposition belongs in the selected task/selection proof and is refreshed before mutation and closeout.

## Completion definition

Global portal completion uses `OTERYN_PORTAL_COMPLETION_SCOPE.json` only as a **non-scheduling disposition projection**. Completion still requires live proof:

- every `REQUIRED` item has a terminal implement/defer/reject disposition;
- every `CONDITIONAL` item whose accepted activation trigger is active has a terminal disposition;
- deferred/inactive conditional work is not falsely described as implemented;
- no launch-critical security/authority finding remains unresolved;
- current exact-head implementation/E2E/CI/PR/task/source-branch lifecycle is terminal for delivered slices;
- production/go-live claims have direct authorized evidence for the exact deployed identity.

A single task is complete only when its observable outcome exists and repository exact-head validation, E2E/N/A, review/PR hygiene, merge, task/Issue and source-branch/resource closeout rules are terminal.

`PORTAL-CLOSEOUT` remains the durable short alias for selecting/resuming this programme from live state.
