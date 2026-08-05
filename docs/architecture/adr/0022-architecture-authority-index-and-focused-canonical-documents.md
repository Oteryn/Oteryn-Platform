# ADR 0022: Architecture authority index and focused canonical documents

- Status: Accepted
- Date: 2026-08-05
- Decision owner: repository owner
- Decision record: Issue #548
- Implementation scope: documentation authority and routing only

## Context

Oteryn Platform contains system, module, security, data, test, roadmap, contract, ADR and execution documents created at different phases. Several remain useful historical evidence, but the repository did not explicitly define how a worker should order them when they disagree.

A single exhaustive architecture document would duplicate focused truth and create broad ownership contention. Retaining informal precedence would leave correctness dependent on each worker's interpretation.

The ADR directory was deterministically inventoried before allocation. Its highest numeric prefix was `0021`, with several historical duplicate identifiers. The next collision-free identifier was therefore `0022`; existing accepted files are not renumbered by this decision.

## Decision

Adopt an **architecture authority index plus focused canonical documents**.

`docs/architecture/ARCHITECTURE_AUTHORITY.md` is the canonical routing and precedence entry point. Detailed truth remains in the focused owner for each concern:

- system context and topology — `SYSTEM_ARCHITECTURE.md`;
- modules and responsibility — `MODULE_CATALOG.md`;
- security — `SECURITY_ARCHITECTURE.md`;
- persistent data — `DATA_OWNERSHIP.md`;
- validation — `TEST_STRATEGY.md`;
- delivery order — `ROADMAP.md`;
- cross-component behavior — `docs/contracts/**`;
- durable decisions — accepted ADRs.

Apply this authority order:

1. repository governance and explicit owner decisions, made durable through accepted ADRs;
2. accepted ADRs for their stated decision scope;
3. operation-specific contracts for their declared producer/consumer, data-write, protocol or compatibility scope;
4. focused canonical architecture documents;
5. exact implementation and validation evidence for delivered state;
6. programme, task, Issue and PR records for active execution state;
7. historical planning and superseded records as context only.

Implementation evidence cannot silently overrule an accepted invariant. A mismatch is a recorded `CONFLICT` that must be reconciled in the canonical owner and, when durable, through a new or superseding ADR.

## Consequences

- Workers receive one stable entry point without forcing all architecture detail into one document.
- Focused owners remain independently maintainable and reduce merge contention.
- Historical planning remains available but cannot silently direct current work.
- Roadmap intent, implementation availability, staging proof and production proof remain distinct.
- New ADR allocation must scan the full directory and open architecture PRs, then use the next integer after the highest observed prefix; gaps are not reused.
- Existing duplicate ADR identifiers remain compatibility debt for a separate repair decision and validator.
- Task records and PR descriptions remain execution evidence rather than permanent architecture authority.

## Rejected alternatives

### One exhaustive living `SYSTEM_ARCHITECTURE.md`

Rejected because it would duplicate module, security, data, test, roadmap and contract detail, increasing drift and ownership contention.

### Informal precedence

Rejected because the repository already contains historical and current documents whose scopes can be confused, and duplicate ADR identifiers make informal allocation unsafe.

## Migration

1. Add `ARCHITECTURE_AUTHORITY.md`.
2. Route repository and agent architecture discovery through it.
3. Mark historical initial-phase sections in `SYSTEM_ARCHITECTURE.md` without deleting evidence.
4. Make the ADR README an exhaustive navigation inventory and explicitly document historical identifier collisions.
5. Defer machine validation and compatibility treatment of duplicate identifiers to a bounded follow-up.

## Rollback

Revert the authority index and routing changes and supersede this ADR with a replacement authority decision. No runtime, database, protocol, deployment or production rollback is required.