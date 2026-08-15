# ADR 0039: Historical work canonicalization and managed recovery retention

- Status: Accepted
- Date: 2026-08-15
- Decision owner: repository owner
- Decision record: Issue #1072
- Extends: ADR 0037
- Steady-state enforcement: Issue #1089

## Context

ADR 0037 correctly made task branches execution resources with an intentional terminal disposition and required fail-closed treatment of ambiguous historical work. Issues #1050 and #1068 then removed exact-SHA historical refs only when deletion could be proven safe.

The approval-free closeout after Issue #1068 left a deliberately conservative set: protected `main`, live open-PR work, 22 historical `RETAIN` refs with unique unmerged history and 15 `RECOVERY` refs. That was the correct safety result for the deletion audit, but it is not the desired long-term repository architecture.

An ordinary branch is optimized for active integration work, not durable archival cataloguing. Keeping historical work indefinitely as branches makes active work harder to distinguish from abandoned work, increases reconciliation cost, and turns a fail-closed intermediate state into permanent repository structure.

A registry containing only an immutable commit SHA is also insufficient when exact Git object reachability must remain guaranteed: textual provenance does not itself pin an otherwise unreachable Git object against eventual garbage collection. Recovery retention therefore needs both provenance and a mechanism that actually preserves the required object/history for as long as the recovery contract demands.

Issue #1072 subsequently completed the one-time historical reconciliation and removed all 37 reviewed legacy refs. That closeout makes a steady-state invariant possible: new unexplained branch debt must be prevented rather than periodically accumulated and cleaned later.

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

### Steady-state branch hygiene

After the terminal Issue #1072 cleanup, protected `main` is the only ordinary long-lived branch. Every other ordinary remote branch is an execution resource and must remain explainable by at least one live control-plane owner:

- an open same-repository pull request;
- an active task/claim recorded by repository governance; or
- a purpose-built managed-recovery mechanism when exact Git reachability is genuinely required.

The hard steady-state target is:

```text
NEW_UNEXPLAINED_BRANCHES = 0
```

There is deliberately **no fixed maximum raw branch count**. Branch count is informational; correctness comes from ownership and lifecycle state.

New active ordinary remote refs must not use routine temporary/archive/recovery namespace markers such as top-level `tmp`, `backup`, `archive`, `recovery`, or `rollback`. Temporary experiments stay local/worktree-scoped. Exact recovery uses the managed-recovery contract instead of an ordinary task branch. Historical Issue #1072 registry entries preserve provenance only and are not an exemption for future branch debt.

One ordinary branch has one accountable delivery lane. Multiple open same-repository PRs for one branch or multiple active task claims for one branch are governance conflicts and fail closed. Parallel agents use separate branches/worktrees and non-overlapping owned paths.

Human/agent task branches should prefer `<type>/issue-<number>-<slug>` for readability and deterministic ownership discovery. This convention is advisory only; bot/system branches are exempt. Naming, age, prefix and inactivity never authorize deletion.

Programme and coordinator records are control-plane state. They do not justify long-lived programme, archive or checkpoint branches merely to preserve programme history.

Repository lifecycle settings are part of the steady-state contract: default branch `main`, squash delivery enabled, merge-commit and rebase-merge delivery disabled, and `delete_branch_on_merge=true`. Audit detects drift but never auto-mutates repository settings.

The existing Historical Branch Audit is the canonical enforcement surface. Normal pull-request validation remains path-scoped. A trusted-base pull-request lifecycle check and a bounded schedule perform read-only steady-state inventory so ownerless branches are detected even when no governance file changed. The historical destructive apply path remains trusted-main-push-only and cannot run from those read-only events.

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
- future cleanup can distinguish active, archival, disposable, and recovery state deterministically;
- new unexplained branch debt is detected continuously without an arbitrary branch-count budget;
- the read-only steady-state path is separated from the historical destructive mutation path.

### Negative

- every remote branch must keep its PR/task ownership current;
- legacy but still-active branch names may produce advisory naming noise until their normal lifecycle ends;
- scheduled read-only inventory consumes a small bounded amount of Actions capacity;
- recovery refs require explicit managed-recovery metadata rather than convenient backup branches.

## Rejected alternatives

- Keep every unique historical branch indefinitely because deletion is uncertain.
- Treat `RETAIN` as a terminal archival classification.
- Use branch names or age as evidence of disposability.
- Record only SHAs in a registry while allowing required Git objects to become unreachable.
- Blindly merge stale branches into `main` to preserve them or trigger automatic deletion.
- Convert every historical branch into an unmanaged tag without first proving tag/release automation safety.
- Set a fixed maximum raw branch count.
- Run broad repository cleanup periodically instead of preventing new unexplained refs.
- Trigger destructive branch cleanup from `pull_request_target`, schedule, or another untrusted/read-only event.

## Implementation

Issue #1072 owns the completed historical reconciliation. Issue #1089 owns steady-state prevention.

The implementation must:

- keep the terminal #1072 registry and destructive evidence as immutable historical provenance;
- enforce `NEW_UNEXPLAINED_BRANCHES = 0` without imposing a fixed raw branch-count cap;
- detect ownerless remote refs, duplicate branch ownership and forbidden active temporary/archive/recovery namespaces;
- report preferred task-branch naming as advisory only;
- detect repository merge-setting drift without auto-mutation;
- reuse the existing Historical Branch Audit for path-scoped validation, trusted-base PR-lifecycle checks and bounded scheduled read-only inventory;
- preserve ADR 0037 exact-head fail-closed deletion semantics and keep destructive apply restricted to trusted protected-main pushes.

ADR 0037 remains authoritative for terminal source-branch deletion mechanics. This ADR defines where historical value belongs before deletion and the steady-state controls that prevent ordinary execution refs from becoming archives again.
