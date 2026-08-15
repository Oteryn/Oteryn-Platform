# ADR 0039: Historical work canonicalization and managed recovery retention

- Status: Accepted
- Date: 2026-08-15
- Decision owner: repository owner
- Decision record: Issue #1072
- Extends: ADR 0037

## Context

ADR 0037 correctly made task branches execution resources with an intentional terminal disposition and required fail-closed treatment of ambiguous historical work. Issues #1050 and #1068 then removed exact-SHA historical refs only when deletion could be proven safe.

The approval-free closeout after Issue #1068 left a deliberately conservative set: protected `main`, live open-PR work, 22 historical `RETAIN` refs with unique unmerged history and 15 `RECOVERY` refs. That was the correct safety result for the deletion audit, but it is not the desired long-term repository architecture.

An ordinary branch is optimized for active integration work, not durable archival cataloguing. Keeping historical work indefinitely as branches makes active work harder to distinguish from abandoned work, increases reconciliation cost, and turns a fail-closed intermediate state into permanent repository structure.

A registry containing only an immutable commit SHA is also insufficient when exact Git object reachability must remain guaranteed: textual provenance does not itself pin an otherwise unreachable Git object against eventual garbage collection. Recovery retention therefore needs both provenance and a mechanism that actually preserves the required object/history for as long as the recovery contract demands.

## Decision

Ordinary task/work branches are **execution resources, not the default historical archive**.

### `RETAIN` is transitional

`RETAIN` means that unique work exists and deletion is not yet proven safe. It is a reconciliation-required state, not an indefinite terminal disposition.

Every historical `RETAIN` ref must be content-audited and end in an intentional durable outcome:

- `CANONICALIZE_TO_MAIN` — valuable current source/configuration/tests/governance are intentionally delivered to protected `main` through a normal reviewed path;
- `DOCUMENT_ARCHIVE` — historical value is evidence/context rather than active source, so the necessary material and provenance are persisted in the appropriate canonical report, archived task, ADR, Issue/PR record, or other approved evidence location;
- `PR_PROVENANCE_DELETE` — exact same-repository PR provenance already preserves the required historical context/recovery authority and the branch ref can be deleted under exact-head and liveness checks;
- `DELETE` — evidence proves the work is obsolete, superseded, disposable, or otherwise no longer required;
- `ACTIVE` — the work is genuinely active and is attached to a concrete Issue/task/PR owner instead of remaining an anonymous historical ref;
- `MANAGED_RECOVERY` — exact Git history must remain intentionally reachable and is transferred to the managed recovery model below.

The implementation may use equivalent machine-readable names, but it may not weaken these semantics.

### Managed recovery

A long-lived recovery object/ref is valid only when all of the following are durable and machine-checkable:

1. exact source commit/revision identity;
2. owner;
3. concrete recovery purpose and failure scenario;
4. the mechanism that keeps the required Git object/history reachable;
5. deterministic restore procedure;
6. review trigger;
7. retention expiry, expiry condition, or explicit reason why retention is indefinite;
8. evidence that the retention mechanism cannot accidentally trigger release, deployment, publication, or another unrelated automation path.

A branch named `backup/*`, `recovery/*`, or `rollback/*` is not sufficient by itself. A SHA recorded in JSON/Markdown is not sufficient by itself when exact object retention matters.

The implementation owner must inspect current tag/ref/release workflows before selecting tags, custom refs, PR provenance, or another repository-native mechanism. Do not assume a tag or custom ref namespace is operationally inert.

### Historical content preservation

Do not blindly merge a historical branch merely to preserve it or make deletion easy.

For valuable content:

- integrate current code intentionally through normal review, resolving conflicts against current `main` rather than importing stale history wholesale;
- archive evidence/documentation in the focused canonical location, preserving source branch/SHA/PR provenance;
- preserve exact Git history only when recovery requires exact history rather than merely the information or resulting current content.

### Deletion safety

ADR 0037 fail-closed deletion rules remain authoritative.

No deletion may be authorized by age, prefix, naming convention, inactivity, apparent supersession, or cosmetic similarity alone. Every destructive action must bind the exact live ref/SHA, re-check liveness/protection/ownership/recovery conditions immediately before mutation, preserve required recovery provenance, and verify the intended ref is absent afterwards.

Branches must never be merged only to trigger `delete_branch_on_merge`.

## Consequences

### Positive

- active branch lists become a reliable view of active work rather than an archive index;
- fail-closed `RETAIN` no longer silently becomes permanent repository debt;
- valuable historical information moves to canonical searchable locations;
- recovery history that genuinely needs Git reachability has explicit ownership and lifecycle;
- future cleanup can distinguish active, archival, disposable, and recovery state deterministically.

### Negative

- the remaining historical refs require one-by-one content/provenance analysis;
- some recovery refs may need a new managed retention mechanism before their branches can be removed;
- current valuable but stale work may require careful reconstruction on top of modern `main` rather than a simple merge;
- additional validators/registry maintenance may be required to keep retention metadata trustworthy.

## Rejected alternatives

- Keep every unique historical branch indefinitely because deletion is uncertain.
- Treat `RETAIN` as a terminal archival classification.
- Use branch names or age as evidence of disposability.
- Record only SHAs in a registry while allowing required Git objects to become unreachable.
- Blindly merge stale branches into `main` to preserve them or trigger automatic deletion.
- Convert every historical branch into an unmanaged tag without first proving tag/release automation safety.

## Implementation

Issue #1072 owns implementation and reconciliation.

The implementation must:

- rebuild the live inventory from the then-current protected `main`;
- reconcile every historical `RETAIN` and `RECOVERY` ref individually;
- add the smallest durable machine-readable policy/registry necessary for managed retention;
- extend deterministic validation so unexplained long-lived `RETAIN` and unmanaged `RECOVERY` states cannot silently become terminal;
- preserve ADR 0037 exact-head fail-closed deletion semantics;
- finish with zero unexplained `RETAIN` refs and zero unmanaged `RECOVERY` refs.

ADR 0037 remains authoritative for terminal source-branch deletion mechanics. This ADR defines where historical value belongs before those mechanics are allowed to remove the execution ref.
