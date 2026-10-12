# Platform PostgreSQL P6b: maintenance-gate state machine

Status: proposed, for acceptance before P6 starts. Governing plan: `PLATFORM_POSTGRES_MIGRATION_PLAN.md`, *P6 / Edge gate instead of maintenance mode*. Decision: ADR 0046. This note changes no workflow, script, configuration or data.

## Fixed by the plan

- The gate is the two canonical tunnel ingress rules (`oteryn.molehill.cloud`, `gateway.molehill.cloud`) switched between their fixed loopback origins and the Cloudflare `http_status:503` service. DNS and the tunnel connector are not touched.
- The gate fails closed.
- No mutating entry point may reopen the origins except `maintenance-open`.
- `maintenance-close` and `maintenance-open` extend the existing `Cloudflare Oteryn Endpoints` workflow and use the `apply` safeguards: confirmation phrase, fresh read, rejection of locally managed tunnels and ambiguous rules, preservation of unrelated rules and the final catch-all, re-read and hash check immediately before `PUT`.
- `maintenance-close` refuses, changing nothing, when any other ingress rule has a service on port 8000 or 8080 of the connector host.
- `audit` reports the closed state as bounded drift, not failure. No bypass secret, cookie or path exists.
- Opening happens only in step 7, or at the end of a rollback after `health-check.sh` passes on MariaDB.

## States

Derived from a fresh read of the tunnel ingress, never from memory of a previous run.

| State | Meaning |
| --- | --- |
| `open` | Both canonical rules on their fixed loopback origins |
| `closed` | Both canonical rules on `http_status:503` |
| `split` | One canonical rule closed, the other not (only an external edit can cause it, because both rules change in one `PUT`) |
| `unknown` | Read failed, tunnel locally managed, rules ambiguous, or any value outside the above |

`split` and `unknown` are treated as not open. They never satisfy the external check.

## Marker and lock

- **Marker (*proposal*).** The marker is the ingress configuration itself: the `503` service on the two canonical rules. No second store exists, so none can disagree with the edge. Each run logs the pre-write and post-write configuration hashes in its summary.
- **Lock (*proposal*).** One GitHub Actions `concurrency` group for the workflow with `cancel-in-progress: false`, so runs of every mode serialize. The pre-`PUT` re-read and hash check is the guard against an edit made outside the workflow.

## Transitions

| From | Entry point | To | Rule |
| --- | --- | --- | --- |
| `open` | `maintenance-close` | `closed` | Other-rule check passes, then one `PUT` of both rules, then read back both rules |
| `closed` | `maintenance-close` | `closed` | No write |
| `split` | `maintenance-close` | `closed` | Completes the close |
| `closed` | `maintenance-open` | `open` | Only with the evidence precondition below. One `PUT` restoring the fixed loopback origins, then read back, then a fresh `audit` and an external request to each hostname |
| `open` | `maintenance-open` | `open` | No write |
| `split` | `maintenance-open` | `open` | Only with the evidence precondition below. Completes the open |
| `unknown` | either mode | unchanged | Refuses, no write |
| `closed`, `split` or `unknown` | `apply` | unchanged | Refuses in every state other than `open`, with no write and no warning-only path. `apply` rebuilds the rules with the loopback origins, so it must never run unless the gate is already `open`; the only reopen path is `maintenance-open` |
| any | DNS or other modes | unchanged | They do not touch the two rules |

**Evidence precondition for `maintenance-open` (*proposal*).** From `closed` or `split` the mode refuses unless the dispatch names a task id and a run id and exactly one of these holds, checked by the workflow before any `PUT`:

1. Cutover completed: the cutover run for that task reached step 7 and recorded success, including the post-cutover health check.
2. Rollback verified: the MariaDB rollback for that task finished and `health-check.sh` passed against it in a recorded run.

A closed gate with a stopped step (for example step 4 on ambiguous trust state) satisfies neither case, so `maintenance-open` refuses and the gate stays closed. Missing, unreadable or unbound evidence also refuses. The evidence run ids are logged in the summary.

Ordering: read, validate, re-read and hash check, `PUT`, read back. A failed read-back leaves the state as read, reported as failure, never as success.

## Crash recovery

- Runner dies before the `PUT`: state unchanged. Re-run the same mode.
- Runner dies after the `PUT`: the new state stands. Re-running the same mode is a no-op.
- Runner dies during `maintenance-open` before its `PUT`: the edge stays closed (fail closed).
- Cutover script dies with the gate closed: the gate stays closed until `maintenance-open` runs in step 7 or at the end of a rollback.
- Hash check fails at `PUT` time: no write, report, re-run from a fresh read.

## Tests per transition (`tests/operations/cloudflare-oteryn-endpoints/**`)

One test per row of the transition table, plus: `apply` refuses from `closed`, `split` and `unknown`; `maintenance-open` refuses without cutover-complete or rollback-verified evidence, with evidence from another task, and with a stopped step; refusal on another rule targeting `127.0.0.1`, `localhost`, `[::1]` or `0.0.0.0` on port 8000 or 8080 (including wildcard, legacy Gateway and catch-all); unrelated rules and the catch-all preserved byte for byte; hash mismatch aborts; read-back failure reported as failure; `audit` reports `closed` as bounded drift; confirmation phrase required; no secret or bypass value in output.

## Open questions for the reviewer

1. Is an edge-derived marker acceptable, or is a separate durable record wanted? A second record would need its own reconcile rule against the edge.
2. Is the task/run-bound evidence precondition for `maintenance-open` enforced by the workflow (this note's proposal) or by operator procedure only? `apply` refusal outside `open` is not optional.
