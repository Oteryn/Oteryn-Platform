---
task_id: OTERYN-20261116-a3-flakes
governing_issue: 536
required_reads: []
search_first: []
optional_reads: []
---

# Wave 3 parallel PostgreSQL flakes

Owner-authorized task on base 4cf2350e, branch
`claude/happy-davinci-ijz3wy-fix-a3-flk`; governing programme #536 is open.
Single writer; owned paths: control evidence transaction in admin, registration
abuse tests in identity, and this packet. Existing active packets and open PRs
were checked; no competing writer on these changes was found. No PR, merge,
deployment, external messages or Jira write requested. Jira mapping pending
coordinator. Runtime browser E2E is NOT_APPLICABLE: database concurrency and test
fixture timing repair, with real HTTP registration exercised in crate tests.

Investigation: claim filters include agent freshness (unavailable error rather
than empty result), issued/expiry times, committed audit, capabilities, target,
other agents, and SKIP LOCKED. A denied replay drops a transaction holding the
request lock; SQLx rollback is asynchronous. Registration uses two independent
fixed minute buckets on the database clock, despite preparing forms first.

Validation evidence is recorded below; publication follows the full crate checks.

Source branch disposition: retain for owner/coordinator review.

## Root causes and controlled evidence

Admin: the replay refusal returns after locking `admin_control_requests` FOR
UPDATE. SQLx transaction Drop queues rollback, and pool maintenance later flushes
it. `claim_pending` can use another connection before that flush and its
`FOR UPDATE OF r SKIP LOCKED` excludes the still-locked row. The fix explicitly
awaits rollback on every evidence refusal before returning. The regression holds
pool return in `after_release` (before SQLx's rollback-flushing ping), without
sleep, retries or timeout changes. The unfixed implementation reproducibly fails
with claimed length 0, expected 1 (11.39 seconds); fixed implementation passes
with the identical gate.

All claim filters were inspected. The only agent is `claim-test`, epoch 1,
with the required `world.configure` version/kind and served world `test`; there
is no conflicting agent or node target. Agent freshness failure returns
Unavailable rather than an empty claim. The request remains queued, its four
minute expiry is live, and committed evidence is present before the replay.
`issued_at` uses the application clock but evidence attachment already checks it
against the database clock before this claim; the deterministic failure needs
no clock skew. No lifetime, ownership, capability or audit condition was relaxed.

Identity: `identity-registration-attempt` (identity adapter) and
`identity-registration` (web route) each allow five requests per fixed epoch
minute, keyed by the source IP. They use `clock_timestamp()` and
`statement_timestamp()` respectively. Preparing forms before submission does
not prevent the submission/password-hashing sequence crossing a minute boundary.
The sixth request can therefore get 303 instead of 429. Schemas isolate both
`web_rate_limits` and the fixed source IP from other tests; the shared source
within this test is deliberate. Work TTL is five minutes and password-range TTL
is an hour; neither explains this controlled reproduction.

A schema-local SQL clock controls both budget clocks without changing production
code. Negative experiment: advance it by exactly one minute before submission
six, retaining the original assertions: red, actual 303 / expected 429 (15.83
seconds). Final test holds it fixed through all seven original assertions,
checks exact counters (web 5, identity saturated at 6), checks two unconsumed work
proofs, then explicitly advances one minute and proves the same refused form
succeeds and consumes exactly one proof. Both original failure assertions remain.

Logs are local under `/tmp/a3-evidence/`: `admin-controlled-red.log`,
`identity-controlled-red.log`, and the corresponding `*-green.log`.
Initial broad load on the setup's default PostgreSQL lock-table capacity produced
SQLSTATE 53200. Local `max_locks_per_transaction` was set to 1024, matching the
committed CI service configuration; server version is 180006. All task runs use
`PORTAL_TEST_DATABASE_URL` pointing to the disposable `portal_a3_flakes` database;
no URL or credentials are recorded. Rust is the repository-pinned 1.97.0.

## Loaded validation

Ten consecutive pairs of target runs passed (20 successes, zero failures), each
launched concurrently with the other crate's target, both pinned to CPU 0–1:

```text
taskset -c 0-1 cargo test --locked -p oteryn-admin stored_evidence_accepts_bound_proof_refuses_replay_foreign_stale_and_audit_failure -- --test-threads=16
taskset -c 0-1 cargo test --locked -p oteryn-identity audit02_registration_dispatch_charges_before_work_across_fresh_sessions -- --test-threads=16
```

Full admin tests also ran concurrently with most repetitions. Target durations
were 1.41–5.38 seconds (admin), 5.80–10.16 seconds (identity). Every log reports
exactly one selected test passed, zero ignored; identity's separate integration
binary correctly reports zero selected tests. Summary: `green-summary.log`,
individual logs: `green-{admin,identity}-{1..10}.log` in the local evidence path.

`cargo fmt --all --check`: PASS.
`cargo clippy --locked -p oteryn-admin -p oteryn-identity --all-targets -- -D warnings`:
PASS. `git diff --check`: PASS. No dependency, migration or production budget
changes; no added retries, sleeps, serial markers, ignores, allows or weakened
assertions.

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-11-16T00:27:19Z
head: 4cf2350ed8a2902790d004f7ba20728f544ef73f
branch: claude/happy-davinci-ijz3wy-fix-a3-flk
pr: none
status: ready
context_routes: []
owned_paths:
  - portal-rs/crates/admin/src/control/integration.rs
  - portal-rs/crates/admin/src/control/integration_tests.rs
  - portal-rs/crates/identity/src/abuse/tests.rs
  - docs/agents/tasks/active/OTERYN-20261116-a3-flakes.md
proven:
  - Controlled admin lock-release red and green
  - Controlled identity minute-rollover red and green
  - Ten consecutive loaded target pairs passed
  - Full admin suite passed 170 tests with no ignores
  - Full identity suite passed 154 unit and 13 integration tests with no ignores
  - Formatting and Clippy passed
derived: []
unknown: []
conflicts: []
first_failure:
  marker: admin queued rollback retains row lock
  evidence: /tmp/a3-evidence/admin-controlled-red.log
rejected_hypotheses:
  - Admin claim agent expiry and schema interference do not explain controlled failure
  - Identity source-IP interference from other schemas is not required
changed_paths:
  - portal-rs/crates/admin/src/control/integration.rs
  - portal-rs/crates/admin/src/control/integration_tests.rs
  - portal-rs/crates/identity/src/abuse/tests.rs
  - docs/agents/tasks/active/OTERYN-20261116-a3-flakes.md
validation:
  - command: cargo fmt --all --check
    result: PASS
    evidence: /tmp/a3-evidence/fmt.log
  - command: cargo clippy --locked -p oteryn-admin -p oteryn-identity --all-targets -- -D warnings
    result: PASS
    evidence: /tmp/a3-evidence/clippy.log
  - command: loaded target tests repeated ten times
    result: PASS
    evidence: /tmp/a3-evidence/green-summary.log
  - command: cargo test --locked --no-fail-fast -p oteryn-admin
    result: PASS
    evidence: /tmp/a3-evidence/full-admin.log
  - command: cargo test --locked --no-fail-fast -p oteryn-identity
    result: PASS
    evidence: /tmp/a3-evidence/full-identity.log
blockers:
  - none
next_action: Publish the single owner-authorized commit and verify the remote head
```


## Complete crate results and publication scope

- `cargo test --locked --no-fail-fast -p oteryn-admin`: 170 passed,
  zero failed or ignored; doc tests zero (105.83 seconds for unit tests).
- `cargo test --locked --no-fail-fast -p oteryn-identity`: 154 unit tests and
  13 integration tests passed, zero failed or ignored; doc tests zero
  (105.08 seconds unit, 8.47 seconds integration).

The initial broad admin reproduction also encountered the existing
`postgres_a3_commerce_locks_payment_owner_before_role_fence` assertion against
database-wide fence 71010. That test was not modified; the final complete suite
passed it. No broad initial run with insufficient PostgreSQL lock capacity is
counted as green evidence.

One commit is requested, titled
`fix(portal-rs): root-cause wave-3 parallel test flakes`, authored by
blakinio <75369544+blakinio@users.noreply.github.com>. Publication is solely to
`Oteryn/Oteryn-Platform` on the task branch, without PR creation, merge or
production effects. Retain the branch for owner/coordinator review; Jira sync
remains pending coordinator. The validated source candidate differs from the
base only in the three Rust files and this task evidence packet.
