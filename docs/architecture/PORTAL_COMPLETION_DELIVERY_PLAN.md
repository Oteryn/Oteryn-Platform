# Oteryn Portal Completion Delivery Plan

## Status and authority

**CURRENT DELIVERY PLAN — subordinate to accepted ADRs, operation-specific contracts, repository governance and `docs/architecture/PORTAL_COMPLETENESS_ARCHITECTURE.md`.**

This plan converts the 2026-08-10 portal review into an implementation order. It does not itself authorize production, protected-environment, payment, live-data or external-repository mutation. Live Git/task/Issue/PR/CI state always overrides dated queue examples.

Companion records:

- review: `docs/agents/reports/OTERYN-20260810-portal-architecture-product-review.md`;
- programme: `docs/agents/programs/OTERYN_PORTAL_COMPLETION.md`;
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

## Programme priorities

### P0 — reliable control plane and source of truth

Goal: make future autonomous task selection trustworthy.

Required outcomes:

1. live task files, `ACTIVE_WORK.md`, `PROJECT_STATE.md`, Issues and PR state are reconciled where drift materially affects routing;
2. no completed task remains falsely active;
3. no open related PR lacks an intentional disposition;
4. historical state is labelled historical rather than silently overwritten;
5. GitHub connector capability is checked before any local-CLI access conclusion;
6. prompt/governance changes follow the prompt-evaluation standard.

Current known drift includes `ACTIVE_WORK.md` reporting no active work while two blocked active task files exist. At the corrected 2026-08-10 closeout refresh PRs #961, #541 and #338 were open: #961 is separate synthetic/no-network research work currently requiring `FIX`, while #541/#338 retain their evidence-dependent portal-review dispositions.

Exit gate: an agent can determine current owner, exact next action and blockers from live durable state without relying on chat history.

### P1 — close proven high-risk security/authority findings

Use the existing Issue-owned remediation programme; do not duplicate repair ownership.

Current live priorities at the 2026-08-10 refresh:

1. **#948 — immutable Download Center artifact reference**
   - enforce machine-testable immutable reference identity;
   - preserve approved-host/scheme/path safeguards;
   - reject mutable aliases/overwriteable references unless storage contract proves immutability;
   - revalidate at publication time;
   - do not falsely claim administrator SHA-256 was independently verified.

2. **#944 — bounded entitlement stale-authority lease**
   - define finite game-consumed authority validity;
   - bind lifecycle revision/effective interval;
   - distinguish current, stale-within-bound, unavailable, expired and revoked authority;
   - prevent rollback/resurrection from delayed or cached older states;
   - cover outage, reconnect, restart, expiry/revoke and clock/skew semantics;
   - do not claim Premium/VIP runtime is already deployed.

3. **#941 — private Today cache isolation**
   - classify any representation containing owner-private PlayerCompanion state as private;
   - prevent shared/CDN/anonymous cache reuse;
   - separate guest and authenticated variants;
   - bind any private server cache to owner/security/privacy revisions;
   - prove two-user and auth↔guest/logout/privacy-tightening isolation.

Exit gate: each repair reaches exact-head validation, applicable E2E/N/A, terminal PR/Issue/task closeout and no material finding remains.

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

Finish the user-critical account/character lifecycle before optional social polish.

Revalidate live product gaps and deliver bounded slices for:

- delete/grace/restore;
- conflict-safe rename;
- world/profile transfer only behind an accepted operation contract;
- native Character Portfolio composition under Accounts;
- canonical `CharacterId` integration and additive migration away from permanent Canary numeric identifiers where authorized;
- typed unavailable/stale/conflict/incompatible portfolio states;
- idempotent command receipts/results for game-owned mutations;
- EN/PL, accessibility, responsive and negative-path E2E.

Server-owned evidence is a dependency, not implied Platform authority. If a slice requires Oteryn-v2/Canary inspection, record the cross-repository decision gate and stop that part until separately authorized.

Exit gate: the Account Center gives a player one authoritative, safe view of owned characters and supported lifecycle actions without hidden legacy identity coupling.

### P2 — LiveOps and Today

Build a first-party “what matters now?” surface rather than more disconnected pages.

#### LiveOps owner responsibilities

Typed current-state projections may include:

- maintenance/server save;
- world/service status with observation age;
- status/maintenance history;
- authoritative raid/boss schedules when a producer exists;
- rotations/boosted systems when their authoritative contract exists.

Never convert stale/unavailable data into `0`, `offline`, `none` or another fabricated fact.

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
- public Today can launch before private personalization;
- private Today waits for the #941-equivalent cache isolation gate;
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

Exit gate: search is useful, privacy-safe, resilient to partial provider failure and has no reverse module cycle.

### P2 — Wiki and GameCatalog completeness

Framework existence is not content completeness. Create/maintain machine-readable expected inventories covering applicable:

- localized categories/articles/slugs/internal links;
- items, creatures, bosses, NPCs, spells, quests, achievements;
- fields, relations, media/fallbacks;
- expected counts and profile/version applicability;
- provenance and effective dates.

Close known media/failure paths rather than hiding HTTP 500s or broken references.

Exit gate: content-completeness claims are machine-checkable against expected inventories and version/profile boundaries.

### P2/P3 — Player Companion P0

Deliver player utility as narrow vertical slices backed by authoritative/versioned data.

Recommended first candidates, subject to live dependency proof:

1. loot/session split/analyzer;
2. hunt finder/guidance;
3. equipment explorer;
4. build/proficiency/perk planner;
5. quest/access tracker;
6. EXP/training calculators.

For every tool:

- formulas/rules carry ruleset/catalog/profile version;
- deterministic facts are distinguished from recommendations;
- private session/history/tracking data is owner-private by default;
- imports and large inputs are bounded/validated;
- shareable artifacts are explicit and revocable where applicable;
- UI explains assumptions and unavailable/incompatible inputs.

Exit gate: at least one complete, repeatedly useful player workflow exists end-to-end, not merely a calculator stub.

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

At the corrected 2026-08-10 closeout refresh #961, #541 and #338 were open. #961 is a separate synthetic/no-network research `FIX` candidate with red exact-head governance/CI/harness validation; #541 and #338 retain the evidence-dependent dispositions documented in the dated review. Future invocations must always revalidate this inventory from live GitHub state.

## Completion definition

The portal programme is not globally complete while required product capabilities, production evidence or P1 security/authority findings remain unresolved.

A single task is complete only when its observable outcome exists and the repository's exact-head validation, E2E/N/A, review/PR hygiene, merge and task/Issue closeout rules are terminal.

`PORTAL-CLOSEOUT` is the durable short alias for selecting/resuming this programme from live state.
