---
task_id: OTERYN-20261116-a3-drc-a
governing_issue: 536
required_reads: []
search_first: []
optional_reads: []
---

# Audit 3 DRC specified delta repairs

Owner-authorized base 8dd4e622, branch claude/happy-davinci-ijz3wy-fix-a3-drc-a.
Live programme #536 is OPEN; open PR inventory checked without overlapping audit PR.
Single writer. Scope is staff kick post-wait authorization, Bazaar listing gate,
gift request and catalog audit budgets, player-data account/issuance ordering,
download publication RBAC fencing, associated PostgreSQL tests and hierarchy README.
No migration changes. No PR creation, merge or deployment. Jira sync is coordinator-owned.

Validation completed below. Every cargo test invocation uses timeout 1200.

## Red evidence

PostgreSQL 18.6 (`server_version_num=180006`), disposable schema fixtures.
`timeout 1200 cargo test ... drc_ -- --nocapture` captured the kick failure in
`/tmp/drc-red.log`: expired grant still returned `Ok(2)`.
The remaining packages ran once with `--no-fail-fast`; `/tmp/drc-red-rest.log`
records five expected failures: listing gate/gift request/catalog audit exceeded
five-second observation bounds; publication bypassed RBAC; admission retained
issuance while waiting for the creation account lock. These are expected red
failures, not unresolved validation failures or retry loops.

All new global-key tests acquire `global_lock_guard` before fixture setup.
Concurrency barriers query PostgreSQL blocking PIDs / lock wait state and have
explicit fail-fast deadlines. The grant-expiry barrier polls the database clock.
No sleeps, ignored tests, migration edits or timeout increases were introduced.

## Implementation

- Kick rechecks durable authorization after `staff_kick` returns from its trigger wait.
- Bazaar listing gate, gift request key 55 and catalog audit key 281510004 establish
  the existing transaction-wide mutation budget before acquisition.
- Admission takes account before issuance, matching web character creation.
- Download publication uses the canonical account/resource/RBAC helper to retain
  account, channel pointer and fence, then rechecks permission before installation.
- The RBAC README remains the sole hierarchy definition.

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-11-16T06:35:08Z
head: 8dd4e622
branch: claude/happy-davinci-ijz3wy-fix-a3-drc-a
pr: none
status: waiting
context_routes: []
owned_paths:
  - portal-rs/crates/admin/src/players/
  - portal-rs/crates/admin/src/rbac/README.md
  - portal-rs/crates/bazaar/src/locking.rs
  - portal-rs/crates/bazaar/tests/postgres.rs
  - portal-rs/crates/store/src/codes.rs
  - portal-rs/crates/store/src/codes_tests.rs
  - portal-rs/crates/playerdata/
  - portal-rs/crates/downloads/
  - portal-rs/bin/oteryn-portal/src/catalog_cli.rs
  - portal-rs/bin/oteryn-portal/src/catalog_cli_tests.rs
  - portal-rs/Cargo.lock
  - docs/agents/tasks/active/OTERYN-20261116-a3-drc-a.md
proven:
  - All six regression tests fail on predecessor implementations and pass with fixes
  - PostgreSQL server_version_num is 180006
  - Formatting and workspace Clippy with warnings denied pass
  - All requested full crate suites and both explicit Chromium tests pass
unknown:
  - none
derived: []
conflicts: []
first_failure:
  marker: none
  evidence: none
rejected_hypotheses: []
changed_paths:
  - portal-rs/
validation:
  - command: cargo fmt --check
    result: PASS
    evidence: /tmp/drc-fmt.log
  - command: timeout 1200 cargo test selected packages --no-fail-fast drc_ -- --nocapture
    result: PASS
    evidence: /tmp/drc-green.log; 6 passed, zero failed or ignored
  - command: cargo clippy --workspace --all-targets -- -D warnings
    result: PASS
    evidence: /tmp/drc-clippy-final.log; Rust 1.97.0
  - command: timeout 1200 cargo test -p each requested crate -- --test-threads=1; explicit browser selections under timeout 1200
    result: PASS
    evidence: /tmp/drc-full-oteryn-*.log plus /tmp/drc-store-browser.log and /tmp/drc-portal-browser.log; 1051 distinct tests passed
