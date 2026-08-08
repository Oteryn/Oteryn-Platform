---
task_id: OTERYN-20260809-federated-search-architecture
programme_id: OTERYN_PLATFORM_ARCHITECTURE_REVIEW
project_lane: oteryn-platform-content
task_kind: architecture
implementation_authorized: true
execution_mode: github
execution_reason: WWW-only architecture can be completed through canonical documentation and GitHub validation
status: implementing
required_reads:
  - AGENTS.md
  - AGENTS.override.md
  - docs/agents/REPOSITORY_MAP.md
  - docs/agents/CONTEXT_ROUTING.md
  - docs/architecture/ARCHITECTURE_AUTHORITY.md
  - docs/architecture/PORTAL_COMPLETENESS_ARCHITECTURE.md
  - docs/architecture/MODULE_CATALOG.md
  - docs/architecture/adr/README.md
  - docs/architecture/adr/0032-portal-composition-tracking-and-server-system-ownership.md
search_first:
  - open Platform PRs and active task ownership
  - existing public search/discoverability architecture
---

# OTERYN-20260809-federated-search-architecture

## Goal

Resolve Issue #935 by defining the durable WWW Platform ownership and contracts for federated public content search without introducing a second source of truth, a duplicate business-logic path or a new deployable service without measured need.

## Acceptance criteria

- [ ] Decide whether federated search is a new top-level module or an existing-module capability.
- [ ] Define source-provider boundaries for CMS/news, Announcements, Events, Wiki, GameCatalog and explicitly approved public PlayerCompanion references.
- [ ] Keep PublicGameData exact-name character search semantically separate unless a later bounded composition explicitly includes it.
- [ ] Preserve source-owned publication, permission, localization, freshness and canonical URL semantics.
- [ ] Define common result identity, provenance, snippets/highlights, pagination and partial-failure behavior.
- [ ] Define bounded query/rate/abuse, cache/index and observability rules.
- [ ] Define future PlatformAPI reuse without duplicating orchestration or ranking logic.
- [ ] Record durable ownership in ADR 0033 and reconcile `MODULE_CATALOG.md`, `PORTAL_COMPLETENESS_ARCHITECTURE.md` and a focused search architecture.
- [ ] Do not access server/client repositories or implement runtime/schema/routes/indexing/production changes.
- [ ] Complete exact-head self-review, required CI, review hygiene, squash merge and lifecycle archive closeout.

## Ownership

```yaml
owned_paths:
  - docs/agents/tasks/active/OTERYN-20260809-federated-search-architecture.md
  - docs/architecture/FEDERATED_SEARCH_ARCHITECTURE.md
  - docs/architecture/PORTAL_COMPLETENESS_ARCHITECTURE.md
  - docs/architecture/MODULE_CATALOG.md
  - docs/architecture/adr/README.md
  - docs/architecture/adr/0033-federated-content-search-and-discoverability.md
modules:
  - PublicPortal
  - CMS
  - Announcements
  - Events
  - Wiki
  - GameCatalog
  - PlayerCompanion
  - PublicGameData
  - PlatformAPI
dependencies:
  - ADR 0022
  - ADR 0025
  - ADR 0032
  - Issue #935
blockers:
  - none
cross_repository_tasks:
  - none
```

Open PR #338 owns GameCatalog consumer implementation paths and does not overlap this documentation package. Open PR #541 owns the existing public-domain task record and does not overlap this task.

## Working decision

The candidate minimum-module design is:

- keep federated public content search inside `PublicPortal` as an application-level `FederatedSearch` / discovery capability;
- source modules continue to own content, publication/visibility rules, localized search semantics and canonical detail URLs;
- PublicPortal provider adapters call source-module application queries and normalize only allowlisted public result metadata;
- PublicPortal owns deterministic cross-source composition/grouping policy and public search presentation, not source relevance internals or source data;
- future `PlatformAPI` reuses the same PublicPortal application service when exposing the same first-party search product;
- exact-name character search remains a separate PublicGameData function and is not silently blended into public content ranking;
- no dedicated search engine/index is required by architecture; any later index is a rebuildable derived projection, never source truth.

