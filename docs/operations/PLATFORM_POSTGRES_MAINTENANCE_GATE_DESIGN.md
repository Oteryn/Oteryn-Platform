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

- **Marker (*proposal*).** The gate state is the ingress configuration itself: the `503` service on the two canonical rules. Each run logs the pre-write and post-write configuration hashes in its summary.
- **Closure identity: OPEN DESIGN QUESTION (P6b).** This note does not choose a mechanism. The requirement is stated in *Evidence precondition* below; how the workflow knows which closure it is looking at is open. See *Open design questions*.
- **Serialization (requirement; mechanism open).** Transitions must be serialized without ever dropping a `maintenance-close` or `maintenance-open`. A GitHub Actions `concurrency` group does not give that: with one run in progress it keeps at most one pending run and replaces an earlier pending run, even with `cancel-in-progress: false`. A pending `maintenance-close` could then be replaced by a later `audit` or other dispatch and the origins would stay open instead of the requested safety transition happening. This note therefore claims no concurrency-group lock. The pre-`PUT` re-read and hash check stays as the guard against an edit made outside the workflow. The serialization mechanism is an open design question below.

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

**Evidence precondition for `maintenance-open` (*proposal*).** From `closed` or `split` the mode refuses unless the dispatch names a task id and a run id and exactly one of these holds, checked by the workflow before any `PUT`. Requirement (not a mechanism): the evidence must belong to the closure that produced the current state. Evidence from an earlier closure of the same task, or from another task, must not authorize the open, and when the workflow cannot establish which closure is current it refuses. How closure identity is established is an open design question below.

1. Cutover ready to open: the cutover run for that task recorded step 6 as done (step 5 native login smoke passed and the cutover was declared done). Step 7 is the opening step, so this evidence is the authorization for `maintenance-open` and cannot require step 7 itself to be complete. The post-open `audit` and external requests are the step 7 result, not its precondition.
2. Rollback verified: the MariaDB rollback for that task finished and `health-check.sh` passed against it in a recorded run.

Under the requirement above, a retry needs fresh evidence: after a rollback and a second close, the first rollback's `health-check.sh` run does not authorize the open. A closed gate with a stopped step (for example step 4 on ambiguous trust state, or step 5 or 6 not recorded) satisfies neither case, so `maintenance-open` refuses and the gate stays closed. Missing, unreadable or unbound evidence also refuses. The evidence run ids are logged in the summary.

Ordering: read, validate, re-read and hash check, `PUT`, read back. A failed read-back leaves the state as read, reported as failure, never as success.

## Crash recovery

- Runner dies before the `PUT`: state unchanged. Re-run the same mode.
- Runner dies after the `PUT`: the new state stands. Re-running the same mode is a no-op.
- Runner dies during `maintenance-open` before its `PUT`: the edge stays closed (fail closed).
- Cutover script dies with the gate closed: the gate stays closed until `maintenance-open` runs in step 7 or at the end of a rollback.
- Hash check fails at `PUT` time: no write, report, re-run from a fresh read.

## Tests per transition (`tests/operations/cloudflare-oteryn-endpoints/**`)

One test per row of the transition table, plus: stale evidence from an earlier close of the same task (rollback, second close, stop at step 4) refused. The tests that exercise closure identity wait for the P6b decision below; classification of one rule on `503` and the other on an arbitrary service as `unknown`, not `split`; `apply` refuses from `closed`, `split` and `unknown`; `maintenance-open` refuses without cutover-complete or rollback-verified evidence, with evidence from another task, and with a stopped step; refusal on another rule targeting `127.0.0.1`, `localhost`, `[::1]` or `0.0.0.0` on port 8000 or 8080 (including wildcard, legacy Gateway and catch-all); unrelated rules and the catch-all preserved byte for byte; hash mismatch aborts; read-back failure reported as failure; `audit` reports `closed` as bounded drift; confirmation phrase required; no secret or bypass value in output.

## Open design questions for P6b (not decided here)

Decision owner for both: the owner, on a recommendation from the follow-up task PG-P6-DESIGN-NOTES. This PR asserts no mechanism for either.

1. **Where closure identity is stored.**
   - Constraint: the workflow must be able to write it with the credentials it is allowed to hold. Today `.github/workflows/cloudflare-oteryn-endpoints.yml` grants `contents: read` and supplies only the Cloudflare token. A GitHub environment variable cannot be updated with that token (the endpoint needs `Environments` write, which `GITHUB_TOKEN` does not expose), and the governing plan claims the gate needs no new credential.
   - Candidates, none chosen: a store the existing token can already write (for example something carried by the Cloudflare side or by the workflow's own artifacts); a separate GitHub App or token with the added credential explicitly authorized and designed; no stored identity, with `maintenance-open` authorized by an operator-supplied value checked against a run that the workflow can read.
2. **Binding identity to a non-replayable edge generation.**
   - Constraint: identical closed rules produce the same configuration hash, so rule content alone cannot prove which closure produced it. An external edit that opens and then restores the identical closed rules must not leave an earlier closure's evidence valid.
   - Candidates, none chosen: a nonce persisted atomically with the edge write; an edge-side version or generation value if the tunnel configuration API exposes one; treating any state not written by the workflow's own latest close as requiring a fresh close before an open.
3. **Serialization that cannot drop a close or open.**
   - Constraint: as stated under *Marker and lock*, a plain `concurrency` group can replace a pending run, and a dropped `maintenance-close` leaves the origins open.
   - Candidates, none chosen: a queue-capable concurrency setting if GitHub offers one for this workflow; refuse-if-busy (a dispatch that finds another transition running fails visibly instead of queuing) combined with a recorded intent that the operator re-dispatches; separate groups so non-mutating modes such as `audit` can never displace a mutating one. Tests: three runs dispatched while one is running, with a close among them.
   - Decision owner: the owner, on a recommendation from follow-up PG-P6-DESIGN-NOTES.

**Hard requirement, not a question:** the evidence precondition for `maintenance-open` is enforced inside the workflow, so the mutation itself refuses before any `PUT`. Operator procedure alone is not acceptable: a direct dispatch while a cutover is stopped at step 4, 5 or 6 must be refused by the workflow. The same holds for the `apply` refusal outside `open`.