blockers:
  - none
next_action: Owner or coordinator reviews the delivered task branch; no merge requested
```

## Validation environment correction

The initial full Bazaar run encountered 20 migration setup failures in the
pre-existing `oteryn_test` database: migration 20261105151420 tried to create
`wallet_accounts`, already present without that ledger entry. The first log is
`/tmp/drc-full-oteryn-bazaar-first.log`. No applied migration or existing database
was changed. A new disposable database `drc_a_20261116_clean` on the same verified
PostgreSQL 18.6 server supplies the second Bazaar attempt and subsequent suites.
Admin's 172 tests had already passed using isolated schema fixtures. No test is
retried beyond the owner's two-attempt bound.

## Explicit browser qualification

Store's default suite has one pre-existing ignored browser test. It was explicitly
executed rather than left unrun. Its first attempt could not load the setup's
missing `@axe-core/playwright` package. The repository E2E package manifests were
copied to `/tmp/drc-browser`, and locked dependencies installed there using a
writable temporary npm cache. Neither repository manifests nor generated product
files were changed. Chromium and axe startup were verified before the second
(and final) test attempt.

`timeout 1200 cargo test -p oteryn-store web::tests::postgres_browser_at_360px -- --ignored --exact --nocapture`
passed: 8 authenticated shop pages at 360px, keyboard navigation, successful
transfer and refusal, axe zero violations. Evidence: `/tmp/drc-store-browser.log`.
The full store proof is 120 default tests plus this explicitly selected test,
121 distinct executed tests passed. The existing ignore annotation was not added,
removed or weakened by this task.

## Standalone bin test feature

Standalone `oteryn-portal` compilation exposed that the new isolation-guard test
needed an explicit `oteryn-db/test-support` dev-dependency. The aggregate focused
run and workspace Clippy had supplied that test feature through other selected
packages. The bin now declares its own test dependency, and every PostgreSQL test
in the touched catalog CLI module starts with the isolation guard. No assertions
were changed. Formatting and full workspace Clippy were rerun and pass;
`/tmp/drc-clippy-final.log` is the final Clippy evidence. The standalone full bin
suite is then run with the same 1200-second external bound.

## Final PostgreSQL 18.6 validation

All commands ran on the final material source with Rust 1.97.0. The only final
bin adjustment after library qualification was its dev-only isolation feature
and guards in the catalog CLI tests; formatting, workspace Clippy and the entire
standalone bin suite were then rerun successfully.

- `cargo fmt --check`: PASS.
- `cargo clippy --workspace --all-targets -- -D warnings`: PASS.
- Each package: `timeout 1200 cargo test -p <package> -- --test-threads=1`.
  Concurrency regressions still run their competing transactions concurrently.
- Both pre-existing ignored browser cases were explicitly selected and passed.
  No selected test remains unexecuted. No new ignore/allow, weakened assertion,
  deleted test, timeout increase or migration change was introduced.

| Package | Distinct tests passed |
|---|---:|
| oteryn-admin | 172 |
| oteryn-bazaar | 132 |
| oteryn-store | 121 |
| oteryn-characters | 94 |
| oteryn-playerdata | 157 |
| oteryn-downloads | 16 |
| oteryn-identity | 167 |
| oteryn-portal | 192 |
| **Total** | **1051** |

Store includes its 120 default cases plus the selected shop browser case. Portal
includes 191 default cases (including executable integration tests) plus its
selected checkout browser case. The latter exercised PL/EN top-up, the dummy
provider, keyboard payment, real test worker credit and transfer at 360px, with
zero axe violations. Evidence: `/tmp/drc-portal-browser.log`.

## Source branch closeout

```yaml
source_branch_disposition: retain
source_branch_reason: owner requested branch delivery for review; no merge requested
source_branch_evidence: owner task request and final normal-push remote SHA readback
```
