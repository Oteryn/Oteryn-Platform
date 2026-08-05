# Oteryn Platform canonical architecture authority review

## Review identity

```yaml
decision_id: OTERYN-ARCH-20260805-001
issue: 548
programme: OTERYN_PLATFORM_ARCHITECTURE_REVIEW
repository: blakinio/Oteryn-Platform
exact_base: 3ab77c072dce796b09004c54b649db009a75d524
classification:
  - contradiction
  - documentation_drift
  - missing_decision
severity: high
confidence: high
runtime_implementation: forbidden
```

## Executive result

The repository has strong focused architecture, contract and delivery evidence, but it does not currently define an explicit precedence model between historical target documents, living module/roadmap documents, accepted ADRs, contracts, machine-readable evidence and implementation state. This creates a real coordination risk: a worker can read a valid repository file and still follow obsolete architecture.

The recommended durable model is **an authority index plus focused canonical documents**. This report does not mark that recommendation accepted. Issue #548 is the decision boundary; a proposed ADR should be created only after live ADR numbering is deterministically reconciled and the decision is accepted for review.

## Primary evidence

### PROVEN — stale system-level baseline

`docs/architecture/SYSTEM_ARCHITECTURE.md` identifies itself as a target for the first implementation phase. It still:

- lists Marketplace/auction systems as an initial non-goal;
- treats important account/player schema and game-login facts as unresolved discovery prerequisites;
- presents a 10-module initial catalogue that omits later first-class boundaries;
- describes cache, queue and operational capabilities as future direction.

Those statements are historical planning facts, not a safe current-state architecture model. Newer accepted repository state exists in `ROADMAP.md`, `MODULE_CATALOG.md`, `DATA_OWNERSHIP.md`, security/testing documents, contracts, ADRs and merged implementation evidence.

### PROVEN — route to a missing canonical file

`docs/agents/REPOSITORY_MAP.md` directs architecture work to lowercase `docs/architecture/overview.md`. That path does not exist on the exact base. The live architecture entry points use uppercase focused files such as `SYSTEM_ARCHITECTURE.md`, `MODULE_CATALOG.md`, `DATA_OWNERSHIP.md`, `SECURITY_ARCHITECTURE.md`, `TEST_STRATEGY.md` and `ROADMAP.md`.

### PROVEN — ADR registry cannot be treated as exhaustive

`docs/architecture/adr/README.md` lists only a subset of the ADR directory. The directory also contains two distinct `0008-*` records. Therefore the README is currently neither an exhaustive inventory nor a collision-proof allocator for the next ADR number.

This report does not rename an accepted ADR. Renumbering accepted historical files can break references and must be decided separately with an alias/supersession or compatibility plan.

### PROVEN — module catalogue drift already audited

Merged PR #453 established that the original module table lags merged delivery and omits first-class boundaries for Products/Entitlements, Legal/Commerce, Operations/Observability, Public Edge and Quality/E2E. That audit also distinguished module availability from complete capability delivery.

This architecture package reuses that evidence. It does not duplicate the module audit and does not upgrade module statuses without exact merged-evidence reconciliation.

### PROVEN — active ownership exclusion

Open PR #542 owns native-protocol implementation and related contract paths. This review does not modify those paths, select protocol behavior or reinterpret its rollout authority.

## Conflict map

| ID | Sources | Conflict | Consequence | Classification |
|---|---|---|---|---|
| ARCH-AUTH-001 | `SYSTEM_ARCHITECTURE.md` vs `ROADMAP.md`/`MODULE_CATALOG.md`/merged code | initial non-goals and unresolved boundaries are presented beside delivered capabilities | agents may reject valid existing capabilities or recreate discovery work | contradiction |
| ARCH-AUTH-002 | `REPOSITORY_MAP.md` vs live tree | architecture entry path is missing | context routing can fail or omit canonical documents | defect |
| ARCH-AUTH-003 | ADR README vs ADR directory | incomplete index and duplicate number | next ADR allocation can collide; accepted decisions can be missed | defect + missing_decision |
| ARCH-AUTH-004 | module table vs PR #453 baseline | status and ownership model lag delivery | implementation and audits can use different module universes | documentation_drift |
| ARCH-AUTH-005 | architecture docs vs implementation evidence | no explicit precedence or staleness lifecycle | conflicts are resolved ad hoc by each worker | missing_decision |

## Non-negotiable invariants

1. Accepted ADRs and operation-specific contracts cannot be silently overridden by general planning prose.
2. Source code and exact validation prove implementation state, but implementation alone cannot create an unrecorded product or authority decision.
3. Historical decisions remain discoverable; supersession is explicit.
4. Proposed architecture text does not grant runtime, workflow, production or cross-repository authority.
5. One concept has one canonical owner; indexes route to truth rather than duplicate it.
6. Conflicts remain explicit until authoritative evidence resolves them.
7. Any generated inventory must fail closed on duplicate identifiers, missing targets and invalid lifecycle status.

## Alternatives

### Option A — one exhaustive living `SYSTEM_ARCHITECTURE.md`

Rewrite the system document as the complete current-state architecture and make every other document subordinate.

Benefits:

- one obvious entry point;
- simple mental model;
- easy human browsing for a small system.

Costs and risks:

- large frequently conflicting document;
- duplicates module, data, security, testing, deployment and contract detail;
- broad ownership and merge contention;
- high drift risk as focused domains evolve;
- difficult machine validation of duplicated assertions.

### Option B — authority index plus focused canonical documents