This decision is not accepted until the canonical docs, ADR and exact-head validation are complete.

## Validation gate

```yaml
validation_gate:
  version: 2
  intensity: HEIGHTENED
  classified_by: implementation owner
  classified_at: 2026-08-09T00:47:00+02:00
  risk: medium
  triggers:
    - durable architecture
    - cross-module dependency direction
    - public discoverability and privacy boundary
    - future API reuse contract
  unknown_or_conflict: []
  rationale: >-
    The task is documentation-only and reversible, but it establishes durable search
    ownership and dependency direction across multiple public modules. HEIGHTENED
    architecture validation is therefore appropriate even though no runtime, schema,
    deployment or external environment is mutated.
  self_review:
    result: PASS
    exact_head: none
    evidence:
      - final exact-head full-diff review will be anchored as a PR review after the final repository-file commit
      - source ownership, negative paths, rollback, compatibility and related PR ownership are mandatory review dimensions
```

## Context checkpoint

```yaml
checkpoint_version: 1
policy_version: 2
updated_at: 2026-08-09T00:47:00+02:00
invocation_started_at: 2026-08-09T00:43:00+02:00
last_progress_at: 2026-08-09T00:47:00+02:00
head: OUT_OF_BAND_CURRENT_BRANCH_HEAD
branch: docs/OTERYN-20260809-federated-search-architecture
pr: none
status: implementing
phase: architecture-contract
session_id: agent-20260809-0043-federated-search
session_role: architecture
project_lane: oteryn-platform-content
execution_mode: github
context_routes:
  - architecture
  - web-cms
owned_paths:
  - docs/agents/tasks/active/OTERYN-20260809-federated-search-architecture.md
  - docs/architecture/FEDERATED_SEARCH_ARCHITECTURE.md
  - docs/architecture/PORTAL_COMPLETENESS_ARCHITECTURE.md
  - docs/architecture/MODULE_CATALOG.md
  - docs/architecture/adr/README.md
  - docs/architecture/adr/0033-federated-content-search-and-discoverability.md
context_pressure: low
context_growth: stable
estimate_confidence: high
decomposition_decision: single
proven:
  - main is 08b83c42e12d4f26904c5c0e480a503687091521 at task start
  - Issue #302 is terminal after PR #933 architecture dispositions
  - federated content search remains DISCOVERY in current portal completeness architecture
  - PublicPortal already owns SEO/discoverability and bounded public composition
  - Wiki has localized published-only search and PublicGameData has a separate public search boundary
  - open PRs #338 and #541 do not overlap owned paths
derived:
  - a PublicPortal application-level federation capability is the minimum-module candidate
unknown: []
conflicts: []
first_failure:
  marker: none
  evidence: no validation failure observed yet
rejected_hypotheses:
  - federated search must be a standalone microservice
  - source modules should export raw database models to a global index
  - exact-name character search should be silently mixed into content relevance
  - public search may index private PlayerCompanion workspaces because the user owns them
changed_paths:
  - docs/agents/tasks/active/OTERYN-20260809-federated-search-architecture.md
validation:
  - command: live ownership and open-PR inspection
    result: PASS
    evidence: only PR #338 and #541 are open and neither owns this architecture package
  - command: Issue #302 architecture-discovery closure
    result: PASS
    evidence: Issue #302 closed completed after merged PR #933 supplied durable dispositions
  - command: runtime E2E
    result: NOT_APPLICABLE
    evidence: architecture/documentation task does not change executable routes, schema, runtime, configuration or deployment
repair_cycles_for_current_gate: 0
blockers:
  - none
next_action: create ADR 0033 and focused federated-search architecture, then reconcile PublicPortal ownership in canonical docs and validate exact head
```

## Notes

This task is WWW Platform-only. It must not inspect, search or mutate Oteryn-v2, Canary or client repositories.