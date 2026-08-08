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
- [x] Define bounded query/rate/abuse, cache/index and observability rules, including privacy-safe collision-resistant cache query identity.
- [x] Define future PlatformAPI reuse without duplicating orchestration or ranking logic.
- [x] Record durable ownership in ADR 0033 and reconcile `MODULE_CATALOG.md`, `PORTAL_COMPLETENESS_ARCHITECTURE.md` and a focused search architecture.
- [x] Account explicitly for existing Announcements/Events -> PublicPortal compatibility dependencies before accepting the target PublicPortal -> provider direction.
- [x] Do not access server/client repositories or implement runtime/schema/routes/indexing/production changes.
- [ ] Complete final repaired exact-head self-review, fresh Codex review, required CI, zero unresolved threads, squash merge, Issue #935 closure and lifecycle archive closeout.

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

The minimum-module design is recorded by ADR 0033 and synchronized with the canonical architecture:

- federated public content search stays inside `PublicPortal` as an application-level `FederatedSearch` / discoverability capability;
- source modules continue to own content, public/search eligibility, publication/visibility rules, localized search semantics, source-local relevance and canonical detail URLs;
- PublicPortal provider adapters call bounded source-module application queries and normalize only allowlisted public result metadata;
- PublicPortal owns deterministic cross-source composition/grouping/interleaving policy, partial-failure semantics and public search presentation, not source relevance internals or source data;
- future `PlatformAPI` reuses the same PublicPortal application service when exposing the same first-party search product;
- exact-name character search remains a separate `PublicGameData` product/vertical and is not silently blended into fuzzy public content ranking;
- Marketplace search/filtering remains Marketplace-owned;
- private PlayerCompanion, Support, Admin, Audit, Identity and Accounts data remains excluded from public federation by default;
- no dedicated search engine/index is required by architecture; any later index is a rebuildable derived projection, never source truth;
- result caches, if used, identify the canonical normalized query through a versioned server-keyed digest such as HMAC-SHA-256 plus locale/filter/provider/generation identity, never raw query text or an unkeyed/plain hash.

### Review repair cycle 1 — existing reverse dependency

Codex review of exact head `d4e5162f7948bf216217c9a224c699ed33799e38` correctly identified that existing Announcements and Events homepage provider/view-model code imports `App\PublicPortal\PublicContentState`. Therefore a literal claim that all source modules already have no reverse PublicPortal dependency was false.

Repair cycle 1 updated ADR 0033, the focused search architecture, `MODULE_CATALOG.md` and portal completeness to make the distinction explicit:

- `PublicPortal -> source-module application query` is the **target federated-search provider direction**;
- the current Announcements/Events -> PublicPortal `PublicContentState` edge is compatibility debt, not accepted search direction;
- Announcements or Events cannot be onboarded as a federated-search provider until that reverse edge is removed;
- preferred cleanup uses source-owned application response/availability types which PublicPortal maps into its own composition/search state;
- a new opposite search edge must not be added while the reverse dependency still exists;
- ADR 0033 does not authorize a generic shared dumping-ground module to hide the cycle.

### Review repair cycle 2 — cache correctness and query privacy

Fresh Codex review of exact head `c18f1797edfeb057b567d2c2b2116e94d07f5e58` correctly identified that the cache policy named locale/filter/provider/index generation but omitted a privacy-safe identity for the normalized query itself. Distinct terms could therefore collide into one cache identity, while simply adding raw query text would violate the query-privacy boundary.

Repair cycle 2 updates ADR 0033 and the focused search architecture so that:

- every result-cache identity includes a collision-resistant identity for the canonical normalized query;
- the identity is a versioned server-keyed digest such as `HMAC-SHA-256(cache-key-secret, normalized-query)`;
- raw query text and unkeyed/plain query hashes are forbidden as the privacy mechanism;
- the cache-key secret is managed as secret material and its key/version namespaces cache generations so rotation is deterministic;
- the full digest is used according to the implementation collision-resistance requirement;
- query digests are not ordinary logs, metric labels, analytics identifiers or authorization tokens;
- implementation validation must prove distinct-query cache isolation and key-rotation namespace behavior.

