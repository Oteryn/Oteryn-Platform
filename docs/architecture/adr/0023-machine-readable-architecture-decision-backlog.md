# ADR 0023: Machine-readable architecture decision backlog

- Status: Accepted
- Date: 2026-08-05
- Accepted on: 2026-08-05
- Decision owner: repository owner
- Decision record: Issue #602
- Owner decision: Option B accepted
- Programme: `OTERYN_PLATFORM_ARCHITECTURE_REVIEW`
- Implementation scope: documentation governance and deterministic validation only

## Context

ADR 0022 defines accepted ADRs as durable architecture decision authority and classifies programme, task, Issue and PR records as execution state. The architecture-review programme currently embeds a compact `decision_backlog` that is useful for continuation but is not independently versioned, schema-governed or validated.

GitHub Issues provide discussion, assignment and owner-decision workflow. They are mutable remote state and cannot by themselves provide reproducible exact-head repository validation. The existing ADR registry already validates accepted and proposed ADR identity and lifecycle and must not be duplicated.

The repository therefore needs an explicit boundary for unresolved architecture decision obligations. That boundary must preserve the distinction between:

- unresolved questions awaiting evidence or owner choice;
- accepted decisions recorded in ADRs or focused canonical architecture documents;
- implementation work owned by Issues, tasks and remediation;
- compact programme continuation state.

## Decision

Adopt a dedicated canonical JSON backlog at:

`docs/architecture/ARCHITECTURE_DECISION_BACKLOG.json`

The backlog is an inventory of unresolved architecture decision obligations. It is not an authority for accepted decisions and grants no runtime, workflow, deployment, infrastructure, cross-repository or production authorization.

### Authority boundary

- accepted ADRs remain the durable authority for adopted decisions;
- focused canonical architecture documents remain the current truth for their concern;
- the backlog records unresolved, blocked or intentionally deferred decision obligations only;
- GitHub Issues own discussion, assignment and explicit owner choice;
- proposed ADRs describe durable alternatives but remain non-authoritative until accepted;
- the architecture programme stores only compact active IDs, current review state and one `next_action`;
- implementation handoffs remain separate Issues/tasks under the remediation programme.

### Canonical document shape

The JSON root contains:

- `schema_version`;
- `registry_name`;
- `authority` and an explicit non-authority statement;
- `records`, ordered deterministically by stable `decision_id`.

Each active record contains at least:

- stable `decision_id` and title;
- lifecycle state: `discovered`, `analysis_ready`, `decision_required`, `blocked` or `deferred`;
- severity, decision owner and recommended owner;
- exact problem statement and decision question;
- impacted canonical architecture owners;
- `PROVEN`, `DERIVED`, `UNKNOWN` and `CONFLICT` evidence lists;
- meaningful options and recommendation when analysis is ready;
- dependencies and blockers;
- Issue and proposed-ADR references when applicable;
- creation/update timestamps;
- explicit `implementation_authorized: false`.

Resolved, rejected or superseded records leave the active backlog in the same bounded package that records their terminal authority or rationale. Their history remains in Git, the Issue, the accepted/rejected ADR and the architecture programme archive/report rather than in a second permanent decision archive.

### Deterministic validation

A standard-library validator fails closed for:

- unsupported schema versions or unknown fields where the schema forbids them;
- duplicate or malformed decision IDs;
- invalid lifecycle values or terminal states retained in the active backlog;
- missing owner, Issue, evidence-state or decision-question fields;
- the same normalized fact appearing in multiple evidence-state lists;
- missing local paths, ADR paths or malformed local references;
- duplicate unresolved obligations with the same canonical owner and normalized decision question;
- a record claiming acceptance, implementation or activation authority;
- programme references to unknown backlog IDs;
- a `decision_required` record without alternatives and one exact blocking owner question.

Remote Issue/PR state may be checked by a separate live reconciliation step when a lifecycle transition depends on it. Ordinary repository validation remains reproducible without remote API access.

## Alternatives

### Validate the full backlog inside the programme Markdown

Rejected because it mixes execution state with durable architecture inventory, increases programme-file merge contention and makes independent schema evolution and reuse harder.

### Use GitHub Issues and labels as the sole backlog

Rejected because it cannot provide deterministic exact-head offline validation or reproducible historical repository snapshots. Labels also do not provide a sufficiently strict record schema.

### Status quo

Rejected because an informal programme queue leaves lifecycle, deduplication, evidence and authority boundaries unenforced.

## Consequences

- One machine-readable inventory exists for unresolved architecture decisions without replacing ADR authority.
- Programme state becomes smaller and lower-contention.
- Exact-head validation can prove record structure and local referential integrity.
- Issue and ADR transitions require synchronized bounded updates.
- A validator and migration are required in a separate implementation package.
- Completed historical programme entries are not imported as active backlog obligations.

## Implementation handoff

A separate bounded remediation package must:

1. add `ARCHITECTURE_DECISION_BACKLOG.json` with schema version 1;
2. add a standard-library validator and positive, negative and boundary tests under `tools/validation/**`;
3. register validation through the existing repository test suite without changing workflow files unless later evidence proves that impossible;
4. seed only still-unresolved architecture decision obligations, excluding completed `ARCH-AUTH` history;
5. replace the programme's full `decision_backlog` with a compact projection of active IDs and current `next_action`;
6. document transition and removal rules in `ARCHITECTURE_AUTHORITY.md` and the backlog file;
7. preserve `implementation_authorized: false` for every decision record.

This ADR authorizes only the repository-owned documentation-governance implementation above. It authorizes no application runtime, migration, production, deployment, infrastructure, workflow or external-repository mutation.

## Rollback

Revert the backlog, validator and programme projection together and supersede this ADR with a replacement authority decision. No runtime, database, deployment or production rollback is involved.