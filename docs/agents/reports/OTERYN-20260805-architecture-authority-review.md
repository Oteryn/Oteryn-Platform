# Oteryn Platform canonical architecture authority review

## Review identity

```yaml
decision_id: OTERYN-ARCH-20260805-001
issue: 548
pull_request: 550
programme: OTERYN_PLATFORM_ARCHITECTURE_REVIEW
repository: blakinio/Oteryn-Platform
original_exact_base: 3ab77c072dce796b09004c54b649db009a75d524
latest_main_reconciled: a7eb03d49e328e8115adb54e772c9c8366b737d3
classification:
  - contradiction
  - documentation_drift
  - missing_decision
severity: high
confidence: high
runtime_implementation: forbidden
accepted_option: B
accepted_on: 2026-08-05
accepted_adr: docs/architecture/adr/0022-architecture-authority-index-and-focused-canonical-documents.md
```

## Executive result

The repository owner accepted **Option B: an authority index plus focused canonical documents**. ADR 0022 records the decision and `docs/architecture/ARCHITECTURE_AUTHORITY.md` is the canonical routing and precedence entry point.

The implementation is documentation-only. It does not change runtime, workflows, migrations, dependencies, deployment, infrastructure, native-protocol PR #542 or public-edge PR #541.

## Final evidence

### PROVEN — system architecture mixed current and historical statements

`SYSTEM_ARCHITECTURE.md` was written as a first-phase target. Its original module list, non-goals and discovery prerequisites could be mistaken for current product or implementation truth despite newer ADRs, contracts, module documentation and merged evidence.

The document now owns only system context, trust boundaries, topology and high-level dependency direction. First-phase module, non-goal and discovery sections are explicitly historical baselines and route current questions to focused owners.

### PROVEN — focused architecture owners already exist

The repository has focused documents for modules, security, data ownership, testing and roadmap, plus operation-specific contracts and ADRs. Option B preserves those owners instead of copying their contents into one exhaustive system document.

### CORRECTED — initial repository-map defect was not reproducible

The initial review package stated that `REPOSITORY_MAP.md` referenced a missing lowercase `docs/architecture/overview.md`. Revalidation of both the task branch and current `main` disproved that statement: the live file already referenced the uppercase focused architecture files.

The final change therefore does not claim to repair a missing link. It adds the new `ARCHITECTURE_AUTHORITY.md` entry and routes architecture-wide work through it. The incorrect preliminary claim remains visible in Issue #548 history but is corrected by this report and a durable Issue comment.

### PROVEN — ADR registry debt is broader than first observed

A deterministic directory inventory found the highest numeric prefix `0021` and historical duplicate identifiers for:

- `0008`;
- `0010`;
- `0011`;
- `0015`;
- `0016`;
- `0017`;
- `0018`;
- `0021`.

The accepted decision was therefore allocated `0022`. No existing ADR was renamed. `adr/README.md` now inventories all observed paths, displays collisions instead of hiding them and defines a max-prefix-plus-one allocation rule.

### PROVEN — module catalogue drift remains separately owned

Merged PR #453 remains the evidence baseline for missing or stale module boundaries. This package does not duplicate that audit or upgrade module capability status without exact merged-evidence reconciliation.

### PROVEN — active ownership exclusions were preserved

PR #542 remains the owner of native-protocol implementation and related contracts. PR #541 remains outside this package. No application, workflow, migration, dependency, deployment or infrastructure path was changed.

## Accepted authority model

For architecture questions, apply the narrowest relevant source in this order:

1. repository governance and explicit owner decisions, made durable through accepted ADRs;
2. accepted ADRs for their stated scope;
3. operation-specific contracts for their declared scope;
4. focused canonical architecture documents;
5. exact implementation and validation evidence for delivered state;
6. programme, task, Issue and PR records for active execution state;
7. historical planning and superseded records as context only.

A lower-ranked source cannot silently override a higher-ranked invariant. The mismatch must be recorded as `CONFLICT` and resolved in the focused owner and, when durable, through a new or superseding ADR.

## Canonical owners established

| Concern | Owner |
|---|---|
| Authority and conflict handling | `ARCHITECTURE_AUTHORITY.md` |
| Durable decisions | `docs/architecture/adr/**` |
| System context/topology | `SYSTEM_ARCHITECTURE.md` |
| Modules/responsibility | `MODULE_CATALOG.md` |
| Security | `SECURITY_ARCHITECTURE.md` |
| Persistent data | `DATA_OWNERSHIP.md` |
| Validation | `TEST_STRATEGY.md` |
| Delivery order | `ROADMAP.md` |
| Cross-component behavior | `docs/contracts/**` |
| Current execution evidence | project state, active task and live PR |

## Invariants

1. Accepted ADRs and operation-specific contracts cannot be silently overridden by general prose.
2. Code and exact validation prove implementation state but cannot create an unrecorded product or authority decision.
3. Historical decisions remain discoverable and supersession is explicit.
4. Proposed architecture does not grant runtime, production or cross-repository authority.
5. One concept has one focused canonical owner; indexes route rather than duplicate.
6. Roadmap intent, implementation availability, staging proof and production proof remain separate facts.
7. Existing ADR identifier collisions are preserved until a compatibility-safe repair is accepted.

## Completed migration slice

### Slice 1 — authority and routing

Completed in PR #550:

- owner acceptance recorded in Issue #548;
- deterministic ADR directory inventory completed;
- collision-free ADR 0022 created and accepted;
- compact authority index added;
- repository map and context routing updated;
- system architecture current/historical scope made explicit;
- ADR README converted into a collision-aware complete path inventory.

Rollback is documentation-only and requires superseding ADR 0022 rather than silently removing the accepted decision.

## Remaining bounded backlog

1. **ADR registry validator:** reject new duplicate IDs, inventory mismatch, missing lifecycle token and broken supersession target while preserving existing colliding paths.
2. **Historical collision compatibility decision:** choose aliases, stable decision IDs or another non-breaking treatment before renaming any accepted ADR.
3. **System/module reconciliation:** use PR #453 and later exact merged evidence to reconcile the current module catalogue and system diagram without conflating availability with completeness.
4. **Machine-readable architecture decision backlog:** add one validated registry only after its schema and owner are accepted.

## Validation boundary

- runtime E2E: `NOT_APPLICABLE`;
- required: exact changed-path audit, link/content audit and exact-head Agent Governance/documentation CI;
- forbidden: weakening runtime gates or treating documentation acceptance as production proof.

## Decision boundary

```yaml
recommended_option: B
accepted: true
accepted_on: 2026-08-05
decision_owner: repository owner
adr: 0022
runtime_authority: none
production_activation_authority: none
next_action: validate exact PR head and present PR 550 for review/merge
```