The focused provider/result/ranking/failure/cache/privacy/SEO/observability/API/dependency-cleanup contract is `docs/architecture/FEDERATED_SEARCH_ARCHITECTURE.md`. `PORTAL_COMPLETENESS_ARCHITECTURE.md` moves federated content search from `DISCOVERY` to `ARCHITECTURE ACCEPTED / PLANNED`, with Announcements/Events onboarding explicitly gated by compatibility cleanup. `MODULE_CATALOG.md` records the same PublicPortal responsibility and current reverse-edge debt without claiming runtime implementation.

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
    - material review-driven architecture repair
  unknown_or_conflict: []
  rationale: >-
    The task is documentation-only and reversible, but it establishes durable search
    ownership, dependency direction and privacy-safe cache identity across multiple
    public modules. HEIGHTENED architecture validation remains appropriate even though
    no runtime, schema, deployment or external environment is mutated.
  self_review:
    result: PASS
    exact_head: none
    evidence:
      - final repaired exact-head full-diff review will be anchored as a PR review after this final repository-file checkpoint
      - all six owned paths must be reviewed together against ADR 0033, current repository dependency evidence, cache/query privacy and source-module authority
      - negative paths include dependency-cycle creation, private-data exposure, fuzzy character enumeration, fabricated zero-result state, cache cross-query collision/raw-query leakage and derived-index authority
      - rollback is documentation-only and compatibility preserves existing source behavior while making cleanup a future implementation prerequisite
      - open PR #338 and #541 ownership is non-overlapping
```

## Heightened validation requirements

- **Architecture consistency:** ADR 0033, `FEDERATED_SEARCH_ARCHITECTURE.md`, `MODULE_CATALOG.md` and `PORTAL_COMPLETENESS_ARCHITECTURE.md` express the same ownership, target dependency direction and compatibility-cleanup prerequisite.
- **Current dependency truth:** existing Announcements/Events imports of `PublicPortal\PublicContentState` are recorded explicitly rather than hidden by the target diagram.
- **Cache correctness/privacy:** distinct normalized queries cannot share one result-cache identity; raw terms and plain/unkeyed hashes are not used as cache privacy boundaries; keyed digest version/rotation creates a deterministic namespace boundary.
- **Negative paths:** no new PublicPortal/provider dependency cycle; no raw cross-module model/table access; no private-data federation; no fuzzy character enumeration by implication; no fabricated zero-result state for failed providers; no global comparability assumption for unrelated provider scores; no cache cross-query collision/raw-query key leakage; no external search engine as hidden authority.
- **Rollback:** revert the bounded documentation package; no schema/data/runtime rollback is required.
- **Compatibility:** existing homepage composition, Wiki/PublicGameData/Marketplace/source-module semantics remain unchanged; ADR 0033 does not supersede ADR 0025 or ADR 0032.
- **Query privacy/security:** raw search terms and keyed cache-query digests are not ordinary log fields or metric labels; query/filter/rate/payload boundaries are explicit.
- **PlatformAPI:** future API reuse points to one PublicPortal application service rather than a parallel orchestration path.
- **E2E:** `NOT_APPLICABLE` because this task creates no executable route, schema, index, runtime, configuration or deployment path.
- **Final CI:** repository-required checks must pass on the unchanged repaired final PR head.
- **Review hygiene:** zero unresolved material review threads and zero requested changes before merge.

## Exact-head self-review mechanism

The task file cannot embed the SHA of the commit containing that same SHA without moving the head. The final self-review is therefore recorded as an anchored PR review against the exact live head after this checkpoint commit. Any later repository-file change invalidates that review and requires a new exact-head review and CI generation.

Required shape:

```yaml
self_review:
  result: PASS
  exact_head: <live final repaired PR head>
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
    - PORTAL_COMPLETENESS_ARCHITECTURE disposition and dependency-prerequisite change
    - Codex P2 reverse-dependency finding and repair evidence
    - Codex P2 cache-query-identity finding and repair evidence
