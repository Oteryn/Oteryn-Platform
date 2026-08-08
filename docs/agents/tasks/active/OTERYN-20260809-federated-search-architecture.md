---
task_id: OTERYN-20260809-federated-search-architecture
programme_id: OTERYN_PLATFORM_ARCHITECTURE_REVIEW
project_lane: oteryn-platform-content
task_kind: architecture
implementation_authorized: true
execution_mode: github
execution_reason: WWW-only architecture can be completed through canonical documentation and GitHub validation
status: validating
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

- [x] Decide whether federated search is a new top-level module or an existing-module capability.
- [x] Define source-provider boundaries for CMS/news, Announcements, Events, Wiki, GameCatalog and explicitly approved public PlayerCompanion references.
- [x] Keep PublicGameData exact-name character search semantically separate unless a later bounded composition explicitly includes it.
- [x] Preserve source-owned publication, permission, localization, freshness and canonical URL semantics.
- [x] Define common result identity, provenance, snippets/highlights, pagination and partial-failure behavior.
- [x] Define bounded query/rate/abuse, cache/index and observability rules.
- [x] Define future PlatformAPI reuse without duplicating orchestration or ranking logic.
- [x] Record durable ownership in ADR 0033 and reconcile `MODULE_CATALOG.md`, `PORTAL_COMPLETENESS_ARCHITECTURE.md` and a focused search architecture.
- [x] Do not access server/client repositories or implement runtime/schema/routes/indexing/production changes.
- [ ] Complete exact-head self-review, required CI, review hygiene, squash merge, Issue #935 closure and lifecycle archive closeout.

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
  - ADR 0033
  - Issue #935
blockers:
  - none
cross_repository_tasks:
  - none
```

Open PR #338 owns GameCatalog consumer implementation paths and does not overlap this documentation package. Open PR #541 owns the existing public-domain task record and does not overlap this task.

## Delivered architecture decision

The minimum-module design is now recorded by ADR 0033 and synchronized with the canonical architecture:

- federated public content search stays inside `PublicPortal` as an application-level `FederatedSearch` / discoverability capability;
- source modules continue to own content, public/search eligibility, publication/visibility rules, localized search semantics, source-local relevance and canonical detail URLs;
- PublicPortal provider adapters call bounded source-module application queries and normalize only allowlisted public result metadata;
- PublicPortal owns deterministic cross-source composition/grouping/interleaving policy, partial-failure semantics and public search presentation, not source relevance internals or source data;
- future `PlatformAPI` reuses the same PublicPortal application service when exposing the same first-party search product;
- exact-name character search remains a separate `PublicGameData` product/vertical and is not silently blended into fuzzy public content ranking;
- Marketplace search/filtering remains Marketplace-owned;
- private PlayerCompanion, Support, Admin, Audit, Identity and Accounts data remains excluded from public federation by default;
- no dedicated search engine/index is required by architecture; any later index is a rebuildable derived projection, never source truth.

The focused provider/result/ranking/failure/cache/privacy/SEO/observability/API contract is `docs/architecture/FEDERATED_SEARCH_ARCHITECTURE.md`. `PORTAL_COMPLETENESS_ARCHITECTURE.md` now moves federated content search from `DISCOVERY` to `ARCHITECTURE ACCEPTED / PLANNED`, and `MODULE_CATALOG.md` records the same PublicPortal responsibility without claiming runtime implementation.

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
      - final exact-head full-diff review is anchored as a PR review after this final repository-file checkpoint
      - all six owned paths must be reviewed together against ADR 0033 and source-module authority
      - negative paths include private-data exclusion, exact-name character-search separation, explicit partial failure and derived-index non-authority
      - rollback is documentation-only and compatibility preserves existing module search/publication ownership
      - open PR #338 and #541 ownership is non-overlapping
```

## Heightened validation requirements

- **Architecture consistency:** ADR 0033, `FEDERATED_SEARCH_ARCHITECTURE.md`, `MODULE_CATALOG.md` and `PORTAL_COMPLETENESS_ARCHITECTURE.md` express the same ownership and dependency direction.
- **Negative paths:** no raw cross-module model/table access; no private-data federation; no fuzzy character enumeration by implication; no fabricated zero-result state for failed providers; no global comparability assumption for unrelated provider scores; no external search engine as hidden authority.
- **Rollback:** revert the bounded documentation package; no schema/data/runtime rollback is required.
- **Compatibility:** existing Wiki/PublicGameData/Marketplace/source-module search semantics remain authoritative and unchanged; ADR 0033 does not supersede ADR 0025 or ADR 0032.
- **Query privacy/security:** raw search terms are not ordinary log fields or metric labels; query/filter/rate/payload boundaries are explicit.
- **PlatformAPI:** future API reuse points to one PublicPortal application service rather than a parallel orchestration path.
- **E2E:** `NOT_APPLICABLE` because this task creates no executable route, schema, index, runtime, configuration or deployment path.
- **Final CI:** repository-required checks must pass on the unchanged exact final PR head.
- **Review hygiene:** zero unresolved material review threads and zero requested changes before merge.