Create a compact authority index that defines precedence, ownership, lifecycle and conflict handling. Keep detailed truth in focused documents and contracts. Make `SYSTEM_ARCHITECTURE.md` a current system context/topology document, not an exhaustive duplicate.

Benefits:

- smallest coherent source for each concern;
- scalable ownership and lower merge contention;
- explicit conflict resolution and staleness lifecycle;
- supports deterministic link/status/ADR inventory validation;
- preserves historical ADRs and focused contracts.

Costs and risks:

- requires a reliable index and validators;
- workers must follow routing rather than read one file;
- initial reconciliation must classify legacy text carefully.

### Option C — status quo with informal precedence

Keep current files and rely on workers to infer recency and authority.

Benefits:

- no immediate migration work;
- no new governance surface.

Costs and risks:

- already-proven stale routing and contradictory text remain;
- repeat audits and duplicate decisions are likely;
- ADR collisions remain possible;
- correctness depends on individual worker judgment.

## Trade-off matrix

Scoring: 1 poor, 5 strong.

| Dimension | A: single document | B: authority index | C: status quo |
|---|---:|---:|---:|
| Correctness under change | 2 | 5 | 1 |
| Explicit authority | 4 | 5 | 1 |
| Ownership isolation | 1 | 5 | 2 |
| Machine validation | 2 | 5 | 1 |
| Migration simplicity | 2 | 3 | 5 |
| Long-term maintainability | 2 | 5 | 1 |
| Reversibility | 3 | 5 | 4 |
| Delivery risk | 2 | 4 | 1 |

## Recommendation

Recommend **Option B** with high confidence.

The authority index should define this precedence for architecture questions:

1. trusted repository governance and explicit owner decisions;
2. accepted ADRs for durable decisions;
3. operation-specific contracts for producer/consumer and data-write behavior;
4. focused canonical architecture documents for current module, data, security, testing, topology and roadmap models;
5. exact implementation and validation evidence for what is currently delivered;
6. programme/task/Issue/PR records for active work and unresolved decisions;
7. historical planning and superseded records as context only.

This ordering must not let code silently overrule an accepted architectural invariant; instead it creates a recorded conflict requiring reconciliation.

## Proposed migration

### Slice 1 — authority and routing

Owner: architecture review.

- accept or reject Option B in Issue #548;
- allocate a collision-free ADR identifier after deterministic ADR inventory;
- add a compact architecture authority/index document;
- correct `REPOSITORY_MAP.md` and `CONTEXT_ROUTING.md` to the live entry point;
- mark current status and scope in `SYSTEM_ARCHITECTURE.md` without deleting history.

Rollback: remove the new index/routing and restore prior links; no runtime effect.

### Slice 2 — ADR inventory and lifecycle validator

Owner: remediation after decision acceptance.

- inventory every ADR path, identifier, title and status;
- decide compatibility treatment for duplicate `0008` identifiers;
- make README generated or validated rather than manually partial;
- fail closed on duplicate new identifiers, missing files, invalid statuses and broken supersession targets;
- add positive, negative and boundary fixtures.

Rollback: revert validator and generated index; preserve all accepted ADR files.

### Slice 3 — current system and module reconciliation

Owner: architecture review for accepted canonical text; remediation only for tooling.

- update `SYSTEM_ARCHITECTURE.md` from initial target to current system context;
- reconcile module status/ownership against PR #453 and later merged evidence;
- add the missing first-class boundaries only with exact responsibility and dependency rules;
- distinguish `AVAILABLE`, capability completeness, staging proof and production proof.

Rollback: restore previous docs; no runtime effect.

### Slice 4 — decision backlog and implementation handoffs

Owner: architecture review + remediation coordinator.

- maintain one machine-readable backlog of unresolved architecture decisions;
- link each item to its canonical Issue, proposed/accepted ADR, dependencies, owner and acceptance;
- hand accepted implementation work to `OTERYN_PLATFORM_REMEDIATION` without broadening authority.

## Security, data and operational implications

- Security: stale trust-boundary text can produce unsafe integration assumptions; accepted contracts must outrank general prose.
- Data: data-owner and writer rules remain in `DATA_OWNERSHIP.md` and operation contracts; the authority index routes to them rather than copying them.
- API/protocol: active native-protocol ownership remains isolated; accepted protocol contracts cannot be altered by this decision package.
- Operations: readiness and production proof remain distinct from repository or staging implementation state.
- CI: a future validator should be documentation/governance scoped and must not weaken runtime gates.

## Validation expectations

For the accepted implementation package:

- deterministic architecture-link and ADR inventory validation;
- fixtures proving duplicate IDs, broken links, missing status and invalid supersession fail;
- exact-head Agent Governance/documentation CI;
- fresh independent documentation audit with zero material findings;
- runtime E2E: `NOT_APPLICABLE`, because no runtime behavior changes.

## Decision boundary

```yaml
recommended_option: B
recommendation_confidence: high
accepted: false
decision_owner: repository owner / authoritative architecture review state
blocking_question: Accept Option B as the canonical architecture authority model, or select A/C with rationale.
safe_default_while_unresolved: Keep current runtime unchanged; route new architecture decisions through Issue #548 and do not allocate a new numeric ADR from the incomplete README.
```

## Remaining backlog after this package

1. `ARCH-AUTH-001`: accept authority model and create proposed/accepted ADR.
2. `ARCH-AUTH-002`: repair live architecture routing.
3. `ARCH-AUTH-003`: reconcile ADR inventory and duplicate numbering.
4. `ARCH-AUTH-004`: reconcile module catalogue/current system state using merged evidence.
5. `ARCH-AUTH-005`: create a single machine-readable decision backlog only after the authority model is accepted.