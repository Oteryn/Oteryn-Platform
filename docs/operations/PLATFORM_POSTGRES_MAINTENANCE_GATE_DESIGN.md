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
| `split` | Exactly one canonical rule is on `http_status:503` and the other is on its fixed loopback origin (only an external edit can cause it, because both rules change in one `PUT`) |
| `unknown` | Everything else: read failed, tunnel locally managed, rules ambiguous or missing, or a canonical rule on any service other than its fixed loopback origin or `http_status:503` (including one rule on `503` and the other on an arbitrary service) |

The four states are disjoint and exhaustive. Classification checks `open`, `closed` and `split` first by exact match on the two rule services; whatever matches none of them is `unknown`.

`split` and `unknown` are treated as not open. They never satisfy the external check.

## Marker and lock

- **Marker (*proposal*).** The gate state is the ingress configuration itself: the `503` service on the two canonical rules. The edge cannot say which closure produced it, so the workflow also keeps one **closure record** in a single repository variable `MAINTENANCE_GATE_RECORD` on the workflow's protected environment, written only by this workflow. It holds `{state, close_id, config_hash, run_id}`, where `close_id` is the run id of the `maintenance-close` that produced the current closure and `config_hash` is the post-write hash of the two canonical rules.
- **Write order.** `maintenance-close`: `PUT`, read back, then write the record with a new `close_id` and the read-back hash. `maintenance-open`: first write the record as `{state: opening}`, then `PUT`, read back, then write `{state: open, config_hash}`. A crash between the edge write and the record write leaves a mismatch, never a bound closure.
- **Reconcile.** On every run the edge read is compared with the record. A closed edge is **bound** only when the record says `closed` and its `config_hash` equals the current hash. A closed edge with a missing, `open`, `opening` or hash-mismatched record (an external edit recreating the closed configuration, close A then open then close B interrupted, a lost variable) is **unbound**: it still blocks traffic, and `maintenance-open` refuses from it, because no evidence can name a closure that is not recorded. `maintenance-close` on an unbound closed edge rewrites nothing at the edge and writes a new record with a new `close_id`, which re-binds the closure. A bound `closed` stays a no-op. Evidence for a later open must name the `close_id` in the record at the time of the open, so evidence from close A never satisfies close B.
- Each run logs the pre-write and post-write configuration hashes and the record before and after in its summary.
- **Lock (*proposal*).** One GitHub Actions `concurrency` group for the workflow with `cancel-in-progress: false`, so runs of every mode serialize. The pre-`PUT` re-read and hash check is the guard against an edit made outside the workflow.

## Transitions

| From | Entry point | To | Rule |
| --- | --- | --- | --- |
| `open` | `maintenance-close` | `closed` | Other-rule check passes, then one `PUT` of both rules, then read back both rules |
| `closed` (bound) | `maintenance-close` | `closed` | No write |
| `closed` (unbound) | `maintenance-close` | `closed` | No edge write; new record with a new `close_id` |
| `split` | `maintenance-close` | `closed` | Completes the close |
| `closed` | `maintenance-open` | `open` | Only with the evidence precondition below. One `PUT` restoring the fixed loopback origins, then read back, then a fresh `audit` and an external request to each hostname |
| `open` | `maintenance-open` | `open` | No write |
| `split` | `maintenance-open` | `open` | Only with the evidence precondition below. Completes the open |
| `unknown` | either mode | unchanged | Refuses, no write |
| `closed`, `split` or `unknown` | `apply` | unchanged | Refuses in every state other than `open`, with no write and no warning-only path. `apply` rebuilds the rules with the loopback origins, so it must never run unless the gate is already `open`; the only reopen path is `maintenance-open` |
| any | DNS or other modes | unchanged | They do not touch the two rules |

**Evidence precondition for `maintenance-open` (*proposal*).** From `closed` or `split` the mode refuses unless the dispatch names a task id and a run id and exactly one of these holds, checked by the workflow before any `PUT`. Both cases are bound to the current closure: the evidence run must name the `close_id` held in the closure record and have finished after that close run; an unbound closure has no `close_id` and refuses. Evidence from an earlier close of the same task (for example a rollback run before a second cutover attempt) does not satisfy either case:

1. Cutover ready to open: the cutover run for that task recorded step 6 as done (step 5 native login smoke passed and the cutover was declared done). Step 7 is the opening step, so this evidence is the authorization for `maintenance-open` and cannot require step 7 itself to be complete. The post-open `audit` and external requests are the step 7 result, not its precondition.
2. Rollback verified: the MariaDB rollback for that task finished and `health-check.sh` passed against it in a recorded run.

A retry therefore needs fresh evidence: after a rollback and a second `maintenance-close`, the first rollback's `health-check.sh` run is stale and `maintenance-open` refuses until a newer linked run exists. A closed gate with a stopped step (for example step 4 on ambiguous trust state, or step 5 or 6 not recorded) satisfies neither case, so `maintenance-open` refuses and the gate stays closed. Missing, unreadable or unbound evidence also refuses. The evidence run ids are logged in the summary.

Ordering: read, validate, re-read and hash check, `PUT`, read back. A failed read-back leaves the state as read, reported as failure, never as success.

## Crash recovery

- Runner dies before the `PUT`: state unchanged. Re-run the same mode.
- Runner dies after the `PUT`: the new state stands. Re-running the same mode is a no-op.
- Runner dies during `maintenance-open` before its `PUT`: the edge stays closed (fail closed).
- Cutover script dies with the gate closed: the gate stays closed until `maintenance-open` runs in step 7 or at the end of a rollback.
- Hash check fails at `PUT` time: no write, report, re-run from a fresh read.

## Tests per transition (`tests/operations/cloudflare-oteryn-endpoints/**`)

One test per row of the transition table, plus: close A, open, close B and an external edit recreating the closed configuration both leave A's evidence unable to open; lost or mismatched record is unbound and refuses; crash between edge write and record write; stale evidence from an earlier close of the same task (rollback, second close, stop at step 4) refused; classification of one rule on `503` and the other on an arbitrary service as `unknown`, not `split`; `apply` refuses from `closed`, `split` and `unknown`; `maintenance-open` refuses without cutover-complete or rollback-verified evidence, with evidence from another task, and with a stopped step; refusal on another rule targeting `127.0.0.1`, `localhost`, `[::1]` or `0.0.0.0` on port 8000 or 8080 (including wildcard, legacy Gateway and catch-all); unrelated rules and the catch-all preserved byte for byte; hash mismatch aborts; read-back failure reported as failure; `audit` reports `closed` as bounded drift; confirmation phrase required; no secret or bypass value in output.

## Open questions for the reviewer

1. Is a GitHub environment variable acceptable as the durable closure record, or is another store wanted? Its reconcile rule against the edge is fixed above.
2. Is the task/run-bound evidence precondition for `maintenance-open` enforced by the workflow (this note's proposal) or by operator procedure only? `apply` refusal outside `open` is not optional.