## Exact-head self-review mechanism

The task file cannot embed the SHA of the commit containing that same SHA without moving the head. The final self-review is therefore recorded as an anchored PR review against the exact live head after this checkpoint commit. Any later repository-file change invalidates that review and requires a new exact-head review and CI generation.

Required shape:

```yaml
self_review:
  result: PASS
  exact_head: <live final PR head>
  acceptance_checked: true
  full_diff_checked: true
  related_prs_checked: true
  negative_paths_checked: true
  rollback_checked: true
  compatibility_checked: true
  findings: []
  evidence:
    - six-path full PR diff
    - ADR 0033 plus ADR inventory
    - canonical MODULE_CATALOG reconciliation
    - focused FEDERATED_SEARCH_ARCHITECTURE contract
    - PORTAL_COMPLETENESS_ARCHITECTURE disposition change
```

## Context checkpoint

```yaml
checkpoint_version: 1
policy_version: 2
updated_at: 2026-08-09T00:54:00+02:00
invocation_started_at: 2026-08-09T00:43:00+02:00
last_progress_at: 2026-08-09T00:54:00+02:00
head: OUT_OF_BAND_FINAL_HEAD_AFTER_THIS_COMMIT
branch: docs/OTERYN-20260809-federated-search-architecture
pr: 936
status: validating
phase: heightened-exact-head-validation
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
  - main was 08b83c42e12d4f26904c5c0e480a503687091521 at task start
  - Issue #302 is terminal after PR #933 architecture dispositions
  - ADR 0033 is created and registered in the ADR inventory
  - FEDERATED_SEARCH_ARCHITECTURE defines provider/result/ranking/failure/cache/privacy/security/SEO/observability/API boundaries
  - MODULE_CATALOG records federated-search orchestration under PublicPortal without claiming implementation
  - PORTAL_COMPLETENESS_ARCHITECTURE moves federated search from DISCOVERY to ARCHITECTURE ACCEPTED / PLANNED
  - PublicGameData exact-name character lookup remains explicitly separate
  - a later dedicated search index is derived/rebuildable rather than source truth
  - open PRs #338 and #541 do not overlap owned paths
  - runtime E2E is NOT_APPLICABLE for this documentation-only package
derived:
  - PublicPortal is the minimum-module owner because its existing responsibility already includes public SEO/discoverability/composition
unknown: []
conflicts: []
first_failure:
  marker: none
  evidence: no validation failure observed yet
rejected_hypotheses:
  - federated search requires a standalone microservice
  - federated search requires a new top-level Discovery module now
  - source modules should export raw database models to a global index
  - raw relevance scores from heterogeneous providers can be sorted directly as one score
  - exact-name character search should be silently mixed into fuzzy public content relevance
  - public search may index private PlayerCompanion workspaces or share-token artefacts by default
  - an external search engine must be selected before the architecture can be implemented
changed_paths:
  - docs/agents/tasks/active/OTERYN-20260809-federated-search-architecture.md
  - docs/architecture/FEDERATED_SEARCH_ARCHITECTURE.md
  - docs/architecture/PORTAL_COMPLETENESS_ARCHITECTURE.md
  - docs/architecture/MODULE_CATALOG.md
  - docs/architecture/adr/README.md
  - docs/architecture/adr/0033-federated-content-search-and-discoverability.md
validation:
  - command: live ownership and open-PR inspection
    result: PASS
    evidence: open PR #338 and #541 do not own any of the six architecture paths
  - command: canonical architecture reconciliation
    result: PASS
    evidence: ADR 0033, focused search architecture, MODULE_CATALOG and portal completeness carry the same PublicPortal/source-module ownership direction
  - command: Issue #302 architecture-discovery closure
    result: PASS
    evidence: Issue #302 closed completed after merged PR #933 supplied durable dispositions for its original optional-tool questions
  - command: runtime E2E
    result: NOT_APPLICABLE
    evidence: documentation-only package changes no executable route, schema, index, runtime, configuration or deployment
  - command: exact-head repository CI
    result: NOT_RUN
    evidence: must run after this final repository-file checkpoint on the exact PR head
repair_cycles_for_current_gate: 0
blockers:
  - none
next_action: perform exact-head full-diff self-review on PR #936, request fresh Codex review, verify required exact-head CI and merge only with zero material findings and zero unresolved threads; then close Issue #935 and archive this task
```

## Notes

This task is WWW Platform-only. It must not inspect, search or mutate Oteryn-v2, Canary or client repositories.