```

## Context checkpoint

```yaml
checkpoint_version: 1
policy_version: 2
updated_at: 2026-08-09T01:16:00+02:00
invocation_started_at: 2026-08-09T00:43:00+02:00
last_progress_at: 2026-08-09T01:16:00+02:00
head: OUT_OF_BAND_FINAL_HEAD_AFTER_THIS_COMMIT
branch: docs/OTERYN-20260809-federated-search-architecture
pr: 936
status: validating
phase: heightened-repair-exact-head-validation
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
  - PublicPortal remains the minimum-module owner for public federated-search orchestration
  - PublicGameData exact-name character lookup remains explicitly separate
  - a later dedicated search index is derived/rebuildable rather than source truth
  - current Announcements and Events provider/view-model paths import App\\PublicPortal\\PublicContentState
  - repaired architecture gates Announcements/Events search onboarding on removal of that reverse compatibility edge
  - repaired cache policy requires versioned server-keyed digest identity for the normalized query
  - open PRs #338 and #541 do not overlap owned paths
  - runtime E2E is NOT_APPLICABLE for this documentation-only package
derived:
  - source-owned availability/response types mapped by PublicPortal are the preferred dependency cleanup because they restore direction without inventing a generic shared module
  - HMAC query identity provides deterministic cache isolation without persisting raw search terms or relying on a dictionary-recoverable unkeyed hash
unknown: []
conflicts: []
first_failure:
  marker: codex-review-d4e5162f-p2-reverse-dependency
  evidence: Codex found existing Announcements/Events imports of PublicPortal\\PublicContentState, contradicting the earlier absolute one-way-dependency claim
rejected_hypotheses:
  - federated search requires a standalone microservice
  - federated search requires a new top-level Discovery module now
  - source modules should export raw database models to a global index
  - raw relevance scores from heterogeneous providers can be sorted directly as one score
  - exact-name character search should be silently mixed into fuzzy public content relevance
  - public search may index private PlayerCompanion workspaces or share-token artefacts by default
  - an external search engine must be selected before the architecture can be implemented
  - existing Announcements/Events reverse dependencies can be ignored when adding opposite search-provider dependencies
  - PublicContentState should automatically be moved into a new generic shared module
  - locale/filter/provider generation is sufficient cache identity without the query itself
  - raw query text or an unkeyed plain hash is acceptable cache-key privacy
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
  - command: canonical architecture reconciliation before review repair
    result: PASS
    evidence: ADR 0033, focused search architecture, MODULE_CATALOG and portal completeness carried the same intended PublicPortal/source-module ownership direction
  - command: exact-head repository CI on d4e5162f7948bf216217c9a224c699ed33799e38
    result: PASS
    evidence: eight pull-request workflow generations completed successfully, including CI and Agent Governance; historical after later repairs changed head
  - command: Codex exact-head review on d4e5162f7948bf216217c9a224c699ed33799e38
    result: FAIL
    evidence: P2 finding identified current Announcements/Events -> PublicPortal PublicContentState imports and required the architecture to account for the reverse edge
  - command: reverse-dependency architecture repair
    result: PASS
    evidence: ADR 0033, focused search architecture, MODULE_CATALOG and portal completeness record the current compatibility edge and gate provider onboarding on its removal
  - command: exact-head repository CI on c18f1797edfeb057b567d2c2b2116e94d07f5e58
    result: PASS
    evidence: eight pull-request workflow generations completed successfully, including CI and Agent Governance; historical after cache-identity repair changed head
  - command: Codex exact-head review on c18f1797edfeb057b567d2c2b2116e94d07f5e58
    result: FAIL
    evidence: P2 finding required a privacy-safe collision-resistant normalized-query identity in every result-cache key
  - command: cache query-identity architecture repair
    result: PASS
    evidence: ADR 0033 and FEDERATED_SEARCH_ARCHITECTURE require a versioned server-keyed digest, secret/version namespace rotation, no raw/plain-hash query key, no logging/metrics reuse and explicit implementation validation
  - command: runtime E2E
    result: NOT_APPLICABLE
    evidence: documentation-only package changes no executable route, schema, index, runtime, configuration or deployment
  - command: final repaired exact-head repository CI
    result: NOT_RUN
    evidence: must run after this final repair checkpoint on the new exact PR head
repair_cycles_for_current_gate: 2
blockers:
  - none
next_action: perform final repaired exact-head full-diff self-review on PR #936, request fresh Codex review, verify required exact-head CI, resolve the repaired cache P2 thread and merge only with zero material findings and zero unresolved threads; then close Issue #935 and archive this task
```

## Notes

This task is WWW Platform-only. It must not inspect, search or mutate Oteryn-v2, Canary or client repositories.