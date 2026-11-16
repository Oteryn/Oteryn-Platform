---
task_id: OTERYN-20261116-a3-drc
governing_issue: 536
required_reads: []
search_first: []
optional_reads: []
---

# Exhaustive lock-site and mutation-authority sweep

Repository: Oteryn/Oteryn-Platform. Governing issue #536 was read live and is OPEN.
Owner-authorized base: `8dd4e622f18d273d80a5f5014c790e530e095cad`.
Branch: `claude/happy-davinci-ijz3wy-fix-a3-drc-b`. Commit and push to this branch
are authorized; no PR creation, merge or deployment is requested. Jira remains
with the coordinator. Initial checkout HEAD was
`90234d8d2d0a51a654b8ff6947d38372a740acba`; Step 0 fetched the supplied branch
and created the allocated branch at the required base.

The normative hierarchy remains solely in
`portal-rs/crates/admin/src/rbac/README.md`. Table levels below reference it.
Tests' deliberate blocker transactions are not application mutations; budget and
live authorization are n.a. for those blockers. Pure references and unlocked
reads are n.a. They remain in the mechanical hit inventory to make exclusions
reviewable. A namespaced serialization prefix has its domain-specific order in
the README advisory inventory; it is not a numerically sorted global key order.

## Acceptance and ownership

- Mechanically enumerate locks, permission readers and privileged writers,
  including SQL migrations and historical trigger bodies.
- Review both explicit and implicit tuple/FK/trigger locks and every opposing
  acquisition edge; fix remaining defects outside drc-a with PostgreSQL red/green
  regressions.
- Preserve applied migrations, acquisition deadlines and every existing assertion.
- Run formatting, workspace Clippy and every required/touched crate on PG 18.6.
- Deliver the requested author, commit message/trailers and allocated branch only.

Owned paths: this packet, the RBAC hierarchy README, remaining portal lock sites
and their regression tests. Shared paths reserved to drc-a are excluded below.
New and touched PostgreSQL tests using global advisory keys retain a global
test guard from test entry through cleanup. The direct numerical-key test scan
also found and repaired missing guards in existing caller modules. Interleavings poll observable PostgreSQL lock
waits with an explicit bound and failure message; no sleeps were introduced.

## Parallel task boundary

These sites are intentionally not edited by this task. The status is the owner's
requested integration disposition, not a claim that this branch includes drc-a.

| Site | Lock(s) and mode | Level | Budget before first lock | Authorization after last wait | Compliant |
|---|---|---|---|---|---|
| `portal-rs/crates/admin/src/players/mod.rs` staff kick | accounts UPDATE; RBAC exclusive; security-revocation effects | 3,5,7 | yes | Same-transaction authority rechecked after final write before commit (MFA approval also request expiry) | fixed in parallel task drc-a |
| `portal-rs/crates/bazaar/src/locking.rs` listing_gate | advisory 64002 shared | 1 | no | n.a. | fixed in parallel task drc-a |
| `portal-rs/crates/store/src/codes.rs` request key 55 | request advisory exclusive; code UPDATE; batch/account key 56; account UPDATE | 2,3,7 | no | n.a. | fixed in parallel task drc-a |
| `portal-rs/bin/oteryn-portal/src/catalog_cli.rs` audit key 281510004 | exclusive advisory in audit transaction | namespace prefix | no | n.a. | fixed in parallel task drc-a |
| `portal-rs/crates/characters/src/create_web/service.rs` | account NO KEY UPDATE versus publication advisory 22005160000 | 3, publication fence | no | n.a. | fixed in parallel task drc-a |
| `portal-rs/crates/characters/src/commands/storage.rs` | publication advisory 22005160000; command/restore rows | 3, publication fence, resources | no | n.a. | fixed in parallel task drc-a |
| `portal-rs/crates/playerdata/src/authority.rs` | publication advisory shared; accounts SHARE; ownership/restore SHARE | 3, publication fence, resources | no | n.a. | fixed in parallel task drc-a |
| `portal-rs/crates/downloads/src/command.rs` | publication permission reader | caller publication protocol | no | yes | fixed in parallel task drc-a |
| `portal-rs/crates/downloads/src/storage.rs` | channel UPDATE/publication fence | caller publication protocol | no | yes | fixed in parallel task drc-a |

## Regressions and findings

- Collection PvP: actor-then-opponent SHARE locks can deadlock an ordered RBAC
  UPDATE writer. Challenge, accept, play and start prepare all participant accounts
  in UUID order and verify discovered bindings after resource acquisition. Start
  refuses a changed entrant set rather than expanding its account set mid-flight.
  Red: the ordered writer failed with PostgreSQL 55P03 acquiring the later account.
  Green: `lock_order_challenge_prepares_all_accounts_before_waiting` passes.
- Browser session rotation: UPDATE implicitly retains the session before its
  authentication trigger waits for the account. Prepare previous/new accounts in
  UUID order before UPDATE. Release request-memory ownership before account waits
  and verify that the request identity is unchanged before applying rotation.
  Red: 55P03 acquiring the session under the retained account. Green:
  `lock_order_rotation_locks_account_before_session` passes. The first draft of
  the regression omitted required fixture locale; this setup failure is not red
  evidence. The corrected fixture reproduces the actual inversion.
- Referral eligibility: SQL-function calls in an aggregate/WHERE clause do not
  establish UUID lock order. A forced sequential scan reproduces the later-account
  lock retained while waiting for the earlier one. Prepare both accounts before
  eligibility evaluation. Red: 55P03; green:
  `lock_order_policy_prepares_accounts_before_eligibility_evaluation` passes.
- Payment intent/account order: browser top-up delivery, generic refunds and
  native coin webhooks now prepare immutable ownership and the account before the
  intent, verify the retained binding, then perform current policy checks after
  the intent wait. Two regression tests cover all three entry paths. Red: 55P03
  on the other transaction's intent acquisition in each path. Green: both pass.
- Character service receipt publication: retain the account and publication fence
  before an existing command/order. Issuance takes that fence before recovery
  credits and wallet effects, preventing an opposite credit/fence edge. The
  INSERT-only trigger does not serialize receipt UPDATEs: the second red probe
  failed because receipt processing never waited on a deliberately held
  publication fence. The regression checks both account and publication blockers;
  green passes both without retaining the command before either earlier wait.
  A separate recovery-credit regression fails with 55P03 when issuance retains
  its credit before the publication fence and passes with the new ordering.

The Bazaar and player-data browser session fixtures now include the real
account migration and actor rows. Browser rotation requires that identity resource; its former isolated
fixture had only test_accounts and web_sessions. No existing assertion changed.

## Validation environment and evidence

PostgreSQL `server_version_num=180006` was queried live. The default test database
contains wallet tables without corresponding migration records. Initial baseline:
Admin 171 passed; Bazaar 91 passed / 20 migration-setup failures (42P07,
wallet_accounts already exists). A fresh local database `portal_drc_b_a3` was
created without modifying the existing database. Credentials remain process-only.
Rust is explicitly 1.97.0, the repository pin; the prepared environment defaults
its shell to 1.100.0, which was used only for the initial baseline.

Final validation is BLOCKED under the owner's two-attempt stop rule. Every
Cargo test invocation used `timeout 1200`.
Full crate runs include existing ignored browser tests with `--include-ignored`;
Chromium and the existing acceptance modules are supplied explicitly. The first
workspace Clippy run caught new-test expect calls; those were replaced with typed
error propagation. Workspace Clippy subsequently passed without lint exceptions.

## Blocked validation report

The Portal bin's full suite passed 187 tests and failed two:

1. `game_login::tests::device_sanctions::audit05_03_device_routes_refuse_effective_suspension`:
   initial enrollment returned HTTP 503 instead of 200 (first failure only).
2. `large_pages_tests::l5_league_fourteen_full_seasons_are_complete`:
   the full run returned `Error: Unavailable`; the one focused second attempt
   reached the page assertion and returned HTTP 503 instead of 200, at
   `bin/oteryn-portal/src/large_pages_tests.rs:375`.

The second League failure triggered the owner's instruction: **“If one test keeps
failing after two attempts, stop and report it rather than looping.”** No further
Cargo test was attempted after that result until the owner explicitly requested
the single separate-checkout base comparison recorded below. The device case was not retried after
this stop. No timeout, acquisition budget, assertion or fixture capacity was
increased to obtain a pass. Publication was initially withheld because the requested all-green proof was
incomplete. The owner subsequently explicitly authorized committing and pushing
the current working tree as-is for local root-cause analysis; validation remains
blocked, not green.

Evidence: `/tmp/a3-oteryn-portal-tests.log` and
`/tmp/a3-portal-league-second.log`. The League implementation and its large-page
fixture are unchanged by this task. The separate-checkout comparison below also
fails on the base; it does not establish the cause of either Portal failure. Investigating the failing phase remains
necessary. The existing three-second League read bound remains unchanged.

| Crate | Recorded full-suite passed | Failed | Scope/evidence |
|---|---:|---:|---|
| oteryn-admin | 171 | 0 | Earlier full pass; planned rerun after final payment changes not reached |
| oteryn-bazaar | 131 | 0 | Full pass, including the strengthened session fixture |
| oteryn-minigames | 67 | 0 | Full pass, including new PvP regression |
| oteryn-web | 60 | 0 | Full pass, including forced browser execution after external dependency setup |
| oteryn-wiki | 37 | 0 | Full pass |
| oteryn-world | 63 | 0 | Full pass |
| oteryn-store | 120 | 0 | Full pass, including forced shop browser execution |
| oteryn-characters | 95 | 0 | Final full pass, including both service regressions |
| oteryn-playerdata | 156 | 0 | Full pass; strengthened/guarded browser fixture also passed its focused final check |
| oteryn-downloads | 15 | 0 | Full pass |
| oteryn-identity | 167 | 0 | Full pass |
| Completed full-suite total | 1082 | 0 | Recorded stages above; not a single final-candidate qualification |
| oteryn-portal bin unit suite | 187 | 2 | Full run failed; package integration targets were not reached |
| oteryn-payments | — | — | Full suite not reached after Portal failure |
| oteryn-community | — | — | Full suite not reached after Portal failure |
| oteryn-gameauth | — | — | Full suite not reached after Portal failure |

Nested filtered helper subprocess results are not double-counted. Every completed
full suite used `--include-ignored --test-threads=1`, with zero ignored tests.
All seven new PostgreSQL regression tests have observed red/green results; their
passes are included in the crate totals, not counted again. The additional
focused helper/fixture runs are likewise not added to totals.

| Check | Result | Evidence |
|---|---|---|
| `cargo fmt --all --check` | PASS | `/tmp/a3-fmt-check.log` |
| `cargo clippy --workspace --all-targets -- -D warnings` | PASS | `/tmp/a3-clippy-final.log` |
| Seven new PostgreSQL regressions | PASS red/green | `/tmp/a3-pvp-{red,green}.log`, corrected session red/green logs, referral red/green logs, payment red/green logs, character service/credit red logs and final full Characters run |
| Per-pattern grep enumeration and multiline cross-check | PASS | 5205 union sites, 704 multiline output lines, zero uncovered multiline lines |
| Reserved drc-a paths unchanged | PASS | Explicit changed-path intersection was empty |
| Applied migrations unchanged | PASS | No migration in changed-path list |
| Task checkpoint structure / diff whitespace | PASS | `tools/agents/checkpoint.py` and `git diff --check` |
| Required all-green final candidate | BLOCKED | Repeated League failure; remaining full suites not run |

Browser dependencies were installed only under `/tmp/a3-browser-deps`:
`@axe-core/playwright@4.11.1` and `playwright-core@1.64.1`. Chromium 155 was used;
no repository dependency lockfile changed. The first Web browser attempt lacked
the axe package; its second attempt passed after external setup. The initial
player-data browser fixture lacked accounts; its corrected second attempt passed.
Existing assertions and ignored-test annotations were preserved, and existing
ignored browser tests were explicitly selected for execution.

Successful full package invocations followed this shape (database URL supplied
only through process environment):

```sh
timeout 1200 cargo test -p <crate> -- --include-ignored --test-threads=1
```

The final working-tree diagnostic attempt before the stop was:

```sh
timeout 1200 cargo test -p oteryn-portal large_pages_tests::l5_league_fourteen_full_seasons_are_complete -- --exact --nocapture
```

No retry loop is pending. The owner subsequently requested as-is branch
publication for local root-cause analysis, with the original author/message/
trailers. This publication does not change the failed validation disposition.

## Exact Portal failure attempts and base comparison

The two failing Portal tests are:

- `game_login::tests::device_sanctions::audit05_03_device_routes_refuse_effective_suspension`
- `large_pages_tests::l5_league_fourteen_full_seasons_are_complete`

Only the task document was changed for this delivery follow-up. No test body,
assertion, deadline, skip/ignore annotation or production implementation was
changed to obtain these diagnostics. Missing fields below are explicitly marked
**not captured**: the existing tests did not print response bodies or low-level
PostgreSQL errors. HTTP 503 or `Unavailable` alone cannot establish a SQLSTATE.
No response body or PostgreSQL failure is reconstructed or guessed.

### Working tree: device suspension, attempt 1 (full Portal suite)

Command, from `/workspace/Oteryn-Platform/portal-rs`:

```sh
timeout 1200 cargo test -p oteryn-portal -- --include-ignored --test-threads=1
```

- Actual status: HTTP **503**; expected **200** at the initial enrollment assertion.
- Response body: **not captured** by the test output.
- PostgreSQL error / SQLSTATE: **not reported** in the captured output; underlying
  database error information, if any, is unavailable.
- Individual test duration: **not captured**; the assertion failed before the
  test's success-path timing print. The shared full unit suite took **564.15 s**;
  its Cargo build took **1m 51s**. These are not this test's individual duration.
- Full Cargo invocation exited **101**. This test was attempted once; the planned
  focused device rerun was never reached because the League rerun failed first.

Exact captured error output:

```text
thread 'game_login::tests::device_sanctions::audit05_03_device_routes_refuse_effective_suspension' (45274) panicked at bin/oteryn-portal/src/game_login_devices_sanctions_tests.rs:124:5:
assertion `left == right` failed
  left: 503
 right: 200
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace
```

### Working tree: League, attempt 1 (same full Portal suite)

- Error: **`Unavailable`**, returned without a status-code assertion.
- HTTP status and response body: **not captured**. The output does not identify
  which fallible phase returned the error.
- PostgreSQL error / SQLSTATE: **not reported**; cannot be inferred.
- Individual duration: **not captured**. Shared full unit suite: **564.15 s**;
  shared Cargo build: **1m 51s**; invocation exit: **101**.

Exact captured per-test error output:

```text
Error: Unavailable
```

Exact shared suite footer:

```text
failures:
    game_login::tests::device_sanctions::audit05_03_device_routes_refuse_effective_suspension
    large_pages_tests::l5_league_fourteen_full_seasons_are_complete

test result: FAILED. 187 passed; 2 failed; 0 ignored; 0 measured; 0 filtered out; finished in 564.15s

error: test failed, to rerun pass `-p oteryn-portal --bin oteryn-portal`
```

### Working tree: League, attempt 2 (focused diagnostic)

```sh
timeout 1200 cargo test -p oteryn-portal large_pages_tests::l5_league_fourteen_full_seasons_are_complete -- --exact --nocapture
```

- Actual status: HTTP **503**; expected **200**, at `large_pages_tests.rs:375`.
- Response body: **not captured**, although the test had collected the response
  bytes before the assertion; those bytes were not printed or saved.
- PostgreSQL error / SQLSTATE: **not reported**; cannot be inferred.
- Focused test runtime: **15.38 s**; Cargo build: **1.17 s**; exit: **101**.
- Total process wall time was not independently captured for this attempt.

Exact captured test output:

```text
running 1 test

thread 'large_pages_tests::l5_league_fourteen_full_seasons_are_complete' (46722) panicked at bin/oteryn-portal/src/large_pages_tests.rs:375:5:
assertion `left == right` failed
  left: 503
 right: 200
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace
test large_pages_tests::l5_league_fourteen_full_seasons_are_complete ... FAILED

failures:

failures:
    large_pages_tests::l5_league_fourteen_full_seasons_are_complete

test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 188 filtered out; finished in 15.38s

error: test failed, to rerun pass `-p oteryn-portal --bin oteryn-portal`
```

### Untouched base: League, exactly one owner-authorized comparison

**It does not pass on base `8dd4e622` in this environment.**

A separate detached checkout was created at
`/workspace/Oteryn-Platform-a3-drc-base`, with HEAD exactly
`8dd4e622f18d273d80a5f5014c790e530e095cad`. `git status --short` there was empty
after the test. Neither the test nor the base implementation was changed.
Rust **1.97.0**, PostgreSQL **18.6** and the same local test database
`portal_drc_b_a3` were used. The only base test invocation was:

```sh
cd /workspace/Oteryn-Platform-a3-drc-base/portal-rs
timeout 1200 cargo test -p oteryn-portal large_pages_tests::l5_league_fourteen_full_seasons_are_complete -- --exact --nocapture
```

- Result: **FAIL**, **`Error: Unavailable`**; exit **101**.
- HTTP status and response body: **not captured**; no status assertion was printed.
- PostgreSQL error / SQLSTATE: **not reported**; cannot be inferred.
- Test runtime: **12.16 s**; Cargo build: **1m 51s**.
- Independently measured command wall time, including build: **123.928 s**.
- Start (UTC): **2026-11-16T07:32:11.592648+00:00**.
- Evidence: `/tmp/a3-base-league-once.log` and `/tmp/a3-base-league-once.json`.

Exact captured base test output:

```text
running 1 test
Error: Unavailable
test large_pages_tests::l5_league_fourteen_full_seasons_are_complete ... FAILED

failures:

failures:
    large_pages_tests::l5_league_fourteen_full_seasons_are_complete

test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 186 filtered out; finished in 12.16s

error: test failed, to rerun pass `-p oteryn-portal --bin oteryn-portal`
```

This establishes reproduction on the unchanged base in this environment; it
neither explains the failure nor proves the device-session failure has the same
cause. No further test attempt was made for this follow-up.

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-11-16T07:36:09Z
head: 8dd4e622f18d273d80a5f5014c790e530e095cad
branch: claude/happy-davinci-ijz3wy-fix-a3-drc-b
pr: none
status: blocked
context_routes: []
owned_paths:
  - "docs/agents/tasks/active/OTERYN-20261116-a3-drc.md"
  - "portal-rs/bin/oteryn-portal/Cargo.toml"
  - "portal-rs/bin/oteryn-portal/src/bootstrap_intent_tests.rs"
  - "portal-rs/bin/oteryn-portal/src/catalog_cli_tests.rs"
  - "portal-rs/bin/oteryn-portal/src/coin_purchases_tests.rs"
  - "portal-rs/bin/oteryn-portal/src/wallet_topup.rs"
  - "portal-rs/bin/oteryn-portal/src/worker_tests.rs"
  - "portal-rs/crates/admin/src/rbac/README.md"
  - "portal-rs/crates/bazaar/src/characters/tests.rs"
  - "portal-rs/crates/characters/src/services/mod.rs"
  - "portal-rs/crates/characters/src/services/tests.rs"
  - "portal-rs/crates/community/Cargo.toml"
  - "portal-rs/crates/community/src/referrals/campaign.rs"
  - "portal-rs/crates/community/src/referrals/tests.rs"
  - "portal-rs/crates/community/src/referrals_tests.rs"
  - "portal-rs/crates/gameauth/Cargo.toml"
  - "portal-rs/crates/gameauth/src/keys/tests.rs"
  - "portal-rs/crates/minigames/Cargo.toml"
  - "portal-rs/crates/minigames/src/collection_pvp/mod.rs"
  - "portal-rs/crates/minigames/src/collection_pvp/tests.rs"
  - "portal-rs/crates/minigames/src/league/tests.rs"
  - "portal-rs/crates/payments/src/coin_store.rs"
  - "portal-rs/crates/payments/src/storage.rs"
  - "portal-rs/crates/playerdata/Cargo.toml"
  - "portal-rs/crates/playerdata/src/authority/tests.rs"
  - "portal-rs/crates/playerdata/src/items/tests.rs"
  - "portal-rs/crates/web/Cargo.toml"
  - "portal-rs/crates/web/src/session.rs"
  - "portal-rs/crates/web/src/session_tests.rs"
  - "portal-rs/crates/wiki/Cargo.toml"
  - "portal-rs/crates/wiki/src/web/tests.rs"
  - "portal-rs/crates/world/Cargo.toml"
  - "portal-rs/crates/world/src/admin_pool_tests.rs"
  - "portal-rs/crates/world/src/admin_tests.rs"
proven:
  - PostgreSQL server version is 180006 and validation selected Rust 1.97.0
  - All seven new PostgreSQL regressions have observed red and green results
  - Recorded completed full suites passed 1082 tests at the stages listed above
  - Reserved drc-a paths and applied migrations are unchanged
  - Portal full unit suite passed 187 tests and failed two
  - The League large-page test failed again on its one focused second attempt
  - One separate clean checkout run reproduces League Unavailable on base 8dd4e622
  - Owner explicitly authorized publishing this working tree as-is despite blocked validation
derived: []
unknown:
  - Cause of the two Portal failures
  - Full Payments and Community and Gameauth results and final Admin rerun
  - Complete final-candidate all-green qualification
conflicts: []
first_failure:
  marker: portal_league_failed_twice_stop_rule
  evidence: /tmp/a3-oteryn-portal-tests.log and /tmp/a3-portal-league-second.log
rejected_hypotheses: []
changed_paths:
  - "docs/agents/tasks/active/OTERYN-20261116-a3-drc.md"
  - "portal-rs/bin/oteryn-portal/Cargo.toml"
  - "portal-rs/bin/oteryn-portal/src/bootstrap_intent_tests.rs"
  - "portal-rs/bin/oteryn-portal/src/catalog_cli_tests.rs"
  - "portal-rs/bin/oteryn-portal/src/coin_purchases_tests.rs"
  - "portal-rs/bin/oteryn-portal/src/wallet_topup.rs"
  - "portal-rs/bin/oteryn-portal/src/worker_tests.rs"
  - "portal-rs/crates/admin/src/rbac/README.md"
  - "portal-rs/crates/bazaar/src/characters/tests.rs"
  - "portal-rs/crates/characters/src/services/mod.rs"
  - "portal-rs/crates/characters/src/services/tests.rs"
  - "portal-rs/crates/community/Cargo.toml"
  - "portal-rs/crates/community/src/referrals/campaign.rs"
  - "portal-rs/crates/community/src/referrals/tests.rs"
  - "portal-rs/crates/community/src/referrals_tests.rs"
  - "portal-rs/crates/gameauth/Cargo.toml"
  - "portal-rs/crates/gameauth/src/keys/tests.rs"
  - "portal-rs/crates/minigames/Cargo.toml"
  - "portal-rs/crates/minigames/src/collection_pvp/mod.rs"
  - "portal-rs/crates/minigames/src/collection_pvp/tests.rs"
  - "portal-rs/crates/minigames/src/league/tests.rs"
  - "portal-rs/crates/payments/src/coin_store.rs"
  - "portal-rs/crates/payments/src/storage.rs"
  - "portal-rs/crates/playerdata/Cargo.toml"
  - "portal-rs/crates/playerdata/src/authority/tests.rs"
  - "portal-rs/crates/playerdata/src/items/tests.rs"
  - "portal-rs/crates/web/Cargo.toml"
  - "portal-rs/crates/web/src/session.rs"
  - "portal-rs/crates/web/src/session_tests.rs"
  - "portal-rs/crates/wiki/Cargo.toml"
  - "portal-rs/crates/wiki/src/web/tests.rs"
  - "portal-rs/crates/world/Cargo.toml"
  - "portal-rs/crates/world/src/admin_pool_tests.rs"
  - "portal-rs/crates/world/src/admin_tests.rs"
validation:
  - command: cargo fmt --all --check
    result: PASS
    evidence: /tmp/a3-fmt-check.log
  - command: cargo clippy --workspace --all-targets -- -D warnings
    result: PASS
    evidence: /tmp/a3-clippy-final.log
  - command: required and touched full crate suites
    result: BLOCKED
    evidence: 1082 passing completed full-suite tests; Portal 187 passed and two failed; League second attempt failed
blockers:
  - Owner two-attempt stop rule triggered by repeated League large-page failure
next_action: Owner root-causes both Portal failures locally using the as-is delivered branch and the recorded clean-base reproduction
```

## Source branch closeout

```yaml
source_branch_disposition: retain
source_branch_reason: owner requested branch delivery for coordinator review
source_branch_evidence: owner explicitly requested as-is commit and branch publication despite blocked validation
```

## Review notes for ordering edges

- Bazaar coin settlement commits before the worker opens the character-transfer
  transaction. The gold branch's settlement adapter retains gold evidence rather
  than wallet-account rows. Character transfer publication precedes the later
  completed-delivery transaction, so these branches do not retain a wallet lock
  across their publication-fence wait.
- Loyalty stream head acquisition is a stream-only serialization prefix; no
  premium mutation or snapshot acquires that head after its authority rows.
  Referral preparation then locks the complete account/authority sets in UUID
  order before 5600. Trigger callers use NOWAIT and raise 40001 on earlier-row
  conflicts rather than adding a blocking reverse edge.
- Stored generic control evidence has request-row locks but its configured
  production Authority authorize/approver implementations deny. Its test adapter
  does not constitute a production RBAC taker; it is recorded as a capability
  seam, not falsely labelled as canonical RBAC acquisition.
- SQL definitions include historical replaced bodies. Runtime integrity triggers
  inherit the application transaction and do not independently establish a
  mutation deadline or staff authority. Applied migrations are unchanged.

## Mechanical inventory

Commands run from the repository root. Counts are matching output lines (not
occurrences); overlapping categories are deliberately counted independently.
`rg` uses its normal tracked/ignore-file scope; no path under portal-rs is manually
excluded. The extra staff/privileged/permission search is a superset to catch
writers whose API names do not contain `authorize`. The complete union below
retains tests, definitions, comments, fixtures and documentation as explicit
reference rows instead of silently dropping them.

| Exact grep command | Hit count |
|---|---:|
| `rg --sort path -n --no-heading -i -e pg_advisory portal-rs/` | 143 |
| `rg --sort path -n --no-heading -i -e 'pg_(try_)?advisory' portal-rs/` | 151 |
| `rg --sort path -n --no-heading -i -e 'FOR[[:space:]]+UPDATE' portal-rs/` | 436 |
| `rg --sort path -n --no-heading -i -e 'FOR[[:space:]]+NO[[:space:]]+KEY[[:space:]]+UPDATE' portal-rs/` | 1 |
| `rg --sort path -n --no-heading -i -e 'FOR[[:space:]]+SHARE' portal-rs/` | 102 |
| `rg --sort path -n --no-heading -i -e 'FOR[[:space:]]+KEY[[:space:]]+SHARE' portal-rs/` | 7 |
| `rg --sort path -n --no-heading -i -e 'LOCK[[:space:]]+TABLE' portal-rs/` | 12 |
| `rg --sort path -n --no-heading -i -e Permission:: portal-rs/` | 448 |
| `rg --sort path -n --no-heading -i -e authorize portal-rs/` | 951 |
| `rg --sort path -n --no-heading -i -e '_in\(' portal-rs/` | 401 |
| `rg --sort path -n --no-heading -i -e role_permissions portal-rs/` | 87 |
| `rg --sort path -n --no-heading -i -e account_roles portal-rs/` | 125 |
| `rg --sort path -n --no-heading -i -e 'staff|privileged|permission' portal-rs/` | 3170 |

Union: 5205 distinct file:line hits.

The broader advisory pattern intentionally includes `pg_try_advisory_*` as well
as `pg_advisory_*`: 151 lines, including eight nonblocking try sites absent from
the narrower spelling. These include the League session lease and analytics
reader probes; they do not wait on another transaction.

The multiline cross-check returned 704 output lines and zero additional source
lines outside the union:

```sh
rg --sort path -n --no-heading -U -i -e 'pg_(try_)?advisory|FOR\s+(?:NO\s+KEY\s+UPDATE|UPDATE|KEY\s+SHARE|SHARE)|LOCK\s+TABLE' portal-rs/
```

### Interpretation and review coverage

The primitive-site rows describe that exact lock acquisition. Authorization
symbols, strings and declarations are reference occurrences, not independent
lock acquisitions; their `n.a.` values do not grant authority to a writer. The
writer-entry table below separately assesses the owning mutation, including
implicit SQL writes, retained adapter locks and live checks. Canonical helper
calls are expanded to their account/resource/RBAC/session protocol.

Budget `yes` on a caller-owned helper means the owning mutation establishes it
before entering the helper, not that the helper restarts a deadline. Budget `no`
on an independent server-domain operation is compatible with the README's
requirement: the shared deadline is mandatory for transactions joining RBAC;
independent worker/read/request domains retain their own existing bounds and do
not join RBAC. Deliberate test blockers, observer queries and applied DDL are
`n.a.`. Authentication and ownership checks in independent player/server domains
are not staff permission reads; staff authority is recorded at its owning entry.

All production primitive families were reviewed together with their takers:
account/RBAC/session; Bazaar gate/listing/issuer/wallet; character issuance,
restore, service and player-data ownership; payment intent/coin/worker/refund;
identity/session/MFA/OAuth/native assignment/signing; referrals/premium/loyalty;
content/wiki/world publication; support/sanctions/enforcement; statistics and
projection feeds; minigames and wallet ledgers. Independent feed/issuer locks
serialize their own rows. SKIP LOCKED/NOWAIT paths refuse or omit conflicting
rows instead of adding a waiting reverse edge. Historical/replaced SQL bodies
remain listed; current application entry acquisition makes trigger reacquisition
reentrant. No applied migration was changed.

### Owning privileged writer entries

These rows include writes reached through permission-bearing adapters and
callbacks even when the entry itself contains no literal SQL lock. `3→4→5→6→7`
means the corresponding retained account/resource/RBAC/session/effect levels,
with unused levels omitted, as defined by the README. Admission-only reads and
fresh audit appends do not authorize later domain effects.

| Site | Locks and mode | Level | Budget before first lock | Authorization after last wait | Compliant |
|---|---|---|---|---|---|
| `portal-rs/crates/admin/src/rbac/storage.rs:32` `change` | ordered accounts UPDATE; prepared resources UPDATE; RBAC exclusive; durable proof UPDATE; effects | 3→4→5→6→7 | yes | Same-transaction authority rechecked after final write before commit (MFA approval also request expiry) | yes |
| `portal-rs/crates/admin/src/rbac/storage.rs:43` `change_from_panel` | ordered accounts UPDATE; prepared resources UPDATE; RBAC exclusive; durable proof UPDATE; effects | 3→4→5→6→7 | yes | Same-transaction authority rechecked after final write before commit (MFA approval also request expiry) | yes |
| `portal-rs/crates/admin/src/rbac/storage.rs:52` `change_impl` | ordered accounts UPDATE; prepared resources UPDATE; RBAC exclusive; durable proof UPDATE; effects | 3→4→5→6→7 | yes | Same-transaction authority rechecked after final write before commit (MFA approval also request expiry) | yes |
| `portal-rs/crates/admin/src/players/mod.rs:282` `note` | ordered accounts UPDATE; prepared resources UPDATE; RBAC exclusive; durable proof UPDATE; effects | 3→4→5→6→7 | yes | Same-transaction authority rechecked after final write before commit (MFA approval also request expiry) | yes |
| `portal-rs/crates/admin/src/players/mod.rs:317` `logout_all` | ordered accounts UPDATE; prepared resources UPDATE; RBAC exclusive; durable proof UPDATE; effects | 3→4→5→6→7 | yes | Same-transaction authority rechecked after final write before commit (MFA approval also request expiry) | yes |
| `portal-rs/crates/admin/src/players/mod.rs:359` `kick` | ordered accounts UPDATE; prepared resources UPDATE; RBAC exclusive; durable proof UPDATE; effects | 3→4→5→6→7 | yes | Same-transaction authority rechecked after final write before commit (MFA approval also request expiry) | fixed in parallel task drc-a |
| `portal-rs/crates/admin/src/players/mod.rs:426` `release_coin_hold` | ordered accounts UPDATE; prepared resources UPDATE; RBAC exclusive; durable proof UPDATE; effects | 3→4→5→6→7 | yes | Same-transaction authority rechecked after final write before commit (MFA approval also request expiry) | yes |
| `portal-rs/crates/admin/src/players/mod.rs:506` `attest_reset_evidence_impl` | ordered accounts UPDATE; prepared resources UPDATE; RBAC exclusive; durable proof UPDATE; effects | 3→4→5→6→7 | yes | Same-transaction authority rechecked after final write before commit (MFA approval also request expiry) | yes |
| `portal-rs/crates/admin/src/players/mod.rs:556` `request_reset` | ordered accounts UPDATE; prepared resources UPDATE; RBAC exclusive; durable proof UPDATE; effects | 3→4→5→6→7 | yes | Same-transaction authority rechecked after final write before commit (MFA approval also request expiry) | yes |
| `portal-rs/crates/admin/src/players/mod.rs:608` `approve_reset` | ordered accounts UPDATE; prepared resources UPDATE; RBAC exclusive; durable proof UPDATE; effects | 3→4→5→6→7 | yes | Same-transaction authority rechecked after final write before commit (MFA approval also request expiry) | yes |
| `portal-rs/crates/admin/src/bazaar.rs:79` `execute` | 64002 exclusive; listing/target UPDATE; ordered accounts UPDATE; RBAC/proof/effects | 1→2→3→5→6→7 | yes | Same-transaction authority rechecked after final write before commit (MFA approval also request expiry) | yes |
| `portal-rs/crates/admin/src/support.rs:252` `execute` | ordered accounts UPDATE; prepared resources UPDATE; RBAC exclusive; durable proof UPDATE; effects | 3→4→5→6→7 | yes | yes | yes |
| `portal-rs/crates/admin/src/commerce/mod.rs:177` `submit` | ordered accounts UPDATE; prepared resources UPDATE; RBAC exclusive; durable proof UPDATE; effects | 3→4→5→6→7 | yes | yes | yes |
| `portal-rs/crates/admin/src/commerce/mod.rs:299` `approve` | ordered accounts UPDATE; prepared resources UPDATE; RBAC exclusive; durable proof UPDATE; effects | 3→4→5→6→7 | yes | yes | yes |
| `portal-rs/crates/admin/src/console/stored.rs:126` `submit_query` | canonical account/RBAC/proof; ownership advisory shared; retained proof SHARE; enqueue/audit | 3→5→6→7 | yes | Same-transaction authority rechecked after final write before commit (MFA approval also request expiry) | yes |
| `portal-rs/crates/admin/src/control/integration.rs:34` `step_up_for_request` | request row UPDATE and evidence inserts through Authority adapter; configured production authorize/approver deny; test adapters exercise storage only | 4→7 | no | n.a. | yes |
| `portal-rs/crates/admin/src/control/integration.rs:95` `record_request_preview` | request row UPDATE and evidence inserts through Authority adapter; configured production authorize/approver deny; test adapters exercise storage only | 4→7 | no | n.a. | yes |
| `portal-rs/crates/admin/src/control/integration.rs:154` `approve_request` | request row UPDATE and evidence inserts through Authority adapter; configured production authorize/approver deny; test adapters exercise storage only | 4→7 | no | n.a. | yes |
| `portal-rs/crates/content/src/pages/staff.rs:234` `persist` | ordered accounts UPDATE; prepared resources UPDATE; RBAC exclusive; durable proof UPDATE; effects | 3→4→5→6→7 | yes | yes | yes |
| `portal-rs/crates/content/src/news.rs:424` `save` | ordered accounts UPDATE; prepared resources UPDATE; RBAC exclusive; durable proof UPDATE; effects | 3→4→5→6→7 | yes | yes | yes |
| `portal-rs/crates/content/src/news.rs:513` `delete` | ordered accounts UPDATE; prepared resources UPDATE; RBAC exclusive; durable proof UPDATE; effects | 3→4→5→6→7 | yes | yes | yes |
| `portal-rs/crates/content/src/media.rs:304` `upload` | ordered accounts UPDATE; prepared resources UPDATE; RBAC exclusive; durable proof UPDATE; effects | 3→4→5→6→7 | yes | yes | yes |
| `portal-rs/crates/content/src/media.rs:445` `delete` | ordered accounts UPDATE; prepared resources UPDATE; RBAC exclusive; durable proof UPDATE; effects | 3→4→5→6→7 | yes | yes | yes |
| `portal-rs/crates/content/src/events/repository.rs:81` `save` | ordered accounts UPDATE; prepared resources UPDATE; RBAC exclusive; durable proof UPDATE; effects | 3→4→5→6→7 | yes | yes | yes |
| `portal-rs/crates/content/src/events/repository.rs:160` `transition` | ordered accounts UPDATE; prepared resources UPDATE; RBAC exclusive; durable proof UPDATE; effects | 3→4→5→6→7 | yes | yes | yes |
| `portal-rs/crates/content/src/announcements.rs:114` `save` | ordered accounts UPDATE; prepared resources UPDATE; RBAC exclusive; durable proof UPDATE; effects | 3→4→5→6→7 | yes | yes | yes |
| `portal-rs/crates/community/src/party/party_store.rs:147` `moderate` | ordered accounts UPDATE; prepared resources UPDATE; RBAC exclusive; durable proof UPDATE; effects | 3→4→5→6→7 | yes | yes | yes |
| `portal-rs/crates/wiki/src/repository.rs:124` `create` | ordered accounts UPDATE; prepared resources UPDATE; RBAC exclusive; durable proof UPDATE; effects | 3→4→5→6→7 | yes | yes | yes |
| `portal-rs/crates/wiki/src/repository.rs:180` `edit` | ordered accounts UPDATE; prepared resources UPDATE; RBAC exclusive; durable proof UPDATE; effects | 3→4→5→6→7 | yes | yes | yes |
| `portal-rs/crates/wiki/src/repository.rs:242` `restore` | ordered accounts UPDATE; prepared resources UPDATE; RBAC exclusive; durable proof UPDATE; effects | 3→4→5→6→7 | yes | yes | yes |
| `portal-rs/crates/wiki/src/repository.rs:298` `transition` | ordered accounts UPDATE; prepared resources UPDATE; RBAC exclusive; durable proof UPDATE; effects | 3→4→5→6→7 | yes | yes | yes |
| `portal-rs/crates/world/src/admin.rs:301` `mutate` | ordered accounts UPDATE; prepared resources UPDATE; RBAC exclusive; durable proof UPDATE; effects | 3→4→5→6→7 | yes | yes | yes |
| `portal-rs/crates/support/src/tickets/mod.rs:220` `change_in` | ordered accounts UPDATE; prepared resources UPDATE; RBAC exclusive; durable proof UPDATE; effects | 3→4→5→6→7 | yes | yes | yes |
| `portal-rs/crates/support/src/tickets/staff.rs:114` `prepare` | ordered accounts UPDATE; prepared resources UPDATE; RBAC exclusive; durable proof UPDATE; effects | 3→4→5→6→7 | yes | yes | yes |
| `portal-rs/crates/support/src/tickets/staff.rs:314` `apply` | ordered accounts UPDATE; prepared resources UPDATE; RBAC exclusive; durable proof UPDATE; effects | 3→4→5→6→7 | yes | yes | yes |
| `portal-rs/crates/support/src/sanctions/store.rs:114` `create` | ordered accounts UPDATE; prepared resources UPDATE; RBAC exclusive; durable proof UPDATE; effects | 3→4→5→6→7 | yes | yes | yes |
| `portal-rs/crates/support/src/sanctions/store.rs:187` `transition` | ordered accounts UPDATE; prepared resources UPDATE; RBAC exclusive; durable proof UPDATE; effects | 3→4→5→6→7 | yes | yes | yes |
| `portal-rs/crates/support/src/sanctions/store.rs:510` `staff_apply` | ordered accounts UPDATE; prepared resources UPDATE; RBAC exclusive; durable proof UPDATE; effects | 3→4→5→6→7 | yes | yes | yes |
| `portal-rs/crates/store/src/catalog.rs:228` `import` | account and retained RBAC via CatalogAccess; activation pointer UPDATE for activate; private import/effects | 3→5→7 | yes | yes | yes |
| `portal-rs/crates/store/src/catalog.rs:267` `activate` | account and retained RBAC via CatalogAccess; activation pointer UPDATE for activate; private import/effects | 3→5→7 | yes | yes | yes |
| `portal-rs/crates/store/src/codes.rs:151` `generate_staff` | CodeAuthority MutationAuthorization; retained account/RBAC/proof; private batch/code/audit inserts | 3→5→6→7 | yes | yes | yes |
| `portal-rs/crates/store/src/codes.rs:172` `generate` | private AuthorizedStaff capability (constructed only by issuance boundary); private inserts | 7 | n.a. | n.a. | yes |
| `portal-rs/crates/store/src/codes.rs:239` `redeem_player` | code/batch authority or redemption request/code/batch/account protocol | 2→3→7 | no | n.a. | fixed in parallel task drc-a |
| `portal-rs/crates/store/src/codes.rs:263` `redeem` | code/batch authority or redemption request/code/batch/account protocol | 2→3→7 | no | n.a. | fixed in parallel task drc-a |
| `portal-rs/crates/store/src/codes.rs:276` `redeem_in` | code/batch authority or redemption request/code/batch/account protocol | 2→3→7 | no | n.a. | fixed in parallel task drc-a |
| `portal-rs/crates/store/src/premium/repository.rs:50` `mutate` | account/authority locks; request hash 27 exclusive; private ledger/effects; staff callers revalidate after return | 3→4→7 | no | n.a. | yes |
| `portal-rs/crates/store/src/premium/repository.rs:135` `mutate_in` | account/authority locks; request hash 27 exclusive; private ledger/effects; staff callers revalidate after return | 3→4→7 | yes | yes | yes |
| `portal-rs/crates/store/src/premium/repository.rs:151` `grant_referral_in` | account/authority locks; request hash 27 exclusive; private ledger/effects; staff callers revalidate after return | 3→4→7 | no | n.a. | yes |
| `portal-rs/crates/minigames/src/pass/repository.rs:31` `publish` | server publication capability adapter; private season/challenge/reward inserts; production adapter denies | 7 | n.a. | n.a. | yes |
| `portal-rs/crates/characters/src/bootstrap/issue.rs:131` `issue` | ordered accounts SHARE; RBAC shared; bootstrap authority UPDATE; permission read after authority | 3→5→7 | yes | yes | yes |
| `portal-rs/crates/downloads/src/storage.rs:62` `publish` | operator account/permission adapter and publication/audit resource (see parallel boundary) | 3→5→7 | no | no | fixed in parallel task drc-a |
| `portal-rs/crates/downloads/src/command.rs:45` `execute` | operator account/permission adapter and publication/audit resource (see parallel boundary) | 3→5→7 | no | no | fixed in parallel task drc-a |
| `portal-rs/crates/downloads/src/command.rs:106` `authorize_in` | operator account/permission adapter and publication/audit resource (see parallel boundary) | 3→5→7 | no | no | fixed in parallel task drc-a |
| `portal-rs/bin/oteryn-portal/src/catalog_cli.rs:16` `account_state` | ordered actor account UPDATE and RBAC exclusive via canonical adapter | 3→5 | yes | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/catalog_cli.rs:104` `audit` | operator account/permission adapter and publication/audit resource (see parallel boundary) | 3→5→7 | no | no | fixed in parallel task drc-a |

### Complete grep-hit union

A reference occurrence is retained for coverage accounting; its owning writer is
assessed above. The source excerpt identifies advisory expressions and lock modes
without duplicating the normative hierarchy. Test fixture rows include deliberate
blockers; observer SQL merely inspects locks/waits. Mode abbreviations: X is
exclusive advisory; S is shared advisory. SQL row modes are written out.

| Site | Lock(s), mode / occurrence | Hierarchy level | Budget before first lock | Authorization after last wait | Compliant |
|---|---|---|---|---|---|
| `portal-rs/README.md:16` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/README.md:40` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/account_closure.rs:281` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/audit_l9_tests.rs:144` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/audit_l9_tests.rs:528` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/audit_l9_tests.rs:534` | implicit SQL write locks; sqlx::query("INSERT INTO account_roles(account_id,role_id) SELECT $1,id FROM roles WHERE key IN ('support','moderator')").bind(staff).execut | 7 (effects; owning entry above) | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/audit_l9_tests.rs:536` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/audit_l9_tests.rs:540` | implicit SQL write locks; let generation:uuid::Uuid=sqlx::query_scalar("INSERT INTO account_mfa_totp(account_id,secret,confirmed) VALUES($1,decode('01','hex'),true) R | 7 (effects; owning entry above) | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/audit_l9_tests.rs:541` | implicit SQL write locks; sqlx::query("INSERT INTO account_mfa_tokens(token_hash,account_id,generation,credential_hash,purpose,expires_at) VALUES($1,$2,$3,$4,'grant', | 7 (effects; owning entry above) | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/audit_l9_tests.rs:542` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/audit_l9_tests.rs:550` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/audit_l9_tests.rs:561` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/bootstrap_intent.rs:11` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/bootstrap_intent_tests.rs:9` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/bootstrap_intent_tests.rs:65` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/bootstrap_intent_tests.rs:75` | implicit SQL write locks; sqlx::query("INSERT INTO account_roles (account_id, role_id) SELECT $1::uuid, id FROM roles WHERE key='game_master_senior'") | 7 (effects; owning entry above) | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/bootstrap_intent_tests.rs:76` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/bootstrap_intent_tests.rs:115` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/bootstrap_intent_tests.rs:116` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/bootstrap_intent_tests.rs:124` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/bootstrap_intent_tests.rs:125` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/bootstrap_intent_tests.rs:127` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/bootstrap_intent_tests.rs:137` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/bootstrap_intent_tests.rs:139` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/bootstrap_intent_tests.rs:154` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/bootstrap_intent_tests.rs:158` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/bootstrap_intent_tests.rs:250` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/bootstrap_intent_tests.rs:257` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/bootstrap_intent_tests.rs:272` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/bootstrap_intent_tests.rs:279` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/bootstrap_intent_tests.rs:293` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/bootstrap_intent_tests.rs:307` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/bootstrap_intent_tests.rs:336` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/bootstrap_intent_tests.rs:365` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/bootstrap_intent_tests.rs:366` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/bootstrap_intent_tests.rs:367` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/bootstrap_intent_tests.rs:384` | implicit SQL write locks; sqlx::query("INSERT INTO web_sessions(id_hash,account_id,idle_expires_at,absolute_expires_at,user_agent_summary,csrf_token,data,authorizatio | 7 (effects; owning entry above) | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/bootstrap_intent_tests.rs:403` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/bootstrap_intent_tests.rs:405` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/bootstrap_intent_tests.rs:426` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/bootstrap_intent_tests.rs:428` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/bootstrap_intent_tests.rs:438` | implicit SQL write locks; sqlx::query("INSERT INTO account_roles(account_id,role_id) SELECT $1,id FROM roles WHERE key='platform_admin'") | 7 (effects; owning entry above) | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/bootstrap_intent_tests.rs:452` | test fixture/blocker: FOR UPDATE; query tables character_bootstrap_intent_authority; sqlx::query("SELECT id FROM character_bootstrap_intent_authority WHERE id=1 FOR UPDATE") | 7 / domain namespace | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/bootstrap_intent_tests.rs:460` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/bootstrap_intent_tests.rs:474` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/bootstrap_intent_tests.rs:477` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/bootstrap_intent_tests.rs:481` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/bootstrap_intent_tests.rs:485` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/bootstrap_intent_tests.rs:487` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/bootstrap_intent_tests.rs:496` | observer SQL; no lock acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/bootstrap_intent_tests.rs:507` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/bootstrap_intent_tests.rs:514` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/bootstrap_intent_tests.rs:523` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/catalog_cli.rs:2` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/catalog_cli.rs:21` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/catalog_cli.rs:25` | FOR SHARE; query tables accounts; sqlx::query_scalar("SELECT status FROM accounts WHERE id=$1 FOR SHARE") | 3 | no | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/catalog_cli.rs:41` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/catalog_cli.rs:52` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/catalog_cli.rs:53` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/catalog_cli.rs:54` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/catalog_cli.rs:56` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/catalog_cli.rs:58` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/catalog_cli.rs:59` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/catalog_cli.rs:117` | pg_advisory_xact_lock; sqlx::query("SELECT pg_advisory_xact_lock(281510004)") | domain namespace (README inventory) | no | n.a. | fixed in parallel task drc-a |
| `portal-rs/bin/oteryn-portal/src/catalog_cli_tests.rs:46` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/catalog_cli_tests.rs:47` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/catalog_cli_tests.rs:48` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/catalog_cli_tests.rs:50` | implicit SQL write locks; sqlx::query("INSERT INTO role_permissions (role_id,permission) VALUES ($1,$2)") | 7 (effects; owning entry above) | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/catalog_cli_tests.rs:52` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/catalog_cli_tests.rs:56` | implicit SQL write locks; sqlx::query("INSERT INTO account_roles (account_id,role_id) VALUES ($1,$2)") | 7 (effects; owning entry above) | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/catalog_cli_tests.rs:187` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/catalog_cli_tests.rs:197` | implicit SQL write locks; sqlx::query("INSERT INTO account_roles (account_id,role_id) SELECT $1,role_id FROM account_roles WHERE account_id=$2").bind(closed).bind(db. | 7 (effects; owning entry above) | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/catalog_cli_tests.rs:265` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/catalog_cli_tests.rs:282` | test fixture/blocker: pg_try_advisory_xact_lock; let free: bool = sqlx::query_scalar("SELECT pg_try_advisory_xact_lock(71010)") | 5 | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/catalog_cli_tests.rs:289` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/catalog_cli_tests.rs:294` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/catalog_cli_tests.rs:299` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/catalog_cli_tests.rs:316` | implicit SQL write locks; sqlx::query("INSERT INTO web_sessions(id_hash,account_id,idle_expires_at,absolute_expires_at,user_agent_summary,csrf_token,data,authorizatio | 7 (effects; owning entry above) | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/catalog_cli_tests.rs:335` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/catalog_cli_tests.rs:337` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/catalog_cli_tests.rs:358` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/catalog_cli_tests.rs:360` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/catalog_cli_tests.rs:370` | implicit SQL write locks; sqlx::query("INSERT INTO account_roles(account_id,role_id) SELECT $1,id FROM roles WHERE key='platform_admin'") | 7 (effects; owning entry above) | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/catalog_cli_tests.rs:375` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/catalog_cli_tests.rs:380` | test fixture/blocker: FOR UPDATE; query tables store_active_catalog; sqlx::query("SELECT id FROM store_active_catalog WHERE id=1 FOR UPDATE") | 7 / domain namespace | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/catalog_cli_tests.rs:403` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/catalog_cli_tests.rs:410` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/catalog_cli_tests.rs:412` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/catalog_cli_tests.rs:420` | observer SQL; no lock acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/catalog_cli_tests.rs:435` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/catalog_cli_tests.rs:440` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/code_routes.rs:31` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/code_routes.rs:32` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/code_routes.rs:106` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/code_routes.rs:109` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/code_routes.rs:110` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/code_routes.rs:112` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/code_routes.rs:125` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/code_routes.rs:145` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/code_routes.rs:175` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/code_routes.rs:176` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/code_routes.rs:181` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/code_routes.rs:184` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/code_routes.rs:223` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/code_routes.rs:243` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/code_routes_tests.rs:15` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/code_routes_tests.rs:16` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/code_routes_tests.rs:17` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/code_routes_tests.rs:52` | implicit SQL write locks; sqlx::query("INSERT INTO account_roles(account_id,role_id) SELECT $1,id FROM roles WHERE key='platform_admin'").bind(actor).execute(&pool).a | 7 (effects; owning entry above) | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/code_routes_tests.rs:54` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/code_routes_tests.rs:100` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/code_routes_tests.rs:111` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/code_routes_tests.rs:188` | implicit SQL write locks; sqlx::query("DELETE FROM role_permissions WHERE permission='store.codes.generate'") | 7 (effects; owning entry above) | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/coin_ceiling_tests.rs:52` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/coin_ceiling_tests.rs:90` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/coin_ceiling_tests.rs:96` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/coin_ceiling_tests.rs:120` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/coin_purchases.rs:92` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/coin_purchases.rs:135` | FOR SHARE; query tables accounts; sqlx::query("SELECT id FROM accounts WHERE id=$1 FOR SHARE") | 3 | no | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/coin_purchases.rs:140` | FOR SHARE OF a; query tables accounts; "SELECT account_value_restriction(a.id) FROM accounts a WHERE a.id=$1 FOR SHARE OF a", | 3 | no | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/coin_purchases.rs:164` | FOR UPDATE; query tables wallet_accounts; sqlx::query("SELECT account_id FROM wallet_accounts WHERE account_id=$1 FOR UPDATE") | 7 / domain namespace | no | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/coin_purchases.rs:175` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/coin_purchases.rs:221` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/coin_purchases.rs:248` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/coin_purchases.rs:283` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/coin_purchases.rs:284` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/coin_purchases_tests.rs:67` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/coin_purchases_tests.rs:88` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/coin_purchases_tests.rs:97` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/coin_purchases_tests.rs:180` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/coin_purchases_tests.rs:504` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/coin_purchases_tests.rs:609` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/coin_purchases_tests.rs:621` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/coin_purchases_tests.rs:629` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/coin_purchases_tests.rs:668` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/coin_purchases_tests.rs:720` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/coin_purchases_tests.rs:722` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/coin_purchases_tests.rs:1462` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/coin_purchases_tests.rs:1464` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/coin_purchases_tests.rs:1472` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/coin_purchases_tests.rs:1475` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/coin_purchases_tests.rs:1484` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/coin_purchases_tests.rs:1487` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/coin_purchases_tests.rs:1839` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/coin_purchases_tests.rs:1875` | implicit SQL write locks; "WITH i AS (INSERT INTO support_sanction_identities(sanction_id,account_id,kind) VALUES(gen_random_uuid(),$1,'ban') RETURNING sanction_id) I | 7 (effects; owning entry above) | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/coin_purchases_tests.rs:1886` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/coin_purchases_tests.rs:1953` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/coin_purchases_tests.rs:1978` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/coin_purchases_tests.rs:1979` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/coin_purchases_tests.rs:1983` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/coin_purchases_tests.rs:1987` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/coin_purchases_tests.rs:1989` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/coin_purchases_tests.rs:1995` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/coin_purchases_tests.rs:1997` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/coin_purchases_tests.rs:2025` | implicit SQL write locks; sqlx::query("INSERT INTO account_roles(account_id,role_id) SELECT $1,id FROM roles WHERE key='platform_admin'") | 7 (effects; owning entry above) | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/coin_purchases_tests.rs:2027` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/coin_purchases_tests.rs:2041` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/coin_purchases_tests.rs:2043` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/coin_purchases_tests.rs:2044` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/coin_purchases_tests.rs:2066` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/coin_purchases_tests.rs:2113` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/coin_purchases_tests.rs:2114` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/coin_purchases_tests.rs:2160` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/coin_purchases_tests.rs:2179` | test fixture/blocker: FOR UPDATE; query tables accounts; sqlx::query("SELECT id FROM accounts WHERE id=$1 FOR UPDATE") | 3 | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/coin_purchases_tests.rs:2184` | implicit SQL write locks; sqlx::query("WITH i AS (INSERT INTO support_sanction_identities(sanction_id,account_id,kind) VALUES(gen_random_uuid(),$1,'ban') RETURNING sa | 7 (effects; owning entry above) | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/coin_purchases_tests.rs:2185` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/coin_purchases_tests.rs:2188` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/coin_purchases_tests.rs:2550` | test fixture/blocker: FOR UPDATE; query tables accounts; sqlx::query("SELECT id FROM accounts WHERE id=$1 FOR UPDATE") | 3 | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/coin_purchases_tests.rs:2572` | test fixture/blocker: FOR UPDATE; query tables payment_intents; let intent_lock = sqlx::query("SELECT id FROM payment_intents WHERE id=$1 FOR UPDATE") | 4 / owning domain resource | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/game_login_devices_sanctions_tests.rs:168` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/game_login_recovery_tests.rs:60` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/game_login_recovery_tests.rs:83` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/game_login_recovery_tests.rs:85` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/game_login_recovery_tests.rs:88` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/game_login_recovery_tests.rs:521` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/game_login_recovery_tests.rs:522` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/game_login_tests.rs:96` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/game_login_tests.rs:228` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/game_login_tests.rs:231` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/game_login_tests.rs:364` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/game_login_tests.rs:367` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/game_login_tests.rs:377` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/game_login_tests.rs:378` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/game_login_tests.rs:420` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/game_login_tests.rs:433` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/game_login_tests.rs:456` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/game_login_tests.rs:462` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/game_login_tests.rs:475` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/game_login_tests.rs:502` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/game_login_tests.rs:640` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/game_login_tests.rs:642` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/game_login_tests.rs:645` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/game_login_tests.rs:649` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/game_login_tests.rs:884` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/game_login_tests.rs:1041` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/game_login_tests.rs:1132` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/game_login_tests.rs:1179` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/game_login_tests.rs:1181` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/game_login_tests.rs:1409` | test fixture/blocker: FOR UPDATE; query tables game_channels; sqlx::query("SELECT channel_id FROM game_channels WHERE channel_id='01934f10-7c03-7001-805b-000000000001' FOR UPDATE") | 4 / owning domain resource | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/game_routes.rs:529` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/game_routes_store_tests.rs:13` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/game_routes_store_tests.rs:134` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/game_routes_store_tests.rs:135` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/game_routes_store_tests.rs:362` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/game_routes_store_tests.rs:366` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/game_routes_store_tests.rs:995` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/game_routes_tests.rs:348` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/game_routes_tests.rs:408` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/game_routes_tests.rs:420` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/game_routes_tests.rs:497` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/game_routes_tests.rs:529` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/game_routes_tests.rs:534` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/game_routes_tests.rs:545` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/game_routes_tests.rs:627` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/game_routes_tests.rs:632` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/game_routes_tests.rs:703` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/game_routes_tests.rs:753` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/game_routes_tests.rs:805` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/game_routes_tests.rs:815` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/game_routes_tests.rs:965` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/internal_tls_tests.rs:304` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/large_pages_tests.rs:11` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/large_pages_tests.rs:64` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/large_pages_tests.rs:65` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/large_pages_tests.rs:66` | implicit SQL write locks; sqlx::query("INSERT INTO account_roles(account_id,role_id) SELECT $1,id FROM roles WHERE key='platform_admin'").bind(staff.0).execute(&self. | 7 (effects; owning entry above) | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/large_pages_tests.rs:67` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/large_pages_tests.rs:71` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/large_pages_tests.rs:72` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/large_pages_tests.rs:73` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/large_pages_tests.rs:74` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/large_pages_tests.rs:80` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/large_pages_tests.rs:82` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/large_pages_tests.rs:88` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/large_pages_tests.rs:90` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/large_pages_tests.rs:101` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/large_pages_tests.rs:131` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/large_pages_tests.rs:144` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/large_pages_tests.rs:160` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/large_pages_tests.rs:169` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/large_pages_tests.rs:172` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/large_pages_tests.rs:173` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/large_pages_tests.rs:197` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/large_pages_tests.rs:224` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/large_pages_tests.rs:225` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/main.rs:250` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/main.rs:594` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/main.rs:595` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/seed.rs:126` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/seed.rs:131` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/seed.rs:270` | FOR UPDATE; query tables accounts; sqlx::query("SELECT id, password_hash FROM accounts WHERE lower(email)=$1 FOR UPDATE") | 3 | no | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/seed.rs:391` | FOR UPDATE; query tables game_channels; let row = sqlx::query("SELECT native_route_host, native_route_port, native_route_tls_server_name, native_route_version, native_route_revisio | 4 / owning domain resource | no | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/seed_tests.rs:420` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/seed_tests.rs:476` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/seed_tests.rs:595` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/seed_tests.rs:598` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/seed_tests.rs:726` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/seed_tests.rs:737` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/seed_tests.rs:784` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/seed_tests.rs:809` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/seed_tests.rs:812` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/wallet_topup.rs:96` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/wallet_topup.rs:98` | FOR SHARE; query tables accounts; sqlx::query("SELECT id FROM accounts WHERE id=$1 FOR SHARE") | 3 | yes | n.a. | fixed in this commit |
| `portal-rs/bin/oteryn-portal/src/wallet_topup.rs:106` | FOR UPDATE; query tables payment_intents; "SELECT account_id,amount,currency,state FROM payment_intents WHERE id=$1 FOR UPDATE", | 4 / owning domain resource | yes | n.a. | fixed in this commit |
| `portal-rs/bin/oteryn-portal/src/wallet_topup.rs:113` | FOR SHARE; query tables store_web_topups; let topup = sqlx::query("SELECT account_id,coins,minor_units,currency FROM store_web_topups WHERE payment_intent_id=$1 FOR SHARE") | 4 / owning domain resource | yes | n.a. | fixed in this commit |
| `portal-rs/bin/oteryn-portal/src/wallet_topup.rs:144` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/wallet_topup.rs:164` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/wallet_topup.rs:171` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/wallet_topup.rs:172` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/worker_binary_test.py:54` | test fixture/blocker: FOR UPDATE; query tables payment_intents; lock.stdin.write(f"SET search_path={schema}; BEGIN; SELECT id FROM payment_intents WHERE id='{intent}' FOR UPDATE;\n") | 4 / owning domain resource | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/worker_binary_test.py:62` | observer SQL; no lock acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/worker_tests.rs:344` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/worker_tests.rs:486` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/worker_tests.rs:491` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/worker_tests.rs:707` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/worker_tests.rs:737` | test fixture/blocker: FOR UPDATE; query tables payment_intents; query("SELECT id FROM payment_intents WHERE id=$1 FOR UPDATE") | 4 / owning domain resource | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/worker_tests.rs:1256` | test fixture/blocker: LOCK TABLE outbox IN ACCESS EXCLUSIVE MODE; query("LOCK TABLE outbox IN ACCESS EXCLUSIVE MODE") | 7 / domain namespace | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/worker_tests.rs:1299` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/worker_tests.rs:1305` | test fixture/blocker: FOR SHARE; query tables accounts; query_scalar("SELECT status='active' FROM accounts WHERE id=$1 FOR SHARE") | 3 | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/worker_tests.rs:1339` | test fixture/blocker: FOR UPDATE; query tables accounts; query("SELECT id FROM accounts WHERE id=$1 FOR UPDATE") | 3 | n.a. | n.a. | yes |
| `portal-rs/bin/oteryn-portal/src/worker_tests.rs:1397` | test fixture/blocker: FOR UPDATE; query tables payment_intents; let intent_lock = query("SELECT id FROM payment_intents WHERE id=$1 FOR UPDATE") | 4 / owning domain resource | n.a. | n.a. | yes |
| `portal-rs/crates/admin/README.md:3` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/analytics/README.md:22` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/analytics/README.md:26` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/analytics/README.md:107` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/analytics/README.md:123` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/analytics/economy.rs:37` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/analytics/economy.rs:61` | pg_try_advisory_xact_lock; \|\| !sqlx::query_scalar::&lt;_, bool&gt;("SELECT pg_try_advisory_xact_lock(71140001)") | domain namespace (README inventory) | no | n.a. | yes |
| `portal-rs/crates/admin/src/analytics/economy.rs:159` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/analytics/mod.rs:163` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/analytics/purchased.rs:19` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/analytics/store.rs:89` | pg_advisory_xact_lock; sqlx::query("SELECT pg_advisory_xact_lock(71140001)") | domain namespace (README inventory) | no | n.a. | yes |
| `portal-rs/crates/admin/src/analytics/store.rs:92` | FOR UPDATE; query tables admin_analytics.fence; let fence=sqlx::query("SELECT epoch::text,privacy_floor,rate_window,rate_count FROM admin_analytics.fence WHERE authority=$1 FOR UPDATE") | 4 / owning domain resource | no | n.a. | yes |
| `portal-rs/crates/admin/src/analytics/store.rs:138` | FOR UPDATE; query tables admin_analytics.watermarks; let old=sqlx::query("SELECT epoch::text,cut::text,observed::text,digest,conflict FROM admin_analytics.watermarks WHERE authority=$1 AND fami | 4 / owning domain resource | no | n.a. | yes |
| `portal-rs/crates/admin/src/analytics/store.rs:317` | FOR UPDATE; query tables admin_analytics.subjects; let old:Option&lt;(String,String)&gt;=sqlx::query_as("SELECT epoch::text,revision::text FROM admin_analytics.subjects WHERE authority=$1 AND famil | 4 / owning domain resource | no | n.a. | yes |
| `portal-rs/crates/admin/src/analytics/store.rs:413` | pg_advisory_xact_lock; sqlx::query("SELECT pg_advisory_xact_lock(71140001)") | domain namespace (README inventory) | no | n.a. | yes |
| `portal-rs/crates/admin/src/analytics/tests.rs:104` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/analytics/tests.rs:297` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/analytics/tests.rs:298` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/analytics/tests.rs:302` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/analytics/tests.rs:308` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/analytics/tests.rs:314` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/analytics/tests.rs:316` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/analytics/tests.rs:327` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/analytics/tests.rs:338` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/analytics/tests.rs:347` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/analytics/tests.rs:355` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/analytics/tests.rs:358` | implicit SQL write locks; "INSERT INTO account_roles(account_id,role_id) SELECT $1,id FROM roles WHERE key=$2", | 7 (effects; owning entry above) | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/analytics/tests.rs:365` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/analytics/tests.rs:377` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/analytics/tests.rs:378` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/analytics/tests.rs:385` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/analytics/tests.rs:391` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/analytics/tests.rs:469` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/analytics/tests.rs:472` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/analytics/tests.rs:481` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/analytics/tests.rs:485` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/analytics/tests.rs:487` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/analytics/tests.rs:494` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/analytics/tests.rs:500` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/analytics/tests.rs:508` | implicit SQL write locks; "INSERT INTO role_permissions(role_id,permission) VALUES($1,'admin.analytics.view')" | 7 (effects; owning entry above) | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/analytics/tests.rs:515` | implicit SQL write locks; sqlx::query("DELETE FROM role_permissions WHERE permission='admin.analytics.view'") | 7 (effects; owning entry above) | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/analytics/tests.rs:528` | implicit SQL write locks; sqlx::query("INSERT INTO role_permissions(role_id,permission) SELECT id,'admin.analytics.view' FROM roles WHERE key='platform_admin'").execu | 7 (effects; owning entry above) | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/analytics/tests.rs:559` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/analytics/tests.rs:700` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/analytics/tests.rs:701` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/analytics/tests.rs:702` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/analytics/tests.rs:713` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/analytics/tests.rs:729` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/analytics/tests.rs:745` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/analytics/tests.rs:861` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/analytics/tests.rs:863` | implicit SQL write locks; sqlx::query("DELETE FROM role_permissions WHERE permission IN ('account.security.kick','payments.coin_hold.release','store.catalog.import',' | 7 (effects; owning entry above) | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/analytics/tests.rs:887` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/analytics/tests.rs:1194` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/analytics/tests.rs:1478` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/analytics/tests.rs:1507` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/analytics/tests.rs:1518` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/analytics/tests.rs:1524` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/analytics/tests.rs:1581` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/analytics/tests.rs:1583` | test fixture/blocker: FOR UPDATE; query tables admin_analytics.pages; sqlx::query("SELECT * FROM admin_analytics.pages FOR UPDATE") | 4 / owning domain resource | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/analytics/web.rs:7` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/analytics/web.rs:8` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/analytics/web.rs:37` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/analytics/web.rs:93` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/analytics/web.rs:108` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/analytics/web.rs:125` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/analytics/web.rs:192` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/analytics/web.rs:208` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/analytics/web.rs:213` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/analytics/web.rs:215` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/analytics/web.rs:230` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/analytics/web.rs:241` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/analytics/web.rs:263` | pg_try_advisory_xact_lock; \|\| !sqlx::query_scalar::&lt;_, bool&gt;("SELECT pg_try_advisory_xact_lock(71140001)") | domain namespace (README inventory) | no | n.a. | yes |
| `portal-rs/crates/admin/src/audit.rs:1` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/audit_control.rs:1` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/audit_control.rs:49` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/bazaar.rs:1` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/bazaar.rs:4` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/bazaar.rs:5` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/bazaar.rs:15` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/bazaar.rs:36` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/bazaar.rs:38` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/bazaar.rs:41` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/bazaar.rs:44` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/bazaar.rs:50` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/bazaar.rs:56` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/bazaar.rs:58` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/bazaar.rs:62` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/bazaar.rs:67` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/bazaar.rs:73` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/bazaar.rs:75` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/bazaar.rs:79` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/bazaar.rs:82` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/bazaar.rs:88` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/bazaar.rs:93` | canonical account/resource/RBAC/durable-proof acquisition + live authority | 3→4→5→6 | yes | Same-transaction authority rechecked after final write before commit (MFA approval also request expiry) | yes |
| `portal-rs/crates/admin/src/bazaar.rs:98` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/bazaar.rs:101` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/bazaar.rs:103` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/bazaar.rs:104` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/bazaar.rs:143` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/bazaar.rs:146` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/bazaar.rs:162` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/bazaar.rs:170` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/bazaar.rs:211` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/bazaar.rs:221` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/bazaar.rs:225` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/bazaar.rs:230` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/bazaar.rs:233` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/bazaar.rs:241` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/bazaar.rs:255` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/bazaar.rs:271` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/bazaar.rs:335` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/bazaar.rs:418` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/bazaar_tests.rs:14` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/bazaar_tests.rs:20` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/bazaar_tests.rs:22` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/bazaar_tests.rs:30` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/bazaar_tests.rs:31` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/bazaar_tests.rs:32` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/bazaar_tests.rs:50` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/bazaar_tests.rs:58` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/bazaar_tests.rs:81` | implicit SQL write locks; sqlx::query("INSERT INTO account_roles(account_id,role_id) SELECT $1,id FROM roles WHERE key='platform_admin'").bind(actor).execute(&pool).a | 7 (effects; owning entry above) | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/bazaar_tests.rs:89` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/bazaar_tests.rs:90` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/bazaar_tests.rs:95` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/bazaar_tests.rs:110` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/bazaar_tests.rs:129` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/bazaar_tests.rs:130` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/bazaar_tests.rs:157` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/bazaar_tests.rs:183` | implicit SQL write locks; sqlx::query("DELETE FROM account_roles WHERE account_id=$1") | 7 (effects; owning entry above) | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/bazaar_tests.rs:211` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/bazaar_tests.rs:221` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/bazaar_tests.rs:247` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/bazaar_tests.rs:248` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/bazaar_tests.rs:263` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/bazaar_tests.rs:265` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/bazaar_tests.rs:277` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/bazaar_tests.rs:279` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/bazaar_tests.rs:307` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/bazaar_tests.rs:316` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/bazaar_tests.rs:325` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/bazaar_tests.rs:335` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/bazaar_tests.rs:339` | implicit SQL write locks; sqlx::query("INSERT INTO account_roles(account_id,role_id) SELECT $1,id FROM roles WHERE key='platform_admin'").bind(second).execute(&db.poo | 7 (effects; owning entry above) | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/bazaar_tests.rs:340` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/bazaar_tests.rs:412` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/bazaar_tests.rs:473` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/bazaar_tests.rs:477` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/bazaar_tests.rs:485` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/bazaar_tests.rs:505` | test fixture/blocker: FOR UPDATE; query tables bazaar_listings; sqlx::query("SELECT id FROM bazaar_listings WHERE id=$1 FOR UPDATE") | 2 | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/bazaar_tests.rs:512` | test fixture/blocker: FOR UPDATE; query tables bazaar_reports; sqlx::query("SELECT id FROM bazaar_reports WHERE id=$1 FOR UPDATE") | 4 / owning domain resource | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/bazaar_tests.rs:519` | test fixture/blocker: pg_advisory_xact_lock; sqlx::query("SELECT pg_advisory_xact_lock(64001)") | domain namespace (README inventory) | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/bazaar_tests.rs:525` | test fixture/blocker: pg_advisory_xact_lock; sqlx::query("SELECT pg_advisory_xact_lock(64002)") | 1 | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/bazaar_tests.rs:569` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/bazaar_tests.rs:575` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/bazaar_tests.rs:576` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/bazaar_tests.rs:590` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/bazaar_tests.rs:603` | test fixture/blocker: pg_advisory_xact_lock_shared; sqlx::query("SELECT pg_advisory_xact_lock_shared(64002)") | 1 | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/bazaar_tests.rs:611` | test fixture/blocker: FOR UPDATE; query tables bazaar_listings; sqlx::query("SELECT id FROM bazaar_listings WHERE id=$1 FOR UPDATE") | 2 | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/bazaar_tests.rs:618` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/bazaar_tests.rs:660` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/bazaar_tests.rs:663` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/bazaar_tests.rs:666` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/bazaar_tests.rs:675` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/commerce/README.md:1` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/commerce/README.md:3` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/commerce/README.md:5` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/commerce/README.md:7` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/commerce/README.md:14` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/commerce/README.md:15` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/commerce/README.md:18` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/commerce/README.md:23` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/commerce/README.md:24` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/commerce/README.md:45` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/commerce/configured.rs:1` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/commerce/configured.rs:12` | FOR SHARE; query tables accounts; match sqlx::query_scalar::&lt;_, bool&gt;("SELECT status='active' AND email_verified_at IS NOT NULL FROM accounts WHERE id=$1 FOR SHARE") | 3 | no | n.a. | yes |
| `portal-rs/crates/admin/src/commerce/configured.rs:20` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/commerce/configured.rs:22` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/commerce/configured.rs:31` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/commerce/effects.rs:45` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/commerce/effects.rs:79` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/commerce/effects.rs:84` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/commerce/effects.rs:109` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/commerce/effects.rs:134` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/commerce/effects.rs:161` | FOR UPDATE OF r; query tables commerce_operations, commerce_refunds; let row = sqlx::query("SELECT r.intent_id,r.account_id,r.amount,r.currency,r.provider,r.provider_reference FROM commerce_refunds r JOIN comm | 4 / owning domain resource | no | n.a. | yes |
| `portal-rs/crates/admin/src/commerce/effects.rs:174` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/commerce/effects.rs:226` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/commerce/effects.rs:246` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/commerce/effects.rs:318` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/commerce/effects.rs:324` | FOR SHARE; query tables accounts; sqlx::query_scalar("SELECT status='active' FROM accounts WHERE id=$1 FOR SHARE") | 3 | no | n.a. | yes |
| `portal-rs/crates/admin/src/commerce/en.ftl:13` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/commerce/en.ftl:20` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/commerce/en.ftl:22` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/commerce/http.rs:2` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/commerce/http.rs:27` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/commerce/http.rs:46` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/commerce/http.rs:76` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/commerce/http.rs:188` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/commerce/http.rs:200` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/commerce/mod.rs:1` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/commerce/mod.rs:6` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/commerce/mod.rs:46` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/commerce/mod.rs:48` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/commerce/mod.rs:49` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/commerce/mod.rs:50` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/commerce/mod.rs:141` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/commerce/mod.rs:148` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/commerce/mod.rs:154` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/commerce/mod.rs:156` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/commerce/mod.rs:158` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/commerce/mod.rs:169` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/commerce/mod.rs:172` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/commerce/mod.rs:173` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/commerce/mod.rs:194` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/commerce/mod.rs:198` | canonical account/resource/RBAC/durable-proof acquisition + live authority | 3→4→5→6 | yes | yes | yes |
| `portal-rs/crates/admin/src/commerce/mod.rs:203` | FOR UPDATE; query tables commerce_operations; sqlx::query("SELECT id FROM commerce_operations WHERE id=$1 FOR UPDATE") | 4 / owning domain resource | yes | yes | yes |
| `portal-rs/crates/admin/src/commerce/mod.rs:212` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/commerce/mod.rs:243` | FOR UPDATE; query tables commerce_operations; let row = sqlx::query("SELECT actor_id,target_id,command,reason_sha256,result FROM commerce_operations WHERE id=$1 FOR UPDATE") | 4 / owning domain resource | yes | yes | yes |
| `portal-rs/crates/admin/src/commerce/mod.rs:253` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/commerce/mod.rs:263` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/commerce/mod.rs:290` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/commerce/mod.rs:321` | canonical account/resource/RBAC/durable-proof acquisition + live authority | 3→4→5→6 | yes | yes | yes |
| `portal-rs/crates/admin/src/commerce/mod.rs:326` | FOR UPDATE; query tables commerce_operations; sqlx::query("SELECT id FROM commerce_operations WHERE id=$1 FOR UPDATE") | 4 / owning domain resource | yes | yes | yes |
| `portal-rs/crates/admin/src/commerce/mod.rs:335` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/commerce/mod.rs:338` | FOR UPDATE; query tables commerce_operations; let row = sqlx::query("SELECT actor_id,target_id,command,reason_sha256,requires_approval,approved_by,result FROM commerce_operations WHERE i | 4 / owning domain resource | yes | yes | yes |
| `portal-rs/crates/admin/src/commerce/mod.rs:344` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/commerce/mod.rs:361` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/commerce/mod.rs:363` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/commerce/mod.rs:384` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/commerce/tests.rs:17` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/commerce/tests.rs:23` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/commerce/tests.rs:25` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/commerce/tests.rs:29` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/commerce/tests.rs:34` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/commerce/tests.rs:58` | test fixture/blocker: FOR SHARE; query tables accounts; match sqlx::query_scalar::&lt;_, String&gt;("SELECT status FROM accounts WHERE id=$1 FOR SHARE") | 3 | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/commerce/tests.rs:69` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/commerce/tests.rs:70` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/commerce/tests.rs:76` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/commerce/tests.rs:77` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/commerce/tests.rs:101` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/commerce/tests.rs:105` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/commerce/tests.rs:106` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/commerce/tests.rs:109` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/commerce/tests.rs:137` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/commerce/tests.rs:139` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/commerce/tests.rs:182` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/commerce/tests.rs:186` | implicit SQL write locks; sqlx::query("INSERT INTO account_roles(account_id,role_id) SELECT $1,id FROM roles WHERE key='platform_admin'").bind(id).execute(&pool).awai | 7 (effects; owning entry above) | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/commerce/tests.rs:252` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/commerce/tests.rs:363` | implicit SQL write locks; sqlx::query("DELETE FROM account_roles WHERE account_id=$1") | 7 (effects; owning entry above) | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/commerce/tests.rs:450` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/commerce/tests.rs:460` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/commerce/tests.rs:487` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/commerce/tests.rs:496` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/commerce/tests.rs:507` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/commerce/tests.rs:537` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/commerce/tests.rs:573` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/commerce/tests.rs:574` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/commerce/tests.rs:716` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/commerce/tests.rs:720` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/commerce/tests.rs:728` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/commerce/tests.rs:762` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/commerce/tests.rs:771` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/commerce/tests.rs:801` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/commerce/tests.rs:802` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/commerce/tests.rs:854` | test fixture/blocker: pg_advisory_xact_lock; sqlx::query("SELECT pg_advisory_xact_lock(71010)") | 5 | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/commerce/tests.rs:895` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/console/access.en.ftl:20` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/console/access.rs:3` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/console/access.rs:64` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/console/access.rs:67` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/console/access.rs:68` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/console/access.rs:80` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/console/access.rs:145` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/console/access.rs:159` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/console/access.rs:160` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/console/access.rs:177` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/console/access.rs:178` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/console/access.rs:181` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/console/access.rs:211` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/console/access.rs:216` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/console/access.rs:232` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/console/access.rs:337` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/console/access.rs:343` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/console/access.rs:344` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/console/access.rs:348` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/console/access.rs:373` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/console/access.rs:377` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/console/access.rs:394` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/console/access.rs:404` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/console/access.rs:407` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/console/access.rs:410` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/console/access.rs:411` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/console/access.rs:414` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/console/access.rs:431` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/console/access.rs:581` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/console/access.rs:589` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/console/access.rs:629` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/console/dashboard.rs:4` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/console/dashboard_tests.rs:14` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/console/dashboard_tests.rs:15` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/console/dashboard_tests.rs:16` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/console/dashboard_tests.rs:17` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/console/dashboard_tests.rs:23` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/console/dashboard_tests.rs:25` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/console/dashboard_tests.rs:31` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/console/dashboard_tests.rs:33` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/console/dashboard_tests.rs:42` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/console/dashboard_tests.rs:125` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/console/dashboard_tests.rs:131` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/console/dashboard_tests.rs:133` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/console/dashboard_tests.rs:134` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/console/dashboard_tests.rs:140` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/console/dashboard_tests.rs:189` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/console/dashboard_tests.rs:196` | implicit SQL write locks; sqlx::query("INSERT INTO role_permissions(role_id,permission) VALUES($1,'admin.access')") | 7 (effects; owning entry above) | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/console/dashboard_tests.rs:201` | implicit SQL write locks; sqlx::query("INSERT INTO account_roles(account_id,role_id) VALUES($1,$2)") | 7 (effects; owning entry above) | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/console/dashboard_tests.rs:270` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/console/dashboard_tests.rs:277` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/console/dashboard_tests.rs:282` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/console/dashboard_tests.rs:395` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/console/dashboard_tests.rs:400` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/console/dashboard_tests.rs:428` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/console/dashboard_tests.rs:430` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/console/dashboard_tests.rs:454` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/console/dashboard_tests.rs:464` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/console/dashboard_tests.rs:465` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/console/dashboard_tests.rs:467` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/console/dashboard_tests.rs:471` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/console/dashboard_tests.rs:515` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/console/dashboard_tests.rs:519` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/console/dashboard_tests.rs:544` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/console/database.rs:2` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/console/mod.rs:20` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/console/mod.rs:70` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/console/mod.rs:72` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/console/mod.rs:73` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/console/mod.rs:74` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/console/mod.rs:75` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/console/mod.rs:100` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/console/mod.rs:119` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/console/mod.rs:120` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/console/mod.rs:132` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/console/mod.rs:136` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/console/mod.rs:138` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/console/mod.rs:147` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/console/mod.rs:153` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/console/mod.rs:162` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/console/mod.rs:166` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/console/mod.rs:335` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/console/mod.rs:349` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/console/mod.rs:355` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/console/mod.rs:363` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/console/mod.rs:364` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/console/mod.rs:384` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/console/mod.rs:385` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/console/mod.rs:398` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/console/mod.rs:523` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/console/mod.rs:542` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/console/mod.rs:545` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/console/mod.rs:625` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/console/mod.rs:674` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/console/mod.rs:695` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/console/mod.rs:718` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/console/mod.rs:741` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/console/nodes.rs:2` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/console/players_tests.rs:72` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/console/players_tests.rs:96` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/console/players_tests.rs:114` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/console/players_tests.rs:320` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/console/players_tests.rs:324` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/console/players_tests.rs:325` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/console/players_tests.rs:335` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/console/players_tests.rs:341` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/console/players_tests.rs:343` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/console/players_tests.rs:369` | implicit SQL write locks; sqlx::query("INSERT INTO account_roles(account_id,role_id) SELECT $1,id FROM roles WHERE key='platform_admin'").bind(actor.account).execute( | 7 (effects; owning entry above) | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/console/players_tests.rs:375` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/console/players_tests.rs:387` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/console/players_tests.rs:405` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/console/players_tests.rs:410` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/console/players_tests.rs:415` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/console/players_tests.rs:439` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/console/players_tests.rs:469` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/console/players_tests.rs:490` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/console/players_tests.rs:492` | implicit SQL write locks; sqlx::query("INSERT INTO account_roles(account_id,role_id) SELECT $1,id FROM roles WHERE key='game_support'").bind(actor.account).execute(&d | 7 (effects; owning entry above) | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/console/players_tests.rs:537` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/console/players_tests.rs:546` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/console/projection.rs:270` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/console/stored.rs:18` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/console/stored.rs:133` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/console/stored.rs:138` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/console/stored.rs:150` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/console/stored.rs:154` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/console/stored.rs:158` | canonical account/resource/RBAC/durable-proof acquisition + live authority | 3→4→5→6 | yes | Same-transaction authority rechecked after final write before commit (MFA approval also request expiry) | yes |
| `portal-rs/crates/admin/src/console/stored.rs:164` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/console/stored.rs:165` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/console/stored.rs:182` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/console/stored.rs:185` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/console/stored.rs:188` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/console/stored.rs:193` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/console/stored.rs:201` | FOR SHARE OF t,s; query tables account_mfa_tokens, web_sessions; let evidence: Option&lt;(OffsetDateTime, String)&gt; = sqlx::query_as("SELECT t.created_at,COALESCE(s.ip::text,'') FROM account_mfa_tokens t JOIN  | 6 | yes | Same-transaction authority rechecked after final write before commit (MFA approval also request expiry) | yes |
| `portal-rs/crates/admin/src/console/stored.rs:204` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/console/stored.rs:223` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/console/stored.rs:250` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/console/stored.rs:369` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/console/stored.rs:399` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/console/stored.rs:484` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/console/stored_tests.rs:12` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/console/stored_tests.rs:13` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/console/stored_tests.rs:14` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/console/stored_tests.rs:90` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/console/stored_tests.rs:138` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/console/stored_tests.rs:180` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/console/stored_tests.rs:228` | implicit SQL write locks; sqlx::query("INSERT INTO account_roles(account_id,role_id) SELECT $1,id FROM roles WHERE key='platform_admin'").bind(actor).execute(&pool).a | 7 (effects; owning entry above) | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/console/stored_tests.rs:229` | implicit SQL write locks; sqlx::query("INSERT INTO role_permissions(role_id,permission) SELECT id,unnest(ARRAY['game.world.read','game.player.read','game.sanction.rea | 7 (effects; owning entry above) | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/console/stored_tests.rs:273` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/console/stored_tests.rs:277` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/console/stored_tests.rs:283` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/console/stored_tests.rs:307` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/console/stored_tests.rs:308` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/console/stored_tests.rs:313` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/console/stored_tests.rs:314` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/console/stored_tests.rs:382` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/console/worlds.rs:2` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/README.md:71` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/README.md:73` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/README.md:75` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/README.md:78` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/README.md:80` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/README.md:87` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/README.md:90` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/README.md:94` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/README.md:116` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/catalog.rs:3` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/catalog.rs:11` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/catalog.rs:18` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/catalog.rs:28` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/catalog.rs:29` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/catalog.rs:30` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/catalog.rs:31` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/catalog.rs:32` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/catalog.rs:48` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/catalog.rs:91` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/catalog.rs:120` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/catalog.rs:131` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/catalog.rs:134` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/catalog.rs:138` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/catalog.rs:146` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/catalog.rs:157` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/catalog.rs:178` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/catalog.rs:181` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/catalog.rs:182` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/catalog.rs:184` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/catalog.rs:185` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/catalog.rs:192` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/catalog.rs:196` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/catalog_tests.rs:4` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/catalog_tests.rs:5` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/catalog_tests.rs:6` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/catalog_tests.rs:7` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/catalog_tests.rs:8` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/catalog_tests.rs:17` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/catalog_tests.rs:29` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/catalog_tests.rs:76` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/catalog_tests.rs:83` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/catalog_tests.rs:100` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/catalog_tests.rs:104` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/catalog_tests.rs:107` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/catalog_tests.rs:108` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/catalog_tests.rs:109` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/catalog_tests.rs:110` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/catalog_tests.rs:111` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/catalog_tests.rs:112` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/catalog_tests.rs:114` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/catalog_tests.rs:123` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/catalog_tests.rs:124` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/catalog_tests.rs:126` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/catalog_tests.rs:132` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/catalog_tests.rs:134` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/catalog_tests.rs:137` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/catalog_tests.rs:144` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/catalog_tests.rs:153` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/catalog_tests.rs:165` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/catalog_tests.rs:167` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/catalog_tests.rs:183` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/catalog_tests.rs:188` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/catalog_tests.rs:191` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/catalog_tests.rs:210` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/catalog_tests.rs:226` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/controls.rs:64` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/controls.rs:68` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/controls.rs:80` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/controls.rs:81` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/controls.rs:84` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/controls.rs:90` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/controls.rs:153` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/controls.rs:161` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/controls.rs:232` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/controls.rs:277` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/controls.rs:320` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/controls.rs:354` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/controls.rs:361` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/controls.rs:363` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/controls.rs:370` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/controls.rs:381` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/controls.rs:383` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/controls.rs:392` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/controls.rs:410` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/controls.rs:411` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/controls.rs:448` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/controls.rs:450` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/controls.rs:451` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/controls.rs:456` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/controls.rs:468` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/controls_http.rs:19` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/controls_http.rs:37` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/controls_http.rs:67` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/controls_http.rs:154` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/controls_http.rs:204` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/controls_http.rs:206` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/controls_http.rs:284` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/controls_http.rs:291` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/controls_http.rs:302` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/controls_http.rs:309` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/controls_http.rs:313` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/controls_http.rs:319` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/controls_tests.rs:68` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/controls_tests.rs:71` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/controls_tests.rs:77` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/controls_tests.rs:93` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/controls_tests.rs:441` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/controls_tests.rs:442` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/controls_tests.rs:443` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/controls_tests.rs:444` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/controls_tests.rs:450` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/controls_tests.rs:452` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/controls_tests.rs:458` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/controls_tests.rs:460` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/controls_tests.rs:475` | implicit SQL write locks; sqlx::query("INSERT INTO account_roles(account_id,role_id) SELECT $1,id FROM roles WHERE key='platform_admin'").bind(session.account_id().aw | 7 (effects; owning entry above) | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/controls_tests.rs:482` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/controls_tests.rs:483` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/controls_tests.rs:579` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/controls_tests.rs:689` | implicit SQL write locks; sqlx::query("INSERT INTO account_roles(account_id,role_id) SELECT $1,id FROM roles WHERE key='platform_admin'").bind(approver.account_id().a | 7 (effects; owning entry above) | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/controls_tests.rs:696` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/controls_tests.rs:697` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/controls_tests.rs:817` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/controls_tests.rs:839` | implicit SQL write locks; sqlx::query("INSERT INTO admin_control_previews(id,requester,operation,permission,intent_digest,preview_digest,preview,reason) VALUES($1,$2, | 7 (effects; owning entry above) | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/controls_tests.rs:840` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/controls_tests.rs:861` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/controls_tests.rs:862` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/controls_tests.rs:911` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/controls_tests.rs:935` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/fixtures/admin-control-v1/SOURCE.md:34` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/fixtures/admin-control-v1/SOURCE.md:74` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/fixtures/admin-control-v1/request.schema.json:106` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/fixtures/admin-control-v1/request.schema.json:129` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/fixtures/admin-control-v1/result.schema.json:791` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/fixtures/admin-control-v1/result.schema.json:814` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/fixtures/admin-control-v1/vectors/busy.json:2` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/fixtures/admin-control-v1/vectors/busy.json:9` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/fixtures/admin-control-v1/vectors/busy.json:29` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/fixtures/admin-control-v1/vectors/closed-params.json:2` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/fixtures/admin-control-v1/vectors/closed-params.json:9` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/fixtures/admin-control-v1/vectors/closed-params.json:29` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/fixtures/admin-control-v1/vectors/disabled-operation.json:2` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/fixtures/admin-control-v1/vectors/disabled-operation.json:18` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/fixtures/admin-control-v1/vectors/disabled-world.json:2` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/fixtures/admin-control-v1/vectors/disabled-world.json:9` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/fixtures/admin-control-v1/vectors/disabled-world.json:29` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/fixtures/admin-control-v1/vectors/excessive-lifetime.json:2` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/fixtures/admin-control-v1/vectors/excessive-lifetime.json:9` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/fixtures/admin-control-v1/vectors/excessive-lifetime.json:29` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/fixtures/admin-control-v1/vectors/expired-key.json:2` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/fixtures/admin-control-v1/vectors/expired-key.json:9` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/fixtures/admin-control-v1/vectors/expired-key.json:29` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/fixtures/admin-control-v1/vectors/expired.json:2` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/fixtures/admin-control-v1/vectors/expired.json:9` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/fixtures/admin-control-v1/vectors/expired.json:29` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/fixtures/admin-control-v1/vectors/future-issued.json:2` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/fixtures/admin-control-v1/vectors/future-issued.json:9` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/fixtures/admin-control-v1/vectors/future-issued.json:29` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/fixtures/admin-control-v1/vectors/future-step-up.json:2` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/fixtures/admin-control-v1/vectors/future-step-up.json:9` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/fixtures/admin-control-v1/vectors/future-step-up.json:29` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/fixtures/admin-control-v1/vectors/idempotency-conflict.json:2` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/fixtures/admin-control-v1/vectors/idempotency-conflict.json:9` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/fixtures/admin-control-v1/vectors/idempotency-conflict.json:46` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/fixtures/admin-control-v1/vectors/idempotency-recorded.json:2` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/fixtures/admin-control-v1/vectors/idempotency-recorded.json:9` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/fixtures/admin-control-v1/vectors/idempotency-recorded.json:46` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/fixtures/admin-control-v1/vectors/internal.json:2` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/fixtures/admin-control-v1/vectors/internal.json:9` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/fixtures/admin-control-v1/vectors/internal.json:29` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/fixtures/admin-control-v1/vectors/invalid-nonce.json:2` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/fixtures/admin-control-v1/vectors/invalid-nonce.json:9` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/fixtures/admin-control-v1/vectors/invalid-nonce.json:29` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/fixtures/admin-control-v1/vectors/limit.json:2` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/fixtures/admin-control-v1/vectors/limit.json:9` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/fixtures/admin-control-v1/vectors/limit.json:29` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/fixtures/admin-control-v1/vectors/missing-reason.json:2` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/fixtures/admin-control-v1/vectors/missing-reason.json:9` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/fixtures/admin-control-v1/vectors/missing-reason.json:29` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/fixtures/admin-control-v1/vectors/missing-step-up.json:2` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/fixtures/admin-control-v1/vectors/missing-step-up.json:9` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/fixtures/admin-control-v1/vectors/missing-step-up.json:29` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/fixtures/admin-control-v1/vectors/permission.json:2` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/fixtures/admin-control-v1/vectors/permission.json:9` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/fixtures/admin-control-v1/vectors/permission.json:29` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/fixtures/admin-control-v1/vectors/precondition.json:2` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/fixtures/admin-control-v1/vectors/precondition.json:9` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/fixtures/admin-control-v1/vectors/precondition.json:29` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/fixtures/admin-control-v1/vectors/preview-mismatch.json:2` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/fixtures/admin-control-v1/vectors/preview-mismatch.json:9` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/fixtures/admin-control-v1/vectors/preview-mismatch.json:29` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/fixtures/admin-control-v1/vectors/replay.json:2` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/fixtures/admin-control-v1/vectors/replay.json:9` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/fixtures/admin-control-v1/vectors/replay.json:29` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/fixtures/admin-control-v1/vectors/risk-downgrade.json:2` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/fixtures/admin-control-v1/vectors/risk-downgrade.json:9` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/fixtures/admin-control-v1/vectors/risk-downgrade.json:29` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/fixtures/admin-control-v1/vectors/rotated-out-key.json:2` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/fixtures/admin-control-v1/vectors/rotated-out-key.json:9` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/fixtures/admin-control-v1/vectors/rotated-out-key.json:29` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/fixtures/admin-control-v1/vectors/rotation-next-key.json:2` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/fixtures/admin-control-v1/vectors/rotation-next-key.json:9` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/fixtures/admin-control-v1/vectors/rotation-next-key.json:29` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/fixtures/admin-control-v1/vectors/self-approval.json:2` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/fixtures/admin-control-v1/vectors/self-approval.json:9` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/fixtures/admin-control-v1/vectors/self-approval.json:29` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/fixtures/admin-control-v1/vectors/stale-approval.json:2` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/fixtures/admin-control-v1/vectors/stale-approval.json:9` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/fixtures/admin-control-v1/vectors/stale-approval.json:29` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/fixtures/admin-control-v1/vectors/stale-step-up.json:2` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/fixtures/admin-control-v1/vectors/stale-step-up.json:9` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/fixtures/admin-control-v1/vectors/stale-step-up.json:29` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/fixtures/admin-control-v1/vectors/superseded.json:2` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/fixtures/admin-control-v1/vectors/superseded.json:9` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/fixtures/admin-control-v1/vectors/superseded.json:29` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/fixtures/admin-control-v1/vectors/tampered-field.json:2` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/fixtures/admin-control-v1/vectors/tampered-field.json:9` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/fixtures/admin-control-v1/vectors/tampered-field.json:29` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/fixtures/admin-control-v1/vectors/target-not-found.json:2` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/fixtures/admin-control-v1/vectors/target-not-found.json:9` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/fixtures/admin-control-v1/vectors/target-not-found.json:29` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/fixtures/admin-control-v1/vectors/unavailable.json:2` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/fixtures/admin-control-v1/vectors/unavailable.json:9` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/fixtures/admin-control-v1/vectors/unavailable.json:29` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/fixtures/admin-control-v1/vectors/unknown-contract.json:2` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/fixtures/admin-control-v1/vectors/unknown-contract.json:9` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/fixtures/admin-control-v1/vectors/unknown-contract.json:29` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/fixtures/admin-control-v1/vectors/unknown-operation.json:2` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/fixtures/admin-control-v1/vectors/unknown-operation.json:9` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/fixtures/admin-control-v1/vectors/unknown-operation.json:29` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/fixtures/admin-control-v1/vectors/unknown-version.json:2` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/fixtures/admin-control-v1/vectors/unknown-version.json:9` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/fixtures/admin-control-v1/vectors/unknown-version.json:29` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/fixtures/admin-control-v1/vectors/valid.json:2` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/fixtures/admin-control-v1/vectors/valid.json:9` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/fixtures/admin-control-v1/vectors/valid.json:29` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/fixtures/admin-control-v1/vectors/wrong-key.json:2` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/fixtures/admin-control-v1/vectors/wrong-key.json:9` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/fixtures/admin-control-v1/vectors/wrong-key.json:29` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/fixtures/staff-actions-v1/01-ban.golden.json:1` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/fixtures/staff-actions-v1/02-unban.golden.json:1` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/fixtures/staff-actions-v1/03-namelock.golden.json:1` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/fixtures/staff-actions-v1/04-warning.golden.json:1` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/fixtures/staff-actions-v1/05-notation.golden.json:1` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/fixtures/staff-actions-v1/README.md:4` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/fixtures/staff-actions-v1/README.md:19` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/fixtures/staff-actions-v1/README.md:20` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/fixtures/staff-actions-v1/README.md:21` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/fixtures/staff-actions-v1/README.md:25` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/fixtures/staff-actions-v1/README.md:32` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/fixtures/staff-actions-v1/README.md:50` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/fixtures/staff-actions-v1/README.md:57` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/fixtures/staff-actions-v1/README.md:59` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/fixtures/staff-actions-v1/README.md:60` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/fixtures/staff-actions-v1/README.md:61` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/fixtures/staff-actions-v1/README.md:62` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/fixtures/staff-actions-v1/README.md:67` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/fixtures/staff-actions-v1/README.md:84` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/fixtures/staff-actions-v1/README.md:95` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/fixtures/staff-actions-v1/README.md:121` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/fixtures/staff-actions-v1/README.md:134` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/fixtures/staff-actions-v1/README.md:143` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/fixtures/staff-actions-v1/README.md:144` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/fixtures/staff-actions-v1/README.md:149` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/fixtures/staff-actions-v1/README.md:151` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/fixtures/staff-actions-v1/SOURCE.md:1` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/fixtures/staff-actions-v1/SOURCE.md:5` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/fixtures/staff-actions-v1/action.schema.json:1` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/fixtures/staff-actions-v1/result.schema.json:1` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/http.rs:237` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/http.rs:299` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/http.rs:414` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/http.rs:432` | pg_advisory_xact_lock; sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1, 17))") | domain namespace (README inventory) | no | n.a. | yes |
| `portal-rs/crates/admin/src/control/http.rs:645` | pg_advisory_xact_lock_shared; "SELECT pg_advisory_xact_lock_shared(hashtextextended('admin-control-http',0))", | domain namespace (README inventory) | no | n.a. | yes |
| `portal-rs/crates/admin/src/control/http.rs:650` | pg_advisory_xact_lock; sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended('admin-control-http',0))") | domain namespace (README inventory) | no | n.a. | yes |
| `portal-rs/crates/admin/src/control/http_tests.rs:98` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/http_tests.rs:491` | test fixture/blocker: LOCK TABLE admin_control_agents IN ACCESS EXCLUSIVE MODE; sqlx::query("LOCK TABLE admin_control_agents IN ACCESS EXCLUSIVE MODE") | 4 / owning domain resource | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/http_tests.rs:511` | test fixture/blocker: pg_advisory_xact_lock_shared; "SELECT pg_advisory_xact_lock_shared(hashtextextended('admin-control-http'%", | domain namespace (README inventory) | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/http_tests.rs:592` | test fixture/blocker: LOCK TABLE admin_control_results IN ACCESS EXCLUSIVE MODE; sqlx::query("LOCK TABLE admin_control_results IN ACCESS EXCLUSIVE MODE") | 4 / owning domain resource | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/http_tests.rs:605` | test fixture/blocker: pg_advisory_xact_lock; "SELECT pg_advisory_xact_lock(hashtextextended($1, 17))", | domain namespace (README inventory) | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/integration.rs:33` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/integration.rs:44` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/integration.rs:75` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/integration.rs:81` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/integration.rs:82` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/integration.rs:104` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/integration.rs:123` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/integration.rs:140` | implicit SQL write locks; sqlx::query("INSERT INTO admin_control_previews(id,requester,operation,permission,intent_digest,preview_digest,preview,reason) VALUES($1,$2, | 7 (effects; owning entry above) | no | n.a. | yes |
| `portal-rs/crates/admin/src/control/integration.rs:141` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/integration.rs:162` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/integration.rs:164` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/integration.rs:172` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/integration.rs:181` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/integration.rs:200` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/integration.rs:216` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/integration.rs:223` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/integration.rs:250` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/integration.rs:255` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/integration.rs:309` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/integration_tests.rs:71` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/integration_tests.rs:307` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/integration_tests.rs:380` | implicit SQL write locks; sqlx::query("INSERT INTO accounts(id,email,password_hash,status,locale,email_verified_at) VALUES($1,'real-staff@example.test',$2,'active','e | 7 (effects; owning entry above) | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/integration_tests.rs:392` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/mod.rs:3` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/mod.rs:4` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/signer.rs:68` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/signer_tests.rs:28` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/signer_tests.rs:343` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/store.rs:285` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/store.rs:290` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/store.rs:343` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/store.rs:347` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/store.rs:414` | FOR UPDATE; query tables admin_control_requests; sqlx::query_as("SELECT * FROM admin_control_requests WHERE request_id=$1 FOR UPDATE") | 4 / owning domain resource | no | n.a. | yes |
| `portal-rs/crates/admin/src/control/store.rs:515` | FOR SHARE; AND last_seen_at &lt;= clock_timestamp() FOR SHARE", | 4 / owning domain resource | no | n.a. | yes |
| `portal-rs/crates/admin/src/control/store.rs:540` | FOR UPDATE OF r SKIP LOCKED; ORDER BY r.issued_at,r.request_id LIMIT $3 FOR UPDATE OF r SKIP LOCKED | 4 / owning domain resource | no | n.a. | yes |
| `portal-rs/crates/admin/src/control/store.rs:589` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/store.rs:594` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/store.rs:603` | FOR UPDATE; query tables admin_control_requests; sqlx::query_as("SELECT * FROM admin_control_requests WHERE request_id=$1 FOR UPDATE") | 4 / owning domain resource | no | n.a. | yes |
| `portal-rs/crates/admin/src/control/store.rs:677` | FOR UPDATE SKIP LOCKED; ORDER BY expires_at,request_id LIMIT 100 FOR UPDATE SKIP LOCKED", | 4 / owning domain resource | no | n.a. | yes |
| `portal-rs/crates/admin/src/control/store.rs:844` | pg_advisory_xact_lock_shared; "SELECT pg_advisory_xact_lock_shared(hashtextextended('admin-control-ownership',0))", | domain namespace (README inventory) | no | n.a. | yes |
| `portal-rs/crates/admin/src/control/store.rs:849` | pg_advisory_xact_lock; sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended('admin-control-ownership',0))") | domain namespace (README inventory) | no | n.a. | yes |
| `portal-rs/crates/admin/src/control/store.rs:859` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/tests.rs:401` | test fixture/blocker: FOR UPDATE; query tables admin_control_requests; sqlx::query("SELECT request_id FROM admin_control_requests WHERE request_id=$1 FOR UPDATE") | 4 / owning domain resource | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/tests.rs:453` | test fixture/blocker: FOR UPDATE; query tables admin_control_requests; sqlx::query("SELECT request_id FROM admin_control_requests WHERE request_id=$1 FOR UPDATE") | 4 / owning domain resource | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/control/tests.rs:764` | test fixture/blocker: FOR UPDATE; query tables admin_control_agents; sqlx::query("SELECT agent_id FROM admin_control_agents WHERE agent_id='agent-a' FOR UPDATE") | 4 / owning domain resource | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/lib.rs:1` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/lib.rs:22` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/lib.rs:28` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/lib.rs:36` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/lib.rs:49` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/players/README.md:1` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/players/README.md:7` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/players/README.md:8` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/players/README.md:11` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/players/README.md:20` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/players/README.md:21` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/players/README.md:23` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/players/README.md:47` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/players/README.md:57` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/players/README.md:58` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/players/README.md:64` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/players/en.ftl:13` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/players/http.rs:1` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/players/http.rs:3` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/players/http.rs:44` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/players/http.rs:51` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/players/http.rs:57` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/players/http.rs:59` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/players/http.rs:65` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/players/http.rs:66` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/players/http.rs:236` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/players/mod.rs:1` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/players/mod.rs:5` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/players/mod.rs:88` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/players/mod.rs:135` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/players/mod.rs:151` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/players/mod.rs:154` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/players/mod.rs:162` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/players/mod.rs:173` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/players/mod.rs:177` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/players/mod.rs:179` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/players/mod.rs:185` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/players/mod.rs:189` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/players/mod.rs:190` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/players/mod.rs:193` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/players/mod.rs:194` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/players/mod.rs:197` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/players/mod.rs:201` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/players/mod.rs:206` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/players/mod.rs:210` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/players/mod.rs:212` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/players/mod.rs:226` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/players/mod.rs:239` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/players/mod.rs:246` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/players/mod.rs:261` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/players/mod.rs:275` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/players/mod.rs:283` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/players/mod.rs:288` | canonical account/resource/RBAC/durable-proof acquisition + live authority | 3→4→5→6 | yes | Same-transaction authority rechecked after final write before commit (MFA approval also request expiry) | yes |
| `portal-rs/crates/admin/src/players/mod.rs:293` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/players/mod.rs:296` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/players/mod.rs:323` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/players/mod.rs:328` | canonical account/resource/RBAC/durable-proof acquisition + live authority | 3→4→5→6 | yes | Same-transaction authority rechecked after final write before commit (MFA approval also request expiry) | yes |
| `portal-rs/crates/admin/src/players/mod.rs:332` | FOR UPDATE; query tables web_sessions; sqlx::query("SELECT id_hash FROM web_sessions WHERE account_id=$1 ORDER BY id_hash FOR UPDATE") | 4 (prepared target sessions) | yes | Same-transaction authority rechecked after final write before commit (MFA approval also request expiry) | yes |
| `portal-rs/crates/admin/src/players/mod.rs:338` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/players/mod.rs:341` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/players/mod.rs:355` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/players/mod.rs:360` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/players/mod.rs:364` | canonical account/resource/RBAC/durable-proof acquisition + live authority | 3→4→5→6 | yes | Same-transaction authority rechecked after final write before commit (MFA approval also request expiry) | fixed in parallel task drc-a |
| `portal-rs/crates/admin/src/players/mod.rs:369` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/players/mod.rs:372` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/players/mod.rs:374` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/players/mod.rs:395` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/players/mod.rs:401` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/players/mod.rs:410` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/players/mod.rs:412` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/players/mod.rs:421` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/players/mod.rs:433` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/players/mod.rs:442` | canonical account/resource/RBAC/durable-proof acquisition + live authority | 3→4→5→6 | yes | Same-transaction authority rechecked after final write before commit (MFA approval also request expiry) | yes |
| `portal-rs/crates/admin/src/players/mod.rs:447` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/players/mod.rs:450` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/players/mod.rs:463` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/players/mod.rs:482` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/players/mod.rs:487` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/players/mod.rs:488` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/players/mod.rs:512` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/players/mod.rs:522` | canonical account/resource/RBAC/durable-proof acquisition + live authority | 3→4→5→6 | yes | Same-transaction authority rechecked after final write before commit (MFA approval also request expiry) | yes |
| `portal-rs/crates/admin/src/players/mod.rs:527` | FOR UPDATE; query tables tickets; sqlx::query("SELECT id FROM tickets WHERE id=$1 FOR UPDATE") | 4 / owning domain resource | yes | Same-transaction authority rechecked after final write before commit (MFA approval also request expiry) | yes |
| `portal-rs/crates/admin/src/players/mod.rs:536` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/players/mod.rs:539` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/players/mod.rs:563` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/players/mod.rs:572` | canonical account/resource/RBAC/durable-proof acquisition + live authority | 3→4→5→6 | yes | Same-transaction authority rechecked after final write before commit (MFA approval also request expiry) | yes |
| `portal-rs/crates/admin/src/players/mod.rs:577` | FOR UPDATE; query tables support_mfa_reset_evidence; sqlx::query("SELECT id FROM support_mfa_reset_evidence WHERE id=$1 FOR UPDATE") | 4 / owning domain resource | yes | Same-transaction authority rechecked after final write before commit (MFA approval also request expiry) | yes |
| `portal-rs/crates/admin/src/players/mod.rs:586` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/players/mod.rs:589` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/players/mod.rs:594` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/players/mod.rs:614` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/players/mod.rs:627` | canonical account/resource/RBAC/durable-proof acquisition + live authority | 3→4→5→6 | yes | Same-transaction authority rechecked after final write before commit (MFA approval also request expiry) | yes |
| `portal-rs/crates/admin/src/players/mod.rs:631` | FOR UPDATE OF e; query tables player_mfa_reset_requests, support_mfa_reset_evidence; sqlx::query("SELECT e.id FROM support_mfa_reset_evidence e JOIN player_mfa_reset_requests r ON r.evidence=e.id WHERE r.id=$1 FOR UPDATE OF e | 4 / owning domain resource | yes | Same-transaction authority rechecked after final write before commit (MFA approval also request expiry) | yes |
| `portal-rs/crates/admin/src/players/mod.rs:632` | FOR UPDATE; query tables web_sessions; sqlx::query("SELECT id_hash FROM web_sessions WHERE account_id=$1 ORDER BY id_hash FOR UPDATE").bind(id).execute(connection).await?; | 4 (prepared target sessions) | yes | Same-transaction authority rechecked after final write before commit (MFA approval also request expiry) | yes |
| `portal-rs/crates/admin/src/players/mod.rs:637` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/players/mod.rs:640` | FOR UPDATE; query tables player_mfa_reset_requests; let row: Option&lt;(Uuid,Uuid)&gt; = sqlx::query_as("SELECT requested_by,evidence FROM player_mfa_reset_requests WHERE id=$1 AND account_id=$2 AND | 4 / owning domain resource | yes | Same-transaction authority rechecked after final write before commit (MFA approval also request expiry) | yes |
| `portal-rs/crates/admin/src/players/mod.rs:645` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/players/mod.rs:647` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/players/mod.rs:649` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/players/mod.rs:655` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/players/tests.rs:5` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/players/tests.rs:16` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/players/tests.rs:17` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/players/tests.rs:19` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/players/tests.rs:25` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/players/tests.rs:27` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/players/tests.rs:95` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/players/tests.rs:156` | implicit SQL write locks; sqlx::query("INSERT INTO account_roles(account_id,role_id) SELECT $1,id FROM roles WHERE key=$2").bind(id).bind(role).execute(&db.pool).awai | 7 (effects; owning entry above) | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/players/tests.rs:160` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/players/tests.rs:168` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/players/tests.rs:238` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/players/tests.rs:314` | implicit SQL write locks; sqlx::query("DELETE FROM account_roles WHERE account_id=$1") | 7 (effects; owning entry above) | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/players/tests.rs:398` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/players/tests.rs:399` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/players/tests.rs:400` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/players/tests.rs:421` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/players/tests.rs:424` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/players/tests.rs:432` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/players/tests.rs:472` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/players/tests.rs:473` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/players/tests.rs:486` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/players/tests.rs:489` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/players/tests.rs:517` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/players/tests.rs:531` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/players/tests.rs:613` | implicit SQL write locks; "INSERT INTO account_roles(account_id,role_id) SELECT $1,id FROM roles WHERE key=$2", | 7 (effects; owning entry above) | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/players/tests.rs:639` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/players/tests.rs:646` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/players/tests.rs:651` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/players/tests.rs:656` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/players/tests.rs:750` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/players/tests.rs:751` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/players/tests.rs:752` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/players/tests.rs:762` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/players/tests.rs:798` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/players/tests.rs:799` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/players/tests.rs:810` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/players/tests.rs:827` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/players/tests.rs:830` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/players/tests.rs:834` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/players/tests.rs:862` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/players/tests.rs:927` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/players/tests.rs:934` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/players/tests.rs:953` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/players/tests.rs:1001` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/players/tests.rs:1132` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/players/tests.rs:1156` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/players/tests.rs:1157` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/players/tests.rs:1173` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/players/tests.rs:1190` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/players/tests.rs:1193` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/players/tests.rs:1201` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/players/tests.rs:1244` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/players/tests.rs:1307` | test fixture/blocker: pg_advisory_xact_lock; sqlx::query("SELECT pg_advisory_xact_lock(71010)") | 5 | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/players/tests.rs:1386` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/players/tests.rs:1423` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/players/tests.rs:1432` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/players/tests.rs:1433` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/players/tests.rs:1440` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/players/tests.rs:1447` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/players/tests.rs:1463` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/players/tests.rs:1464` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/players/tests.rs:1466` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/players/tests.rs:1485` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/players/tests.rs:1547` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/players/tests.rs:1559` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/players/tests.rs:1646` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/players/tests.rs:1663` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/players/tests.rs:1701` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/players/tests.rs:1748` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/players/tests.rs:1756` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/players/tests.rs:1796` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/players/tests.rs:1802` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/README.md:18` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/README.md:19` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/README.md:20` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/README.md:21` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/README.md:23` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/README.md:24` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/README.md:44` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/README.md:46` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/README.md:49` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/README.md:55` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/README.md:62` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/README.md:63` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/README.md:64` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/README.md:100` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:1` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:5` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:7` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:65` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:72` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:81` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:82` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:83` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:84` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:86` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:87` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:151` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:224` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:240` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:252` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:311` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:314` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:315` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:316` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:317` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:318` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:319` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:320` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:321` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:322` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:323` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:324` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:325` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:326` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:327` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:328` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:329` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:330` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:331` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:332` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:333` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:334` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:335` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:336` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:337` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:338` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:339` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:340` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:341` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:342` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:343` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:344` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:345` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:346` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:347` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:348` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:349` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:350` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:353` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:354` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:355` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:358` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:359` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:360` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:361` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:362` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:363` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:364` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:365` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:366` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:367` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:368` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:369` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:370` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:373` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:374` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:375` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:378` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:379` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:380` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:381` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:384` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:385` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:386` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:387` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:388` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:389` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:390` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:391` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:392` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:393` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:394` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:397` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:398` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:399` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:400` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:401` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:402` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:403` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:404` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:405` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:408` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:409` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:410` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:411` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:412` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:413` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:414` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:415` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:416` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:417` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:418` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:419` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:422` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:423` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:424` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:425` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:426` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:427` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:428` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:429` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:430` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:431` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:432` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:433` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:434` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:435` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:436` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:437` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:438` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:441` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:442` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:443` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:444` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:445` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:446` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:447` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:448` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:449` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:450` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:451` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:452` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:453` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:454` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:457` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:458` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:459` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:460` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:461` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:462` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:463` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:464` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:465` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:466` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:467` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:468` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:469` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:470` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:471` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:472` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:473` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:474` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:477` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:478` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:479` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:480` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:483` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:484` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:485` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:486` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:489` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:490` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:491` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:492` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:493` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:494` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:495` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/catalog.rs:497` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/guard.rs:4` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/guard.rs:11` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/guard.rs:12` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/guard.rs:13` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/guard.rs:32` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/guard.rs:33` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/guard.rs:42` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/guard.rs:43` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/guard.rs:44` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/guard.rs:45` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/mod.rs:4` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/mod.rs:11` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/mod.rs:22` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/mod.rs:67` | FOR UPDATE; query tables accounts; sqlx::query("SELECT id FROM accounts WHERE id=ANY($1) ORDER BY id FOR UPDATE") | 3 | yes | yes | yes |
| `portal-rs/crates/admin/src/rbac/mod.rs:72` | pg_advisory_xact_lock; sqlx::query("SELECT pg_advisory_xact_lock(71010)") | 5 | yes | yes | yes |
| `portal-rs/crates/admin/src/rbac/mod.rs:132` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/mod.rs:139` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/mod.rs:147` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/mod.rs:158` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/mod.rs:173` | FOR UPDATE OF s,m,t; query tables account_mfa_tokens, account_mfa_totp, web_sessions; sqlx::query("SELECT s.id_hash FROM web_sessions s JOIN account_mfa_totp m ON m.account_id=s.account_id JOIN account_mfa_tokens t ON t.accoun | 6 | yes | yes | yes |
| `portal-rs/crates/admin/src/rbac/mod.rs:176` | FOR UPDATE OF s,m,t; query tables account_mfa_tokens, account_mfa_totp, accounts, web_sessions; sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM web_sessions s JOIN accounts a ON a.id=s.account_id JOIN account_mfa_totp m ON m.account_id= | 6 | yes | yes | yes |
| `portal-rs/crates/admin/src/rbac/mod.rs:181` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/mod.rs:185` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/mod.rs:214` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/mod.rs:233` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/mod.rs:235` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/mod.rs:242` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/mod.rs:245` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/mod.rs:248` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/mod.rs:253` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/mod.rs:258` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/mod.rs:265` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/mod.rs:276` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/mod.rs:277` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/mod.rs:278` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/mod.rs:280` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/mod.rs:286` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/mod.rs:292` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/mod.rs:299` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/mod.rs:306` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/mod.rs:310` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/mod.rs:312` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/mod.rs:314` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/mod.rs:321` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/mod.rs:325` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/mod.rs:331` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/mod.rs:336` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/mod.rs:340` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/mod.rs:342` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/mod.rs:345` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/mod.rs:348` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/mod.rs:351` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/mod.rs:355` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/mod.rs:356` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/mod.rs:358` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/mod.rs:363` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/mod.rs:391` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/mod.rs:394` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/mod.rs:399` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/mod.rs:401` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/mod.rs:405` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/mod.rs:411` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/mod.rs:415` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/mod.rs:417` | canonical account/resource/RBAC/durable-proof acquisition + live authority | 3→4→5→6 | yes | yes | yes |
| `portal-rs/crates/admin/src/rbac/mod.rs:422` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/mod.rs:433` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/mod.rs:435` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/mod.rs:440` | canonical account/resource/RBAC/durable-proof acquisition + live authority | 3→4→5→6 | yes | yes | yes |
| `portal-rs/crates/admin/src/rbac/mod.rs:445` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/mod.rs:447` | canonical account/resource/RBAC/durable-proof acquisition + live authority | 3→4→5→6 | yes | yes | yes |
| `portal-rs/crates/admin/src/rbac/mod.rs:453` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/mod.rs:459` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/mod.rs:465` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/mod.rs:473` | canonical account/resource/RBAC/durable-proof acquisition + live authority | 3→4→5→6 | yes | yes | yes |
| `portal-rs/crates/admin/src/rbac/mod.rs:479` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/mod.rs:485` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/mod.rs:491` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/mod.rs:499` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/mod.rs:505` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/mod.rs:507` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/panel_tests.rs:1` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/panel_tests.rs:42` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/panel_tests.rs:46` | implicit SQL write locks; sqlx::query("INSERT INTO account_roles(account_id,role_id) SELECT $1,id FROM roles WHERE key='platform_admin'").bind(actor.account).execute( | 7 (effects; owning entry above) | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/panel_tests.rs:66` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/panel_tests.rs:76` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/panel_tests.rs:77` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/panel_tests.rs:78` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/panel_tests.rs:81` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/panel_tests.rs:92` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/panel_tests.rs:107` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/panel_tests.rs:129` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/panel_tests.rs:130` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/panel_tests.rs:137` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/panel_tests.rs:140` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/panel_tests.rs:159` | implicit SQL write locks; sqlx::query("INSERT INTO account_roles(account_id,role_id) VALUES($1,$2)") | 7 (effects; owning entry above) | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/panel_tests.rs:166` | implicit SQL write locks; sqlx::query("INSERT INTO account_roles(account_id,role_id) VALUES($1,$2)") | 7 (effects; owning entry above) | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/panel_tests.rs:183` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/panel_tests.rs:195` | implicit SQL write locks; sqlx::query("INSERT INTO account_roles(account_id,role_id) VALUES($1,$2),($3,$2)") | 7 (effects; owning entry above) | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/panel_tests.rs:213` | implicit SQL write locks; sqlx::query("INSERT INTO account_roles(account_id,role_id) VALUES($1,$2)") | 7 (effects; owning entry above) | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/panel_tests.rs:257` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/panel_tests.rs:281` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/panel_tests.rs:283` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/panel_tests.rs:292` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/panel_tests.rs:293` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/panel_tests.rs:299` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/panel_tests.rs:305` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/panel_tests.rs:315` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/panel_tests.rs:328` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/panel_tests.rs:338` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/panel_tests.rs:344` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/panel_tests.rs:362` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/panel_tests.rs:378` | test fixture/blocker: pg_advisory_xact_lock; sqlx::query("SELECT pg_advisory_xact_lock(71010)") | 5 | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/panel_tests.rs:396` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/panel_tests.rs:426` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/panel_tests.rs:428` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/panel_tests.rs:430` | test fixture/blocker: pg_advisory_xact_lock; sqlx::query("SELECT pg_advisory_xact_lock(71010)") | 5 | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/panel_tests.rs:440` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/panel_tests.rs:494` | implicit SQL write locks; sqlx::query("INSERT INTO account_roles(account_id,role_id) VALUES($1,$2)") | 7 (effects; owning entry above) | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/panel_tests.rs:550` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/panel_tests.rs:551` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/panel_tests.rs:594` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/panel_tests.rs:610` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/panel_tests.rs:611` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/panel_tests.rs:619` | test fixture/blocker: FOR UPDATE; query tables accounts; sqlx::query("SELECT id FROM accounts WHERE id=$1 FOR UPDATE") | 3 | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/panel_tests.rs:623` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/panel_tests.rs:645` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/panel_tests.rs:646` | observer SQL; no lock acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/panel_tests.rs:650` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/panel_tests.rs:656` | test fixture/blocker: pg_advisory_xact_lock; sqlx::query("SELECT pg_advisory_xact_lock(71010)") | 5 | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/panel_tests.rs:683` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/panel_tests.rs:690` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/panel_tests.rs:703` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/panel_tests.rs:716` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/panel_tests.rs:717` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/panel_tests.rs:719` | test fixture/blocker: FOR UPDATE; query tables accounts; sqlx::query("SELECT id FROM accounts WHERE id=$1 FOR UPDATE") | 3 | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/panel_tests.rs:729` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/panel_tests.rs:736` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/panel_tests.rs:744` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/storage.rs:1` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/storage.rs:12` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/storage.rs:14` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/storage.rs:16` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/storage.rs:28` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/storage.rs:41` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/storage.rs:68` | canonical account/resource/RBAC/durable-proof acquisition + live authority | 3→4→5→6 | yes | Same-transaction authority rechecked after final write before commit (MFA approval also request expiry) | yes |
| `portal-rs/crates/admin/src/rbac/storage.rs:73` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/storage.rs:76` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/storage.rs:79` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/storage.rs:81` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/storage.rs:82` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/storage.rs:83` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/storage.rs:90` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/storage.rs:95` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/storage.rs:103` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/storage.rs:105` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/storage.rs:106` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/storage.rs:112` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/storage.rs:117` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/storage.rs:121` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/storage.rs:122` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/storage.rs:129` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/storage.rs:131` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/storage.rs:133` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/storage.rs:138` | implicit SQL write locks; sqlx::query("DELETE FROM role_permissions WHERE role_id = $1") | 7 (effects; owning entry above) | yes | Same-transaction authority rechecked after final write before commit (MFA approval also request expiry) | yes |
| `portal-rs/crates/admin/src/rbac/storage.rs:142` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/storage.rs:148` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/storage.rs:150` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/storage.rs:151` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/storage.rs:164` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/storage.rs:171` | implicit SQL write locks; sqlx::query("INSERT INTO account_roles (account_id, role_id) VALUES ($1, $2) ON CONFLICT DO NOTHING") | 7 (effects; owning entry above) | yes | Same-transaction authority rechecked after final write before commit (MFA approval also request expiry) | yes |
| `portal-rs/crates/admin/src/rbac/storage.rs:174` | implicit SQL write locks; sqlx::query("DELETE FROM account_roles WHERE account_id = $1 AND role_id = $2") | 7 (effects; owning entry above) | yes | Same-transaction authority rechecked after final write before commit (MFA approval also request expiry) | yes |
| `portal-rs/crates/admin/src/rbac/storage.rs:230` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/storage.rs:231` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/storage.rs:232` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/storage.rs:238` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/storage.rs:239` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/storage.rs:262` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/storage.rs:265` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/storage.rs:267` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/storage.rs:268` | implicit SQL write locks; sqlx::query("INSERT INTO role_permissions (role_id, permission) VALUES ($1, $2)") | 7 (effects; owning entry above) | yes | Same-transaction authority rechecked after final write before commit (MFA approval also request expiry) | yes |
| `portal-rs/crates/admin/src/rbac/storage.rs:270` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/storage.rs:277` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/storage.rs:279` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/storage.rs:282` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/tests.rs:3` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/tests.rs:17` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/tests.rs:23` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/tests.rs:25` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/tests.rs:37` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/tests.rs:44` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/tests.rs:45` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/tests.rs:47` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/tests.rs:48` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/tests.rs:49` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/tests.rs:52` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/tests.rs:57` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/tests.rs:58` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/tests.rs:59` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/tests.rs:61` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/tests.rs:62` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/tests.rs:67` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/tests.rs:82` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/tests.rs:99` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/tests.rs:101` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/tests.rs:105` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/tests.rs:107` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/tests.rs:111` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/tests.rs:117` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/tests.rs:118` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/tests.rs:127` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/tests.rs:132` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/tests.rs:193` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/tests.rs:194` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/tests.rs:208` | implicit SQL write locks; sqlx::query("INSERT INTO account_roles (account_id, role_id) VALUES ($1, $2)") | 7 (effects; owning entry above) | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/tests.rs:227` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/tests.rs:235` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/tests.rs:245` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/tests.rs:256` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/tests.rs:270` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/tests.rs:291` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/tests.rs:302` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/tests.rs:316` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/tests.rs:329` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/tests.rs:342` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/tests.rs:352` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/tests.rs:356` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/tests.rs:362` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/tests.rs:364` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/tests.rs:370` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/tests.rs:439` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/tests.rs:468` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/tests.rs:492` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/tests.rs:523` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/tests.rs:531` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/tests.rs:543` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/tests.rs:545` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/tests.rs:549` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/tests.rs:553` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/tests.rs:556` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/tests.rs:558` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/tests.rs:571` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/tests.rs:577` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/tests.rs:579` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/tests.rs:593` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/tests.rs:607` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/tests.rs:624` | implicit SQL write locks; sqlx::query("INSERT INTO role_permissions (role_id, permission) VALUES ($1, 'admin.*')") | 7 (effects; owning entry above) | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/tests.rs:645` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/tests.rs:666` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/tests.rs:765` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/tests.rs:809` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/tests.rs:810` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/tests.rs:814` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/tests.rs:818` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/tests.rs:823` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/tests.rs:837` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/tests.rs:838` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/tests.rs:843` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/tests.rs:848` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/tests.rs:852` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/tests.rs:859` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/tests.rs:867` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/tests.rs:873` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/tests.rs:877` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/tests.rs:897` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/tests.rs:898` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/tests.rs:899` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/tests.rs:907` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/tests.rs:913` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/tests.rs:921` | implicit SQL write locks; sqlx::query("INSERT INTO role_permissions (role_id, permission) VALUES ($1, $2)") | 7 (effects; owning entry above) | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/tests.rs:961` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/tests.rs:963` | test fixture/blocker: pg_advisory_xact_lock; sqlx::query("SELECT pg_advisory_xact_lock(71010)") | 5 | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/tests.rs:973` | test fixture/blocker: pg_advisory_xact_lock; sqlx::query("SELECT pg_advisory_xact_lock(71010)").execute(&mut *tx).await.unwrap(); | 5 | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/tests.rs:974` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/tests.rs:975` | implicit SQL write locks; sqlx::query("INSERT INTO account_roles(account_id,role_id) SELECT $1,id FROM roles WHERE key='support'") | 7 (effects; owning entry above) | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/tests.rs:1014` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/tests.rs:1019` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/tests.rs:1026` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/tests.rs:1039` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/tests.rs:1062` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/tests.rs:1076` | test fixture/blocker: pg_advisory_xact_lock; sqlx::query("SELECT pg_advisory_xact_lock(71010)") | 5 | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/rbac/tests.rs:1081` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/support.rs:1` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/support.rs:8` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/support.rs:22` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/support.rs:24` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/support.rs:25` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/support.rs:26` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/support.rs:137` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/support.rs:172` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/support.rs:177` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/support.rs:254` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/support.rs:268` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/support.rs:271` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/support.rs:272` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/support.rs:283` | canonical account/resource/RBAC/durable-proof acquisition + live authority | 3→4→5→6 | yes | yes | yes |
| `portal-rs/crates/admin/src/support.rs:288` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/support.rs:295` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/support.rs:296` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/support.rs:321` | canonical account/resource/RBAC/durable-proof acquisition + live authority | 3→4→5→6 | yes | yes | yes |
| `portal-rs/crates/admin/src/support.rs:326` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/support_en.ftl:3` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/support_en.ftl:22` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/support_pagination_tests.rs:6` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/support_pagination_tests.rs:12` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/support_pagination_tests.rs:14` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/support_pagination_tests.rs:69` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/support_pl.ftl:22` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/support_tests.rs:113` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/support_tests.rs:119` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/support_tests.rs:121` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/support_tests.rs:192` | implicit SQL write locks; sqlx::query("INSERT INTO account_roles (account_id,role_id) SELECT $1,id FROM roles WHERE key='platform_admin'").bind(actor).execute(&pool). | 7 (effects; owning entry above) | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/support_tests.rs:193` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/support_tests.rs:196` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/support_web.rs:3` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/support_web.rs:16` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/support_web.rs:54` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/support_web.rs:63` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/support_web.rs:69` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/support_web.rs:70` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/support_web.rs:81` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/support_web.rs:90` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/support_web.rs:349` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/support_web.rs:440` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/test_staff.rs:1` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/test_staff.rs:20` | implicit SQL write locks; sqlx::query("INSERT INTO web_sessions(id_hash,account_id,idle_expires_at,absolute_expires_at,user_agent_summary,csrf_token,data,authorizatio | 7 (effects; owning entry above) | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/test_staff.rs:72` | test fixture/blocker: FOR UPDATE; query tables accounts; sqlx::query("SELECT id FROM accounts WHERE id=$1 FOR UPDATE") | 3 | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/test_staff.rs:88` | test fixture/blocker: pg_try_advisory_xact_lock; let free: bool = sqlx::query_scalar("SELECT pg_try_advisory_xact_lock(71010)") | 5 | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/web.rs:1` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/web.rs:5` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/web.rs:6` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/web.rs:26` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/web.rs:31` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/web.rs:35` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/web.rs:39` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/web.rs:40` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/web.rs:65` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/web.rs:66` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/web.rs:77` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/web.rs:84` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/web.rs:92` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/web.rs:95` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/web.rs:101` | FOR SHARE; query tables accounts; "SELECT status,email_verified_at IS NOT NULL FROM accounts WHERE id=$1 FOR SHARE", | 3 | no | n.a. | yes |
| `portal-rs/crates/admin/src/web.rs:135` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/web.rs:137` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/web.rs:173` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/web.rs:175` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/web.rs:182` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/web.rs:191` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/web.rs:196` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/web.rs:200` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/web.rs:206` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/web.rs:212` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/web.rs:214` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/web.rs:229` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/web.rs:241` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/web.rs:259` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/web.rs:274` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/web.rs:314` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/web.rs:338` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/web.rs:373` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/web.rs:377` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/web.rs:380` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/web.rs:382` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/web.rs:384` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/web.rs:469` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/web.rs:470` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/web.rs:496` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/web.rs:499` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/web.rs:506` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/web.rs:554` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/web.rs:556` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/web.rs:562` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/web.rs:563` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/web.rs:564` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/web.rs:569` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/web.rs:575` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/web.rs:578` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/web.rs:580` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/web.rs:615` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/web.rs:647` | canonical account/resource/RBAC/durable-proof acquisition + live authority | 3→4→5→6 | yes | Same-transaction authority rechecked after final write before commit (MFA approval also request expiry) | yes |
| `portal-rs/crates/admin/src/web.rs:652` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/web.rs:655` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/web.rs:656` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/web.rs:664` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/web.rs:673` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/web.rs:679` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/web.rs:688` | implicit SQL write locks; sqlx::query("DELETE FROM account_roles WHERE account_id=$1 AND role_id=$2") | 7 (effects; owning entry above) | no | n.a. | yes |
| `portal-rs/crates/admin/src/web_tests.rs:10` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/web_tests.rs:11` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/web_tests.rs:12` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/web_tests.rs:18` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/web_tests.rs:24` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/web_tests.rs:26` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/web_tests.rs:34` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/web_tests.rs:39` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/web_tests.rs:46` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/web_tests.rs:52` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/web_tests.rs:56` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/web_tests.rs:82` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/web_tests.rs:87` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/web_tests.rs:98` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/web_tests.rs:102` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/web_tests.rs:108` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/web_tests.rs:115` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/web_tests.rs:183` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/web_tests.rs:188` | implicit SQL write locks; "INSERT INTO account_roles(account_id,role_id) SELECT $1,id FROM roles WHERE key='support'", | 7 (effects; owning entry above) | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/web_tests.rs:194` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/web_tests.rs:200` | implicit SQL write locks; sqlx::query("INSERT INTO role_permissions(role_id,permission) VALUES ($1,'admin.access')") | 7 (effects; owning entry above) | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/web_tests.rs:205` | implicit SQL write locks; sqlx::query("INSERT INTO account_roles(account_id,role_id) VALUES ($1,$2)") | 7 (effects; owning entry above) | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/web_tests.rs:211` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/web_tests.rs:225` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/web_tests.rs:239` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/web_tests.rs:244` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/web_tests.rs:251` | implicit SQL write locks; "INSERT INTO role_permissions(role_id,permission) VALUES ($1,'admin.roles.manage')", | 7 (effects; owning entry above) | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/web_tests.rs:260` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/web_tests.rs:268` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/web_tests.rs:305` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/web_tests.rs:308` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/web_tests.rs:319` | implicit SQL write locks; sqlx::query_scalar("INSERT INTO roles(key) VALUES ('ordinary_staff') RETURNING id") | 7 (effects; owning entry above) | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/web_tests.rs:331` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/web_tests.rs:334` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/web_tests.rs:349` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/web_tests.rs:361` | implicit SQL write locks; sqlx::query("INSERT INTO account_roles(account_id,role_id) VALUES ($1,$2)") | 7 (effects; owning entry above) | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/web_tests.rs:367` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/web_tests.rs:370` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/web_tests.rs:378` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/web_tests.rs:419` | implicit SQL write locks; sqlx::query("INSERT INTO account_roles(account_id,role_id) VALUES ($1,$2)") | 7 (effects; owning entry above) | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/web_tests.rs:430` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/web_tests.rs:441` | implicit SQL write locks; sqlx::query("INSERT INTO account_roles(account_id,role_id) VALUES ($1,$2)") | 7 (effects; owning entry above) | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/web_tests.rs:488` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/web_tests.rs:489` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/web_tests.rs:490` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/web_tests.rs:516` | implicit SQL write locks; "INSERT INTO accounts(email,password_hash,status,locale,email_verified_at) VALUES('staff-fixture@local.test',$1,'active','en',now()) RETURNI | 7 (effects; owning entry above) | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/web_tests.rs:518` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/web_tests.rs:531` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/web_tests.rs:548` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/web_tests.rs:556` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/admin/src/web_tests.rs:557` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/atlas/src/bestiary/README.md:5` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/atlas/src/bestiary/mod.rs:94` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/atlas/src/bestiary/web.rs:106` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/atlas/src/spawns.rs:185` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/atlas/src/spawns.rs:245` | FOR UPDATE; query tables accounts; sqlx::query_scalar("SELECT status='active' FROM accounts WHERE id=$1 FOR UPDATE") | 3 | no | n.a. | yes |
| `portal-rs/crates/atlas/src/spawns.rs:295` | FOR UPDATE; query tables accounts; sqlx::query_scalar("SELECT status='active' FROM accounts WHERE id=$1 FOR UPDATE") | 3 | no | n.a. | yes |
| `portal-rs/crates/atlas/src/spawns_tests.rs:107` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/atlas/src/spawns_tests.rs:274` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/atlas/src/spawns_tests.rs:293` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/atlas/src/tiles.rs:110` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/atlas/src/tiles/tests.rs:202` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/bazaar/README.md:13` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/bazaar/README.md:76` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/bazaar/src/bounded.rs:19` | FOR UPDATE; query tables bazaar_offers; Ok(sqlx::query_as("SELECT * FROM bazaar_offers WHERE listing_id=$1 AND state='pending' AND ($2::uuid IS NULL OR id&lt;&gt;$2) ORDER BY id LIMIT 64 | 2 | no | n.a. | yes |
| `portal-rs/crates/bazaar/src/characters/http.rs:76` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/bazaar/src/characters/http.rs:186` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/bazaar/src/characters/http.rs:211` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/bazaar/src/characters/http.rs:255` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/bazaar/src/characters/http.rs:271` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/bazaar/src/characters/http.rs:304` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/bazaar/src/characters/http.rs:344` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/bazaar/src/characters/http.rs:465` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/bazaar/src/characters/service.rs:104` | pg_advisory_xact_lock; sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1,59))") | domain namespace (README inventory) | no | n.a. | yes |
| `portal-rs/crates/bazaar/src/characters/service.rs:171` | FOR UPDATE; query tables bazaar_listings; "SELECT * FROM bazaar_listings WHERE id=$1 AND kind='character' FOR UPDATE", | 2 | no | n.a. | yes |
| `portal-rs/crates/bazaar/src/characters/service.rs:227` | FOR SHARE; query tables bazaar_listings; let l: Listing = sqlx::query_as("SELECT * FROM bazaar_listings WHERE id=$1 AND kind='character' AND state='active' AND ends_at&gt;clock_timesta | 2 | no | n.a. | yes |
| `portal-rs/crates/bazaar/src/characters/service.rs:307` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/bazaar/src/characters/tests.rs:111` | test fixture/blocker: FOR UPDATE; query tables test_accounts; sqlx::query_scalar("SELECT active FROM test_accounts WHERE id=$1 FOR UPDATE") | 4 / owning domain resource | n.a. | n.a. | yes |
| `portal-rs/crates/bazaar/src/characters/tests.rs:133` | test fixture/blocker: FOR UPDATE; query tables test_funds; sqlx::query("SELECT id FROM test_funds WHERE id=$1 FOR UPDATE") | 4 / owning domain resource | n.a. | n.a. | yes |
| `portal-rs/crates/bazaar/src/characters/tests.rs:435` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/bazaar/src/game/authority.rs:93` | FOR UPDATE; query tables bazaar_game_issuer; let row=sqlx::query("SELECT projection_epoch::text AS epoch,recovery_generation::text AS generation FROM bazaar_game_issuer WHERE singleton  | 4 / owning domain resource | no | n.a. | yes |
| `portal-rs/crates/bazaar/src/game/authority.rs:120` | FOR UPDATE; query tables bazaar_game_outbox; let row=sqlx::query("SELECT intent,binding_digest,projection_epoch::text AS epoch FROM bazaar_game_outbox WHERE operation_id=$1 FOR UPDATE") | 7 / domain namespace | no | n.a. | yes |
| `portal-rs/crates/bazaar/src/game/authority_evidence.rs:32` | FOR UPDATE; query tables bazaar_resync_progress; let progress=sqlx::query("SELECT listing_cursor,operation_cursor::text AS operation_cursor,worlds FROM bazaar_resync_progress WHERE projecti | 4 / owning domain resource | no | n.a. | yes |
| `portal-rs/crates/bazaar/src/game/authority_exchange.rs:205` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/bazaar/src/game/authority_http.rs:126` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/bazaar/src/game/authority_http.rs:250` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/bazaar/src/game/authority_tests.rs:226` | test fixture/blocker: FOR UPDATE; query tables bazaar_game_issuer; sqlx::query("SELECT singleton FROM bazaar_game_issuer FOR UPDATE") | 4 / owning domain resource | n.a. | n.a. | yes |
| `portal-rs/crates/bazaar/src/game/authority_tests.rs:677` | test fixture/blocker: FOR UPDATE; query tables bazaar_game_issuer; sqlx::query("SELECT singleton FROM bazaar_game_issuer FOR UPDATE") | 4 / owning domain resource | n.a. | n.a. | yes |
| `portal-rs/crates/bazaar/src/game/authority_tests.rs:1590` | test fixture/blocker: FOR UPDATE; query tables bazaar_listings; sqlx::query("SELECT id FROM bazaar_listings WHERE id=$1 FOR UPDATE") | 2 | n.a. | n.a. | yes |
| `portal-rs/crates/bazaar/src/game/authority_tests.rs:1697` | test fixture/blocker: pg_advisory_xact_lock; sqlx::query("SELECT pg_advisory_xact_lock(22005160000)") | domain namespace (README inventory) | n.a. | n.a. | yes |
| `portal-rs/crates/bazaar/src/game/authority_tests.rs:1947` | test fixture/blocker: pg_advisory_xact_lock; sqlx::query("SELECT pg_advisory_xact_lock(22005160000)") | domain namespace (README inventory) | n.a. | n.a. | yes |
| `portal-rs/crates/bazaar/src/game/authority_tests.rs:2126` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/bazaar/src/game/authority_tests.rs:2620` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/bazaar/src/game/authority_tests.rs:2961` | test fixture/blocker: FOR UPDATE; query tables bazaar_game_issuer; sqlx::query("SELECT singleton FROM bazaar_game_issuer FOR UPDATE") | 4 / owning domain resource | n.a. | n.a. | yes |
| `portal-rs/crates/bazaar/src/game/authority_tests.rs:3008` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/bazaar/src/game/authority_tests.rs:3017` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/bazaar/src/game/authority_tests.rs:3032` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/bazaar/src/game/coin_ports.rs:30` | FOR SHARE; query tables accounts; sqlx::query_scalar("SELECT status FROM accounts WHERE id=$1 FOR SHARE") | 3 | no | n.a. | yes |
| `portal-rs/crates/bazaar/src/game/coin_ports.rs:46` | FOR SHARE; query tables accounts; let row=sqlx::query("SELECT status,email_verified_at IS NOT NULL AS verified,email_change_blocked_until&gt;now() AS blocked FROM accounts WHERE | 3 | no | n.a. | yes |
| `portal-rs/crates/bazaar/src/game/coin_ports.rs:56` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/bazaar/src/game/coin_ports.rs:92` | FOR SHARE; query tables accounts; sqlx::query_scalar("SELECT status FROM accounts WHERE id=$1 FOR SHARE") | 3 | no | n.a. | yes |
| `portal-rs/crates/bazaar/src/game/coin_ports.rs:158` | FOR UPDATE; query tables wallet_accounts; sqlx::query("SELECT account_id FROM wallet_accounts WHERE account_id=$1 FOR UPDATE") | 7 / domain namespace | no | n.a. | yes |
| `portal-rs/crates/bazaar/src/game/coin_ports.rs:177` | FOR UPDATE; query tables bazaar_coin_holds; let old=sqlx::query("SELECT amount,settlement_id,completed FROM bazaar_coin_holds WHERE id=$1 AND account_id=$2 FOR UPDATE").bind(h.id).bind | 4 / owning domain resource | no | n.a. | yes |
| `portal-rs/crates/bazaar/src/game/coin_ports.rs:195` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/bazaar/src/game/coin_ports.rs:198` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/bazaar/src/game/coin_ports.rs:224` | FOR UPDATE; query tables bazaar_coin_holds; let row=sqlx::query("SELECT amount,settlement_id FROM bazaar_coin_holds WHERE id=$1 AND account_id=$2 FOR UPDATE").bind(id).bind(account).fe | 4 / owning domain resource | no | n.a. | yes |
| `portal-rs/crates/bazaar/src/game/coin_ports.rs:241` | FOR UPDATE; query tables bazaar_coin_dispositions; "SELECT completed FROM bazaar_coin_dispositions WHERE settlement_id=$1 FOR UPDATE", | 4 / owning domain resource | no | n.a. | yes |
| `portal-rs/crates/bazaar/src/game/coin_ports.rs:322` | FOR UPDATE; query tables bazaar_coin_dispositions; "SELECT binding FROM bazaar_coin_dispositions WHERE settlement_id=$1 FOR UPDATE", | 4 / owning domain resource | no | n.a. | yes |
| `portal-rs/crates/bazaar/src/game/coin_ports.rs:339` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/bazaar/src/game/coin_ports.rs:344` | FOR UPDATE; query tables bazaar_coin_holds; let row=sqlx::query("SELECT amount,settlement_id FROM bazaar_coin_holds WHERE id=$1 AND account_id=$2 FOR UPDATE").bind(s.reservation).bind( | 4 / owning domain resource | no | n.a. | yes |
| `portal-rs/crates/bazaar/src/game/coin_ports.rs:374` | FOR UPDATE; query tables bazaar_coin_dispositions; "SELECT completed FROM bazaar_coin_dispositions WHERE settlement_id=$1 FOR UPDATE", | 4 / owning domain resource | no | n.a. | yes |
| `portal-rs/crates/bazaar/src/game/coin_ports.rs:383` | FOR UPDATE; query tables bazaar_coin_holds; "SELECT amount FROM bazaar_coin_holds WHERE id=$1 AND account_id=$2 FOR UPDATE", | 4 / owning domain resource | no | n.a. | yes |
| `portal-rs/crates/bazaar/src/game/coin_ports.rs:409` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/bazaar/src/game/coin_ports_lock_tests.rs:78` | test fixture/blocker: FOR UPDATE; query tables accounts; sqlx::query("SELECT id FROM accounts WHERE id=$1 FOR UPDATE") | 3 | n.a. | n.a. | yes |
| `portal-rs/crates/bazaar/src/game/coin_ports_lock_tests.rs:108` | test fixture/blocker: FOR UPDATE NOWAIT; query tables wallet_accounts; sqlx::query("SELECT account_id FROM wallet_accounts WHERE account_id=$1 FOR UPDATE NOWAIT") | 7 / domain namespace | n.a. | n.a. | yes |
| `portal-rs/crates/bazaar/src/game/game_projection.rs:249` | FOR UPDATE; query tables bazaar_game_issuer; let floor=sqlx::query("SELECT projection_epoch::text AS epoch,recovery_generation::text AS generation FROM bazaar_game_issuer WHERE singleto | 4 / owning domain resource | no | n.a. | yes |
| `portal-rs/crates/bazaar/src/game/game_projection.rs:284` | FOR UPDATE; query tables bazaar_game_projections; let old=sqlx::query("SELECT projection_epoch::text AS epoch,projection_revision::text AS revision,source_observed_at::text AS observed,conte | 4 / owning domain resource | no | n.a. | yes |
| `portal-rs/crates/bazaar/src/game/game_store.rs:17` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/bazaar/src/game/game_store.rs:93` | FOR UPDATE; query tables bazaar_game_issuer; let floor=sqlx::query("SELECT source_revision::text AS revision, projection_epoch::text AS epoch, recovery_generation::text AS generation FR | 4 / owning domain resource | no | n.a. | yes |
| `portal-rs/crates/bazaar/src/game/game_store.rs:152` | FOR UPDATE; query tables bazaar_game_issuer; let row=sqlx::query("SELECT projection_epoch::text AS epoch,recovery_generation::text AS generation FROM bazaar_game_issuer WHERE singleton  | 4 / owning domain resource | no | n.a. | yes |
| `portal-rs/crates/bazaar/src/game/mod.rs:6` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/bazaar/src/game/runtime/characters.rs:506` | pg_advisory_xact_lock; sqlx::query("SELECT pg_advisory_xact_lock(22005160000)") | domain namespace (README inventory) | no | n.a. | yes |
| `portal-rs/crates/bazaar/src/game/runtime/gold_worker.rs:90` | FOR UPDATE; query tables bazaar_listings; let l: Listing = sqlx::query_as("SELECT * FROM bazaar_listings WHERE id=$1 FOR UPDATE") | 2 | no | n.a. | yes |
| `portal-rs/crates/bazaar/src/game/runtime/gold_worker.rs:95` | FOR UPDATE; query tables bazaar_authority_gold_bids; sqlx::query("SELECT * FROM bazaar_authority_gold_bids WHERE request_id=$1 FOR UPDATE") | 4 / owning domain resource | no | n.a. | yes |
| `portal-rs/crates/bazaar/src/game/runtime/mod.rs:45` | FOR UPDATE; query tables bazaar_authority_snapshot_keys; let old=sqlx::query("SELECT public_key,revoked FROM bazaar_authority_snapshot_keys WHERE kid=$1 FOR UPDATE").bind(kid).fetch_optional(&mut * | 4 / owning domain resource | no | n.a. | yes |
| `portal-rs/crates/bazaar/src/game/runtime/mod.rs:343` | FOR UPDATE; query tables bazaar_listings; let row: Listing = sqlx::query_as("SELECT * FROM bazaar_listings WHERE id=$1 FOR UPDATE") | 2 | no | n.a. | yes |
| `portal-rs/crates/bazaar/src/game/runtime/mod.rs:363` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/bazaar/src/game/runtime/mod.rs:377` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/bazaar/src/game/runtime/mod.rs:482` | FOR UPDATE; query tables bazaar_listings; let l: Listing = sqlx::query_as("SELECT * FROM bazaar_listings WHERE id=$1 FOR UPDATE") | 2 | no | n.a. | yes |
| `portal-rs/crates/bazaar/src/game/runtime/mod.rs:545` | pg_advisory_xact_lock; sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1,64004))") | domain namespace (README inventory) | no | n.a. | yes |
| `portal-rs/crates/bazaar/src/game/runtime/tls.rs:260` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/bazaar/src/game/runtime/tls.rs:319` | FOR UPDATE; query tables bazaar_game_issuer; sqlx::query("SELECT singleton FROM bazaar_game_issuer FOR UPDATE") | 4 / owning domain resource | no | n.a. | yes |
| `portal-rs/crates/bazaar/src/game/runtime/tls.rs:324` | FOR UPDATE; query tables bazaar_game_issuer; sqlx::query("SELECT singleton FROM bazaar_game_issuer FOR UPDATE") | 4 / owning domain resource | no | n.a. | yes |
| `portal-rs/crates/bazaar/src/game/runtime/transport.rs:136` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/bazaar/src/game/runtime/worker.rs:139` | FOR UPDATE; query tables bazaar_listings; let l: Listing = sqlx::query_as("SELECT * FROM bazaar_listings WHERE id=$1 FOR UPDATE") | 2 | no | n.a. | yes |
| `portal-rs/crates/bazaar/src/game/runtime/worker.rs:390` | pg_advisory_xact_lock; sqlx::query("SELECT pg_advisory_xact_lock(22005160000)") | domain namespace (README inventory) | no | n.a. | yes |
| `portal-rs/crates/bazaar/src/game/runtime/worker.rs:678` | pg_advisory_xact_lock; sqlx::query("SELECT pg_advisory_xact_lock(22005160000)") | domain namespace (README inventory) | no | n.a. | yes |
| `portal-rs/crates/bazaar/src/items/README.md:37` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/bazaar/src/items/service.rs:29` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/bazaar/src/items/service.rs:69` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/bazaar/src/items/service.rs:81` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/bazaar/src/items/service.rs:82` | pg_advisory_xact_lock; sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1,60))") | domain namespace (README inventory) | no | n.a. | yes |
| `portal-rs/crates/bazaar/src/items/service.rs:125` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/bazaar/src/items/service.rs:160` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/bazaar/src/items/service.rs:243` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/bazaar/src/items/service.rs:268` | pg_advisory_xact_lock; sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1,57))") | domain namespace (README inventory) | no | n.a. | yes |
| `portal-rs/crates/bazaar/src/items/service.rs:321` | FOR UPDATE; query tables bazaar_offers; "SELECT * FROM bazaar_offers WHERE listing_id=$1 AND id=$2 FOR UPDATE", | 2 | no | n.a. | yes |
| `portal-rs/crates/bazaar/src/items/service.rs:332` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/bazaar/src/items/service.rs:429` | FOR UPDATE; query tables bazaar_offers; sqlx::query_as("SELECT * FROM bazaar_offers WHERE listing_id=$1 AND id=$2 FOR UPDATE") | 2 | no | n.a. | yes |
| `portal-rs/crates/bazaar/src/items/service.rs:438` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/bazaar/src/items/service.rs:472` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/bazaar/src/items/service.rs:555` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/bazaar/src/items/service.rs:591` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/bazaar/src/items/service.rs:678` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/bazaar/src/items/service.rs:724` | FOR UPDATE OF l,d; query tables bazaar_item_details, bazaar_listings; sqlx::query_as("SELECT l.* FROM bazaar_listings l JOIN bazaar_item_details d ON d.id=l.id WHERE l.id=$1 AND l.kind='item' FOR UPDATE OF l,d" | 2 | no | n.a. | yes |
| `portal-rs/crates/bazaar/src/items/tests.rs:88` | test fixture/blocker: pg_advisory_xact_lock; sqlx::query("SELECT pg_advisory_xact_lock(600)") | domain namespace (README inventory) | n.a. | n.a. | yes |
| `portal-rs/crates/bazaar/src/locking.rs:2` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/bazaar/src/locking.rs:6` | pg_advisory_xact_lock_shared; sqlx::query("SELECT pg_advisory_xact_lock_shared(64002)") | 1 | no | n.a. | fixed in parallel task drc-a |
| `portal-rs/crates/bazaar/src/locking.rs:18` | FOR SHARE; query tables accounts; sqlx::query("SELECT id FROM accounts WHERE id=ANY($1) ORDER BY id FOR SHARE") | 3 | no | n.a. | fixed in parallel task drc-a |
| `portal-rs/crates/bazaar/src/moderation.rs:119` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/bazaar/src/moderation.rs:180` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/bazaar/src/moderation.rs:181` | pg_advisory_xact_lock; sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1,64))") | domain namespace (README inventory) | no | n.a. | yes |
| `portal-rs/crates/bazaar/src/moderation.rs:238` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/bazaar/src/moderation.rs:322` | FOR UPDATE; query tables bazaar_listings; sqlx::query_as("SELECT owner,state FROM bazaar_listings WHERE id=$1 FOR UPDATE") | 2 | no | n.a. | yes |
| `portal-rs/crates/bazaar/src/moderation.rs:337` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/bazaar/src/moderation.rs:338` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/bazaar/src/moderation.rs:341` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/bazaar/src/moderation.rs:377` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/bazaar/src/moderation.rs:400` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/bazaar/src/moderation.rs:401` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/bazaar/src/moderation.rs:403` | pg_advisory_xact_lock; sqlx::query("SELECT pg_advisory_xact_lock(64002)") | 1 | no | n.a. | yes |
| `portal-rs/crates/bazaar/src/moderation.rs:407` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/bazaar/src/moderation.rs:408` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/bazaar/src/moderation.rs:411` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/bazaar/src/moderation.rs:420` | FOR UPDATE; query tables bazaar_invalidation_requests; sqlx::query("SELECT id FROM bazaar_invalidation_requests WHERE id=$1 FOR UPDATE") | 4 / owning domain resource | no | n.a. | yes |
| `portal-rs/crates/bazaar/src/moderation.rs:425` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/bazaar/src/moderation.rs:426` | FOR UPDATE; query tables bazaar_reports; sqlx::query("SELECT id FROM bazaar_reports WHERE id=$1 FOR UPDATE") | 4 / owning domain resource | no | n.a. | yes |
| `portal-rs/crates/bazaar/src/moderation.rs:431` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/bazaar/src/moderation.rs:432` | pg_advisory_xact_lock; sqlx::query("SELECT pg_advisory_xact_lock(64001)") | domain namespace (README inventory) | no | n.a. | yes |
| `portal-rs/crates/bazaar/src/moderation.rs:436` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/bazaar/src/moderation.rs:437` | pg_advisory_xact_lock; sqlx::query("SELECT pg_advisory_xact_lock(64002)") | 1 | no | n.a. | yes |
| `portal-rs/crates/bazaar/src/moderation.rs:445` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/bazaar/src/moderation.rs:446` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/bazaar/src/moderation.rs:449` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/bazaar/src/moderation.rs:453` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/bazaar/src/moderation.rs:455` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/bazaar/src/moderation.rs:467` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/bazaar/src/moderation.rs:485` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/bazaar/src/moderation.rs:495` | FOR UPDATE; query tables bazaar_invalidation_requests; let (requester,revision,state,reviewer):(Uuid,i64,String,Option&lt;Uuid&gt;)=sqlx::query_as("SELECT requested_by,hold_revision,state,approved_by F | 4 / owning domain resource | no | n.a. | yes |
| `portal-rs/crates/bazaar/src/moderation.rs:521` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/bazaar/src/moderation.rs:523` | FOR UPDATE; query tables bazaar_reports; sqlx::query_scalar("SELECT reporter FROM bazaar_reports WHERE id=$1 FOR UPDATE") | 4 / owning domain resource | no | n.a. | yes |
| `portal-rs/crates/bazaar/src/moderation.rs:537` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/bazaar/src/moderation.rs:549` | pg_advisory_xact_lock; sqlx::query("SELECT pg_advisory_xact_lock(64001)") | domain namespace (README inventory) | no | n.a. | yes |
| `portal-rs/crates/bazaar/src/moderation.rs:555` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/bazaar/src/moderation.rs:574` | pg_advisory_xact_lock; sqlx::query("SELECT pg_advisory_xact_lock(64002)") | 1 | no | n.a. | yes |
| `portal-rs/crates/bazaar/src/moderation_README.md:30` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/bazaar/src/moderation_README.md:32` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/bazaar/src/moderation_README.md:48` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/bazaar/src/moderation_README.md:64` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/bazaar/src/moderation_en.ftl:13` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/bazaar/src/moderation_en.ftl:38` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/bazaar/src/moderation_pl.ftl:13` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/bazaar/src/moderation_tests.rs:100` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/bazaar/src/moderation_tests.rs:102` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/bazaar/src/moderation_tests.rs:183` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/bazaar/src/moderation_tests.rs:184` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/bazaar/src/moderation_tests.rs:193` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/bazaar/src/moderation_tests.rs:194` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/bazaar/src/moderation_tests.rs:217` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/bazaar/src/moderation_tests.rs:266` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/bazaar/src/moderation_tests.rs:295` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/bazaar/src/moderation_tests.rs:296` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/bazaar/src/moderation_tests.rs:298` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/bazaar/src/moderation_tests.rs:306` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/bazaar/src/moderation_tests.rs:310` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/bazaar/src/moderation_tests.rs:312` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/bazaar/src/moderation_tests.rs:339` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/bazaar/src/moderation_tests.rs:340` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/bazaar/src/moderation_tests.rs:342` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/bazaar/src/moderation_tests.rs:349` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/bazaar/src/moderation_tests.rs:351` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/bazaar/src/moderation_tests.rs:356` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/bazaar/src/moderation_tests.rs:358` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/bazaar/src/moderation_tests.rs:365` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/bazaar/src/moderation_tests.rs:367` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/bazaar/src/moderation_tests.rs:391` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/bazaar/src/moderation_tests.rs:393` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/bazaar/src/moderation_tests.rs:395` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/bazaar/src/moderation_tests.rs:415` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/bazaar/src/moderation_tests.rs:468` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/bazaar/src/moderation_tests.rs:472` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/bazaar/src/moderation_tests.rs:475` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/bazaar/src/moderation_tests.rs:499` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/bazaar/src/moderation_tests.rs:501` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/bazaar/src/moderation_tests.rs:548` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/bazaar/src/moderation_tests.rs:619` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/bazaar/src/moderation_tests.rs:632` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/bazaar/src/moderation_tests.rs:636` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/bazaar/src/moderation_tests.rs:638` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/bazaar/src/moderation_tests.rs:642` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/bazaar/src/moderation_tests.rs:644` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/bazaar/src/moderation_tests.rs:665` | test fixture/blocker: FOR UPDATE; query tables bazaar_listings; sqlx::query("SELECT id FROM bazaar_listings WHERE id=$1 FOR UPDATE") | 2 | n.a. | n.a. | yes |
| `portal-rs/crates/bazaar/src/moderation_tests.rs:669` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/bazaar/src/moderation_tests.rs:681` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/bazaar/src/moderation_tests.rs:708` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/bazaar/src/moderation_tests.rs:709` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/bazaar/src/moderation_tests.rs:719` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/bazaar/src/moderation_tests.rs:732` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/bazaar/src/notify.rs:207` | FOR UPDATE SKIP LOCKED; query tables bazaar_notification_outbox; let pending:Option&lt;Uuid&gt;=sqlx::query_scalar("SELECT id FROM bazaar_notification_outbox WHERE id=$1 AND state='pending' FOR UPDATE SKIP LOCKE | 7 / domain namespace | no | n.a. | yes |
| `portal-rs/crates/bazaar/src/notify/tests.rs:312` | test fixture/blocker: pg_advisory_xact_lock; sqlx::query("SELECT pg_advisory_xact_lock(64006)") | domain namespace (README inventory) | n.a. | n.a. | yes |
| `portal-rs/crates/bazaar/src/notify/tests.rs:335` | test fixture/blocker: pg_advisory_xact_lock; sqlx::query("SELECT pg_advisory_xact_lock(64006)") | domain namespace (README inventory) | n.a. | n.a. | yes |
| `portal-rs/crates/bazaar/src/repository.rs:45` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/bazaar/src/repository.rs:70` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/bazaar/src/repository.rs:80` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/bazaar/src/repository.rs:109` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/bazaar/src/repository.rs:139` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/bazaar/src/repository.rs:161` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/bazaar/src/repository.rs:167` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/bazaar/src/repository.rs:185` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/bazaar/src/repository.rs:278` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/bazaar/src/repository.rs:301` | pg_advisory_xact_lock; sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1,57))") | domain namespace (README inventory) | no | n.a. | yes |
| `portal-rs/crates/bazaar/src/repository.rs:372` | FOR UPDATE; query tables bazaar_offers; "SELECT * FROM bazaar_offers WHERE listing_id=$1 AND id=$2 FOR UPDATE", | 2 | no | n.a. | yes |
| `portal-rs/crates/bazaar/src/repository.rs:395` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/bazaar/src/repository.rs:451` | FOR UPDATE; query tables bazaar_offers; let offer:Offer=sqlx::query_as("SELECT * FROM bazaar_offers WHERE listing_id=$1 AND id=$2 AND state='pending' FOR UPDATE").bind(id).bind(off | 2 | no | n.a. | yes |
| `portal-rs/crates/bazaar/src/repository.rs:455` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/bazaar/src/repository.rs:479` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/bazaar/src/repository.rs:552` | FOR UPDATE; query tables bazaar_settlements; "SELECT idempotency_key FROM bazaar_settlements WHERE listing_id=$1 FOR UPDATE", | 4 / owning domain resource | no | n.a. | yes |
| `portal-rs/crates/bazaar/src/repository.rs:590` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/bazaar/src/repository.rs:655` | FOR UPDATE; query tables bazaar_listings; sqlx::query_as("SELECT * FROM bazaar_listings WHERE id=$1 FOR UPDATE") | 2 | no | n.a. | yes |
| `portal-rs/crates/bazaar/src/repository.rs:670` | FOR UPDATE; query tables bazaar_settlements; sqlx::query_as("SELECT * FROM bazaar_settlements WHERE listing_id=$1 FOR UPDATE") | 4 / owning domain resource | no | n.a. | yes |
| `portal-rs/crates/bazaar/src/search.rs:177` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/bazaar/src/search.rs:207` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/bazaar/src/search.rs:297` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/bazaar/src/search.rs:329` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/bazaar/src/search.rs:410` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/bazaar/src/search.rs:428` | pg_advisory_xact_lock; sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1,64005))") | domain namespace (README inventory) | no | n.a. | yes |
| `portal-rs/crates/bazaar/src/search.rs:465` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/bazaar/src/search.rs:466` | pg_advisory_xact_lock; sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1,61))") | domain namespace (README inventory) | no | n.a. | yes |
| `portal-rs/crates/bazaar/src/search.rs:490` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/bazaar/src/search.rs:512` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/bazaar/src/search_tests.rs:25` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/bazaar/src/search_tests.rs:95` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/bazaar/src/search_tests.rs:97` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/bazaar/tests/fixtures/plat3/README.md:59` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/bazaar/tests/fixtures/plat3/README.md:76` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/bazaar/tests/fixtures/plat3/plat3_authority.proto:8` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/bazaar/tests/postgres.rs:52` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/bazaar/tests/postgres.rs:102` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/bazaar/tests/postgres.rs:146` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/bazaar/tests/postgres.rs:359` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/bazaar/tests/postgres.rs:374` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/bazaar/tests/postgres.rs:482` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/bazaar/tests/postgres.rs:534` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/bazaar/tests/postgres.rs:561` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/bazaar/tests/postgres.rs:596` | test fixture/blocker: FOR SHARE; query tables accounts; sqlx::query("SELECT id FROM accounts WHERE id=$1 FOR SHARE") | 3 | n.a. | n.a. | yes |
| `portal-rs/crates/bazaar/tests/postgres.rs:639` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/bazaar/tests/postgres.rs:640` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/bazaar/tests/postgres.rs:641` | test fixture/blocker: pg_advisory_xact_lock; sqlx::query("SELECT pg_advisory_xact_lock(64002)") | 1 | n.a. | n.a. | yes |
| `portal-rs/crates/bazaar/tests/postgres.rs:642` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/bazaar/tests/postgres.rs:645` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/bazaar/tests/postgres.rs:663` | test fixture/blocker: FOR UPDATE NOWAIT; query tables accounts; let account_lock = sqlx::query("SELECT id FROM accounts WHERE id=$1 FOR UPDATE NOWAIT") | 3 | n.a. | n.a. | yes |
| `portal-rs/crates/bazaar/tests/postgres.rs:665` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/bazaar/tests/postgres.rs:667` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/bazaar/tests/postgres.rs:682` | test fixture/blocker: FOR SHARE; query tables accounts; sqlx::query("SELECT id FROM accounts WHERE id=ANY($1) ORDER BY id FOR SHARE") | 3 | n.a. | n.a. | yes |
| `portal-rs/crates/bazaar/tests/postgres.rs:745` | test fixture/blocker: FOR UPDATE; query tables accounts; sqlx::query("SELECT id FROM accounts WHERE id=$1 FOR UPDATE") | 3 | n.a. | n.a. | yes |
| `portal-rs/crates/bazaar/tests/postgres.rs:770` | test fixture/blocker: FOR UPDATE NOWAIT; query tables accounts; let later = sqlx::query("SELECT id FROM accounts WHERE id=$1 FOR UPDATE NOWAIT") | 3 | n.a. | n.a. | yes |
| `portal-rs/crates/bazaar/tests/support/mod.rs:151` | test fixture/blocker: FOR UPDATE; query tables test_ledger; sqlx::query_scalar("SELECT snapshot FROM test_ledger WHERE id=1 FOR UPDATE") | 7 / domain namespace | n.a. | n.a. | yes |
| `portal-rs/crates/bazaar/tests/support/mod.rs:326` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/characters/src/badges/README.md:34` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/characters/src/badges/http.rs:67` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/characters/src/badges/http.rs:97` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/characters/src/badges/http.rs:238` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/characters/src/badges/mod.rs:207` | FOR UPDATE; query tables accounts; sqlx::query_scalar("SELECT status FROM accounts WHERE id=$1 FOR UPDATE") | 3 | no | n.a. | yes |
| `portal-rs/crates/characters/src/bootstrap/issue.rs:14` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/characters/src/bootstrap/issue.rs:15` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/characters/src/bootstrap/issue.rs:23` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/characters/src/bootstrap/issue.rs:143` | FOR SHARE; query tables accounts; sqlx::query("SELECT id FROM accounts WHERE id=ANY($1) ORDER BY id FOR SHARE") | 3 | no | n.a. | yes |
| `portal-rs/crates/characters/src/bootstrap/issue.rs:151` | FOR SHARE; query tables accounts; sqlx::query_as("SELECT id, status FROM accounts WHERE id=$1 FOR SHARE") | 3 | no | n.a. | yes |
| `portal-rs/crates/characters/src/bootstrap/issue.rs:163` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/characters/src/bootstrap/issue.rs:165` | pg_advisory_xact_lock_shared; sqlx::query("SELECT pg_advisory_xact_lock_shared(71010)") | 5 | no | n.a. | yes |
| `portal-rs/crates/characters/src/bootstrap/issue.rs:175` | FOR UPDATE; query tables character_bootstrap_intent_authority; "SELECT last_source_revision, last_issuer_decision_id FROM character_bootstrap_intent_authority WHERE id=1 FOR UPDATE", | 7 / domain namespace | no | n.a. | yes |
| `portal-rs/crates/characters/src/bootstrap/issue.rs:181` | FOR SHARE OF a; query tables account_roles, accounts, role_permissions; let allowed: Option&lt;bool&gt; = sqlx::query_scalar("SELECT a.status='active' AND EXISTS(SELECT 1 FROM account_roles ar JOIN role_permissions rp  | 3 | no | n.a. | yes |
| `portal-rs/crates/characters/src/bootstrap/issue.rs:183` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/characters/src/bootstrap/issue.rs:188` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/characters/src/bootstrap/issue_tests.rs:14` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/characters/src/bootstrap/issue_tests.rs:18` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/characters/src/bootstrap/issue_tests.rs:19` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/characters/src/bootstrap/issue_tests.rs:63` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/characters/src/bootstrap/issue_tests.rs:64` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/characters/src/bootstrap/issue_tests.rs:81` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/characters/src/bootstrap/issue_tests.rs:84` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/characters/src/bootstrap/issue_tests.rs:86` | implicit SQL write locks; "INSERT INTO role_permissions VALUES ($1::uuid, $2), ($3::uuid, 'game.player.read')", | 7 (effects; owning entry above) | n.a. | n.a. | yes |
| `portal-rs/crates/characters/src/bootstrap/issue_tests.rs:89` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/characters/src/bootstrap/issue_tests.rs:93` | implicit SQL write locks; sqlx::query("INSERT INTO account_roles VALUES ($1::uuid, $3::uuid), ($2::uuid, $3::uuid), ($4::uuid, $5::uuid)") | 7 (effects; owning entry above) | n.a. | n.a. | yes |
| `portal-rs/crates/characters/src/bootstrap/issue_tests.rs:94` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/characters/src/bootstrap/issue_tests.rs:233` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/characters/src/bootstrap/issue_tests.rs:256` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/characters/src/bootstrap/issue_tests.rs:273` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/characters/src/bootstrap/issue_tests.rs:280` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/characters/src/bootstrap/issue_tests.rs:285` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/characters/src/bootstrap/issue_tests.rs:302` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/characters/src/bootstrap/issue_tests.rs:309` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/characters/src/bootstrap/issue_tests.rs:315` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/characters/src/bootstrap/issue_tests.rs:327` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/characters/src/bootstrap/issue_tests.rs:334` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/characters/src/bootstrap/issue_tests.rs:345` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/characters/src/bootstrap/issue_tests.rs:353` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/characters/src/bootstrap/issue_tests.rs:368` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/characters/src/bootstrap/issue_tests.rs:382` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/characters/src/bootstrap/issue_tests.rs:402` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/characters/src/bootstrap/issue_tests.rs:405` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/characters/src/bootstrap/issue_tests.rs:412` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/characters/src/bootstrap/issue_tests.rs:420` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/characters/src/bootstrap/issue_tests.rs:425` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/characters/src/bootstrap/issue_tests.rs:436` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/characters/src/bootstrap/issue_tests.rs:437` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/characters/src/bootstrap/issue_tests.rs:457` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/characters/src/bootstrap/issue_tests.rs:464` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/characters/src/bootstrap/issue_tests.rs:489` | test fixture/blocker: FOR UPDATE; query tables character_bootstrap_intent_authority; sqlx::query("SELECT id FROM character_bootstrap_intent_authority WHERE id=1 FOR UPDATE") | 7 / domain namespace | n.a. | n.a. | yes |
| `portal-rs/crates/characters/src/bootstrap/issue_tests.rs:496` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/characters/src/bootstrap/mod.rs:51` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/characters/src/bootstrap/tests.rs:115` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/characters/src/commands/README.md:4` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/characters/src/commands/README.md:26` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/characters/src/commands/http.rs:109` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/characters/src/commands/mod.rs:3` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/characters/src/commands/mod.rs:151` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/characters/src/commands/mod.rs:152` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/characters/src/commands/mod.rs:186` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/characters/src/commands/mod.rs:200` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/characters/src/commands/mod.rs:210` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/characters/src/commands/services.rs:68` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/characters/src/commands/services.rs:70` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/characters/src/commands/services.rs:82` | pg_advisory_xact_lock; sqlx::query("SELECT pg_advisory_xact_lock(22005160000)") | domain namespace (README inventory) | no | n.a. | yes |
| `portal-rs/crates/characters/src/commands/storage.rs:14` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/characters/src/commands/storage.rs:51` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/characters/src/commands/storage.rs:55` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/characters/src/commands/storage.rs:65` | pg_advisory_xact_lock; sqlx::query("SELECT pg_advisory_xact_lock(22005160000)") | domain namespace (README inventory) | no | n.a. | fixed in parallel task drc-a |
| `portal-rs/crates/characters/src/commands/storage.rs:131` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/characters/src/commands/storage.rs:226` | FOR SHARE; query tables character_restore_locks; sqlx::query_scalar("SELECT id FROM character_restore_locks WHERE id=1 FOR SHARE") | 4 / owning domain resource | no | n.a. | fixed in parallel task drc-a |
| `portal-rs/crates/characters/src/commands/storage.rs:233` | FOR UPDATE; query tables character_commands; let row=sqlx::query("SELECT command,binding_json,receipt_json FROM character_commands WHERE id=$1 FOR UPDATE").bind(id).fetch_optional(&mut  | 4 / owning domain resource | no | n.a. | fixed in parallel task drc-a |
| `portal-rs/crates/characters/src/commands/tests.rs:239` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/characters/src/commands/tests.rs:268` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/characters/src/create_web/README.md:32` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/characters/src/create_web/en.ftl:11` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/characters/src/create_web/en.ftl:29` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/characters/src/create_web/names.rs:34` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/characters/src/create_web/pl.ftl:29` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/characters/src/create_web/service.rs:75` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/characters/src/create_web/service.rs:102` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/characters/src/create_web/service.rs:104` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/characters/src/create_web/service.rs:108` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/characters/src/create_web/service.rs:120` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/characters/src/create_web/service.rs:122` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/characters/src/create_web/service.rs:141` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/characters/src/create_web/service.rs:143` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/characters/src/create_web/service.rs:177` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/characters/src/create_web/service.rs:179` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/characters/src/create_web/service.rs:199` | FOR NO KEY UPDATE; query tables accounts; sqlx::query_scalar("SELECT id FROM accounts WHERE id=$1 FOR NO KEY UPDATE") | 3 | no | n.a. | fixed in parallel task drc-a |
| `portal-rs/crates/characters/src/create_web/service.rs:206` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/characters/src/create_web/service.rs:215` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/characters/src/create_web/service.rs:220` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/characters/src/create_web/service.rs:227` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/characters/src/create_web/service.rs:234` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/characters/src/create_web/service.rs:239` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/characters/src/create_web/tests.rs:63` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/characters/src/create_web/tests.rs:582` | test fixture/blocker: FOR UPDATE; query tables accounts; sqlx::query("SELECT id FROM accounts WHERE id=ANY($1) FOR UPDATE") | 3 | n.a. | n.a. | yes |
| `portal-rs/crates/characters/src/lcfa/http.rs:161` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/characters/src/lcfa/http.rs:234` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/characters/src/lcfa/mod.rs:2` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/characters/src/lcfa/mod.rs:18` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/characters/src/lcfa/store.rs:19` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/characters/src/lcfa/store.rs:76` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/characters/src/lcfa/store.rs:91` | FOR UPDATE; query tables account_character_snapshots; let current = sqlx::query("SELECT projection_epoch, projection_revision, content_digest, invalid FROM account_character_snapshots WHERE acco | 4 / owning domain resource | no | n.a. | yes |
| `portal-rs/crates/characters/src/lcfa/store.rs:165` | FOR SHARE; query tables account_character_feed; let Some(feed) = sqlx::query("SELECT highest_epoch, watermark_epoch, source_authority, complete_through FROM account_character_feed WHERE id | 4 / owning domain resource | no | n.a. | yes |
| `portal-rs/crates/characters/src/lcfa/store.rs:220` | FOR UPDATE; query tables account_character_feed; "SELECT highest_epoch FROM account_character_feed WHERE id = 1 FOR UPDATE", | 4 / owning domain resource | no | n.a. | yes |
| `portal-rs/crates/characters/src/lcfa/tests.rs:309` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/characters/src/lcfa/tests.rs:333` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/characters/src/lcfa/tests.rs:399` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/characters/src/lcfa/tests.rs:620` | test fixture/blocker: FOR UPDATE; query tables account_character_feed; sqlx::query("SELECT id FROM account_character_feed WHERE id=1 FOR UPDATE") | 4 / owning domain resource | n.a. | n.a. | yes |
| `portal-rs/crates/characters/src/overview/README.md:16` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/characters/src/overview/wallet.rs:5` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/characters/src/overview/wallet.rs:11` | FOR SHARE; query tables accounts; match sqlx::query_scalar::&lt;_, String&gt;("SELECT status FROM accounts WHERE id=$1 FOR SHARE") | 3 | no | n.a. | yes |
| `portal-rs/crates/characters/src/overview/wallet.rs:23` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/characters/src/overview/wallet.rs:24` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/characters/src/profile/README.md:28` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/characters/src/profile/README.md:35` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/characters/src/profile/README.md:37` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/characters/src/profile/README.md:45` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/characters/src/profile/mod.rs:2` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/characters/src/profile/mod.rs:97` | FOR SHARE; query tables account_character_feed; sqlx::query("SELECT id FROM account_character_feed WHERE id=1 FOR SHARE") | 4 / owning domain resource | no | n.a. | yes |
| `portal-rs/crates/characters/src/profile/mod.rs:222` | FOR SHARE; query tables accounts; sqlx::query("SELECT id FROM accounts WHERE id=$1 AND status='active' FOR SHARE") | 3 | no | n.a. | yes |
| `portal-rs/crates/characters/src/profile/mod.rs:271` | FOR UPDATE; query tables accounts; sqlx::query_scalar::&lt;_, String&gt;("SELECT status FROM accounts WHERE id=$1 FOR UPDATE") | 3 | no | n.a. | yes |
| `portal-rs/crates/characters/src/profile/mod.rs:304` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/characters/src/profile/mod.rs:339` | FOR SHARE; query tables accounts; sqlx::query("SELECT id FROM accounts WHERE id=$1 AND status='active' FOR SHARE") | 3 | no | n.a. | yes |
| `portal-rs/crates/characters/src/profile/mod.rs:365` | FOR SHARE; query tables accounts; sqlx::query("SELECT id FROM accounts WHERE id=$1 AND status='active' FOR SHARE") | 3 | no | n.a. | yes |
| `portal-rs/crates/characters/src/restore/README.md:27` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/characters/src/restore/http.rs:117` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/characters/src/restore/http.rs:188` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/characters/src/restore/mod.rs:45` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/characters/src/restore/mod.rs:46` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/characters/src/restore/mod.rs:82` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/characters/src/restore/mod.rs:86` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/characters/src/restore/storage.rs:7` | FOR UPDATE; query tables character_restore_locks; "SELECT id FROM character_restore_locks WHERE id=1 FOR UPDATE" | 4 / owning domain resource | no | n.a. | yes |
| `portal-rs/crates/characters/src/restore/storage.rs:9` | FOR SHARE; query tables character_restore_locks; "SELECT id FROM character_restore_locks WHERE id=1 FOR SHARE" | 4 / owning domain resource | no | n.a. | yes |
| `portal-rs/crates/characters/src/restore/storage.rs:82` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/characters/src/restore/tests.rs:185` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/characters/src/restore/tests.rs:213` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/characters/src/services/README.md:12` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/characters/src/services/browser.rs:70` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/characters/src/services/mod.rs:79` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/characters/src/services/mod.rs:87` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/characters/src/services/mod.rs:95` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/characters/src/services/mod.rs:96` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/characters/src/services/mod.rs:145` | pg_advisory_xact_lock; sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1,47))") | domain namespace (README inventory) | yes | n.a. | fixed in this commit |
| `portal-rs/crates/characters/src/services/mod.rs:150` | pg_advisory_xact_lock; sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1,46))") | domain namespace (README inventory) | yes | n.a. | fixed in this commit |
| `portal-rs/crates/characters/src/services/mod.rs:163` | pg_advisory_xact_lock; sqlx::query("SELECT pg_advisory_xact_lock(22005160000)") | domain namespace (README inventory) | yes | n.a. | fixed in this commit |
| `portal-rs/crates/characters/src/services/mod.rs:187` | FOR UPDATE; query tables character_service_orders; credit=sqlx::query_scalar::&lt;_,Uuid&gt;("SELECT operation_id FROM character_service_orders WHERE account_id=$1 AND character_id=$2 AND financial | 4 / owning domain resource | yes | n.a. | fixed in this commit |
| `portal-rs/crates/characters/src/services/mod.rs:203` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/characters/src/services/mod.rs:220` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/characters/src/services/mod.rs:256` | FOR SHARE; query tables accounts; sqlx::query("SELECT id FROM accounts WHERE id=$1 FOR SHARE") | 3 | yes | n.a. | fixed in this commit |
| `portal-rs/crates/characters/src/services/mod.rs:260` | pg_advisory_xact_lock; sqlx::query("SELECT pg_advisory_xact_lock(22005160000)") | domain namespace (README inventory) | yes | n.a. | fixed in this commit |
| `portal-rs/crates/characters/src/services/mod.rs:263` | FOR UPDATE OF c,o; query tables character_commands, character_service_orders; let row=sqlx::query("SELECT c.intent_json,c.receipt_json,o.account_id,o.character_id,o.reservation_id,o.settlement_id,o.financial_state,o.cr | 4 / owning domain resource | yes | n.a. | fixed in this commit |
| `portal-rs/crates/characters/src/services/mod.rs:292` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/characters/src/services/mod.rs:322` | FOR UPDATE; query tables character_service_orders; let parent = sqlx::query("SELECT account_id,character_id,financial_state,credit_used_by FROM character_service_orders WHERE operation_id=$1  | 4 / owning domain resource | yes | n.a. | fixed in this commit |
| `portal-rs/crates/characters/src/services/mod.rs:355` | FOR SHARE; query tables character_restore_locks; if sqlx::query("SELECT id FROM character_restore_locks WHERE id=1 FOR SHARE") | 4 / owning domain resource | no | n.a. | yes |
| `portal-rs/crates/characters/src/services/mod.rs:379` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/characters/src/services/projection.rs:139` | FOR UPDATE; query tables account_character_feed; "SELECT highest_epoch FROM account_character_feed WHERE id=1 FOR UPDATE", | 4 / owning domain resource | no | n.a. | yes |
| `portal-rs/crates/characters/src/services/projection.rs:151` | FOR UPDATE; query tables character_service_feed; let row=sqlx::query("SELECT epoch::text,complete_through,observed_at FROM character_service_feed WHERE id=1 FOR UPDATE").fetch_one(&mut *tx) | 4 / owning domain resource | no | n.a. | yes |
| `portal-rs/crates/characters/src/services/projection.rs:170` | FOR UPDATE; query tables character_service_states; if let Some(row)=sqlx::query("SELECT epoch::text,revision::text,snapshot_json,invalid FROM character_service_states WHERE account_id=$1 AND  | 4 / owning domain resource | no | n.a. | yes |
| `portal-rs/crates/characters/src/services/projection.rs:197` | FOR SHARE OF f,w,s; query tables account_character_feed, character_service_feed, character_service_states; let row=sqlx::query("SELECT f.highest_epoch,s.epoch::text,s.snapshot_json,s.invalid AS state_invalid,w.epoch::text AS watermark_epoch,w.comp | 4 / owning domain resource | no | n.a. | yes |
| `portal-rs/crates/characters/src/services/tests.rs:245` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/characters/src/services/tests.rs:270` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/characters/src/services/tests.rs:305` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/characters/src/services/tests.rs:316` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/characters/src/services/tests.rs:330` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/characters/src/services/tests.rs:338` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/characters/src/services/tests.rs:645` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/characters/src/services/tests.rs:669` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/characters/src/services/tests.rs:1345` | test fixture/blocker: FOR UPDATE; query tables accounts; sqlx::query("SELECT id FROM accounts WHERE id=$1 FOR UPDATE") | 3 | n.a. | n.a. | yes |
| `portal-rs/crates/characters/src/services/tests.rs:1351` | test fixture/blocker: pg_advisory_xact_lock; sqlx::query("SELECT pg_advisory_xact_lock(22005160000)") | domain namespace (README inventory) | n.a. | n.a. | yes |
| `portal-rs/crates/characters/src/services/tests.rs:1369` | test fixture/blocker: FOR UPDATE; query tables character_commands; let command_lock = sqlx::query("SELECT id FROM character_commands WHERE id=$1 FOR UPDATE") | 4 / owning domain resource | n.a. | n.a. | yes |
| `portal-rs/crates/characters/src/services/tests.rs:1447` | test fixture/blocker: pg_advisory_xact_lock; sqlx::query("SELECT pg_advisory_xact_lock(22005160000)") | domain namespace (README inventory) | n.a. | n.a. | yes |
| `portal-rs/crates/characters/src/services/tests.rs:1467` | test fixture/blocker: FOR UPDATE; query tables character_service_orders; "SELECT operation_id FROM character_service_orders WHERE operation_id=$1 FOR UPDATE", | 4 / owning domain resource | n.a. | n.a. | yes |
| `portal-rs/crates/characters/src/services/transport.rs:136` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/characters/src/services/transport.rs:250` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/community/src/deaths.rs:166` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/community/src/deaths.rs:200` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/community/src/deaths/README.md:32` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/community/src/deaths/README.md:34` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/community/src/deaths/storage.rs:25` | FOR UPDATE; query tables public_projection_state; "SELECT highest_epoch::text FROM public_projection_state WHERE id=1 FOR UPDATE", | 4 / owning domain resource | no | n.a. | yes |
| `portal-rs/crates/community/src/deaths/tests.rs:178` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/community/src/deaths/web.rs:185` | FOR SHARE; query tables public_projection_state; "SELECT highest_epoch::text FROM public_projection_state WHERE id=1 FOR SHARE", | 4 / owning domain resource | no | n.a. | yes |
| `portal-rs/crates/community/src/deaths/web.rs:374` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/community/src/families.rs:222` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/community/src/families.rs:272` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/community/src/families/storage.rs:22` | FOR UPDATE; query tables public_projection_state; "SELECT highest_epoch::text FROM public_projection_state WHERE id=1 FOR UPDATE", | 4 / owning domain resource | no | n.a. | yes |
| `portal-rs/crates/community/src/families/storage.rs:51` | FOR UPDATE; query tables public_character_families_v2; let old = sqlx::query("SELECT epoch::text AS epoch,revision::text AS revision,body FROM public_character_families_v2 WHERE family=$1 AND cha | 4 / owning domain resource | no | n.a. | yes |
| `portal-rs/crates/community/src/families/storage.rs:113` | FOR UPDATE; query tables public_character_families_v2_watermarks; let old = sqlx::query("SELECT epoch::text AS epoch,complete_through FROM public_character_families_v2_watermarks WHERE family=$1 FOR UPDATE" | 4 / owning domain resource | no | n.a. | yes |
| `portal-rs/crates/community/src/families/tests.rs:288` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/community/src/families/tests.rs:535` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/community/src/families/tests.rs:536` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/community/src/families/tests.rs:539` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/community/src/families/tests.rs:540` | implicit SQL write locks; db.exec("INSERT INTO roles(key) VALUES('g4_staff')", &[]) | 7 (effects; owning entry above) | n.a. | n.a. | yes |
| `portal-rs/crates/community/src/families/tests.rs:542` | implicit SQL write locks; db.exec("INSERT INTO role_permissions(role_id,permission) SELECT id,p FROM roles, unnest(ARRAY['admin.access','support.tickets.manage','supp | 7 (effects; owning entry above) | n.a. | n.a. | yes |
| `portal-rs/crates/community/src/families/tests.rs:543` | implicit SQL write locks; db.exec("INSERT INTO account_roles(account_id,role_id) SELECT $1::uuid,id FROM roles WHERE key='g4_staff'", &[&staff]).await; | 7 (effects; owning entry above) | n.a. | n.a. | yes |
| `portal-rs/crates/community/src/families/tests.rs:591` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/community/src/families/tests.rs:607` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/community/src/families/tests.rs:952` | test fixture/blocker: LOCK TABLE public_character_families_v2 IN ACCESS EXCLUSIVE MODE; sqlx::query("LOCK TABLE public_character_families_v2 IN ACCESS EXCLUSIVE MODE") | 4 / owning domain resource | n.a. | n.a. | yes |
| `portal-rs/crates/community/src/families/web.rs:4` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/community/src/families/web.rs:5` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/community/src/families/web.rs:317` | FOR SHARE; query tables public_projection_state; sqlx::query("SELECT highest_epoch FROM public_projection_state WHERE id=1 FOR SHARE") | 4 / owning domain resource | no | n.a. | yes |
| `portal-rs/crates/community/src/families/web.rs:431` | FOR SHARE; query tables public_projection_state; sqlx::query("SELECT highest_epoch FROM public_projection_state WHERE id=1 FOR SHARE") | 4 / owning domain resource | no | n.a. | yes |
| `portal-rs/crates/community/src/guilds.rs:106` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/community/src/guilds.rs:143` | FOR UPDATE; query tables public_projection_state; "SELECT highest_epoch::text FROM public_projection_state WHERE id=1 FOR UPDATE", | 4 / owning domain resource | no | n.a. | yes |
| `portal-rs/crates/community/src/guilds_pages.rs:305` | FOR SHARE; query tables public_projection_state; "SELECT highest_epoch::text FROM public_projection_state WHERE id=1 FOR SHARE", | 4 / owning domain resource | no | n.a. | yes |
| `portal-rs/crates/community/src/guilds_tests.rs:57` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/community/src/houses.rs:315` | FOR SHARE; query tables public_projection_state; sqlx::query("SELECT id FROM public_projection_state WHERE id=1 FOR SHARE") | 4 / owning domain resource | no | n.a. | yes |
| `portal-rs/crates/community/src/houses.rs:422` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/community/src/houses_ingest.rs:32` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/community/src/houses_ingest.rs:71` | FOR UPDATE; query tables public_projection_state; "SELECT highest_epoch::text FROM public_projection_state WHERE id=1 FOR UPDATE", | 4 / owning domain resource | no | n.a. | yes |
| `portal-rs/crates/community/src/houses_tests.rs:136` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/community/src/ingest.rs:175` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/community/src/ingest.rs:215` | FOR UPDATE; query tables public_projection_state; "SELECT highest_epoch::text FROM public_projection_state WHERE id=1 FOR UPDATE", | 4 / owning domain resource | no | n.a. | yes |
| `portal-rs/crates/community/src/ingest.rs:272` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/community/src/ingest/ingest_tests.rs:55` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/community/src/ingest_budget.rs:44` | FOR UPDATE; query tables community_ingest_budgets; "SELECT 1 FROM community_ingest_budgets WHERE publisher=$1 AND route=$2 FOR UPDATE", | 4 / owning domain resource | no | n.a. | yes |
| `portal-rs/crates/community/src/online.rs:174` | FOR SHARE; query tables public_projection_state; sqlx::query("SELECT id FROM public_projection_state WHERE id=1 FOR SHARE") | 4 / owning domain resource | no | n.a. | yes |
| `portal-rs/crates/community/src/online.rs:355` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/community/src/party.rs:3` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/community/src/party.rs:35` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/community/src/party.rs:76` | FOR SHARE; query tables public_projection_state; sqlx::query("SELECT highest_epoch FROM public_projection_state WHERE id=1 FOR SHARE") | 4 / owning domain resource | no | n.a. | yes |
| `portal-rs/crates/community/src/party.rs:108` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/community/src/party.rs:111` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/community/src/party.rs:114` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/community/src/party.rs:115` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/community/src/party.rs:116` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/community/src/party.rs:118` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/community/src/party.rs:144` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/community/src/party.rs:145` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/community/src/party.rs:147` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/community/src/party.rs:149` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/community/src/party.rs:153` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/community/src/party.rs:162` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/community/src/party.rs:167` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/community/src/party.rs:170` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/community/src/party/party_store.rs:62` | pg_advisory_xact_lock; sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1,83))") | domain namespace (README inventory) | no | n.a. | yes |
| `portal-rs/crates/community/src/party/party_store.rs:90` | pg_advisory_xact_lock; sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1,83))") | domain namespace (README inventory) | no | n.a. | yes |
| `portal-rs/crates/community/src/party/party_store.rs:94` | FOR UPDATE; query tables pd_party_posts; let owner: Option&lt;Uuid&gt; = sqlx::query_scalar("SELECT account_id FROM pd_party_posts WHERE id=$1 AND status='active' AND expires_at&gt;now() FOR | 4 / owning domain resource | no | n.a. | yes |
| `portal-rs/crates/community/src/party/party_store.rs:156` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/community/src/party/party_store.rs:164` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/community/src/party/party_store.rs:169` | FOR UPDATE; query tables pd_party_posts; sqlx::query("SELECT id FROM pd_party_posts WHERE id=$1 FOR UPDATE") | 4 / owning domain resource | no | n.a. | yes |
| `portal-rs/crates/community/src/party/party_store.rs:179` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/community/src/party/party_store.rs:180` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/community/src/party/party_tests.rs:15` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/community/src/party/party_tests.rs:24` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/community/src/party/party_tests.rs:36` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/community/src/party/party_tests.rs:81` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/community/src/party/party_tests.rs:107` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/community/src/party/party_tests.rs:108` | implicit SQL write locks; sqlx::query("INSERT INTO account_roles(account_id,role_id) SELECT $1,id FROM roles WHERE key='support' ON CONFLICT DO NOTHING") | 7 (effects; owning entry above) | n.a. | n.a. | yes |
| `portal-rs/crates/community/src/party/party_tests.rs:109` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/community/src/party/party_tests.rs:111` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/community/src/party/party_tests.rs:147` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/community/src/party/party_tests.rs:148` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/community/src/party/party_tests.rs:150` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/community/src/party/party_tests.rs:157` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/community/src/party/party_tests.rs:159` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/community/src/party/party_tests.rs:161` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/community/src/party/party_tests.rs:323` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/community/src/party/party_tests.rs:326` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/community/src/party/party_tests.rs:331` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/community/src/party/party_tests.rs:440` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/community/src/party/party_tests.rs:443` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/community/src/party/party_tests.rs:447` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/community/src/party/party_tests.rs:451` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/community/src/party/party_tests.rs:457` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/community/src/party/party_tests.rs:459` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/community/src/party/party_tests.rs:465` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/community/src/party/party_tests.rs:467` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/community/src/party/party_tests.rs:479` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/community/src/party/party_tests.rs:483` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/community/src/party/party_tests.rs:493` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/community/src/party/party_tests.rs:496` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/community/src/party/party_tests.rs:498` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/community/src/party/party_tests.rs:525` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/community/src/party/party_tests.rs:532` | implicit SQL write locks; "INSERT INTO account_roles(account_id,role_id) SELECT $1,id FROM roles WHERE key='support'", | 7 (effects; owning entry above) | n.a. | n.a. | yes |
| `portal-rs/crates/community/src/party/party_tests.rs:534` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/community/src/party/party_tests.rs:541` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/community/src/party/party_tests.rs:546` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/community/src/party/party_tests.rs:554` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/community/src/party/party_tests.rs:569` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/community/src/party/party_tests.rs:582` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/community/src/party/party_tests.rs:600` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/community/src/party/party_tests.rs:612` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/community/src/party/party_tests.rs:617` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/community/src/party/party_tests.rs:620` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/community/src/party/party_tests.rs:689` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/community/src/party/party_tests.rs:691` | implicit SQL write locks; "INSERT INTO account_roles(account_id,role_id) SELECT $1,id FROM roles WHERE key='support'", | 7 (effects; owning entry above) | n.a. | n.a. | yes |
| `portal-rs/crates/community/src/party/party_tests.rs:693` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/community/src/party/party_tests.rs:697` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/community/src/party/party_tests.rs:702` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/community/src/party/party_tests.rs:703` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/community/src/party/party_tests.rs:708` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/community/src/party/party_tests.rs:716` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/community/src/party/party_tests.rs:738` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/community/src/party/party_tests.rs:747` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/community/src/party/party_tests.rs:749` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/community/src/party/party_tests.rs:750` | implicit SQL write locks; sqlx::query("INSERT INTO account_roles(account_id,role_id) SELECT $1,id FROM roles WHERE key='support'") | 7 (effects; owning entry above) | n.a. | n.a. | yes |
| `portal-rs/crates/community/src/party/party_tests.rs:751` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/community/src/party/party_tests.rs:761` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/community/src/party/party_tests.rs:762` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/community/src/party/party_tests.rs:767` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/community/src/party/party_tests.rs:777` | test fixture/blocker: FOR UPDATE; query tables pd_party_posts; sqlx::query("SELECT id FROM pd_party_posts WHERE id=$1 FOR UPDATE") | 4 / owning domain resource | n.a. | n.a. | yes |
| `portal-rs/crates/community/src/party/party_tests.rs:783` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/community/src/party/party_tests.rs:784` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/community/src/party/party_tests.rs:806` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/community/src/party/party_tests.rs:813` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/community/src/party/party_tests.rs:818` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/community/src/party/party_tests.rs:819` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/community/src/party/party_tests.rs:823` | implicit SQL write locks; sqlx::query("INSERT INTO account_roles(account_id,role_id) SELECT $1,id FROM roles WHERE key='platform_admin'") | 7 (effects; owning entry above) | n.a. | n.a. | yes |
| `portal-rs/crates/community/src/party/party_tests.rs:831` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/community/src/party/party_tests.rs:835` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/community/src/party/party_tests.rs:837` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/community/src/party/party_tests.rs:889` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/community/src/party/party_tests.rs:890` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/community/src/privacy.md:18` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/community/src/privacy.md:31` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/community/src/privacy.md:65` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/community/src/profiles.md:33` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/community/src/profiles.rs:1` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/community/src/profiles.rs:148` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/community/src/profiles.rs:240` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/community/src/referrals.rs:6` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/community/src/referrals.rs:173` | FOR KEY SHARE; query tables accounts; sqlx::query("SELECT id FROM accounts WHERE id=ANY($1) ORDER BY id FOR KEY SHARE") | 3 | no | n.a. | yes |
| `portal-rs/crates/community/src/referrals.rs:183` | pg_advisory_xact_lock; "SELECT pg_advisory_xact_lock(5600) FROM (SELECT $1::uuid[]) ids" | domain namespace (README inventory) | no | n.a. | yes |
| `portal-rs/crates/community/src/referrals.rs:254` | FOR UPDATE; query tables referral_invitations; "SELECT * FROM referral_invitations WHERE id=$1 AND invitee_id=$2 FOR UPDATE", | 4 / owning domain resource | no | n.a. | yes |
| `portal-rs/crates/community/src/referrals.rs:291` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/community/src/referrals.rs:340` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/community/src/referrals/campaign.rs:28` | FOR SHARE; query tables accounts; sqlx::query("SELECT id FROM accounts WHERE id=ANY($1) ORDER BY id FOR SHARE") | 3 | yes | n.a. | fixed in this commit |
| `portal-rs/crates/community/src/referrals/campaign.rs:52` | FOR UPDATE; query tables referral_activity; return Ok(sqlx::query_scalar("SELECT last_activity&lt;=clock_timestamp()-interval '12 months' AND created_at&lt;=clock_timestamp()-interval '12 mo | 4 / owning domain resource | yes | n.a. | fixed in this commit |
| `portal-rs/crates/community/src/referrals/campaign.rs:75` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/community/src/referrals/campaign.rs:414` | FOR UPDATE SKIP LOCKED; query tables referral_campaign_rewards; let locked: Option&lt;Uuid&gt;=sqlx::query_scalar("SELECT id FROM referral_campaign_rewards WHERE id=$1 AND applied_at IS NULL FOR UPDATE SKIP LOC | 4 / owning domain resource | no | n.a. | yes |
| `portal-rs/crates/community/src/referrals/campaign.rs:419` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/community/src/referrals/campaign.rs:431` | pg_advisory_xact_lock; sqlx::query("SELECT pg_advisory_xact_lock(5601)") | domain namespace (README inventory) | no | n.a. | yes |
| `portal-rs/crates/community/src/referrals/campaign.rs:434` | FOR UPDATE OF a SKIP LOCKED; query tables accounts, referral_email_attempts, referral_email_invitations; let row=sqlx::query("SELECT a.id,i.id AS code,i.recipient,owner.locale FROM referral_email_attempts a JOIN referral_email_invitations i ON i | 3 | no | n.a. | yes |
| `portal-rs/crates/community/src/referrals/tests.rs:42` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/community/src/referrals/tests.rs:43` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/community/src/referrals/tests.rs:1463` | test fixture/blocker: FOR KEY SHARE NOWAIT; query tables accounts; let error = sqlx::query("SELECT id FROM accounts WHERE id=$1 FOR KEY SHARE NOWAIT") | 3 | n.a. | n.a. | yes |
| `portal-rs/crates/community/src/referrals/tests.rs:1476` | test fixture/blocker: pg_try_advisory_xact_lock; let available: bool = sqlx::query_scalar("SELECT pg_try_advisory_xact_lock(5600)") | domain namespace (README inventory) | n.a. | n.a. | yes |
| `portal-rs/crates/community/src/referrals/tests.rs:1574` | test fixture/blocker: FOR UPDATE; query tables accounts; sqlx::query("SELECT id FROM accounts WHERE id=$1 FOR UPDATE") | 3 | n.a. | n.a. | yes |
| `portal-rs/crates/community/src/referrals/tests.rs:1692` | test fixture/blocker: FOR UPDATE; query tables premium_authority; sqlx::query("SELECT account_id FROM premium_authority WHERE account_id=$1 FOR UPDATE") | 7 / domain namespace | n.a. | n.a. | yes |
| `portal-rs/crates/community/src/referrals/tests.rs:1725` | test fixture/blocker: FOR UPDATE; query tables accounts; sqlx::query("SELECT id FROM accounts WHERE id=$1 FOR UPDATE") | 3 | n.a. | n.a. | yes |
| `portal-rs/crates/community/src/referrals/tests.rs:1762` | test fixture/blocker: FOR UPDATE; query tables accounts; sqlx::query("SELECT id FROM accounts WHERE id=$1 FOR UPDATE") | 3 | n.a. | n.a. | yes |
| `portal-rs/crates/community/src/referrals/tests.rs:1820` | test fixture/blocker: FOR UPDATE; query tables accounts; sqlx::query("SELECT id FROM accounts WHERE id=$1 FOR UPDATE") | 3 | n.a. | n.a. | yes |
| `portal-rs/crates/community/src/referrals/tests.rs:2249` | test fixture/blocker: FOR KEY SHARE NOWAIT; query tables accounts; "SELECT id FROM accounts WHERE id=$1 FOR KEY SHARE NOWAIT", | 3 | n.a. | n.a. | yes |
| `portal-rs/crates/community/src/referrals/tests.rs:2250` | test fixture/blocker: FOR KEY SHARE NOWAIT; query tables premium_authority; "SELECT account_id FROM premium_authority WHERE account_id=$1 FOR KEY SHARE NOWAIT", | 7 / domain namespace | n.a. | n.a. | yes |
| `portal-rs/crates/community/src/referrals/tests.rs:2288` | test fixture/blocker: FOR UPDATE; query tables accounts; sqlx::query("SELECT id FROM accounts WHERE id=$1 FOR UPDATE") | 3 | n.a. | n.a. | yes |
| `portal-rs/crates/community/src/referrals/tests.rs:2312` | test fixture/blocker: FOR UPDATE NOWAIT; query tables accounts; sqlx::query("SELECT id FROM accounts WHERE id=$1 FOR UPDATE NOWAIT") | 3 | n.a. | n.a. | yes |
| `portal-rs/crates/community/src/referrals/tests.rs:2349` | test fixture/blocker: FOR UPDATE; query tables accounts; sqlx::query("SELECT id FROM accounts WHERE id=$1 FOR UPDATE") | 3 | n.a. | n.a. | yes |
| `portal-rs/crates/community/src/referrals/tests.rs:2371` | test fixture/blocker: FOR UPDATE NOWAIT; query tables accounts; sqlx::query("SELECT id FROM accounts WHERE id=$1 FOR UPDATE NOWAIT") | 3 | n.a. | n.a. | yes |
| `portal-rs/crates/community/src/referrals/tests.rs:2499` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/community/src/referrals/tests.rs:2543` | test fixture/blocker: FOR UPDATE; query tables accounts; sqlx::query("SELECT id FROM accounts WHERE id=$1 FOR UPDATE") | 3 | n.a. | n.a. | yes |
| `portal-rs/crates/community/src/referrals/tests.rs:2575` | test fixture/blocker: FOR UPDATE; query tables accounts; let later_lock = sqlx::query("SELECT id FROM accounts WHERE id=$1 FOR UPDATE") | 3 | n.a. | n.a. | yes |
| `portal-rs/crates/community/src/referrals_tests.rs:3` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/community/src/referrals_tests.rs:42` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/community/src/referrals_tests.rs:43` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/community/src/today.rs:60` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/community/src/today/README.md:52` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/community/src/today/http.rs:100` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/community/src/today/http.rs:132` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/community/src/today/http.rs:366` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/community/src/today/store.rs:18` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/community/src/today/store.rs:23` | FOR UPDATE; query tables public_projection_state; "SELECT highest_epoch::text FROM public_projection_state WHERE id=1 FOR UPDATE", | 4 / owning domain resource | no | n.a. | yes |
| `portal-rs/crates/community/src/today/store.rs:37` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/community/src/today/view.rs:171` | FOR SHARE; query tables public_projection_state; sqlx::query("SELECT highest_epoch FROM public_projection_state WHERE id=1 FOR SHARE") | 4 / owning domain resource | no | n.a. | yes |
| `portal-rs/crates/community/src/today/view.rs:174` | FOR SHARE; query tables today_snapshots; sqlx::query("SELECT 1 FROM today_snapshots WHERE authority=$1 AND world_id=ANY($2) FOR SHARE") | 4 / owning domain resource | no | n.a. | yes |
| `portal-rs/crates/community/src/v2_watermark.rs:107` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/community/src/v2_watermark.rs:270` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/community/src/worlds.rs:74` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/community/tests/today_contract.rs:198` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/community/tests/today_contract.rs:209` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/community/tests/today_contract.rs:457` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/community/tests/today_contract.rs:746` | test fixture/blocker: LOCK TABLE world_admin_names IN ACCESS EXCLUSIVE MODE; sqlx::query("LOCK TABLE world_admin_names IN ACCESS EXCLUSIVE MODE") | 4 / owning domain resource | n.a. | n.a. | yes |
| `portal-rs/crates/community/tests/today_contract.rs:795` | test fixture/blocker: LOCK TABLE world_admin_names IN ACCESS EXCLUSIVE MODE; sqlx::query("LOCK TABLE world_admin_names IN ACCESS EXCLUSIVE MODE") | 4 / owning domain resource | n.a. | n.a. | yes |
| `portal-rs/crates/community/tests/worlds.rs:323` | test fixture/blocker: LOCK TABLE public_snapshots IN ACCESS EXCLUSIVE MODE; sqlx::query("LOCK TABLE public_snapshots IN ACCESS EXCLUSIVE MODE") | 4 / owning domain resource | n.a. | n.a. | yes |
| `portal-rs/crates/community/tests/worlds.rs:342` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/community/tests/worlds.rs:361` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/src/announcements.rs:4` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/src/announcements.rs:113` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/src/announcements.rs:116` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/src/announcements.rs:122` | canonical account/resource/RBAC/durable-proof acquisition + live authority | 3→4→5→6 | yes | yes | yes |
| `portal-rs/crates/content/src/announcements.rs:128` | FOR UPDATE; query tables announcements; sqlx::query("SELECT id FROM announcements WHERE id=$1 FOR UPDATE") | 4 / owning domain resource | yes | yes | yes |
| `portal-rs/crates/content/src/announcements.rs:138` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/src/announcements.rs:141` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/src/announcements.rs:142` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/src/announcements.rs:149` | FOR UPDATE; query tables announcements; "SELECT lock_version,publication_state,translation_published_at IS NOT DISTINCT FROM $2::timestamptz,translation_source_updated_at FROM anno | 4 / owning domain resource | yes | yes | yes |
| `portal-rs/crates/content/src/announcements.rs:266` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/src/announcements.rs:267` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/src/announcements.rs:268` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/src/announcements_staff.rs:1` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/src/announcements_staff.rs:10` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/src/announcements_staff.rs:18` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/src/announcements_staff.rs:24` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/src/announcements_staff.rs:36` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/src/announcements_staff.rs:234` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/src/announcements_tests.rs:13` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/src/announcements_tests.rs:19` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/src/announcements_tests.rs:21` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/src/announcements_tests.rs:104` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/src/announcements_tests.rs:109` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/src/announcements_tests.rs:110` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/src/announcements_tests.rs:118` | implicit SQL write locks; sqlx::query("INSERT INTO account_roles (account_id,role_id) SELECT $1,id FROM roles WHERE key='content_editor'").bind(actor).execute(&pool). | 7 (effects; owning entry above) | n.a. | n.a. | yes |
| `portal-rs/crates/content/src/announcements_tests.rs:121` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/src/announcements_tests.rs:130` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/src/announcements_tests.rs:326` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/src/announcements_tests.rs:327` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/src/announcements_tests.rs:331` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/src/announcements_tests.rs:332` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/src/announcements_tests.rs:339` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/src/announcements_tests.rs:343` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/src/announcements_tests.rs:366` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/src/announcements_tests.rs:372` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/src/announcements_tests.rs:378` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/src/announcements_tests.rs:383` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/src/announcements_tests.rs:399` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/src/announcements_tests.rs:406` | implicit SQL write locks; sqlx::query("INSERT INTO account_roles(account_id,role_id) SELECT $1,id FROM roles WHERE key='content_editor'").bind(actor).execute(&pool).a | 7 (effects; owning entry above) | n.a. | n.a. | yes |
| `portal-rs/crates/content/src/announcements_tests.rs:407` | implicit SQL write locks; sqlx::query("DELETE FROM role_permissions WHERE permission='portal.announcements.manage'") | 7 (effects; owning entry above) | n.a. | n.a. | yes |
| `portal-rs/crates/content/src/announcements_tests.rs:412` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/src/announcements_tests.rs:419` | implicit SQL write locks; sqlx::query("INSERT INTO role_permissions(role_id,permission) SELECT id,'portal.announcements.manage' FROM roles WHERE key='content_editor'" | 7 (effects; owning entry above) | n.a. | n.a. | yes |
| `portal-rs/crates/content/src/announcements_tests.rs:421` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/src/announcements_tests.rs:428` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/src/announcements_tests.rs:455` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/src/announcements_tests.rs:459` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/src/announcements_tests.rs:473` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/src/announcements_tests.rs:533` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/src/announcements_tests.rs:578` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/src/announcements_tests.rs:592` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/src/announcements_tests.rs:599` | implicit SQL write locks; sqlx::query("DELETE FROM account_roles WHERE account_id=$1") | 7 (effects; owning entry above) | n.a. | n.a. | yes |
| `portal-rs/crates/content/src/announcements_tests.rs:605` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/src/announcements_tests.rs:618` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/src/events/repository.rs:4` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/src/events/repository.rs:83` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/src/events/repository.rs:89` | canonical account/resource/RBAC/durable-proof acquisition + live authority | 3→4→5→6 | yes | yes | yes |
| `portal-rs/crates/content/src/events/repository.rs:95` | FOR UPDATE; query tables events; sqlx::query("SELECT id FROM events WHERE id=$1 FOR UPDATE") | 4 / owning domain resource | yes | yes | yes |
| `portal-rs/crates/content/src/events/repository.rs:105` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/src/events/repository.rs:108` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/src/events/repository.rs:109` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/src/events/repository.rs:115` | FOR UPDATE; query tables events; sqlx::query_as("SELECT status,lock_version FROM events WHERE id=$1 FOR UPDATE") | 4 / owning domain resource | yes | yes | yes |
| `portal-rs/crates/content/src/events/repository.rs:162` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/src/events/repository.rs:169` | canonical account/resource/RBAC/durable-proof acquisition + live authority | 3→4→5→6 | yes | yes | yes |
| `portal-rs/crates/content/src/events/repository.rs:174` | FOR UPDATE; query tables events; sqlx::query("SELECT id FROM events WHERE id=$1 FOR UPDATE") | 4 / owning domain resource | yes | yes | yes |
| `portal-rs/crates/content/src/events/repository.rs:184` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/src/events/repository.rs:185` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/src/events/repository.rs:186` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/src/events/repository.rs:190` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/src/events/repository.rs:191` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/src/events/repository.rs:192` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/src/events/repository.rs:195` | FOR UPDATE; query tables events; let current: Event=sqlx::query_as("SELECT id,status,starts_at,ends_at,featured,lock_version FROM events WHERE id=$1 FOR UPDATE").bind(id).fe | 4 / owning domain resource | yes | yes | yes |
| `portal-rs/crates/content/src/events/tests.rs:9` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/src/events/tests.rs:10` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/src/events/tests.rs:22` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/src/events/tests.rs:23` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/src/events/tests.rs:24` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/src/events/tests.rs:32` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/src/events/tests.rs:38` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/src/events/tests.rs:40` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/src/events/tests.rs:51` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/src/events/tests.rs:156` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/src/events/tests.rs:158` | implicit SQL write locks; sqlx::query("INSERT INTO account_roles(account_id,role_id) SELECT $1,id FROM roles WHERE key='platform_admin'").bind(actor).execute(&self.po | 7 (effects; owning entry above) | n.a. | n.a. | yes |
| `portal-rs/crates/content/src/events/tests.rs:166` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/src/events/tests.rs:229` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/src/events/tests.rs:247` | implicit SQL write locks; sqlx::query("INSERT INTO role_permissions(role_id,permission) VALUES ($1,'events.manage')") | 7 (effects; owning entry above) | n.a. | n.a. | yes |
| `portal-rs/crates/content/src/events/tests.rs:252` | implicit SQL write locks; sqlx::query("INSERT INTO account_roles(account_id,role_id) VALUES ($1,$2)") | 7 (effects; owning entry above) | n.a. | n.a. | yes |
| `portal-rs/crates/content/src/events/tests.rs:281` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/src/events/tests.rs:288` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/src/events/tests.rs:293` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/src/events/tests.rs:300` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/src/events/tests.rs:329` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/src/events/tests.rs:354` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/src/events/tests.rs:355` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/src/events/tests.rs:368` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/src/events/tests.rs:369` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/src/events/tests.rs:397` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/src/events/tests.rs:398` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/src/events/tests.rs:412` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/src/events/tests.rs:438` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/src/events/tests.rs:479` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/src/events/tests.rs:488` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/src/events/tests.rs:500` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/src/events/tests.rs:513` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/src/events/tests.rs:514` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/src/events/tests.rs:519` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/src/events/web.rs:1` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/src/events/web.rs:13` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/src/events/web.rs:14` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/src/events/web.rs:22` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/src/events/web.rs:24` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/src/events/web.rs:55` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/src/events/web.rs:163` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/src/events/web.rs:164` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/src/events/web.rs:167` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/src/events/web.rs:173` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/src/events/web.rs:180` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/src/events/web.rs:183` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/src/events/web.rs:184` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/src/events/web.rs:185` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/src/events/web.rs:224` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/src/events/web.rs:327` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/src/events/web.rs:373` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/src/events/web.rs:412` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/src/events/web.rs:420` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/src/events/web.rs:429` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/src/events/web.rs:451` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/src/events/web.rs:458` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/src/events/web.rs:474` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/src/lib.rs:14` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/src/lib.rs:30` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/src/lib.rs:31` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/src/media.rs:14` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/src/media.rs:120` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/src/media.rs:142` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/src/media.rs:339` | canonical account/resource/RBAC/durable-proof acquisition + live authority | 3→4→5→6 | yes | yes | yes |
| `portal-rs/crates/content/src/media.rs:344` | pg_advisory_xact_lock; sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1,12))") | domain namespace (README inventory) | yes | yes | yes |
| `portal-rs/crates/content/src/media.rs:353` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/src/media.rs:360` | pg_advisory_xact_lock; sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1,12))") | domain namespace (README inventory) | yes | yes | yes |
| `portal-rs/crates/content/src/media.rs:365` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/src/media.rs:452` | canonical account/resource/RBAC/durable-proof acquisition + live authority | 3→4→5→6 | yes | yes | yes |
| `portal-rs/crates/content/src/media.rs:457` | FOR UPDATE; query tables media; sqlx::query("SELECT id FROM media WHERE id=$1 FOR UPDATE") | 4 / owning domain resource | yes | yes | yes |
| `portal-rs/crates/content/src/media.rs:466` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/src/media.rs:474` | FOR UPDATE; query tables media; "SELECT content,sha256,thumbnail,thumbnail_sha256 FROM media WHERE id=$1 FOR UPDATE", | 4 / owning domain resource | yes | yes | yes |
| `portal-rs/crates/content/src/media.rs:481` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/src/media.rs:535` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/src/media_tests.rs:85` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/src/media_tests.rs:89` | implicit SQL write locks; sqlx::query("INSERT INTO account_roles(account_id,role_id) SELECT $1,id FROM roles WHERE key='platform_admin'").bind(context.account).execut | 7 (effects; owning entry above) | n.a. | n.a. | yes |
| `portal-rs/crates/content/src/media_tests.rs:93` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/src/media_tests.rs:98` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/src/media_tests.rs:150` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/src/media_tests.rs:176` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/src/media_tests.rs:196` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/src/media_tests.rs:197` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/src/media_tests.rs:201` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/src/media_tests.rs:202` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/src/media_tests.rs:208` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/src/media_tests.rs:210` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/src/media_tests.rs:216` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/src/media_tests.rs:218` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/src/media_tests.rs:235` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/src/media_tests.rs:237` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/src/media_tests.rs:238` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/src/media_tests.rs:239` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/src/media_tests.rs:251` | implicit SQL write locks; sqlx::query("INSERT INTO account_roles(account_id,role_id) SELECT $1,id FROM roles WHERE key='platform_admin'").bind(actor).execute(pool).aw | 7 (effects; owning entry above) | n.a. | n.a. | yes |
| `portal-rs/crates/content/src/media_tests.rs:257` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/src/media_tests.rs:286` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/src/media_tests.rs:287` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/src/media_tests.rs:344` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/src/media_tests.rs:367` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/src/media_tests.rs:370` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/src/media_tests.rs:429` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/src/media_tests.rs:445` | implicit SQL write locks; sqlx::query("INSERT INTO accounts(id,email,password_hash,status,locale,email_verified_at) VALUES($1,$2,'image-staff-password','active','en', | 7 (effects; owning entry above) | n.a. | n.a. | yes |
| `portal-rs/crates/content/src/media_tests.rs:446` | implicit SQL write locks; sqlx::query("INSERT INTO account_roles(account_id,role_id) SELECT $1,id FROM roles WHERE key='platform_admin'").bind(actor).execute(&pool).a | 7 (effects; owning entry above) | n.a. | n.a. | yes |
| `portal-rs/crates/content/src/media_tests.rs:449` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/src/media_tests.rs:450` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/src/media_tests.rs:455` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/src/media_tests.rs:456` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/src/media_tests.rs:457` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/src/media_tests.rs:468` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/src/media_tests.rs:470` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/src/news.rs:1` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/src/news.rs:11` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/src/news.rs:152` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/src/news.rs:435` | canonical account/resource/RBAC/durable-proof acquisition + live authority | 3→4→5→6 | yes | yes | yes |
| `portal-rs/crates/content/src/news.rs:440` | FOR UPDATE; query tables news; sqlx::query("SELECT id FROM news WHERE id=$1 FOR UPDATE") | 4 / owning domain resource | yes | yes | yes |
| `portal-rs/crates/content/src/news.rs:449` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/src/news.rs:457` | FOR UPDATE; query tables news; sqlx::query_scalar("SELECT to_jsonb(news) FROM news WHERE id=$1 FOR UPDATE") | 4 / owning domain resource | yes | yes | yes |
| `portal-rs/crates/content/src/news.rs:466` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/src/news.rs:521` | canonical account/resource/RBAC/durable-proof acquisition + live authority | 3→4→5→6 | yes | yes | yes |
| `portal-rs/crates/content/src/news.rs:526` | FOR UPDATE; query tables news; sqlx::query("SELECT id FROM news WHERE id=$1 FOR UPDATE") | 4 / owning domain resource | yes | yes | yes |
| `portal-rs/crates/content/src/news.rs:535` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/src/news.rs:543` | FOR UPDATE; query tables news; sqlx::query_scalar("SELECT to_jsonb(news) FROM news WHERE id=$1 FOR UPDATE") | 4 / owning domain resource | yes | yes | yes |
| `portal-rs/crates/content/src/news.rs:549` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/src/news.rs:607` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/src/news_tests.rs:53` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/src/news_tests.rs:57` | implicit SQL write locks; sqlx::query("INSERT INTO account_roles(account_id,role_id) SELECT $1,id FROM roles WHERE key='content_editor'").bind(context.account).execut | 7 (effects; owning entry above) | n.a. | n.a. | yes |
| `portal-rs/crates/content/src/news_tests.rs:61` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/src/news_tests.rs:66` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/src/news_tests.rs:80` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/src/news_tests.rs:212` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/src/news_tests.rs:269` | implicit SQL write locks; sqlx::query("DELETE FROM account_roles WHERE account_id=$1") | 7 (effects; owning entry above) | n.a. | n.a. | yes |
| `portal-rs/crates/content/src/news_tests.rs:278` | implicit SQL write locks; sqlx::query("INSERT INTO account_roles(account_id,role_id) VALUES ($1,$2)") | 7 (effects; owning entry above) | n.a. | n.a. | yes |
| `portal-rs/crates/content/src/news_tests.rs:284` | implicit SQL write locks; sqlx::query("INSERT INTO role_permissions(role_id,permission) VALUES ($1,'admin.access')") | 7 (effects; owning entry above) | n.a. | n.a. | yes |
| `portal-rs/crates/content/src/news_tests.rs:295` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/src/news_tests.rs:299` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/src/news_tests.rs:305` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/src/news_tests.rs:313` | implicit SQL write locks; sqlx::query("INSERT INTO role_permissions(role_id,permission) VALUES ($1,'cms.news.manage')") | 7 (effects; owning entry above) | n.a. | n.a. | yes |
| `portal-rs/crates/content/src/news_tests.rs:342` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/src/news_tests.rs:349` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/src/news_tests.rs:350` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/src/news_tests.rs:440` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/src/news_tests.rs:441` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/src/news_tests.rs:442` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/src/news_tests.rs:450` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/src/news_tests.rs:456` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/src/news_tests.rs:458` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/src/news_tests.rs:550` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/src/news_tests.rs:601` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/src/news_tests.rs:633` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/src/pages.rs:1` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/src/pages.rs:11` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/src/pages.rs:14` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/src/pages.rs:183` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/src/pages/README.md:23` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/src/pages/README.md:26` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/src/pages/README.md:38` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/src/pages/staff.rs:5` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/src/pages/staff.rs:6` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/src/pages/staff.rs:17` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/src/pages/staff.rs:21` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/src/pages/staff.rs:22` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/src/pages/staff.rs:34` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/src/pages/staff.rs:247` | canonical account/resource/RBAC/durable-proof acquisition + live authority | 3→4→5→6 | yes | yes | yes |
| `portal-rs/crates/content/src/pages/staff.rs:252` | pg_advisory_xact_lock; sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1,71102))") | domain namespace (README inventory) | yes | yes | yes |
| `portal-rs/crates/content/src/pages/staff.rs:261` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/src/pages/staff.rs:264` | pg_advisory_xact_lock; sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1,71102))") | domain namespace (README inventory) | yes | yes | yes |
| `portal-rs/crates/content/src/pages/staff.rs:268` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/src/pages/tests.rs:1` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/src/pages/tests.rs:49` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/src/pages/tests.rs:53` | implicit SQL write locks; sqlx::query("INSERT INTO account_roles(account_id,role_id) SELECT $1,id FROM roles WHERE key='content_editor'").bind(ctx.account).execute(&p | 7 (effects; owning entry above) | n.a. | n.a. | yes |
| `portal-rs/crates/content/src/pages/tests.rs:56` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/src/pages/tests.rs:58` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/src/pages/tests.rs:74` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/src/pages/tests.rs:260` | implicit SQL write locks; sqlx::query("DELETE FROM account_roles WHERE account_id=$1") | 7 (effects; owning entry above) | n.a. | n.a. | yes |
| `portal-rs/crates/content/src/pages/tests.rs:265` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/src/pages/tests.rs:269` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/src/pages/tests.rs:274` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/src/pages/tests.rs:278` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/src/pages/tests.rs:283` | implicit SQL write locks; sqlx::query("INSERT INTO account_roles(account_id,role_id) SELECT $1,id FROM roles WHERE key='content_editor'").bind(ctx.account).execute(&p | 7 (effects; owning entry above) | n.a. | n.a. | yes |
| `portal-rs/crates/content/src/pages/tests.rs:284` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/src/pages/tests.rs:288` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/src/pages/tests.rs:312` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/src/pages/tests.rs:317` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/src/pages/tests.rs:321` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/src/pages/tests.rs:323` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/src/pages/tests.rs:332` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/src/pages/tests.rs:336` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/src/pages/tests.rs:349` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/src/pages/tests.rs:360` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/src/pages/tests.rs:363` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/src/pages/tests.rs:367` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/src/pages/tests.rs:380` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/src/pages/tests.rs:410` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/src/pages/tests.rs:418` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/src/pages/tests.rs:421` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/src/pages/tests.rs:443` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/src/pages/tests.rs:447` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/src/pages/tests.rs:448` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/src/pages/tests.rs:456` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/src/pages/tests.rs:462` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/src/pages/tests.rs:464` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/src/pages/tests.rs:501` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/src/pages/tests.rs:529` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/src/pagination_tests.rs:7` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/src/pagination_tests.rs:8` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/src/pagination_tests.rs:17` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/src/pagination_tests.rs:18` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/src/pagination_tests.rs:19` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/src/pagination_tests.rs:20` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/src/pagination_tests.rs:30` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/src/pagination_tests.rs:40` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/src/pagination_tests.rs:68` | implicit SQL write locks; sqlx::query("INSERT INTO account_roles(account_id,role_id) SELECT $1,id FROM roles WHERE key='platform_admin'").bind(actor).execute(&pool).a | 7 (effects; owning entry above) | n.a. | n.a. | yes |
| `portal-rs/crates/content/src/pagination_tests.rs:89` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/src/pagination_tests.rs:95` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/src/pagination_tests.rs:100` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/src/pagination_tests.rs:103` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/src/pagination_tests.rs:114` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/src/pagination_tests.rs:116` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/src/pagination_tests.rs:136` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/tests/transaction_authorization.rs:2` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/tests/transaction_authorization.rs:11` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/tests/transaction_authorization.rs:17` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/tests/transaction_authorization.rs:19` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/tests/transaction_authorization.rs:65` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/tests/transaction_authorization.rs:69` | implicit SQL write locks; sqlx::query("INSERT INTO account_roles(account_id,role_id) SELECT $1,id FROM roles WHERE key='platform_admin'").bind(actor.account).execute( | 7 (effects; owning entry above) | n.a. | n.a. | yes |
| `portal-rs/crates/content/tests/transaction_authorization.rs:96` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/tests/transaction_authorization.rs:136` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/tests/transaction_authorization.rs:173` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/tests/transaction_authorization.rs:174` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/tests/transaction_authorization.rs:178` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/tests/transaction_authorization.rs:179` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/tests/transaction_authorization.rs:213` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/tests/transaction_authorization.rs:226` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/tests/transaction_authorization.rs:228` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/tests/transaction_authorization.rs:235` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/tests/transaction_authorization.rs:243` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/tests/transaction_authorization.rs:245` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/tests/transaction_authorization.rs:255` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/tests/transaction_authorization.rs:288` | test fixture/blocker: FOR UPDATE; query tables news; sqlx::query("SELECT id FROM news WHERE id=$1 FOR UPDATE") | 4 / owning domain resource | n.a. | n.a. | yes |
| `portal-rs/crates/content/tests/transaction_authorization.rs:303` | test fixture/blocker: pg_advisory_xact_lock; sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended('fenced-page',71102))") | domain namespace (README inventory) | n.a. | n.a. | yes |
| `portal-rs/crates/content/tests/transaction_authorization.rs:313` | test fixture/blocker: pg_advisory_xact_lock; sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1,12))") | domain namespace (README inventory) | n.a. | n.a. | yes |
| `portal-rs/crates/content/tests/transaction_authorization.rs:330` | test fixture/blocker: FOR UPDATE; query tables media; sqlx::query("SELECT id FROM media WHERE id=$1 FOR UPDATE") | 4 / owning domain resource | n.a. | n.a. | yes |
| `portal-rs/crates/content/tests/transaction_authorization.rs:342` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/tests/transaction_authorization.rs:343` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/tests/transaction_authorization.rs:352` | implicit SQL write locks; sqlx::query("DELETE FROM account_roles WHERE account_id=$1") | 7 (effects; owning entry above) | n.a. | n.a. | yes |
| `portal-rs/crates/content/tests/transaction_authorization.rs:366` | implicit SQL write locks; sqlx::query("INSERT INTO account_roles(account_id,role_id) SELECT $1,id FROM roles WHERE key='platform_admin'").bind(db.actor.account).execu | 7 (effects; owning entry above) | n.a. | n.a. | yes |
| `portal-rs/crates/content/tests/transaction_authorization.rs:412` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/tests/transaction_authorization.rs:417` | implicit SQL write locks; sqlx::query("DELETE FROM role_permissions WHERE permission='admin.access'") | 7 (effects; owning entry above) | n.a. | n.a. | yes |
| `portal-rs/crates/content/tests/transaction_authorization.rs:438` | implicit SQL write locks; sqlx::query("INSERT INTO role_permissions(role_id,permission) SELECT id,'admin.access' FROM roles WHERE key='platform_admin'").execute(&db.p | 7 (effects; owning entry above) | n.a. | n.a. | yes |
| `portal-rs/crates/content/tests/transaction_authorization.rs:453` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/tests/transaction_authorization.rs:463` | test fixture/blocker: FOR UPDATE; query tables announcements; sqlx::query("SELECT id FROM announcements WHERE id=$1 FOR UPDATE") | 4 / owning domain resource | n.a. | n.a. | yes |
| `portal-rs/crates/content/tests/transaction_authorization.rs:468` | test fixture/blocker: FOR UPDATE; query tables events; sqlx::query("SELECT id FROM events WHERE id=$1 FOR UPDATE") | 4 / owning domain resource | n.a. | n.a. | yes |
| `portal-rs/crates/content/tests/transaction_authorization.rs:474` | test fixture/blocker: pg_advisory_xact_lock; sqlx::query("SELECT pg_advisory_xact_lock(71010)") | 5 | n.a. | n.a. | yes |
| `portal-rs/crates/content/tests/transaction_authorization.rs:525` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/tests/transaction_authorization.rs:592` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/tests/transaction_authorization.rs:615` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/content/tests/transaction_authorization.rs:625` | test fixture/blocker: FOR UPDATE; query tables announcements; "SELECT id FROM announcements WHERE id=$1 FOR UPDATE" | 4 / owning domain resource | n.a. | n.a. | yes |
| `portal-rs/crates/content/tests/transaction_authorization.rs:627` | test fixture/blocker: FOR UPDATE; query tables events; "SELECT id FROM events WHERE id=$1 FOR UPDATE" | 4 / owning domain resource | n.a. | n.a. | yes |
| `portal-rs/crates/content/tests/transaction_authorization.rs:642` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/db/src/outbox.rs:75` | FOR UPDATE SKIP LOCKED; FOR UPDATE SKIP LOCKED LIMIT $1 | 4 / owning domain resource | no | n.a. | yes |
| `portal-rs/crates/db/src/outbox.rs:89` | FOR UPDATE SKIP LOCKED; ORDER BY run_after, id FOR UPDATE SKIP LOCKED LIMIT $1 | 4 / owning domain resource | no | n.a. | yes |
| `portal-rs/crates/db/src/test_support.rs:34` | test fixture/blocker: pg_advisory_lock; sqlx::query("SELECT pg_advisory_lock(684326110016002)") | domain namespace (README inventory) | n.a. | n.a. | yes |
| `portal-rs/crates/db/tests/store_offers_coins_only.rs:139` | lock release; sqlx::query("SELECT pg_advisory_unlock_all()") | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/downloads/README.md:6` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/downloads/README.md:8` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/downloads/README.md:80` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/downloads/src/command.rs:106` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/downloads/src/command.rs:112` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/downloads/src/command.rs:126` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/downloads/src/domain.rs:41` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/downloads/src/domain.rs:55` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/downloads/src/domain.rs:56` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/downloads/src/http.rs:14` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/downloads/src/storage.rs:44` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/downloads/src/storage.rs:68` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/downloads/src/storage.rs:84` | FOR UPDATE; query tables client_channels; "SELECT id, active_publication_id FROM client_channels WHERE name = $1 FOR UPDATE", | 4 / owning domain resource | no | n.a. | fixed in parallel task drc-a |
| `portal-rs/crates/downloads/src/storage.rs:91` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/downloads/tests/command.rs:14` | implicit SQL write locks; sqlx::query("INSERT INTO account_roles(account_id,role_id) SELECT $1,role_id FROM role_permissions WHERE permission='downloads.manage' LIMIT | 7 (effects; owning entry above) | n.a. | n.a. | yes |
| `portal-rs/crates/downloads/tests/command.rs:126` | implicit SQL write locks; sqlx::query("INSERT INTO account_roles(account_id,role_id) SELECT $1,role_id FROM role_permissions WHERE permission='downloads.manage' LIMIT | 7 (effects; owning entry above) | n.a. | n.a. | yes |
| `portal-rs/crates/downloads/tests/command.rs:161` | implicit SQL write locks; sqlx::query("INSERT INTO account_roles(account_id,role_id) SELECT $1,role_id FROM role_permissions WHERE permission='downloads.manage' LIMIT | 7 (effects; owning entry above) | n.a. | n.a. | yes |
| `portal-rs/crates/downloads/tests/common/mod.rs:16` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/downloads/tests/publication.rs:314` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/gameauth/src/accept.rs:71` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/gameauth/src/admission/audit05_tests.rs:32` | test fixture/blocker: FOR UPDATE; query tables accounts; sqlx::query_scalar("SELECT status FROM accounts WHERE id=$1 FOR UPDATE") | 3 | n.a. | n.a. | yes |
| `portal-rs/crates/gameauth/src/admission/audit05_tests.rs:298` | test fixture/blocker: FOR UPDATE; query tables accounts; sqlx::query("SELECT id FROM accounts WHERE id=$1 FOR UPDATE") | 3 | n.a. | n.a. | yes |
| `portal-rs/crates/gameauth/src/admission/audit05_tests.rs:386` | test fixture/blocker: FOR UPDATE; query tables native_recovery_handles; sqlx::query_scalar("SELECT expires_at FROM native_recovery_handles FOR UPDATE") | 4 / owning domain resource | n.a. | n.a. | yes |
| `portal-rs/crates/gameauth/src/admission/audit05_tests.rs:587` | test fixture/blocker: FOR UPDATE; query tables native_scope_assignments; sqlx::query("SELECT world_id FROM native_scope_assignments FOR UPDATE") | 4 / owning domain resource | n.a. | n.a. | yes |
| `portal-rs/crates/gameauth/src/admission/audit05_tests.rs:677` | test fixture/blocker: FOR UPDATE; query tables accounts; sqlx::query("SELECT id FROM accounts WHERE id=$1 FOR UPDATE") | 3 | n.a. | n.a. | yes |
| `portal-rs/crates/gameauth/src/admission/audit05_tests.rs:745` | test fixture/blocker: FOR UPDATE; query tables native_recovery_handles; sqlx::query("SELECT handle_hash FROM native_recovery_handles WHERE handle_hash=$1 FOR UPDATE") | 4 / owning domain resource | n.a. | n.a. | yes |
| `portal-rs/crates/gameauth/src/admission/grant.rs:6` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/gameauth/src/admission/grant.rs:266` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/gameauth/src/admission/grant.rs:594` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/gameauth/src/admission/grant.rs:667` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/gameauth/src/admission/grant.rs:692` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/gameauth/src/admission/grant.rs:747` | FOR SHARE; query tables game_channels; let row = sqlx::query("SELECT channel_id, native_route_host, native_route_port, native_route_tls_server_name, native_route_version, native_r | 4 / owning domain resource | no | n.a. | yes |
| `portal-rs/crates/gameauth/src/admission/grant.rs:765` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/gameauth/src/admission/grant_tests.rs:114` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/gameauth/src/admission/grant_tests.rs:136` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/gameauth/src/admission/grant_tests.rs:156` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/gameauth/src/admission/http.rs:167` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/gameauth/src/admission/http.rs:168` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/gameauth/src/admission/mod.rs:2` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/gameauth/src/admission/mod.rs:24` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/gameauth/src/admission/protocol3.rs:6` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/gameauth/src/admission/protocol3.rs:128` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/gameauth/src/admission/protocol3.rs:166` | FOR SHARE; query tables device_sessions; let row = sqlx::query("SELECT device_key_thumbprint, absolute_expires_at FROM device_sessions WHERE id = $1 AND account_id = $2 AND revoked_ | 4 / owning domain resource | no | n.a. | yes |
| `portal-rs/crates/gameauth/src/admission/protocol3.rs:221` | FOR UPDATE; query tables native_recovery_handles; let Some(old) = sqlx::query("SELECT handle_hash, expires_at FROM native_recovery_handles WHERE attempt_ref = $1 AND revoked_at IS NULL AND e | 4 / owning domain resource | no | n.a. | yes |
| `portal-rs/crates/gameauth/src/admission/recovery.rs:88` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/gameauth/src/admission/recovery.rs:445` | FOR UPDATE; query tables accounts; sqlx::query("SELECT id FROM accounts WHERE id=$1 FOR UPDATE") | 3 | no | n.a. | yes |
| `portal-rs/crates/gameauth/src/admission/recovery.rs:471` | FOR UPDATE; query tables native_recovery_handles; let row = sqlx::query("SELECT account_id, character_id, world_id, channel_id, security_generation, play_generation, device_family_id, device | 4 / owning domain resource | no | n.a. | yes |
| `portal-rs/crates/gameauth/src/admission/recovery.rs:485` | FOR SHARE; query tables accounts; let identity = sqlx::query("SELECT status, native_security_generation::text AS native FROM accounts WHERE id=$1 FOR SHARE") | 3 | no | n.a. | yes |
| `portal-rs/crates/gameauth/src/admission/recovery.rs:506` | FOR SHARE OF d; query tables accounts, device_sessions; let current = sqlx::query("SELECT d.device_public_key, d.device_key_thumbprint, d.absolute_expires_at, d.idle_expires_at FROM device_session | 3 | no | n.a. | yes |
| `portal-rs/crates/gameauth/src/admission/recovery.rs:542` | FOR SHARE; query tables native_play_generations; "SELECT generation FROM native_play_generations WHERE account_id=$1 FOR SHARE", | 4 / owning domain resource | no | n.a. | yes |
| `portal-rs/crates/gameauth/src/admission/recovery.rs:670` | FOR SHARE; query tables native_assignment_epoch_locks; sqlx::query_scalar("SELECT id FROM native_assignment_epoch_locks WHERE id=1 FOR SHARE") | 4 / owning domain resource | no | n.a. | yes |
| `portal-rs/crates/gameauth/src/admission/recovery.rs:678` | FOR UPDATE; query tables native_recoveries; let existing = sqlx::query("SELECT handle_hash, request_digest, response_body, expires_at_ms FROM native_recoveries WHERE attempt_ref=$1 FOR | 4 / owning domain resource | no | n.a. | yes |
| `portal-rs/crates/gameauth/src/admission/recovery_tests.rs:320` | test fixture/blocker: FOR UPDATE; query tables accounts; sqlx::query_scalar("SELECT status FROM accounts WHERE id=$1 FOR UPDATE") | 3 | n.a. | n.a. | yes |
| `portal-rs/crates/gameauth/src/admission/recovery_tests.rs:505` | test fixture/blocker: FOR UPDATE; query tables accounts; sqlx::query("SELECT id FROM accounts WHERE id=$1 FOR UPDATE") | 3 | n.a. | n.a. | yes |
| `portal-rs/crates/gameauth/src/admission/recovery_tests.rs:571` | test fixture/blocker: FOR UPDATE; query tables native_recovery_handles; sqlx::query("SELECT handle_hash FROM native_recovery_handles FOR UPDATE") | 4 / owning domain resource | n.a. | n.a. | yes |
| `portal-rs/crates/gameauth/src/admission/store.rs:25` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/gameauth/src/admission/store.rs:40` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/gameauth/src/admission/store.rs:54` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/gameauth/src/admission/store.rs:70` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/gameauth/src/admission/store.rs:85` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/gameauth/src/admission/store.rs:93` | pg_advisory_xact_lock; sqlx::query("SELECT pg_advisory_xact_lock($1)") | domain namespace (README inventory) | no | n.a. | yes |
| `portal-rs/crates/gameauth/src/admission/store.rs:168` | FOR UPDATE; query tables native_admissions; let existing = sqlx::query("SELECT account_id, ticket_hash, request_digest, response_body, expires_at_ms FROM native_admissions WHERE attemp | 4 / owning domain resource | no | n.a. | yes |
| `portal-rs/crates/gameauth/src/admission/tests.rs:52` | test fixture/blocker: FOR UPDATE; query tables fixture_accounts; sqlx::query_scalar("SELECT state FROM fixture_accounts WHERE id = $1 FOR UPDATE") | 4 / owning domain resource | n.a. | n.a. | yes |
| `portal-rs/crates/gameauth/src/admission/tests.rs:72` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/gameauth/src/admission/tests.rs:77` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/gameauth/src/admission/tests.rs:95` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/gameauth/src/admission/tests.rs:311` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/gameauth/src/admission/tests.rs:312` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/gameauth/src/admission/tests.rs:389` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/gameauth/src/admission/tests.rs:403` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/gameauth/src/admission/tests.rs:446` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/gameauth/src/admission/tests.rs:454` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/gameauth/src/admission/tickets.rs:7` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/gameauth/src/admission/tickets.rs:46` | FOR SHARE; query tables account_character_feed, clock_timestamp; let live: Option&lt;bool&gt; = sqlx::query_scalar("SELECT highest_epoch IS NOT NULL AND highest_epoch=watermark_epoch AND source_authority=$1 AND  | 4 / owning domain resource | no | n.a. | yes |
| `portal-rs/crates/gameauth/src/admission/tickets.rs:90` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/gameauth/src/admission/tickets.rs:100` | FOR UPDATE; query tables accounts; sqlx::query("SELECT id FROM accounts WHERE id=$1 FOR UPDATE") | 3 | no | n.a. | yes |
| `portal-rs/crates/gameauth/src/admission/tickets.rs:105` | FOR UPDATE; query tables game_login_tickets; let ticket = sqlx::query("SELECT account_id, game_generation, native_generation, audience, used_at IS NULL AND expires_at &gt; clock_timestamp( | 4 / owning domain resource | no | n.a. | yes |
| `portal-rs/crates/gameauth/src/admission/tickets.rs:121` | FOR UPDATE; query tables accounts; let current = sqlx::query("SELECT status, game_auth_generation::text AS game, native_security_generation::text AS native FROM accounts WHERE | 3 | no | n.a. | yes |
| `portal-rs/crates/gameauth/src/admission/tickets.rs:138` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/gameauth/src/admission/tickets.rs:167` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/gameauth/src/admission/wire.rs:31` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/gameauth/src/devices.rs:197` | FOR UPDATE; query tables accounts; "SELECT status, native_security_generation FROM accounts WHERE id=$1 FOR UPDATE", | 3 | no | n.a. | yes |
| `portal-rs/crates/gameauth/src/devices.rs:379` | FOR UPDATE OF f,c; query tables device_session_credentials, device_sessions; let row = sqlx::query("SELECT c.secret_hash,f.persistence_class,f.sequence,f.absolute_expires_at,f.idle_expires_at,f.revoked_at,c.consumed_a | 4 / owning domain resource | no | n.a. | yes |
| `portal-rs/crates/gameauth/src/devices.rs:486` | FOR UPDATE; query tables accounts; sqlx::query_scalar("SELECT status FROM accounts WHERE id=$1 FOR UPDATE") | 3 | no | n.a. | yes |
| `portal-rs/crates/gameauth/src/devices.rs:504` | FOR UPDATE; query tables accounts; sqlx::query("SELECT id FROM accounts WHERE id=$1 FOR UPDATE") | 3 | no | n.a. | yes |
| `portal-rs/crates/gameauth/src/devices_authority.rs:35` | FOR SHARE; query tables oauth_native_clients; let allowed: Option&lt;bool&gt; = sqlx::query_scalar("SELECT enabled AND first_party AND public_client AND native_ticket FROM oauth_native_clients | 4 / owning domain resource | no | n.a. | yes |
| `portal-rs/crates/gameauth/src/devices_authority.rs:68` | FOR UPDATE; query tables accounts; let current: Option&lt;(String,i64,i64,bool)&gt; = sqlx::query_as("SELECT status,game_auth_generation,native_security_generation,email_verified_at | 3 | no | n.a. | yes |
| `portal-rs/crates/gameauth/src/devices_authority.rs:109` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/gameauth/src/devices_authority.rs:126` | FOR SHARE; query tables account_character_feed, clock_timestamp; let live: Option&lt;bool&gt; = sqlx::query_scalar("SELECT highest_epoch IS NOT NULL AND highest_epoch=watermark_epoch AND source_authority=$1 AND  | 4 / owning domain resource | no | n.a. | yes |
| `portal-rs/crates/gameauth/src/devices_authority.rs:170` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/gameauth/src/devices_authority.rs:183` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/gameauth/src/devices_authority.rs:187` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/gameauth/src/devices_http.rs:18` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/gameauth/src/devices_http.rs:200` | FOR UPDATE SKIP LOCKED; query tables clock_timestamp, device_session_source_limits; sqlx::query("DELETE FROM device_session_source_limits WHERE source_hash IN (SELECT source_hash FROM device_session_source_limits WHERE windo | 4 / owning domain resource | no | n.a. | yes |
| `portal-rs/crates/gameauth/src/devices_tests.rs:442` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/gameauth/src/devices_tests.rs:769` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/gameauth/src/evidence.rs:471` | FOR UPDATE; query tables accounts; "SELECT status, native_security_generation FROM accounts WHERE id=$1 FOR UPDATE", | 3 | no | n.a. | yes |
| `portal-rs/crates/gameauth/src/evidence.rs:521` | FOR UPDATE; query tables native_game_signing_trust_profiles; let profile = sqlx::query("SELECT id, issuer_revision, revoked_at IS NOT NULL AS revoked FROM native_game_signing_trust_profiles WHERE issue | 4 / owning domain resource | no | n.a. | yes |
| `portal-rs/crates/gameauth/src/evidence.rs:535` | FOR UPDATE; query tables native_game_signing_trust_key_versions; let row = sqlx::query("SELECT public_key, trusted, key_revision FROM native_game_signing_trust_key_versions WHERE profile_id=$1 AND key_id=$ | 4 / owning domain resource | no | n.a. | yes |
| `portal-rs/crates/gameauth/src/evidence.rs:686` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/gameauth/src/evidence_tests.rs:201` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/gameauth/src/keys/mod.rs:380` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/gameauth/src/keys/tests.rs:302` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/gameauth/src/keys/tests.rs:305` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/gameauth/src/keys/tests.rs:315` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/gameauth/src/keys/tests.rs:316` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/gameauth/src/keys/tests.rs:320` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/gameauth/src/keys/tests.rs:338` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/gameauth/src/keys/tests.rs:340` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/gameauth/src/keys/tests.rs:353` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/gameauth/src/keys/tests.rs:357` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/gameauth/src/keys/tests.rs:768` | test fixture/blocker: pg_advisory_xact_lock; sqlx::query("SELECT pg_advisory_xact_lock(715110005)") | domain namespace (README inventory) | n.a. | n.a. | yes |
| `portal-rs/crates/gameauth/src/keys/trust.rs:20` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/gameauth/src/keys/trust.rs:98` | pg_advisory_xact_lock; sqlx::query("SELECT pg_advisory_xact_lock(715110005)") | domain namespace (README inventory) | no | n.a. | yes |
| `portal-rs/crates/gameauth/src/keys/trust.rs:102` | FOR UPDATE; query tables native_game_signing_trust_profiles; let profile = sqlx::query("SELECT id, issuer_revision, revoked_at IS NOT NULL AS revoked FROM native_game_signing_trust_profiles WHERE issue | 4 / owning domain resource | no | n.a. | yes |
| `portal-rs/crates/gameauth/src/keys/trust.rs:180` | FOR UPDATE; query tables native_game_signing_trust_profiles; let row = sqlx::query("SELECT id, issuer_revision, revoked_at IS NOT NULL AS revoked FROM native_game_signing_trust_profiles WHERE issuer=$1 | 4 / owning domain resource | no | n.a. | yes |
| `portal-rs/crates/gameauth/src/keys/trust.rs:226` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/gameauth/src/keys/trust.rs:228` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/gameauth/src/keys/trust.rs:233` | FOR UPDATE; query tables native_game_signing_trust_profiles; let row = sqlx::query("SELECT id, issuer_revision, revoked_at IS NOT NULL AS revoked FROM native_game_signing_trust_profiles WHERE issuer=$1 | 4 / owning domain resource | no | n.a. | yes |
| `portal-rs/crates/gameauth/src/locator.rs:207` | FOR SHARE; query tables native_assignment_epoch_locks; sqlx::query_scalar("SELECT id FROM native_assignment_epoch_locks WHERE id = 1 FOR SHARE") | 4 / owning domain resource | no | n.a. | yes |
| `portal-rs/crates/gameauth/src/locator.rs:215` | FOR SHARE; query tables native_scope_assignments; let assignment = sqlx::query("SELECT assignment_epoch::text AS epoch, ownership_generation::text AS generation, node_identity FROM native_sc | 4 / owning domain resource | no | n.a. | yes |
| `portal-rs/crates/gameauth/src/locator.rs:220` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/gameauth/src/locator.rs:226` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/gameauth/src/locator.rs:254` | pg_advisory_xact_lock; "SELECT pg_advisory_xact_lock(hashtextextended('native-recovery-locator:' \|\| $1::text, 0))", | domain namespace (README inventory) | no | n.a. | yes |
| `portal-rs/crates/gameauth/src/locator.rs:260` | FOR UPDATE; query tables native_recovery_locators; let entry = sqlx::query("SELECT assignment_epoch::text AS epoch, locator_revision::text AS revision, scope_ownership_generation::text AS gen | 4 / owning domain resource | no | n.a. | yes |
| `portal-rs/crates/gameauth/src/locator.rs:369` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/gameauth/src/locator.rs:375` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/gameauth/src/locator.rs:382` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/gameauth/src/locator.rs:407` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/gameauth/src/locator.rs:410` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/gameauth/src/locator_tests.rs:188` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/gameauth/src/locator_tests.rs:192` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/gameauth/src/locator_tests.rs:198` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/gameauth/src/locator_tests.rs:212` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/gameauth/src/locator_tests.rs:215` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/gameauth/src/locator_tests.rs:392` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/gameauth/src/locator_tests.rs:404` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/gameauth/src/locator_tests.rs:411` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/gameauth/src/oauth.rs:76` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/gameauth/src/oauth.rs:167` | FOR UPDATE; query tables accounts; let row = sqlx::query("SELECT password_hash, email, email_verified_at::text AS verified, login_mfa_required, status, game_auth_generation FR | 3 | no | n.a. | yes |
| `portal-rs/crates/gameauth/src/oauth.rs:208` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/gameauth/src/oauth.rs:255` | FOR UPDATE OF s; query tables accounts, web_sessions; let grant: Option&lt;Option&lt;String&gt;&gt; = sqlx::query_scalar("SELECT s.data-&gt;&gt;'mfa_grant' FROM web_sessions s JOIN accounts a ON a.id=s.account_id | 3 | no | n.a. | yes |
| `portal-rs/crates/gameauth/src/oauth.rs:362` | FOR UPDATE; query tables oauth_authorization_codes; let row = sqlx::query("SELECT *, expires_at&gt;clock_timestamp() AND used_at IS NULL AND revoked_at IS NULL AS valid FROM oauth_authorization_c | 4 / owning domain resource | no | n.a. | yes |
| `portal-rs/crates/gameauth/src/oauth.rs:411` | FOR UPDATE; query tables oauth_token_families; let row = sqlx::query("SELECT *, expires_at&gt;clock_timestamp() AND revoked_at IS NULL AS valid FROM oauth_token_families WHERE id=$1 FOR UPDA | 4 / owning domain resource | no | n.a. | yes |
| `portal-rs/crates/gameauth/src/oauth.rs:467` | FOR UPDATE; query tables accounts; sqlx::query("SELECT id FROM accounts WHERE id=$1 FOR UPDATE") | 3 | no | n.a. | yes |
| `portal-rs/crates/gameauth/src/oauth.rs:506` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/gameauth/src/oauth.rs:567` | FOR UPDATE OF f,t; query tables oauth_token_families, oauth_tokens; let row = sqlx::query("SELECT t.id, t.access_hash, f.account_id, f.client_id, f.credential_hash, f.game_generation, f.native_generation FROM | 4 / owning domain resource | no | n.a. | yes |
| `portal-rs/crates/gameauth/src/oauth/oauth_fixture_tests.rs:271` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/gameauth/src/oauth/oauth_http.rs:40` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/gameauth/src/oauth/oauth_http.rs:68` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/gameauth/src/oauth/oauth_http.rs:160` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/gameauth/src/oauth/oauth_http.rs:188` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/gameauth/src/oauth/oauth_http.rs:216` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/gameauth/src/oauth/oauth_http.rs:272` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/gameauth/src/oauth/oauth_http.rs:333` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/gameauth/src/oauth/oauth_tests.rs:95` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/gameauth/src/oauth/oauth_tests.rs:393` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/gameauth/src/oauth/oauth_tests.rs:400` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/gameauth/src/oauth/oauth_tests.rs:437` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/gameauth/src/oauth/oauth_tests.rs:441` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/gameauth/src/oauth/oauth_tests.rs:447` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/gameauth/src/oauth/oauth_tests.rs:468` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/gameauth/src/oauth/oauth_tests.rs:482` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/gameauth/src/oauth/oauth_tests.rs:495` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/gameauth/src/oauth/oauth_tests.rs:501` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/gameauth/src/oauth/oauth_tests.rs:512` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/gameauth/src/oauth/oauth_tests.rs:578` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/gameauth/src/oauth/oauth_tests.rs:610` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/gameauth/src/oauth/oauth_tests.rs:680` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/gameauth/src/oauth/oauth_tests.rs:697` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/gameauth/src/oauth/oauth_tests.rs:703` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/gameauth/src/oauth/oauth_tests.rs:706` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/gameauth/src/oauth/oauth_tests.rs:710` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/gameauth/src/oauth/oauth_tests.rs:752` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/gameauth/src/oauth/oauth_tests.rs:761` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/gameauth/src/oauth/oauth_tests.rs:781` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/gameauth/src/oauth/oauth_tests.rs:790` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/gameauth/src/oauth/oauth_tests.rs:988` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/gameauth/src/runtime.rs:294` | FOR SHARE; query tables native_assignment_epoch_locks; sqlx::query_scalar("SELECT id FROM native_assignment_epoch_locks WHERE id = 1 FOR SHARE") | 4 / owning domain resource | no | n.a. | yes |
| `portal-rs/crates/gameauth/src/runtime.rs:303` | FOR UPDATE; query tables native_scope_assignments; "SELECT assignment_epoch::text, ownership_generation::text, node_identity FROM native_scope_assignments WHERE world_id = $1 AND channel_id = | 4 / owning domain resource | no | n.a. | yes |
| `portal-rs/crates/gameauth/src/runtime.rs:305` | FOR SHARE; query tables native_scope_assignments; "SELECT assignment_epoch::text, ownership_generation::text, node_identity FROM native_scope_assignments WHERE world_id = $1 AND channel_id = | 4 / owning domain resource | no | n.a. | yes |
| `portal-rs/crates/gameauth/src/runtime.rs:326` | FOR UPDATE; query tables native_runtime_status_reports; let row = sqlx::query("SELECT assignment_epoch::text AS epoch, scope_ownership_generation::text AS generation, source_revision::text AS revi | 4 / owning domain resource | no | n.a. | yes |
| `portal-rs/crates/gameauth/src/runtime.rs:391` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/gameauth/src/runtime.rs:395` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/gameauth/src/runtime.rs:412` | FOR SHARE; query tables native_assignment_epoch_locks; sqlx::query_scalar("SELECT id FROM native_assignment_epoch_locks WHERE id=1 FOR SHARE") | 4 / owning domain resource | no | n.a. | yes |
| `portal-rs/crates/gameauth/src/runtime.rs:419` | FOR SHARE; query tables native_scope_assignments; sqlx::query("SELECT world_id FROM native_scope_assignments WHERE world_id=$1 AND channel_id=$2 FOR SHARE") | 4 / owning domain resource | no | n.a. | yes |
| `portal-rs/crates/gameauth/src/runtime.rs:434` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/gameauth/src/runtime.rs:440` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/gameauth/src/runtime.rs:452` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/gameauth/src/runtime.rs:494` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/gameauth/src/runtime.rs:562` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/gameauth/src/runtime.rs:572` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/gameauth/src/runtime.rs:579` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/gameauth/src/runtime.rs:599` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/gameauth/src/runtime_tests.rs:219` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/gameauth/src/runtime_tests.rs:408` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/gameauth/src/runtime_tests.rs:563` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/gameauth/src/runtime_tests.rs:573` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/gameauth/src/runtime_tests.rs:603` | test fixture/blocker: FOR UPDATE; query tables native_scope_assignments; sqlx::query("SELECT world_id FROM native_scope_assignments FOR UPDATE") | 4 / owning domain resource | n.a. | n.a. | yes |
| `portal-rs/crates/gameauth/src/runtime_tests.rs:611` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/gameauth/src/scopes.rs:1` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/gameauth/src/scopes.rs:176` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/gameauth/src/scopes.rs:201` | FOR UPDATE; query tables native_assignment_epoch_locks; sqlx::query_scalar("SELECT id FROM native_assignment_epoch_locks WHERE id=1 FOR UPDATE") | 4 / owning domain resource | no | n.a. | yes |
| `portal-rs/crates/gameauth/src/scopes.rs:215` | FOR UPDATE; query tables native_scope_assignments; let row = sqlx::query("SELECT assignment_epoch::text AS epoch, ownership_generation::text AS generation, node_identity, assigned_at, revoked | 4 / owning domain resource | no | n.a. | yes |
| `portal-rs/crates/gameauth/src/scopes.rs:261` | FOR SHARE; query tables native_assignment_epoch_locks; sqlx::query_scalar("SELECT id FROM native_assignment_epoch_locks WHERE id=1 FOR SHARE") | 4 / owning domain resource | no | n.a. | yes |
| `portal-rs/crates/gameauth/src/scopes.rs:333` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/gameauth/src/scopes.rs:343` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/gameauth/src/scopes_tests.rs:94` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/gameauth/src/scopes_tests.rs:188` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/gameauth/src/tickets/mod.rs:12` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/gameauth/src/tickets/mod.rs:50` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/gameauth/src/tickets/mod.rs:89` | FOR SHARE OF d; query tables clock_timestamp, device_sessions, oauth_tokens; let family: Option&lt;Uuid&gt; = sqlx::query_scalar("SELECT d.id FROM device_sessions d JOIN oauth_tokens t ON d.enrollment_token_id=t.id::text WH | 4 / owning domain resource | no | n.a. | yes |
| `portal-rs/crates/gameauth/src/tickets/mod.rs:140` | FOR UPDATE SKIP LOCKED; LIMIT $1 FOR UPDATE SKIP LOCKED) \ | 4 / owning domain resource | no | n.a. | yes |
| `portal-rs/crates/gameauth/src/tickets/mod.rs:155` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/gameauth/src/tickets/tests.rs:13` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/gameauth/src/tickets/tests.rs:23` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/gateway/src/lib.rs:896` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/gateway/tests/fixtures/go_vectors.jsonl:249` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/gateway/tests/go_e2e/native_login_test.go:40` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/gateway/tests/recover.rs:326` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/hirelings/src/commands.rs:249` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/hirelings/src/commands.rs:447` | FOR UPDATE; query tables hireling_command_sequence; "SELECT revision::text FROM hireling_command_sequence WHERE singleton=true FOR UPDATE", | 4 / owning domain resource | no | n.a. | yes |
| `portal-rs/crates/hirelings/src/commands.rs:458` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/hirelings/src/commands_tests.rs:166` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/hirelings/src/commands_tests.rs:170` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/hirelings/src/commands_tests.rs:254` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/hirelings/src/commands_tests.rs:259` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/hirelings/src/service.rs:131` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/hirelings/src/web.rs:61` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/hirelings/tests/flow.rs:149` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/identity/src/abuse.rs:78` | pg_advisory_xact_lock; sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1, 0))") | domain namespace (README inventory) | no | n.a. | yes |
| `portal-rs/crates/identity/src/account_lifecycle_http.rs:33` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/identity/src/account_lifecycle_http.rs:44` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/identity/src/account_lifecycle_http.rs:55` | FOR UPDATE; query tables wallet_accounts; sqlx::query("SELECT account_id FROM wallet_accounts WHERE account_id=$1 FOR UPDATE") | 7 / domain namespace | no | n.a. | yes |
| `portal-rs/crates/identity/src/account_lifecycle_http.rs:442` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/identity/src/accounts.rs:42` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/identity/src/accounts.rs:84` | pg_advisory_xact_lock; sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1,0))") | domain namespace (README inventory) | no | n.a. | yes |
| `portal-rs/crates/identity/src/consents.rs:103` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/identity/src/consents.rs:153` | FOR UPDATE; query tables accounts; sqlx::query_scalar("SELECT id FROM accounts WHERE id=$1 AND status='active' FOR UPDATE") | 3 | no | n.a. | yes |
| `portal-rs/crates/identity/src/consents_en.ftl:6` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/identity/src/consents_en.ftl:7` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/identity/src/consents_en.ftl:8` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/identity/src/consents_en.ftl:11` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/identity/src/consents_en.ftl:12` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/identity/src/consents_tests.rs:391` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/identity/src/credentials.rs:146` | FOR UPDATE OF a; query tables accounts, web_sessions; let row: Option&lt;(AccountId,String)&gt; = sqlx::query_as("SELECT a.id,a.password_hash FROM accounts a JOIN web_sessions s ON s.account_id=a.id W | 3 | no | n.a. | yes |
| `portal-rs/crates/identity/src/credentials_http.rs:52` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/identity/src/credentials_http.rs:256` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/identity/src/credentials_http.rs:284` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/identity/src/credentials_http.rs:326` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/identity/src/credentials_http.rs:336` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/identity/src/email_change.rs:60` | FOR UPDATE; query tables account_recovery_keys; let stored: Option&lt;String&gt; = sqlx::query_scalar("SELECT key_hash FROM account_recovery_keys WHERE account_id=$1 AND confirmed_at IS NOT NULL | 4 / owning domain resource | no | n.a. | yes |
| `portal-rs/crates/identity/src/email_change.rs:168` | FOR UPDATE; query tables accounts; let row: Option&lt;(String,String,String)&gt; = sqlx::query_as("SELECT email,password_hash,locale FROM accounts WHERE id=$1 AND status='active' AN | 3 | no | n.a. | yes |
| `portal-rs/crates/identity/src/email_change.rs:238` | FOR UPDATE; query tables accounts; let active: Option&lt;String&gt; = sqlx::query_scalar("SELECT email FROM accounts WHERE id=$1 AND status='active' AND email_verified_at IS NOT NUL | 3 | no | n.a. | yes |
| `portal-rs/crates/identity/src/email_change.rs:242` | FOR UPDATE; query tables account_email_changes; let change: Change = sqlx::query_as("SELECT * FROM account_email_changes WHERE (NOT $2 AND confirmation_hash=$1) OR ($2 AND revert_hash=$1)  | 4 / owning domain resource | no | n.a. | yes |
| `portal-rs/crates/identity/src/email_change.rs:352` | FOR UPDATE; query tables accounts; let current: Option&lt;String&gt; = sqlx::query_scalar("SELECT email FROM accounts WHERE id=$1 AND status='active' AND email_verified_at IS NOT NU | 3 | no | n.a. | yes |
| `portal-rs/crates/identity/src/email_change.rs:354` | FOR UPDATE; query tables account_email_changes; sqlx::query_as("SELECT * FROM account_email_changes WHERE id=$1 FOR UPDATE") | 4 / owning domain resource | no | n.a. | yes |
| `portal-rs/crates/identity/src/email_change.rs:413` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/identity/src/email_change.rs:434` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/identity/src/email_change.rs:438` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/identity/src/email_change.rs:440` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/identity/src/email_change.rs:441` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/identity/src/email_change.rs:442` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/identity/src/email_change_mail.rs:98` | FOR UPDATE OF m SKIP LOCKED; query tables account_email_change_mail, account_email_changes; let row:Option&lt;(Uuid,String,String,String)&gt;=sqlx::query_as("SELECT m.id,m.recipient,m.subject,m.body FROM account_email_change_mail m JOIN a | 4 / owning domain resource | no | n.a. | yes |
| `portal-rs/crates/identity/src/email_change_tests.rs:498` | test fixture/blocker: FOR UPDATE; query tables accounts; sqlx::query("SELECT id FROM accounts WHERE id=$1 FOR UPDATE") | 3 | n.a. | n.a. | yes |
| `portal-rs/crates/identity/src/export.rs:53` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/identity/src/export_tests.rs:25` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/identity/src/lib.rs:30` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/identity/src/login.rs:42` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/identity/src/login.rs:58` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/identity/src/login.rs:100` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/identity/src/login.rs:101` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/identity/src/login.rs:103` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/identity/src/login.rs:104` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/identity/src/login.rs:107` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/identity/src/login.rs:110` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/identity/src/login.rs:112` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/identity/src/login_http.rs:165` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/identity/src/login_http.rs:188` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/identity/src/login_http.rs:230` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/identity/src/login_http.rs:240` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/identity/src/login_http_tests.rs:308` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/identity/src/login_http_tests.rs:317` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/identity/src/login_http_tests.rs:321` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/identity/src/login_http_tests.rs:417` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/identity/src/login_http_tests.rs:473` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/identity/src/login_tests.rs:23` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/identity/src/login_tests.rs:24` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/identity/src/login_tests.rs:25` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/identity/src/login_tests.rs:26` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/identity/src/login_tests.rs:30` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/identity/src/login_tests.rs:31` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/identity/src/login_tests.rs:32` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/identity/src/login_tests.rs:34` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/identity/src/login_tests.rs:35` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/identity/src/login_tests.rs:36` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/identity/src/login_tests.rs:37` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/identity/src/login_tests.rs:38` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/identity/src/login_tests.rs:39` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/identity/src/login_tests.rs:40` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/identity/src/login_tests.rs:43` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/identity/src/mfa.rs:94` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/identity/src/mfa.rs:125` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/identity/src/mfa.rs:223` | FOR UPDATE; query tables accounts; sqlx::query_scalar("SELECT password_hash FROM accounts WHERE id=$1 AND status='active' AND email_verified_at IS NOT NULL FOR UPDATE").bind(i | 3 | no | n.a. | yes |
| `portal-rs/crates/identity/src/mfa.rs:612` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/identity/src/mfa.rs:644` | FOR UPDATE; query tables account_mfa_totp; let state: Option&lt;State&gt; = sqlx::query_as("SELECT secret,confirmed,last_step,generation FROM account_mfa_totp WHERE account_id=$1 FOR UPDATE | 6 | no | n.a. | yes |
| `portal-rs/crates/identity/src/mfa.rs:727` | FOR UPDATE SKIP LOCKED; query tables account_mfa_tokens; sqlx::query("DELETE FROM account_mfa_tokens WHERE token_hash IN (SELECT token_hash FROM account_mfa_tokens WHERE expires_at&lt;=now() LIMIT 100 | 6 | no | n.a. | yes |
| `portal-rs/crates/identity/src/mfa.rs:744` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/identity/src/mfa.rs:745` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/identity/src/mfa.rs:751` | FOR UPDATE; query tables accounts; sqlx::query("SELECT id FROM accounts WHERE id=$1 FOR UPDATE") | 3 | no | n.a. | yes |
| `portal-rs/crates/identity/src/mfa.rs:755` | FOR UPDATE OF e; query tables accounts, support_mfa_reset_evidence, tickets; Ok(sqlx::query_scalar::&lt;_, Uuid&gt;("SELECT e.id FROM support_mfa_reset_evidence e WHERE e.id=$1 AND e.account_id=$2 AND e.consumed_at IS NULL  | 3 | no | n.a. | yes |
| `portal-rs/crates/identity/src/mfa.rs:759` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/identity/src/mfa.rs:760` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/identity/src/mfa.rs:765` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/identity/src/mfa.rs:773` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/identity/src/mfa_http.rs:33` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/identity/src/mfa_http.rs:125` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/identity/src/mfa_http.rs:337` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/identity/src/mfa_http.rs:348` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/identity/src/mfa_http.rs:351` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/identity/src/mfa_http.rs:358` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/identity/src/mfa_http.rs:477` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/identity/src/mfa_http.rs:527` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/identity/src/mfa_http.rs:589` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/identity/src/mfa_http_tests.rs:67` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/identity/src/mfa_http_tests.rs:69` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/identity/src/mfa_tests.rs:306` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/identity/src/mfa_tests.rs:316` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/identity/src/mfa_tests.rs:375` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/identity/src/mfa_tests.rs:381` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/identity/src/oauth_mfa_web_tests.rs:1` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/identity/src/oauth_mfa_web_tests.rs:34` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/identity/src/oauth_mfa_web_tests.rs:131` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/identity/src/oauth_mfa_web_tests.rs:142` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/identity/src/passkeys.rs:128` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/identity/src/passkeys.rs:140` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/identity/src/passkeys.rs:148` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/identity/src/passkeys.rs:201` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/identity/src/passkeys.rs:233` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/identity/src/passkeys.rs:235` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/identity/src/passkeys.rs:250` | FOR UPDATE; query tables account_passkeys; let value: Option&lt;Value&gt; = sqlx::query_scalar("SELECT credential FROM account_passkeys WHERE account_id=$1 AND credential_id=$2 FOR UPDATE") | 4 / owning domain resource | no | n.a. | yes |
| `portal-rs/crates/identity/src/passkeys.rs:294` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/identity/src/passkeys.rs:303` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/identity/src/passkeys.rs:325` | FOR UPDATE; query tables accounts; let valid: Option&lt;bool&gt; = sqlx::query_scalar("SELECT status='active' AND email_verified_at IS NOT NULL AND NOT login_mfa_required FROM accou | 3 | no | n.a. | yes |
| `portal-rs/crates/identity/src/passkeys.rs:333` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/identity/src/passkeys.rs:385` | FOR UPDATE SKIP LOCKED; query tables passkey_ceremonies; sqlx::query("DELETE FROM passkey_ceremonies WHERE token_hash IN (SELECT token_hash FROM passkey_ceremonies WHERE expires_at&lt;=now() LIMIT 100 | 4 / owning domain resource | no | n.a. | yes |
| `portal-rs/crates/identity/src/passkeys_http.rs:266` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/identity/src/passkeys_http.rs:305` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/identity/src/passkeys_http.rs:309` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/identity/src/passkeys_http.rs:335` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/identity/src/passkeys_http.rs:368` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/identity/src/passkeys_tests.rs:39` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/identity/src/passkeys_tests.rs:50` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/identity/src/passkeys_tests.rs:539` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/identity/src/passkeys_tests.rs:576` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/identity/src/passkeys_tests.rs:617` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/identity/src/passkeys_tests.rs:622` | test fixture/blocker: FOR UPDATE; query tables accounts; sqlx::query("SELECT id FROM accounts WHERE id=$1 FOR UPDATE") | 3 | n.a. | n.a. | yes |
| `portal-rs/crates/identity/src/passkeys_tests.rs:696` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/identity/src/passkeys_tests.rs:714` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/identity/src/passkeys_tests.rs:728` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/identity/src/recovery_key.rs:96` | FOR UPDATE; query tables accounts; sqlx::query_as("SELECT * FROM accounts WHERE id=$1 FOR UPDATE") | 3 | no | n.a. | yes |
| `portal-rs/crates/identity/src/recovery_key.rs:104` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/identity/src/recovery_key.rs:121` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/identity/src/recovery_key.rs:145` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/identity/src/recovery_key.rs:158` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/identity/src/recovery_key.rs:192` | FOR UPDATE; query tables accounts; sqlx::query_scalar("SELECT id FROM accounts WHERE lower(email)=$1 FOR UPDATE") | 3 | no | n.a. | yes |
| `portal-rs/crates/identity/src/reset.rs:85` | FOR UPDATE OF a; query tables account_backup_emails, accounts; let id: Option&lt;AccountId&gt; = sqlx::query_scalar("SELECT a.id FROM accounts a WHERE (lower(a.email)=$1 OR EXISTS(SELECT 1 FROM account_backup_ | 3 | no | n.a. | yes |
| `portal-rs/crates/identity/src/reset.rs:121` | FOR UPDATE OF a; query tables account_backup_emails, accounts; let id: Option&lt;AccountId&gt; = sqlx::query_scalar("SELECT a.id FROM accounts a WHERE (lower(a.email)=$1 OR EXISTS(SELECT 1 FROM account_backup_ | 3 | no | n.a. | yes |
| `portal-rs/crates/identity/src/reset.rs:125` | FOR UPDATE; query tables account_backup_emails, account_password_resets; let stored: Option&lt;Vec&lt;u8&gt;&gt; = sqlx::query_scalar("SELECT token_hash FROM account_password_resets WHERE account_id=$1 AND used_at IS NULL AND | 4 / owning domain resource | no | n.a. | yes |
| `portal-rs/crates/identity/src/reset_tests.rs:183` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/identity/src/reset_tests.rs:240` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/identity/src/reset_tests.rs:330` | test fixture/blocker: FOR UPDATE; query tables accounts; sqlx::query("SELECT id FROM accounts WHERE id=$1 FOR UPDATE") | 3 | n.a. | n.a. | yes |
| `portal-rs/crates/identity/src/reset_tests.rs:347` | test fixture/blocker: FOR UPDATE; query tables accounts; sqlx::query("SELECT id FROM accounts WHERE id=$1 FOR UPDATE") | 3 | n.a. | n.a. | yes |
| `portal-rs/crates/identity/src/reset_tests.rs:352` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/identity/src/reset_tests.rs:357` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/identity/src/reset_tests.rs:359` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/identity/src/reset_tests.rs:438` | test fixture/blocker: FOR UPDATE; query tables accounts; sqlx::query("SELECT id FROM accounts WHERE id=$1 FOR UPDATE") | 3 | n.a. | n.a. | yes |
| `portal-rs/crates/identity/src/security_revocations.rs:166` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/identity/src/security_revocations.rs:171` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/identity/src/security_revocations.rs:178` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/identity/src/security_revocations_tests.rs:82` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/identity/src/security_revocations_tests.rs:542` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/identity/src/security_revocations_tests.rs:548` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/identity/src/security_revocations_tests.rs:563` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/identity/src/security_revocations_tests.rs:565` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/identity/src/security_revocations_tests.rs:583` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/identity/src/security_wizard.rs:63` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/identity/src/security_wizard.rs:179` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/identity/src/security_wizard_tests.rs:96` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/identity/src/sessions.rs:58` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/identity/src/sessions.rs:154` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/identity/src/sessions.rs:155` | FOR UPDATE; query tables accounts; let active: Option&lt;Uuid&gt; = sqlx::query_scalar("SELECT id FROM accounts WHERE id=$1 AND status='active' AND email_verified_at IS NOT NULL AND | 3 | no | n.a. | yes |
| `portal-rs/crates/identity/src/sessions.rs:160` | FOR UPDATE; query tables web_sessions; sqlx::query_scalar("SELECT session_id FROM web_sessions WHERE account_id=$1 AND id_hash=$2 AND idle_expires_at&gt;clock_timestamp() AND absolut | 6 | no | n.a. | yes |
| `portal-rs/crates/identity/src/sessions.rs:165` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/identity/src/sessions.rs:175` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/identity/src/sessions.rs:203` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/identity/src/sessions.rs:267` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/identity/src/sessions.rs:269` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/identity/src/sessions.rs:281` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/identity/src/sessions.rs:283` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/identity/src/sessions_http.rs:398` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/identity/src/sessions_http_tests.rs:145` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/identity/src/termination.rs:1` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/identity/src/termination.rs:23` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/identity/src/termination.rs:40` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/identity/src/termination.rs:131` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/identity/src/termination.rs:154` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/identity/src/termination.rs:227` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/identity/src/termination.rs:257` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/identity/src/termination.rs:380` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/identity/src/termination.rs:386` | FOR UPDATE; query tables accounts; sqlx::query_as("SELECT * FROM accounts WHERE id=$1 FOR UPDATE") | 3 | no | n.a. | yes |
| `portal-rs/crates/identity/src/termination/game_closure.rs:136` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/identity/src/termination/game_closure.rs:139` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/identity/src/termination/game_closure.rs:155` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/identity/src/termination/game_closure.rs:219` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/identity/src/termination/game_closure.rs:229` | FOR UPDATE; query tables plat3_closure_feed; "SELECT highest_epoch,invalid FROM plat3_closure_feed WHERE id=1 FOR UPDATE", | 4 / owning domain resource | no | n.a. | yes |
| `portal-rs/crates/identity/src/termination/game_closure.rs:242` | FOR UPDATE; query tables plat3_closure_snapshots; let existing:Option&lt;(Vec&lt;u8&gt;,String,String,bool)&gt;=sqlx::query_as("SELECT snapshot,epoch,revision,invalid FROM plat3_closure_snapshots WHERE  | 4 / owning domain resource | no | n.a. | yes |
| `portal-rs/crates/identity/src/termination/game_closure.rs:296` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/identity/src/termination/game_closure.rs:300` | FOR UPDATE; query tables plat3_closure_feed; "SELECT highest_epoch,watermark,invalid FROM plat3_closure_feed WHERE id=1 FOR UPDATE", | 4 / owning domain resource | no | n.a. | yes |
| `portal-rs/crates/identity/src/termination/game_closure.rs:369` | FOR UPDATE; query tables plat3_closure_work; let (generation,close,resolution):(i64,Option&lt;Uuid&gt;,Option&lt;String&gt;)=sqlx::query_as("SELECT generation,close_operation_id,resolution FROM pla | 4 / owning domain resource | no | n.a. | yes |
| `portal-rs/crates/identity/src/termination/game_closure.rs:431` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/identity/src/termination/game_closure.rs:437` | FOR UPDATE; query tables plat3_closure_lifecycle; "SELECT generation FROM plat3_closure_lifecycle WHERE account_id=$1 FOR UPDATE", | 4 / owning domain resource | no | n.a. | yes |
| `portal-rs/crates/identity/src/termination/game_closure.rs:459` | FOR UPDATE; query tables plat3_closure_feed; sqlx::query("SELECT id FROM plat3_closure_feed WHERE id=1 FOR UPDATE") | 4 / owning domain resource | no | n.a. | yes |
| `portal-rs/crates/identity/src/termination/game_closure.rs:533` | FOR UPDATE OF w,l; query tables plat3_closure_lifecycle, plat3_closure_work; let (generation,operation):(i64,Uuid)=sqlx::query_as("SELECT w.generation,w.close_operation_id FROM plat3_closure_work w JOIN plat3_closure_ | 4 / owning domain resource | no | n.a. | yes |
| `portal-rs/crates/identity/src/termination/game_closure.rs:555` | FOR UPDATE; query tables plat3_closure_feed; let (epoch,watermark,signature,invalid):(String,Option&lt;Vec&lt;u8&gt;&gt;,Option&lt;Vec&lt;u8&gt;&gt;,bool)=sqlx::query_as("SELECT highest_epoch,watermark,signatu | 4 / owning domain resource | no | n.a. | yes |
| `portal-rs/crates/identity/src/termination/game_closure.rs:573` | FOR UPDATE; query tables plat3_closure_snapshots; let (raw,signature,invalid):(Vec&lt;u8&gt;,Vec&lt;u8&gt;,bool)=sqlx::query_as("SELECT snapshot,signature,invalid FROM plat3_closure_snapshots WHERE acco | 4 / owning domain resource | no | n.a. | yes |
| `portal-rs/crates/identity/src/termination/game_closure.rs:595` | FOR UPDATE; query tables account_character_feed; sqlx::query("SELECT id FROM account_character_feed WHERE id=1 FOR UPDATE") | 4 / owning domain resource | no | n.a. | yes |
| `portal-rs/crates/identity/src/termination/game_closure.rs:599` | FOR UPDATE; query tables account_character_snapshots; "SELECT account_id FROM account_character_snapshots WHERE account_id=$1 FOR UPDATE", | 4 / owning domain resource | no | n.a. | yes |
| `portal-rs/crates/identity/src/termination/game_closure.rs:617` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/identity/src/termination/game_closure.rs:625` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/identity/src/termination/game_closure/http.rs:133` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/identity/src/termination/game_closure/http.rs:135` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/identity/src/termination/game_closure/http.rs:137` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/identity/src/termination/game_closure/http.rs:140` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/identity/src/termination/game_closure/http.rs:144` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/identity/src/termination/game_closure/http.rs:153` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/identity/src/termination/game_closure/runtime.rs:139` | FOR SHARE; query tables character_restore_locks; sqlx::query("SELECT id FROM character_restore_locks WHERE id=1 FOR SHARE") | 4 / owning domain resource | no | n.a. | yes |
| `portal-rs/crates/identity/src/termination/game_closure/runtime.rs:167` | FOR UPDATE; query tables account_terminations; "SELECT * FROM account_terminations WHERE id=$1 AND account_id=$2 FOR UPDATE", | 4 / owning domain resource | no | n.a. | yes |
| `portal-rs/crates/identity/src/termination/game_closure/runtime.rs:174` | FOR UPDATE OF w,l; query tables plat3_closure_lifecycle, plat3_closure_work; let (generation, current, resolution): (i64,i64,Option&lt;String&gt;) = sqlx::query_as("SELECT w.generation,l.generation,w.resolution FROM plat3_c | 4 / owning domain resource | no | n.a. | yes |
| `portal-rs/crates/identity/src/termination/game_closure/runtime.rs:196` | FOR UPDATE; query tables wallet_accounts; sqlx::query("SELECT account_id FROM wallet_accounts WHERE account_id=$1 FOR UPDATE") | 7 / domain namespace | no | n.a. | yes |
| `portal-rs/crates/identity/src/termination/game_closure/tests.rs:46` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/identity/src/termination/game_closure/tests.rs:56` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/identity/src/termination/game_closure/tests.rs:803` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/identity/src/termination/game_closure/tests.rs:823` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/identity/src/termination/game_closure/tests.rs:951` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/identity/src/termination_tests.rs:70` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/identity/src/termination_tests.rs:80` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/identity/src/termination_tests.rs:184` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/identity/src/termination_tests.rs:200` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/identity/src/termination_tests.rs:284` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/identity/src/termination_tests.rs:295` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/identity/src/verify.rs:43` | FOR UPDATE; query tables accounts; sqlx::query("SELECT id FROM accounts WHERE id=$1 FOR UPDATE") | 3 | no | n.a. | yes |
| `portal-rs/crates/identity/src/verify.rs:56` | FOR UPDATE; query tables accounts; let locale:Option&lt;String&gt;=sqlx::query_scalar("SELECT locale FROM accounts WHERE id=$1 AND status='pending_verification' AND email_verified_a | 3 | no | n.a. | yes |
| `portal-rs/crates/internal-api/src/guard.rs:104` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/internal-api/src/guard.rs:238` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/internal-api/tests/contracts.rs:117` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/internal-api/tests/contracts.rs:151` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/minigames/src/collection/http.rs:167` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/minigames/src/collection/store.rs:50` | FOR UPDATE; query tables accounts; "SELECT status,email_verified_at IS NOT NULL FROM accounts WHERE id=$1 FOR UPDATE", | 3 | no | n.a. | yes |
| `portal-rs/crates/minigames/src/collection/store.rs:255` | FOR UPDATE; query tables collection_decks; let current: (Uuid,Vec&lt;Uuid&gt;) = sqlx::query_as("SELECT character_id,cards FROM collection_decks WHERE id=$1 AND account_id=$2 FOR UPDATE").b | 4 / owning domain resource | no | n.a. | yes |
| `portal-rs/crates/minigames/src/collection/store.rs:320` | FOR UPDATE; query tables collection_battles; "SELECT state FROM collection_battles WHERE id=$1 AND account_id=$2 FOR UPDATE", | 4 / owning domain resource | no | n.a. | yes |
| `portal-rs/crates/minigames/src/collection_pvp/mod.rs:136` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/minigames/src/collection_pvp/mod.rs:140` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/minigames/src/collection_pvp/mod.rs:143` | FOR SHARE; query tables accounts; sqlx::query("SELECT id FROM accounts WHERE id=ANY($1) ORDER BY id FOR SHARE") | 3 | yes | n.a. | fixed in this commit |
| `portal-rs/crates/minigames/src/collection_pvp/mod.rs:153` | pg_advisory_xact_lock; sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1,92))") | domain namespace (README inventory) | yes | n.a. | fixed in this commit |
| `portal-rs/crates/minigames/src/collection_pvp/mod.rs:191` | FOR UPDATE; query tables collection_pvp_battles; let row: (Uuid, Value, String) = sqlx::query_as("SELECT right_account,left_deck,status FROM collection_pvp_battles WHERE id=$1 FOR UPDATE") | 4 / owning domain resource | yes | n.a. | fixed in this commit |
| `portal-rs/crates/minigames/src/collection_pvp/mod.rs:255` | FOR UPDATE; query tables collection_pvp_tournaments; sqlx::query("SELECT id FROM collection_pvp_tournaments WHERE id=$1 FOR UPDATE") | 4 / owning domain resource | yes | n.a. | fixed in this commit |
| `portal-rs/crates/minigames/src/collection_pvp/mod.rs:260` | FOR UPDATE; query tables collection_pvp_battles; let battle: Battle = sqlx::query_as("SELECT id,left_account,right_account,status,revision,state,winner,tournament_id FROM collection_pvp_bat | 4 / owning domain resource | yes | n.a. | fixed in this commit |
| `portal-rs/crates/minigames/src/collection_pvp/mod.rs:282` | FOR UPDATE; query tables collection_pvp_tournaments; sqlx::query("SELECT id FROM collection_pvp_tournaments WHERE id=$1 FOR UPDATE") | 4 / owning domain resource | yes | n.a. | fixed in this commit |
| `portal-rs/crates/minigames/src/collection_pvp/mod.rs:343` | FOR UPDATE; query tables collection_pvp_tournaments; "SELECT status,capacity FROM collection_pvp_tournaments WHERE id=$1 FOR UPDATE", | 4 / owning domain resource | yes | n.a. | fixed in this commit |
| `portal-rs/crates/minigames/src/collection_pvp/mod.rs:378` | FOR UPDATE; query tables collection_pvp_tournaments; "SELECT organizer,status FROM collection_pvp_tournaments WHERE id=$1 FOR UPDATE", | 4 / owning domain resource | yes | n.a. | fixed in this commit |
| `portal-rs/crates/minigames/src/collection_pvp/mod.rs:437` | pg_advisory_xact_lock; sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1,91))") | domain namespace (README inventory) | yes | n.a. | fixed in this commit |
| `portal-rs/crates/minigames/src/collection_pvp/mod.rs:454` | FOR SHARE; query tables collection_pvp_sets; let offer: PublishedSet=sqlx::query_as("SELECT id,version,price,cards FROM collection_pvp_sets WHERE id=$1 AND published FOR SHARE").bind(se | 4 / owning domain resource | yes | n.a. | fixed in this commit |
| `portal-rs/crates/minigames/src/collection_pvp/mod.rs:553` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/minigames/src/collection_pvp/tests.rs:461` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/minigames/src/collection_pvp/tests.rs:465` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/minigames/src/collection_pvp/tests.rs:466` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/minigames/src/collection_pvp/tests.rs:527` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/minigames/src/collection_pvp/tests.rs:587` | test fixture/blocker: FOR UPDATE; query tables accounts; sqlx::query("SELECT id FROM accounts WHERE id=$1 FOR UPDATE") | 3 | n.a. | n.a. | yes |
| `portal-rs/crates/minigames/src/collection_pvp/tests.rs:612` | test fixture/blocker: FOR UPDATE; query tables accounts; let later_lock = sqlx::query("SELECT id FROM accounts WHERE id=$1 FOR UPDATE") | 3 | n.a. | n.a. | yes |
| `portal-rs/crates/minigames/src/league/README.md:43` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/minigames/src/league/fixtures/CROSS_REPOSITORY_CONTRACT_LOCK.json:59` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/minigames/src/league/fixtures/CROSS_REPOSITORY_CONTRACT_LOCK.json:63` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/minigames/src/league/fixtures/CROSS_REPOSITORY_CONTRACT_LOCK.json:98` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/minigames/src/league/fixtures/CROSS_REPOSITORY_CONTRACT_LOCK.json:110` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/minigames/src/league/fixtures/CROSS_REPOSITORY_CONTRACT_LOCK.json:114` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/minigames/src/league/fixtures/CROSS_REPOSITORY_CONTRACT_LOCK.json:149` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/minigames/src/league/fixtures/GAME_EVENT_FOUNDATION_REGISTRY.json:49` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/minigames/src/league/fixtures/GAME_EVENT_FOUNDATION_REGISTRY.json:58` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/minigames/src/league/fixtures/GAME_EVENT_FOUNDATION_REGISTRY.json:59` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/minigames/src/league/fixtures/GAME_EVENT_FOUNDATION_REGISTRY.json:69` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/minigames/src/league/fixtures/GAME_EVENT_FOUNDATION_REGISTRY.json:81` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/minigames/src/league/fixtures/GAME_EVENT_FOUNDATION_REGISTRY.json:82` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/minigames/src/league/fixtures/GAME_EVENT_FOUNDATION_REGISTRY.json:83` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/minigames/src/league/fixtures/GAME_EVENT_FOUNDATION_REGISTRY.json:96` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/minigames/src/league/fixtures/GAME_EVENT_FOUNDATION_REGISTRY.json:97` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/minigames/src/league/fixtures/GAME_EVENT_FOUNDATION_REGISTRY.json:98` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/minigames/src/league/fixtures/GAME_EVENT_FOUNDATION_REGISTRY.json:111` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/minigames/src/league/fixtures/GAME_EVENT_FOUNDATION_REGISTRY.json:112` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/minigames/src/league/fixtures/GAME_EVENT_FOUNDATION_REGISTRY.json:113` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/minigames/src/league/fixtures/GAME_EVENT_FOUNDATION_REGISTRY.json:127` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/minigames/src/league/fixtures/GAME_EVENT_FOUNDATION_REGISTRY.json:128` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/minigames/src/league/fixtures/GAME_EVENT_FOUNDATION_REGISTRY.json:129` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/minigames/src/league/fixtures/GAME_EVENT_FOUNDATION_REGISTRY.json:136` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/minigames/src/league/fixtures/GAME_EVENT_FOUNDATION_REGISTRY.json:138` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/minigames/src/league/fixtures/GAME_EVENT_FOUNDATION_REGISTRY.json:143` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/minigames/src/league/fixtures/GAME_EVENT_FOUNDATION_REGISTRY.json:144` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/minigames/src/league/fixtures/GAME_EVENT_FOUNDATION_REGISTRY.json:151` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/minigames/src/league/fixtures/GAME_EVENT_FOUNDATION_REGISTRY.json:218` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/minigames/src/league/fixtures/GAME_EVENT_FOUNDATION_REGISTRY.json:220` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/minigames/src/league/fixtures/GAME_EVENT_FOUNDATION_REGISTRY.json:221` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/minigames/src/league/fixtures/GAME_EVENT_FOUNDATION_REGISTRY.json:225` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/minigames/src/league/fixtures/plat3_authority.proto:8` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/minigames/src/league/runtime.rs:41` | FOR SHARE; query tables league_feed_state; let r=sqlx::query("SELECT epoch::text,recovery_generation::text,ready,conflicted,event_complete,season_complete FROM league_feed_state WHERE | 4 / owning domain resource | no | n.a. | yes |
| `portal-rs/crates/minigames/src/league/runtime.rs:195` | pg_advisory_xact_lock; sqlx::query("SELECT pg_advisory_xact_lock(94000003)") | domain namespace (README inventory) | no | n.a. | yes |
| `portal-rs/crates/minigames/src/league/runtime.rs:200` | FOR UPDATE; query tables league_event_questions; let prior:Option&lt;String&gt;=sqlx::query_scalar("SELECT question_hash FROM league_event_questions WHERE authority=$1 AND world_id=$2 AND subject | 4 / owning domain resource | no | n.a. | yes |
| `portal-rs/crates/minigames/src/league/runtime.rs:241` | FOR KEY SHARE; query tables world_status; sqlx::query_scalar("SELECT id FROM world_status WHERE id=$1 FOR KEY SHARE") | 4 / owning domain resource | no | n.a. | yes |
| `portal-rs/crates/minigames/src/league/runtime.rs:258` | pg_advisory_xact_lock; sqlx::query("SELECT pg_advisory_xact_lock(94000003)") | domain namespace (README inventory) | no | n.a. | yes |
| `portal-rs/crates/minigames/src/league/runtime.rs:261` | FOR UPDATE; query tables league_feed_state; let old:Option&lt;(String,String)&gt;=sqlx::query_as("SELECT epoch::text,recovery_generation::text FROM league_feed_state WHERE authority=$1 FOR U | 4 / owning domain resource | no | n.a. | yes |
| `portal-rs/crates/minigames/src/league/runtime.rs:305` | pg_advisory_xact_lock; sqlx::query("SELECT pg_advisory_xact_lock(94000003)") | domain namespace (README inventory) | no | n.a. | yes |
| `portal-rs/crates/minigames/src/league/runtime.rs:309` | FOR UPDATE; query tables league_feed_requests; "SELECT fingerprint,state FROM league_feed_requests WHERE request_id=$1 FOR UPDATE", | 4 / owning domain resource | no | n.a. | yes |
| `portal-rs/crates/minigames/src/league/runtime.rs:418` | pg_try_advisory_lock; let owns: bool = sqlx::query_scalar("SELECT pg_try_advisory_lock(94000004)") | domain namespace (README inventory) | no | n.a. | yes |
| `portal-rs/crates/minigames/src/league/runtime.rs:466` | pg_advisory_xact_lock; sqlx::query("SELECT pg_advisory_xact_lock(94000003)") | domain namespace (README inventory) | no | n.a. | yes |
| `portal-rs/crates/minigames/src/league/runtime.rs:534` | pg_advisory_xact_lock; sqlx::query("SELECT pg_advisory_xact_lock(94000003)") | domain namespace (README inventory) | no | n.a. | yes |
| `portal-rs/crates/minigames/src/league/runtime.rs:540` | FOR UPDATE; query tables league_subjects; let old: Option&lt;(String, String)&gt; = sqlx::query_as("SELECT epoch::text,revision::text FROM league_subjects WHERE authority=$1 AND family=$2  | 4 / owning domain resource | no | n.a. | yes |
| `portal-rs/crates/minigames/src/league/runtime.rs:571` | pg_advisory_xact_lock; sqlx::query("SELECT pg_advisory_xact_lock(94000003)") | domain namespace (README inventory) | no | n.a. | yes |
| `portal-rs/crates/minigames/src/league/runtime.rs:577` | FOR UPDATE; query tables league_feed_state; "SELECT event_complete FROM league_feed_state WHERE authority=$1 FOR UPDATE" | 4 / owning domain resource | no | n.a. | yes |
| `portal-rs/crates/minigames/src/league/runtime.rs:579` | FOR UPDATE; query tables league_feed_state; "SELECT season_complete FROM league_feed_state WHERE authority=$1 FOR UPDATE" | 4 / owning domain resource | no | n.a. | yes |
| `portal-rs/crates/minigames/src/league/runtime.rs:603` | pg_advisory_xact_lock; sqlx::query("SELECT pg_advisory_xact_lock(94000003)") | domain namespace (README inventory) | no | n.a. | yes |
| `portal-rs/crates/minigames/src/league/runtime.rs:606` | FOR UPDATE; query tables league_feed_state; let r = sqlx::query("SELECT conflicted,event_complete,season_complete FROM league_feed_state WHERE authority=$1 FOR UPDATE").bind(&c.authori | 4 / owning domain resource | no | n.a. | yes |
| `portal-rs/crates/minigames/src/league/runtime.rs:651` | pg_advisory_xact_lock; sqlx::query("SELECT pg_advisory_xact_lock(94000003)") | domain namespace (README inventory) | no | n.a. | yes |
| `portal-rs/crates/minigames/src/league/store.rs:89` | pg_advisory_xact_lock; sqlx::query("SELECT pg_advisory_xact_lock(94000003)") | domain namespace (README inventory) | no | n.a. | yes |
| `portal-rs/crates/minigames/src/league/store.rs:116` | FOR UPDATE; query tables league_subjects; let old = sqlx::query("SELECT wire,epoch::text,revision::text,recovery_generation::text,record_hash,content_hash,final_state,finalized_at,qu | 4 / owning domain resource | no | n.a. | yes |
| `portal-rs/crates/minigames/src/league/store.rs:204` | pg_advisory_xact_lock_shared; sqlx::query("SELECT pg_advisory_xact_lock_shared(94000003)") | domain namespace (README inventory) | no | n.a. | yes |
| `portal-rs/crates/minigames/src/league/store.rs:265` | pg_advisory_xact_lock; sqlx::query("SELECT pg_advisory_xact_lock(94000003)") | domain namespace (README inventory) | no | n.a. | yes |
| `portal-rs/crates/minigames/src/league/tests.rs:1296` | test fixture/blocker: pg_advisory_xact_lock; sqlx::query("SELECT pg_advisory_xact_lock(94000003)") | domain namespace (README inventory) | n.a. | n.a. | yes |
| `portal-rs/crates/minigames/src/league/transport.rs:72` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/minigames/src/league/transport.rs:75` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/minigames/src/league/transport.rs:198` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/minigames/src/league/transport.rs:205` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/minigames/src/league/transport.rs:206` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/minigames/src/league/transport.rs:276` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/minigames/src/league/transport.rs:367` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/minigames/src/pass/mod.rs:217` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/minigames/src/pass/repository.rs:29` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/minigames/src/pass/repository.rs:74` | FOR SHARE; query tables pass_seasons; sqlx::query_scalar("SELECT price FROM pass_seasons WHERE id=$1 FOR SHARE") | 4 / owning domain resource | no | n.a. | yes |
| `portal-rs/crates/minigames/src/pass/repository.rs:82` | FOR UPDATE; query tables pass_progress; "SELECT xp,paid FROM pass_progress WHERE account_id=$1 AND season_id=$2 FOR UPDATE", | 4 / owning domain resource | no | n.a. | yes |
| `portal-rs/crates/minigames/src/pass/tests.rs:454` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/minigames/src/pass/tests.rs:458` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/minigames/src/pass/tests.rs:459` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/minigames/src/pass/tests.rs:694` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/minigames/src/pass/web.rs:200` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/minigames/src/pass/web.rs:236` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/minigames/src/pass/web.rs:253` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/minigames/src/puzzles/http.rs:156` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/minigames/src/puzzles/mod.rs:1` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/minigames/src/puzzles/store.rs:154` | FOR UPDATE; query tables accounts; "SELECT status,email_verified_at IS NOT NULL FROM accounts WHERE id=$1 FOR UPDATE", | 3 | no | n.a. | yes |
| `portal-rs/crates/minigames/src/puzzles/store.rs:168` | FOR UPDATE; query tables puzzles_streaks; sqlx::query("SELECT account_id FROM puzzles_streaks WHERE account_id=$1 FOR UPDATE") | 4 / owning domain resource | no | n.a. | yes |
| `portal-rs/crates/minigames/src/puzzles/tests.rs:710` | test fixture/blocker: FOR UPDATE; query tables puzzles_streaks; sqlx::query("SELECT account_id FROM puzzles_streaks WHERE account_id=$1 FOR UPDATE") | 4 / owning domain resource | n.a. | n.a. | yes |
| `portal-rs/crates/notify/src/service.rs:265` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/notify/src/service.rs:267` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/notify/src/service.rs:289` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/notify/src/service.rs:304` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/notify/src/service.rs:356` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/payments/README.md:34` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/payments/README.md:81` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/payments/README.md:90` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/payments/src/account_tests.rs:334` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/payments/src/coin_http.rs:26` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/payments/src/coin_http.rs:72` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/payments/src/coin_http.rs:74` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/payments/src/coin_http.rs:100` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/payments/src/coin_http.rs:102` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/payments/src/coin_http.rs:108` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/payments/src/coin_store.rs:27` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/payments/src/coin_store.rs:140` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/payments/src/coin_store.rs:145` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/payments/src/coin_store.rs:154` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/payments/src/coin_store.rs:274` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/payments/src/coin_store.rs:373` | pg_advisory_xact_lock; sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1::text, 90105))") | domain namespace (README inventory) | no | n.a. | yes |
| `portal-rs/crates/payments/src/coin_store.rs:377` | FOR UPDATE OF q; query tables coin_purchase_orders, coin_purchase_quotes; sqlx::query("DELETE FROM coin_purchase_quotes WHERE id IN (SELECT q.id FROM coin_purchase_quotes q WHERE q.payer_account_id=$1 AND q.consume | 4 / owning domain resource | no | n.a. | yes |
| `portal-rs/crates/payments/src/coin_store.rs:496` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/payments/src/coin_store.rs:542` | pg_advisory_xact_lock; sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1::text, 90105))") | domain namespace (README inventory) | no | n.a. | yes |
| `portal-rs/crates/payments/src/coin_store.rs:558` | FOR UPDATE; query tables coin_purchase_quotes; let quote = sqlx::query("SELECT payer_account_id,character_id,package_id,coins,minor_units,currency,exponent,environment,consumed_at IS NOT  | 4 / owning domain resource | no | n.a. | yes |
| `portal-rs/crates/payments/src/coin_store.rs:694` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/payments/src/coin_store.rs:706` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/payments/src/coin_store.rs:713` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/payments/src/coin_store.rs:727` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/payments/src/coin_store.rs:798` | FOR UPDATE; query tables payment_intents; let intent = sqlx::query("SELECT account_id,amount,currency,state,provider_reference FROM payment_intents WHERE id=$1 AND provider=$2 FOR UP | 4 / owning domain resource | yes | n.a. | fixed in this commit |
| `portal-rs/crates/payments/src/coin_store.rs:806` | FOR UPDATE; query tables coin_purchase_orders; " FROM coin_purchase_orders WHERE payment_intent_id=$1 FOR UPDATE" | 4 / owning domain resource | yes | n.a. | fixed in this commit |
| `portal-rs/crates/payments/src/coin_store.rs:897` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/payments/src/coin_store.rs:1004` | FOR UPDATE; query tables wallet_accounts; "SELECT account_id FROM wallet_accounts WHERE account_id=$1 FOR UPDATE", | 7 / domain namespace | no | n.a. | yes |
| `portal-rs/crates/payments/src/coin_store.rs:1011` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/payments/src/coin_store.rs:1131` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/payments/src/coin_store.rs:1137` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/payments/src/coin_store.rs:1140` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/payments/src/coin_store.rs:1153` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/payments/src/core.rs:43` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/payments/src/core.rs:49` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/payments/src/core.rs:54` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/payments/src/provider_webhook.rs:339` | pg_advisory_xact_lock; "SELECT pg_advisory_xact_lock(hashtextextended('payment_provider_webhook:' \|\| $1, 0))", | domain namespace (README inventory) | no | n.a. | yes |
| `portal-rs/crates/payments/src/provider_webhook.rs:422` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/payments/src/provider_webhook.rs:459` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/payments/src/storage.rs:25` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/payments/src/storage.rs:42` | FOR SHARE; query tables accounts; sqlx::query("SELECT id FROM accounts WHERE id=$1 FOR SHARE") | 3 | yes | n.a. | fixed in this commit |
| `portal-rs/crates/payments/src/storage.rs:173` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/payments/src/storage.rs:174` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/payments/src/storage.rs:180` | FOR SHARE; query tables payment_intents; let row = sqlx::query("SELECT account_id, amount, currency, state, provider_reference FROM payment_intents WHERE id=$1 AND provider=$2 FOR S | 4 / owning domain resource | no | n.a. | yes |
| `portal-rs/crates/payments/src/storage.rs:183` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/payments/src/storage.rs:213` | FOR UPDATE; query tables payment_intents; let row = sqlx::query("SELECT account_id, amount, currency, state, provider_reference FROM payment_intents WHERE id=$1 AND provider=$2 FOR U | 4 / owning domain resource | yes | n.a. | fixed in this commit |
| `portal-rs/crates/payments/src/storage.rs:268` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/payments/src/storage.rs:270` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/payments/tests/payments.rs:15` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/payments/tests/provider_webhook.rs:39` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/payments/tests/provider_webhook.rs:59` | test fixture/blocker: pg_advisory_xact_lock; sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1::text, 90101))") | domain namespace (README inventory) | n.a. | n.a. | yes |
| `portal-rs/crates/payments/tests/provider_webhook.rs:89` | test fixture/blocker: pg_advisory_xact_lock; sqlx::query("SELECT pg_advisory_xact_lock(hashtext($1::text))") | domain namespace (README inventory) | n.a. | n.a. | yes |
| `portal-rs/crates/payments/tests/provider_webhook.rs:513` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/payments/tests/provider_webhook.rs:519` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/player-api/README.md:49` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/player-api/src/account_feed.rs:162` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/player-api/src/account_feed.rs:164` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/player-api/src/account_feed.rs:166` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/player-api/src/account_feed.rs:172` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/player-api/src/account_feed.rs:178` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/player-api/src/client_fixtures_tests.rs:232` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/player-api/src/lib.rs:71` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/player-api/src/lib.rs:73` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/player-api/src/lib.rs:75` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/player-api/src/lib.rs:77` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/player-api/src/lib.rs:83` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/player-api/src/lib.rs:99` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/player-api/src/lib.rs:153` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/player-api/src/lib.rs:172` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/player-api/src/tests.rs:97` | test fixture/blocker: FOR UPDATE; query tables test_oauth; sqlx::query("SELECT active, revoked FROM test_oauth WHERE token = $1 FOR UPDATE") | 4 / owning domain resource | n.a. | n.a. | yes |
| `portal-rs/crates/player-api/src/tests.rs:102` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/player-api/src/tests.rs:107` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/player-api/src/tests.rs:212` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/player-api/src/tests.rs:235` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/player-api/src/tests.rs:300` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/player-api/src/tests.rs:312` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/player-api/src/tests.rs:419` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/player-api/tests/account_feed.rs:62` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/player-api/tests/account_feed.rs:87` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/player-api/tests/account_feed.rs:98` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/playerdata/src/authority.rs:85` | pg_advisory_xact_lock_shared; sqlx::query("SELECT pg_advisory_xact_lock_shared($1)") | domain namespace (README inventory) | no | n.a. | fixed in parallel task drc-a |
| `portal-rs/crates/playerdata/src/authority.rs:89` | FOR SHARE; query tables character_restore_locks; sqlx::query("SELECT id FROM character_restore_locks WHERE id=1 FOR SHARE") | 4 / owning domain resource | no | n.a. | fixed in parallel task drc-a |
| `portal-rs/crates/playerdata/src/authority.rs:114` | FOR SHARE; query tables accounts; sqlx::query_scalar("SELECT status FROM accounts WHERE id=$1 FOR SHARE") | 3 | no | n.a. | fixed in parallel task drc-a |
| `portal-rs/crates/playerdata/src/authority.rs:128` | FOR SHARE; query tables account_character_snapshots; let row = sqlx::query("SELECT projection_epoch,invalid,source_authority FROM account_character_snapshots WHERE account_id=$1 FOR SHARE") | 4 / owning domain resource | no | n.a. | fixed in parallel task drc-a |
| `portal-rs/crates/playerdata/src/authority.rs:144` | FOR SHARE; query tables account_character_feed; let feed = sqlx::query("SELECT highest_epoch,watermark_epoch,source_authority,complete_through FROM account_character_feed WHERE id=1 FOR SH | 4 / owning domain resource | no | n.a. | fixed in parallel task drc-a |
| `portal-rs/crates/playerdata/src/authority.rs:225` | FOR SHARE; query tables character_commands; let created = sqlx::query("SELECT receipt_json FROM character_commands WHERE command='CreateCharacter' AND state='TERMINAL' AND receipt_json | 4 / owning domain resource | no | n.a. | fixed in parallel task drc-a |
| `portal-rs/crates/playerdata/src/authority.rs:234` | FOR SHARE; query tables accepted_at, pd_ownership_baselines; let base = sqlx::query("SELECT account_id,world_id,character_revision::text AS cr,from_projection_revision::text AS f,to_projection_revision | 4 / owning domain resource | no | n.a. | fixed in parallel task drc-a |
| `portal-rs/crates/playerdata/src/authority.rs:255` | FOR SHARE; query tables character_commands; let rows = sqlx::query("SELECT state,receipt_json FROM character_commands WHERE command='TransferCharacterOwnership' AND binding_json::jsonb | 4 / owning domain resource | no | n.a. | fixed in parallel task drc-a |
| `portal-rs/crates/playerdata/src/authority.rs:284` | FOR SHARE OF c, s; query tables account_character_snapshots, account_characters; let projection = sqlx::query("SELECT c.account_id,c.world_id,s.projection_epoch,s.projection_revision,s.invalid FROM account_characters c JO | 4 / owning domain resource | no | n.a. | fixed in parallel task drc-a |
| `portal-rs/crates/playerdata/src/authority/reads.rs:102` | FOR SHARE; query tables pd_streams; let row = sqlx::query("SELECT published_through::text AS through,observed_ms,baseline_complete,watermark_recovery::text AS recovery,erased F | 4 / owning domain resource | no | n.a. | yes |
| `portal-rs/crates/playerdata/src/authority/reads.rs:357` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/playerdata/src/authority/tests.rs:495` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/playerdata/src/authority/tests.rs:501` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/playerdata/src/authority/tests.rs:508` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/playerdata/src/authority/tests.rs:1004` | test fixture/blocker: for update; query tables in; for update in [ | 4 / owning domain resource | n.a. | n.a. | yes |
| `portal-rs/crates/playerdata/src/authority/tests.rs:1079` | test fixture/blocker: pg_advisory_xact_lock; db.exec("CREATE FUNCTION pause_login_promotion() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN IF NEW.source_revision=1 THEN PERFORM pg_advis | domain namespace (README inventory) | n.a. | n.a. | yes |
| `portal-rs/crates/playerdata/src/authority/tests.rs:1082` | test fixture/blocker: pg_advisory_xact_lock; sqlx::query("SELECT pg_advisory_xact_lock(326787645)") | domain namespace (README inventory) | n.a. | n.a. | yes |
| `portal-rs/crates/playerdata/src/dashboard/README.md:9` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/playerdata/src/dashboard/README.md:44` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/playerdata/src/dashboard/mod.rs:23` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/playerdata/src/dashboard/mod.rs:26` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/playerdata/src/dashboard/mod.rs:27` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/playerdata/src/dashboard/mod.rs:94` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/playerdata/src/dashboard/mod.rs:129` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/playerdata/src/dashboard/mod.rs:235` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/playerdata/src/dashboard/mod.rs:238` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/playerdata/src/dashboard/mod.rs:253` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/playerdata/src/deaths/README.md:3` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/playerdata/src/deaths/http.rs:122` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/playerdata/src/deaths/http.rs:199` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/playerdata/src/deaths/storage.rs:14` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/playerdata/src/deaths/storage.rs:239` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/playerdata/src/deaths/storage.rs:258` | FOR SHARE; query tables pd_t78_streams; let row = sqlx::query("SELECT complete_through::text,observed_at_ms,baseline_complete FROM pd_t78_streams WHERE source_authority=$1 AND acco | 4 / owning domain resource | no | n.a. | yes |
| `portal-rs/crates/playerdata/src/deaths/storage.rs:331` | FOR UPDATE; query tables pd_t78_streams; sqlx::query("SELECT 1 FROM pd_t78_streams WHERE source_authority=$1 AND account_id=$2 AND projection_epoch=$3::text::numeric AND recovery_ge | 4 / owning domain resource | no | n.a. | yes |
| `portal-rs/crates/playerdata/src/deaths/tests.rs:219` | test fixture/blocker: FOR SHARE; query tables test_authority; sqlx::query_as("SELECT allowed FROM test_authority WHERE id=1 FOR SHARE") | 4 / owning domain resource | n.a. | n.a. | yes |
| `portal-rs/crates/playerdata/src/deaths/tests.rs:384` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/playerdata/src/deaths/tests.rs:616` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/playerdata/src/deaths/tests.rs:624` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/playerdata/src/drops.rs:242` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/playerdata/src/hunts.rs:43` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/playerdata/src/hunts_store.rs:46` | FOR UPDATE; query tables pd_hunt_sessions; let same: bool = sqlx::query_scalar("SELECT stream_generation=$3 AND character_revision=$4::text::numeric AND expires_at=$5 AND facts=$6 FRO | 4 / owning domain resource | no | n.a. | yes |
| `portal-rs/crates/playerdata/src/items/README.md:16` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/playerdata/src/items/store.rs:83` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/playerdata/src/items/store.rs:87` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/playerdata/src/items/store.rs:109` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/playerdata/src/items/store.rs:207` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/playerdata/src/items/store.rs:215` | FOR UPDATE; query tables pd_t76_streams; let stream=sqlx::query("SELECT fenced,cursor::text,retention_floor::text,coverage_start,staging_start::text,staging_at,last_bank_entry,last_ | 4 / owning domain resource | no | n.a. | yes |
| `portal-rs/crates/playerdata/src/items/store.rs:390` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/playerdata/src/items/store.rs:467` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/playerdata/src/items/tests.rs:302` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/playerdata/src/items/tests.rs:492` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/playerdata/src/items/tests.rs:505` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/playerdata/src/items/tests.rs:790` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/playerdata/src/items/web.rs:182` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/playerdata/src/items/web.rs:271` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/playerdata/src/journal_tests.rs:359` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/playerdata/src/journal_tests.rs:371` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/playerdata/src/journal_tests.rs:385` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/playerdata/src/journal_tests.rs:909` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/playerdata/src/journal_tests.rs:946` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/playerdata/src/logins.rs:261` | FOR UPDATE; query tables pd_game_logins; let old: Option&lt;ExistingLogin&gt; = sqlx::query_as("SELECT entered_at,left_at,character_id,end_reason FROM pd_game_logins WHERE account_id=$1 A | 4 / owning domain resource | no | n.a. | yes |
| `portal-rs/crates/playerdata/src/logins.rs:300` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/playerdata/src/logins/tests.rs:158` | test fixture/blocker: FOR UPDATE; query tables accounts; "SELECT id FROM accounts WHERE id=$1 AND status='active' FOR UPDATE", | 3 | n.a. | n.a. | yes |
| `portal-rs/crates/playerdata/src/logins/tests.rs:532` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/playerdata/src/logins/tests.rs:554` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/playerdata/src/logins/tests.rs:561` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/playerdata/src/places/README.md:25` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/playerdata/src/places/retention.rs:10` | pg_advisory_xact_lock; sqlx::query("SELECT pg_advisory_xact_lock(81005230117)") | domain namespace (README inventory) | no | n.a. | yes |
| `portal-rs/crates/playerdata/src/places/store.rs:220` | pg_advisory_xact_lock; sqlx::query("SELECT pg_advisory_xact_lock(81005230117)") | domain namespace (README inventory) | no | n.a. | yes |
| `portal-rs/crates/playerdata/src/places/store.rs:244` | pg_advisory_xact_lock; sqlx::query("SELECT pg_advisory_xact_lock(81005230117)") | domain namespace (README inventory) | no | n.a. | yes |
| `portal-rs/crates/playerdata/src/places/store.rs:289` | pg_advisory_xact_lock; sqlx::query("SELECT pg_advisory_xact_lock(81005230117)") | domain namespace (README inventory) | no | n.a. | yes |
| `portal-rs/crates/playerdata/src/places/tests.rs:100` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/playerdata/src/places/tests.rs:136` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/playerdata/src/places/tests.rs:190` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/playerdata/src/places/tests.rs:222` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/playerdata/src/places/tests.rs:241` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/playerdata/src/pricing/mod.rs:255` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/playerdata/src/pricing/mod.rs:306` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/playerdata/src/pricing/mod.rs:310` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/playerdata/src/pricing/mod.rs:327` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/playerdata/src/pricing/mod.rs:350` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/playerdata/src/pricing/mod.rs:358` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/playerdata/src/pricing/mod.rs:406` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/playerdata/src/pricing/mod.rs:408` | FOR SHARE; query tables pd_price_overrides, updated_at; "SELECT selection, floor(extract(epoch FROM updated_at))::bigint FROM pd_price_overrides WHERE account_id=$1 AND world_id=$2 AND item_key=$3 | 4 / owning domain resource | no | n.a. | yes |
| `portal-rs/crates/playerdata/src/pricing/npc.rs:38` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/playerdata/src/progress/mod.rs:62` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/playerdata/src/progress/mod.rs:66` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/playerdata/src/progress/mod.rs:200` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/playerdata/src/progress/mod.rs:224` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/playerdata/src/progress/store.rs:38` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/playerdata/src/progress/store.rs:42` | FOR UPDATE; query tables pd_progress_streams; let stream = sqlx::query("SELECT authority,epoch,recovery,complete_through::text AS cut FROM pd_progress_streams WHERE account_id=$1 AND fam | 4 / owning domain resource | no | n.a. | yes |
| `portal-rs/crates/playerdata/src/progress/store.rs:111` | FOR UPDATE; query tables pd_progress_streams; let row = sqlx::query("SELECT authority,epoch,recovery,complete_through::text AS cut,observed_at,baseline_complete FROM pd_progress_streams  | 4 / owning domain resource | no | n.a. | yes |
| `portal-rs/crates/playerdata/src/progress/store.rs:136` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/playerdata/src/progress/store.rs:193` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/playerdata/src/progress/tests.rs:220` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/playerdata/src/progress/tests.rs:326` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/playerdata/src/progress/tests.rs:348` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/playerdata/src/reminders/mod.rs:135` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/playerdata/src/reminders/mod.rs:161` | FOR UPDATE; query tables pd_scoped_reminders; let row: Option&lt;(String,)&gt; = sqlx::query_as("SELECT state FROM pd_scoped_reminders WHERE id=$1 AND account_id=$2 AND character_id IS NOT DIS | 4 / owning domain resource | no | n.a. | yes |
| `portal-rs/crates/playerdata/src/reminders/mod.rs:192` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/playerdata/src/reminders/mod.rs:246` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/playerdata/src/reminders/tests.rs:70` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/playerdata/src/reminders/tests.rs:121` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/playerdata/src/reminders/tests.rs:122` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/playerdata/src/reminders/tests.rs:123` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/playerdata/src/reminders/tests.rs:261` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/playerdata/src/reminders/tests.rs:284` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/playerdata/src/reminders/tests.rs:285` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/playerdata/src/reminders/tests.rs:286` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/playerdata/src/reminders/tests.rs:289` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/playerdata/src/reports.rs:126` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/playerdata/src/reports.rs:131` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/playerdata/src/reports.rs:139` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/playerdata/src/reports.rs:274` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/playerdata/src/reports.rs:289` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/playerdata/src/reports.rs:315` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/playerdata/src/reports.rs:334` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/playerdata/src/reports.rs:366` | FOR SHARE; query tables accounts; let current: Option&lt;(String, String)&gt; = sqlx::query_as("SELECT email,locale FROM accounts WHERE id=$1 AND status='active' AND email_verified | 3 | no | n.a. | yes |
| `portal-rs/crates/playerdata/src/reports.rs:387` | FOR UPDATE; query tables pd_report_preferences; "SELECT email_enabled FROM pd_report_preferences WHERE account_id=$1 FOR UPDATE", | 4 / owning domain resource | no | n.a. | yes |
| `portal-rs/crates/playerdata/src/reports.rs:395` | FOR UPDATE; query tables account_notification_prefs; let pref: Option&lt;bool&gt; = sqlx::query_scalar("SELECT email_enabled FROM account_notification_prefs WHERE account_id=$1 AND category='news' FO | 4 / owning domain resource | no | n.a. | yes |
| `portal-rs/crates/playerdata/src/reports/README.md:17` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/playerdata/src/reports/README.md:51` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/playerdata/src/reports/README.md:64` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/playerdata/src/reports_tests.rs:63` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/playerdata/src/retention.rs:22` | FOR UPDATE SKIP LOCKED; query tables pd_g9_records; sqlx::query("UPDATE pd_g9_records SET canonical_bytes=NULL,record=NULL,purged_at=COALESCE(purged_at,$1) WHERE ctid IN (SELECT ctid FROM pd_g | 4 / owning domain resource | no | n.a. | yes |
| `portal-rs/crates/playerdata/src/retention.rs:23` | FOR UPDATE SKIP LOCKED; query tables pd_records; sqlx::query("UPDATE pd_records SET payload=NULL WHERE ctid IN (SELECT ctid FROM pd_records WHERE payload IS NOT NULL AND expires_ms&lt;=$1 ORDE | 4 / owning domain resource | no | n.a. | yes |
| `portal-rs/crates/playerdata/src/retention.rs:25` | FOR UPDATE SKIP LOCKED; query tables pd_t76_streams; sqlx::query("UPDATE pd_t76_streams SET snapshot=NULL,snapshot_at=NULL WHERE ctid IN (SELECT ctid FROM pd_t76_streams WHERE snapshot IS NOT N | 4 / owning domain resource | no | n.a. | yes |
| `portal-rs/crates/playerdata/src/retention.rs:27` | FOR UPDATE OF r SKIP LOCKED; query tables pd_t76_records, pd_t76_streams; sqlx::query("UPDATE pd_t76_records SET canonical_bytes=NULL WHERE ctid IN (SELECT r.ctid FROM pd_t76_records r WHERE canonical_bytes IS NOT  | 4 / owning domain resource | no | n.a. | yes |
| `portal-rs/crates/playerdata/src/retention.rs:29` | FOR UPDATE SKIP LOCKED; query tables pd_t76_events; sqlx::query("UPDATE pd_t76_events SET canonical_bytes=NULL WHERE ctid IN (SELECT ctid FROM pd_t76_events WHERE canonical_bytes IS NOT NULL A | 4 / owning domain resource | no | n.a. | yes |
| `portal-rs/crates/playerdata/src/retention.rs:31` | FOR UPDATE OF r SKIP LOCKED; query tables pd_t78_records; sqlx::query("UPDATE pd_t78_records SET payload=NULL WHERE ctid IN (SELECT r.ctid FROM pd_t78_records r WHERE payload IS NOT NULL AND ((famil | 4 / owning domain resource | no | n.a. | yes |
| `portal-rs/crates/playerdata/src/retention.rs:37` | FOR UPDATE SKIP LOCKED; query tables candidates, pd_records, pd_streams; sqlx::query("WITH candidates AS (SELECT stream_key FROM pd_streams s WHERE EXISTS(SELECT 1 FROM pd_records r WHERE r.stream_key=s.stream_key | 4 / owning domain resource | no | n.a. | yes |
| `portal-rs/crates/playerdata/src/retention.rs:39` | FOR UPDATE OF r SKIP LOCKED; query tables pd_records, pd_streams; sqlx::query("DELETE FROM pd_records WHERE ctid IN (SELECT r.ctid FROM pd_records r JOIN pd_streams s USING(stream_key) WHERE r.source_revisi | 4 / owning domain resource | no | n.a. | yes |
| `portal-rs/crates/playerdata/src/retention.rs:41` | FOR UPDATE SKIP LOCKED; query tables pd_deleted_scopes; sqlx::query("DELETE FROM pd_deleted_scopes WHERE ctid IN (SELECT ctid FROM pd_deleted_scopes WHERE expires_ms&lt;=$1 LIMIT 512 FOR UPDATE SKIP  | 4 / owning domain resource | no | n.a. | yes |
| `portal-rs/crates/playerdata/src/retention.rs:43` | FOR UPDATE SKIP LOCKED; query tables pd_conflicts; sqlx::query("DELETE FROM pd_conflicts WHERE ctid IN (SELECT ctid FROM pd_conflicts WHERE expires_ms&lt;=$1 LIMIT 512 FOR UPDATE SKIP LOCKED)") | 4 / owning domain resource | no | n.a. | yes |
| `portal-rs/crates/playerdata/src/retention.rs:47` | FOR UPDATE SKIP LOCKED; query tables candidates, pd_t76_records, pd_t76_streams, s.updated_at; sqlx::query("WITH candidates AS (SELECT stream_key FROM pd_t76_streams s WHERE EXISTS(SELECT 1 FROM pd_t76_records r WHERE r.stream_key=s.st | 4 / owning domain resource | no | n.a. | yes |
| `portal-rs/crates/playerdata/src/retention.rs:49` | FOR UPDATE OF r SKIP LOCKED; query tables pd_t76_records, pd_t76_streams; sqlx::query("DELETE FROM pd_t76_records WHERE ctid IN (SELECT r.ctid FROM pd_t76_records r JOIN pd_t76_streams s USING(stream_key) WHERE r.s | 4 / owning domain resource | no | n.a. | yes |
| `portal-rs/crates/playerdata/src/retention.rs:51` | FOR UPDATE OF r SKIP LOCKED; query tables pd_t76_events, pd_t76_streams, s.updated_at; sqlx::query("DELETE FROM pd_t76_events WHERE ctid IN (SELECT r.ctid FROM pd_t76_events r JOIN pd_t76_streams s USING(stream_key) WHERE r.occ | 4 / owning domain resource | no | n.a. | yes |
| `portal-rs/crates/playerdata/src/retention.rs:68` | FOR UPDATE SKIP LOCKED; query tables pd_erasure_jobs; "SELECT id,account_id,character_id,closed_revision::text FROM pd_erasure_jobs ORDER BY id LIMIT 32 FOR UPDATE SKIP LOCKED", | 4 / owning domain resource | no | n.a. | yes |
| `portal-rs/crates/playerdata/src/sessions.rs:178` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/playerdata/src/sessions.rs:223` | pg_advisory_xact_lock; sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1,73))") | domain namespace (README inventory) | no | n.a. | yes |
| `portal-rs/crates/playerdata/src/sessions.rs:236` | pg_advisory_xact_lock; sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1,729))") | domain namespace (README inventory) | no | n.a. | yes |
| `portal-rs/crates/playerdata/src/sessions.rs:241` | FOR UPDATE; query tables pd_g9_streams; let stream=sqlx::query("SELECT admitted,blocked,conflicted,recovery_generation::text AS recovery,stored_through::text FROM pd_g9_streams WHE | 4 / owning domain resource | no | n.a. | yes |
| `portal-rs/crates/playerdata/src/sessions.rs:373` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/playerdata/src/sessions.rs:379` | pg_advisory_xact_lock; sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1,729))") | domain namespace (README inventory) | no | n.a. | yes |
| `portal-rs/crates/playerdata/src/sessions.rs:390` | FOR UPDATE; query tables pd_g9_commands; "SELECT intent,source_authority FROM pd_g9_commands WHERE operation_id=$1 FOR UPDATE", | 4 / owning domain resource | no | n.a. | yes |
| `portal-rs/crates/playerdata/src/sessions.rs:442` | FOR UPDATE; query tables pd_g9_commands; let row=sqlx::query("SELECT intent,binding_digest,receipt FROM pd_g9_commands WHERE operation_id=$1 AND source_authority=$2 FOR UPDATE").bin | 4 / owning domain resource | no | n.a. | yes |
| `portal-rs/crates/playerdata/src/store.rs:172` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/playerdata/src/store.rs:334` | pg_advisory_xact_lock; sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1,73))") | domain namespace (README inventory) | no | n.a. | yes |
| `portal-rs/crates/playerdata/src/store.rs:341` | pg_advisory_xact_lock; sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1,72))") | domain namespace (README inventory) | no | n.a. | yes |
| `portal-rs/crates/playerdata/src/store.rs:350` | pg_advisory_xact_lock_shared; sqlx::query("SELECT pg_advisory_xact_lock_shared(hashtextextended($1,72))") | domain namespace (README inventory) | no | n.a. | yes |
| `portal-rs/crates/playerdata/src/store.rs:359` | FOR UPDATE; query tables pd_streams; let row=sqlx::query("SELECT highest_seen::text,stored_through::text,applied_through::text,erased,GREATEST(recovery_high_water,watermark_reco | 4 / owning domain resource | no | n.a. | yes |
| `portal-rs/crates/playerdata/src/store.rs:438` | pg_advisory_xact_lock; sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1,74))") | domain namespace (README inventory) | no | n.a. | yes |
| `portal-rs/crates/playerdata/src/store/README.md:44` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/playerdata/src/store/README.md:45` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/playerdata/src/store/http.rs:113` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/playerdata/src/store/tests.rs:248` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/playerdata/src/store/tests.rs:546` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/stats/README.md:38` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/stats/README.md:46` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/stats/src/character/README.md:24` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/stats/src/character/http.rs:225` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/stats/src/character/mod.rs:2` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/stats/src/character/mod.rs:22` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/stats/src/character/mod.rs:55` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/stats/src/character/mod.rs:65` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/stats/src/character/mod.rs:67` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/stats/src/character/mod.rs:71` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/stats/src/character/mod.rs:123` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/stats/src/character/mod.rs:131` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/stats/src/character/mod.rs:133` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/stats/src/character/mod.rs:233` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/stats/src/character/mod.rs:236` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/stats/src/character/mod.rs:237` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/stats/src/character/mod.rs:238` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/stats/src/character/mod.rs:245` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/stats/src/character/mod.rs:275` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/stats/src/character/mod.rs:278` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/stats/src/character/mod.rs:279` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/stats/src/character/mod.rs:280` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/stats/src/character/mod.rs:282` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/stats/src/character/tests.rs:14` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/stats/src/character/tests.rs:283` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/stats/src/guilds.rs:20` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/stats/src/guilds.rs:98` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/stats/src/guilds/tests.rs:2` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/stats/src/guilds/tests.rs:11` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/stats/src/guilds/tests.rs:13` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/stats/src/guilds/tests.rs:16` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/stats/src/online.rs:54` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/stats/src/online.rs:62` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/stats/src/online/README.md:13` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/stats/src/online/README.md:103` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/stats/src/online/storage.rs:22` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/stats/src/online/storage.rs:27` | FOR UPDATE; query tables public_projection_state; "SELECT highest_epoch::text FROM public_projection_state WHERE id=1 FOR UPDATE", | 4 / owning domain resource | no | n.a. | yes |
| `portal-rs/crates/stats/src/online/storage.rs:154` | FOR UPDATE; query tables public_projection_state; sqlx::query("SELECT id FROM public_projection_state WHERE id=1 FOR UPDATE") | 4 / owning domain resource | no | n.a. | yes |
| `portal-rs/crates/stats/src/online/storage.rs:172` | FOR SHARE; query tables public_projection_state; "SELECT highest_epoch::text FROM public_projection_state WHERE id=1 FOR SHARE", | 4 / owning domain resource | no | n.a. | yes |
| `portal-rs/crates/stats/src/privacy.rs:2` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/stats/src/privacy.rs:28` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/stats/src/privacy.rs:38` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/stats/src/privacy.rs:40` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/stats/src/privacy.rs:69` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/stats/src/privacy.rs:70` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/stats/src/privacy.rs:168` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/stats/src/rankings_online.rs:16` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/stats/src/rankings_online.rs:67` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/stats/src/rankings_online.rs:134` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/stats/src/rankings_online.rs:135` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/stats/src/rankings_online.rs:139` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/stats/src/rankings_online.rs:146` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/stats/src/store.rs:25` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/stats/src/store.rs:26` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/stats/src/store.rs:28` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/stats/src/store.rs:29` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/stats/src/store.rs:31` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/stats/src/store.rs:32` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/stats/src/store.rs:36` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/stats/src/store.rs:204` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/stats/src/store.rs:249` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/stats/src/tests.rs:56` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/stats/src/tests.rs:57` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/stats/src/tests.rs:477` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/stats/src/tests.rs:478` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/stats/src/tests.rs:480` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/stats/src/tests.rs:520` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/stats/src/tests.rs:523` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/stats/src/watch.rs:103` | pg_advisory_xact_lock; sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1,70))") | domain namespace (README inventory) | no | n.a. | yes |
| `portal-rs/crates/stats/src/watch.rs:167` | FOR UPDATE; query tables stats_watches; let rows: Vec&lt;(Uuid,String,Uuid)&gt; = sqlx::query_as("SELECT account_id,target_kind,target_id FROM stats_watches WHERE event_kind=$1 AND ((tar | 4 / owning domain resource | no | n.a. | yes |
| `portal-rs/crates/stats/src/watch/README.md:8` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/stats/src/watch/README.md:27` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/stats/src/watch/README.md:28` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/stats/src/watch/http.rs:1` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/stats/src/watch/http.rs:80` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/stats/src/watch/http.rs:82` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/stats/src/watch/http.rs:90` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/stats/src/watch/tests.rs:281` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/stats/src/watch/tests.rs:303` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/stats/src/worker.rs:44` | pg_advisory_xact_lock; sqlx::query("SELECT pg_advisory_xact_lock(65001)") | domain namespace (README inventory) | no | n.a. | yes |
| `portal-rs/crates/stats/src/worker.rs:52` | FOR SHARE; query tables public_projection_state; sqlx::query("SELECT highest_epoch FROM public_projection_state WHERE id=1 FOR SHARE") | 4 / owning domain resource | no | n.a. | yes |
| `portal-rs/crates/stats/src/worker.rs:170` | pg_advisory_xact_lock; sqlx::query("SELECT pg_advisory_xact_lock(65001)") | domain namespace (README inventory) | no | n.a. | yes |
| `portal-rs/crates/stats/src/world.rs:33` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/stats/src/world.rs:74` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/stats/src/world.rs:75` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/stats/src/world.rs:82` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/stats/src/world/README.md:13` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/stats/src/world/http.rs:237` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/stats/src/world/http.rs:250` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/stats/src/world/http.rs:251` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/stats/src/world/kills.rs:20` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/stats/src/world/kills.rs:96` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/stats/src/world/kills.rs:154` | FOR UPDATE; query tables public_projection_state; "SELECT highest_epoch::text FROM public_projection_state WHERE id=1 FOR UPDATE", | 4 / owning domain resource | no | n.a. | yes |
| `portal-rs/crates/stats/src/world/kills.rs:168` | FOR UPDATE; query tables stats_world_kills_v2; let old = sqlx::query("SELECT epoch::text,revision::text,content_digest,retired FROM stats_world_kills_v2 WHERE source_authority=$1 AND worl | 4 / owning domain resource | no | n.a. | yes |
| `portal-rs/crates/stats/src/world/tests.rs:2` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/stats/src/world/tests.rs:7` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/stats/src/world/tests.rs:8` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/stats/src/world/tests.rs:11` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/stats/src/world/tests.rs:171` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/stats/src/world/tests.rs:172` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/stats/tests/online.rs:138` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/stats/tests/online.rs:402` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/stats/tests/online.rs:407` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/stats/tests/online.rs:838` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/stats/tests/online.rs:904` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/stats/tests/online.rs:950` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/stats/tests/privacy.rs:62` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/stats/tests/privacy.rs:63` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/stats/tests/privacy.rs:92` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/stats/tests/privacy.rs:114` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/stats/tests/privacy.rs:129` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/stats/tests/privacy.rs:144` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/stats/tests/world_kills.rs:42` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/store/src/catalog.rs:213` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/store/src/catalog.rs:239` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/store/src/catalog.rs:281` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/store/src/catalog.rs:284` | FOR UPDATE; query tables store_active_catalog; "SELECT catalog_revision FROM store_active_catalog WHERE id=1 FOR UPDATE", | 7 / domain namespace | no | n.a. | yes |
| `portal-rs/crates/store/src/catalog.rs:298` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/store/src/catalog_tests.rs:250` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/store/src/catalog_tests.rs:252` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/store/src/catalog_tests.rs:253` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/store/src/catalog_tests.rs:423` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/store/src/catalog_tests.rs:425` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/store/src/catalog_tests.rs:426` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/store/src/catalog_tests.rs:515` | test fixture/blocker: FOR SHARE; query tables accounts; sqlx::query("SELECT id FROM accounts WHERE id=$1 FOR SHARE") | 3 | n.a. | n.a. | yes |
| `portal-rs/crates/store/src/catalog_tests.rs:527` | test fixture/blocker: FOR UPDATE; query tables accounts; sqlx::query("SELECT id FROM accounts WHERE id=$1 FOR UPDATE") | 3 | n.a. | n.a. | yes |
| `portal-rs/crates/store/src/catalog_tests.rs:540` | test fixture/blocker: FOR SHARE; query tables store_active_catalog; sqlx::query("SELECT catalog_revision FROM store_active_catalog WHERE id=1 FOR SHARE") | 7 / domain namespace | n.a. | n.a. | yes |
| `portal-rs/crates/store/src/codes.rs:37` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/store/src/codes.rs:38` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/store/src/codes.rs:39` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/store/src/codes.rs:45` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/store/src/codes.rs:46` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/store/src/codes.rs:48` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/store/src/codes.rs:149` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/store/src/codes.rs:151` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/store/src/codes.rs:154` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/store/src/codes.rs:160` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/store/src/codes.rs:165` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/store/src/codes.rs:174` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/store/src/codes.rs:180` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/store/src/codes.rs:184` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/store/src/codes.rs:187` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/store/src/codes.rs:192` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/store/src/codes.rs:206` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/store/src/codes.rs:259` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/store/src/codes.rs:272` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/store/src/codes.rs:276` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/store/src/codes.rs:290` | pg_advisory_xact_lock; sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1,55))") | domain namespace (README inventory) | no | n.a. | fixed in parallel task drc-a |
| `portal-rs/crates/store/src/codes.rs:294` | FOR UPDATE OF c; query tables code_batches, gift_codes; let row = sqlx::query("SELECT c.id,c.code_hash,c.batch_id,b.kind,b.amount,b.max_per_code,b.max_per_account,b.created_by,b.valid_from,b.valid | 2 | no | n.a. | fixed in parallel task drc-a |
| `portal-rs/crates/store/src/codes.rs:318` | pg_advisory_xact_lock; sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1,56))") | domain namespace (README inventory) | no | n.a. | fixed in parallel task drc-a |
| `portal-rs/crates/store/src/codes.rs:326` | FOR UPDATE; query tables accounts; let active: Option&lt;bool&gt; = sqlx::query_scalar("SELECT status='active' AND email_verified_at IS NOT NULL FROM accounts WHERE id=$1 FOR UPDATE | 3 | no | n.a. | fixed in parallel task drc-a |
| `portal-rs/crates/store/src/codes.rs:374` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/store/src/codes.rs:387` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/store/src/codes.rs:392` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/store/src/codes_tests.rs:3` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/store/src/codes_tests.rs:18` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/store/src/codes_tests.rs:19` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/store/src/codes_tests.rs:158` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/store/src/codes_tests.rs:159` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/store/src/codes_tests.rs:211` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/store/src/codes_tests.rs:271` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/store/src/codes_tests.rs:298` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/store/src/codes_tests.rs:310` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/store/src/codes_tests.rs:316` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/store/src/codes_tests.rs:322` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/store/src/codes_tests.rs:332` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/store/src/codes_tests.rs:346` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/store/src/codes_tests.rs:380` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/store/src/codes_tests.rs:411` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/store/src/codes_tests.rs:422` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/store/src/codes_tests.rs:448` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/store/src/codes_tests.rs:456` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/store/src/codes_tests.rs:465` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/store/src/codes_tests.rs:485` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/store/src/codes_tests.rs:494` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/store/src/codes_tests.rs:498` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/store/src/codes_tests.rs:501` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/store/src/codes_tests.rs:515` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/store/src/codes_tests.rs:535` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/store/src/codes_tests.rs:620` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/store/src/codes_tests.rs:719` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/store/src/codes_tests.rs:729` | test fixture/blocker: FOR UPDATE; query tables gift_codes; sqlx::query("SELECT id FROM gift_codes WHERE batch_id=$1 FOR UPDATE") | 2 | n.a. | n.a. | yes |
| `portal-rs/crates/store/src/codes_tests.rs:752` | test fixture/blocker: FOR UPDATE NOWAIT; query tables accounts; let account_lock = sqlx::query("SELECT id FROM accounts WHERE id=$1 FOR UPDATE NOWAIT") | 3 | n.a. | n.a. | yes |
| `portal-rs/crates/store/src/delivery.rs:49` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/store/src/delivery.rs:93` | pg_advisory_xact_lock; sqlx::query("SELECT pg_advisory_xact_lock(29,1)") | domain namespace (README inventory) | no | n.a. | yes |
| `portal-rs/crates/store/src/delivery.rs:167` | pg_advisory_xact_lock; sqlx::query("SELECT pg_advisory_xact_lock(29,1)") | domain namespace (README inventory) | no | n.a. | yes |
| `portal-rs/crates/store/src/delivery.rs:170` | FOR UPDATE; query tables store_operations; let op = sqlx::query("SELECT intent_bytes,entitlement_id,state FROM store_operations WHERE issuer_authority=$1 AND operation_kind=$2 AND ope | 4 / owning domain resource | no | n.a. | yes |
| `portal-rs/crates/store/src/delivery.rs:199` | FOR UPDATE; query tables store_fulfilment_units; "SELECT quantity FROM store_fulfilment_units WHERE entitlement_id=$1 FOR UPDATE", | 4 / owning domain resource | no | n.a. | yes |
| `portal-rs/crates/store/src/delivery.rs:230` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/store/src/delivery.rs:247` | FOR UPDATE; query tables store_delivery_fence; "SELECT new_recovery_generation::text FROM store_delivery_fence WHERE id=1 FOR UPDATE", | 4 / owning domain resource | no | n.a. | yes |
| `portal-rs/crates/store/src/delivery.rs:254` | pg_advisory_xact_lock; sqlx::query("SELECT pg_advisory_xact_lock(29,1)") | domain namespace (README inventory) | no | n.a. | yes |
| `portal-rs/crates/store/src/delivery.rs:257` | FOR UPDATE OF o; query tables store_operations, store_receipts; let rows = sqlx::query("SELECT o.operation_id,o.entitlement_id,o.operation_kind FROM store_operations o LEFT JOIN store_receipts r ON r.oper | 4 / owning domain resource | no | n.a. | yes |
| `portal-rs/crates/store/src/delivery.rs:279` | FOR UPDATE; query tables character_restore_locks; "SELECT id::integer FROM character_restore_locks WHERE id=1 FOR UPDATE" | 4 / owning domain resource | no | n.a. | yes |
| `portal-rs/crates/store/src/delivery.rs:281` | FOR SHARE; query tables character_restore_locks; "SELECT id::integer FROM character_restore_locks WHERE id=1 FOR SHARE" | 4 / owning domain resource | no | n.a. | yes |
| `portal-rs/crates/store/src/delivery_http.rs:186` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/store/src/delivery_tests.rs:429` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/store/src/delivery_tests.rs:456` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/store/src/delivery_tests.rs:499` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/store/src/delivery_tests.rs:509` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/store/src/delivery_tests.rs:518` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/store/src/delivery_tests.rs:626` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/store/src/delivery_tests.rs:638` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/store/src/delivery_tests.rs:674` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/store/src/delivery_tests.rs:765` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/store/src/delivery_tests.rs:1018` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/store/src/delivery_tests.rs:1051` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/store/src/delivery_tests.rs:1080` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/store/src/loyalty.rs:60` | FOR UPDATE; query tables premium_authority; let (authority, last): (i64, Option&lt;OffsetDateTime&gt;) = sqlx::query_as("SELECT authority_revision,last_issued_at FROM premium_authority WHERE | 7 / domain namespace | no | n.a. | yes |
| `portal-rs/crates/store/src/loyalty_stream.rs:140` | FOR UPDATE; query tables loyalty_stream_head; "SELECT position FROM loyalty_stream_head WHERE singleton FOR UPDATE", | 7 / domain namespace | no | n.a. | yes |
| `portal-rs/crates/store/src/loyalty_stream.rs:190` | FOR KEY SHARE; query tables accounts; sqlx::query("SELECT id FROM accounts WHERE id=$1 FOR KEY SHARE") | 3 | no | n.a. | yes |
| `portal-rs/crates/store/src/loyalty_stream.rs:211` | FOR KEY SHARE; query tables premium_authority; "SELECT account_id FROM premium_authority WHERE account_id=$1 FOR KEY SHARE", | 7 / domain namespace | no | n.a. | yes |
| `portal-rs/crates/store/src/loyalty_stream_http.rs:39` | FOR SHARE; query tables accounts; sqlx::query_scalar("SELECT id FROM accounts WHERE id=$1 FOR SHARE") | 3 | no | n.a. | yes |
| `portal-rs/crates/store/src/premium.rs:104` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/store/src/premium.rs:109` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/store/src/premium/repository.rs:40` | FOR UPDATE; query tables premium_authority; "SELECT authority_revision FROM premium_authority WHERE account_id=$1 FOR UPDATE", | 7 / domain namespace | no | n.a. | yes |
| `portal-rs/crates/store/src/premium/repository.rs:58` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/store/src/premium/repository.rs:77` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/store/src/premium/repository.rs:79` | pg_advisory_xact_lock; sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1, 27))") | domain namespace (README inventory) | no | n.a. | yes |
| `portal-rs/crates/store/src/premium/repository.rs:135` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/store/src/premium/repository.rs:144` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/store/src/premium/repository.rs:147` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/store/src/premium/repository.rs:150` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/store/src/premium/repository.rs:151` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/store/src/premium/repository.rs:159` | FOR UPDATE; query tables referral_campaign_rewards, referral_campaign_settings; let row = sqlx::query("SELECT account_id,amount FROM referral_campaign_rewards WHERE id=$1 AND kind='premium' AND EXISTS(SELECT 1 FROM refer | 4 / owning domain resource | no | n.a. | yes |
| `portal-rs/crates/store/src/premium/repository.rs:166` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/store/src/premium/repository.rs:178` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/store/src/premium/repository.rs:199` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/store/src/premium/repository.rs:201` | pg_advisory_xact_lock; sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1, 27))") | domain namespace (README inventory) | no | n.a. | yes |
| `portal-rs/crates/store/src/premium_profile_b.rs:192` | FOR UPDATE; query tables profile_b_authority; "SELECT authority_revision,last_issued_at FROM profile_b_authority WHERE account_id=$1 FOR UPDATE", | 4 / owning domain resource | no | n.a. | yes |
| `portal-rs/crates/store/src/premium_profile_b_tests.rs:613` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/store/src/premium_profile_b_tests.rs:633` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/store/src/premium_tests.rs:254` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/store/src/premium_tests.rs:308` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/store/src/purchase.rs:19` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/store/src/purchase_accounts.rs:5` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/store/src/purchase_accounts.rs:25` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/store/src/purchase_accounts.rs:26` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/store/src/purchase_accounts.rs:34` | FOR SHARE; query tables accounts; sqlx::query_scalar("SELECT account_value_eligible(id) FROM accounts WHERE id=$1 FOR SHARE") | 3 | no | n.a. | yes |
| `portal-rs/crates/store/src/purchase_accounts.rs:72` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/store/src/purchase_accounts.rs:73` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/store/src/purchase_accounts.rs:77` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/store/src/purchase_http.rs:200` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/store/src/purchase_repository.rs:38` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/store/src/purchase_repository.rs:67` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/store/src/purchase_repository.rs:155` | pg_advisory_xact_lock; sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1,29))") | domain namespace (README inventory) | no | n.a. | yes |
| `portal-rs/crates/store/src/purchase_repository.rs:183` | FOR SHARE; query tables store_active_catalog; "SELECT catalog_revision FROM store_active_catalog WHERE id=1 FOR SHARE", | 7 / domain namespace | no | n.a. | yes |
| `portal-rs/crates/store/src/purchase_repository.rs:205` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/store/src/purchase_repository.rs:244` | pg_advisory_xact_lock; sqlx::query("SELECT pg_advisory_xact_lock(29,1)") | domain namespace (README inventory) | no | n.a. | yes |
| `portal-rs/crates/store/src/purchase_repository.rs:258` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/store/src/purchase_repository.rs:285` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/store/src/purchase_repository.rs:418` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/store/src/purchase_repository.rs:459` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/store/src/purchase_tests.rs:17` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/store/src/purchase_tests.rs:280` | test fixture/blocker: FOR UPDATE; query tables purchase_test_accounts; sqlx::query_scalar("SELECT state FROM purchase_test_accounts WHERE id=$1 FOR UPDATE") | 4 / owning domain resource | n.a. | n.a. | yes |
| `portal-rs/crates/store/src/purchase_tests.rs:321` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/store/src/purchase_tests.rs:322` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/store/src/purchase_tests.rs:1023` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/store/src/purchase_tests.rs:1037` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/store/src/purchase_tests.rs:1109` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/store/src/purchase_tests.rs:1116` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/store/src/purchase_tests.rs:1150` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/store/src/purchase_tests.rs:1227` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/store/src/purchase_tests.rs:1276` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/store/src/purchase_tests.rs:1548` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/store/src/purchase_tests.rs:1565` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/store/src/purchase_tests.rs:1706` | test fixture/blocker: FOR UPDATE; query tables accounts; sqlx::query("SELECT id FROM accounts WHERE id=$1 FOR UPDATE") | 3 | n.a. | n.a. | yes |
| `portal-rs/crates/store/src/purchase_tests.rs:1733` | test fixture/blocker: FOR UPDATE NOWAIT; query tables wallet_accounts; sqlx::query("SELECT account_id FROM wallet_accounts WHERE account_id=$1 FOR UPDATE NOWAIT") | 7 / domain namespace | n.a. | n.a. | yes |
| `portal-rs/crates/store/src/web.rs:40` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/store/src/web.rs:59` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/store/src/web.rs:125` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/store/src/web_README.md:18` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/store/src/web_README.md:119` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/store/src/web_README.md:121` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/store/src/web_backend.rs:225` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/store/src/web_backend.rs:237` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/store/src/web_backend.rs:244` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/store/src/web_backend.rs:248` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/store/src/web_backend.rs:250` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/store/src/web_backend.rs:251` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/store/src/web_backend.rs:252` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/store/src/web_backend.rs:275` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/store/src/web_payments.rs:12` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/store/src/web_payments.rs:17` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/store/src/web_payments.rs:23` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/store/src/web_payments.rs:30` | FOR SHARE; query tables accounts; let active: Option&lt;bool&gt; = sqlx::query_scalar("SELECT status='active' AND email_verified_at IS NOT NULL FROM accounts WHERE id=$1 FOR SHARE" | 3 | no | n.a. | yes |
| `portal-rs/crates/store/src/web_payments.rs:41` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/store/src/web_tests.rs:69` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/store/src/web_tests.rs:73` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/store/src/web_tests.rs:74` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/appeals.rs:4` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/appeals.rs:6` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/appeals.rs:180` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/appeals.rs:181` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/appeals.rs:194` | FOR UPDATE; query tables support_appeal_workflows; let row = sqlx::query("SELECT * FROM support_appeal_workflows WHERE sanction_id = $1 AND owner_id = $2 FOR UPDATE") | 4 / owning domain resource | no | n.a. | yes |
| `portal-rs/crates/support/src/appeals.rs:223` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/appeals.rs:224` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/appeals.rs:225` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/appeals.rs:234` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/appeals.rs:236` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/appeals.rs:254` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/appeals.rs:309` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/appeals.rs:349` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/appeals/tests.rs:206` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/attachments.rs:88` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/attachments.rs:105` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/attachments.rs:167` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/attachments/tests.rs:70` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/attachments/tests.rs:89` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/lib.rs:14` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/lib.rs:15` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/reports.rs:159` | FOR UPDATE; query tables accounts; sqlx::query_scalar("SELECT status FROM accounts WHERE id=$1 FOR UPDATE") | 3 | no | n.a. | yes |
| `portal-rs/crates/support/src/reports.rs:253` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/reports.rs:254` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/reports.rs:262` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/reports.rs:263` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/reports.rs:270` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/reports.rs:279` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/reports.rs:280` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/reports.rs:281` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/reports.rs:284` | FOR UPDATE; query tables accounts; sqlx::query_scalar("SELECT status FROM accounts WHERE id=$1 FOR UPDATE") | 3 | no | n.a. | yes |
| `portal-rs/crates/support/src/reports.rs:288` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/reports.rs:293` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/reports.rs:296` | FOR UPDATE; query tables player_reports; sqlx::query_as("SELECT status,lock_version FROM player_reports WHERE id=$1 FOR UPDATE") | 4 / owning domain resource | no | n.a. | yes |
| `portal-rs/crates/support/src/reports.rs:300` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/reports.rs:302` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/reports.rs:306` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/reports.rs:307` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/sanctions/access.rs:4` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/sanctions/access.rs:25` | FOR SHARE; query tables accounts; sqlx::query_scalar("SELECT status FROM accounts WHERE id=$1 FOR SHARE") | 3 | no | n.a. | yes |
| `portal-rs/crates/support/src/sanctions/access.rs:110` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/sanctions/access.rs:144` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/sanctions/access.rs:182` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/sanctions/enforcement.rs:210` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/sanctions/enforcement.rs:316` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/sanctions/enforcement.rs:337` | FOR UPDATE; query tables support_enforcement_counter; "SELECT position+1 FROM support_enforcement_counter WHERE singleton FOR UPDATE", | 4 / owning domain resource | no | n.a. | yes |
| `portal-rs/crates/support/src/sanctions/enforcement.rs:707` | FOR UPDATE; query tables support_enforcement_commands; let command = sqlx::query("SELECT sanction_stream_id,intent,command_digest FROM support_enforcement_commands WHERE operation_id=$1 FOR UPDAT | 4 / owning domain resource | no | n.a. | yes |
| `portal-rs/crates/support/src/sanctions/enforcement_tests.rs:369` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/sanctions/enforcement_tests.rs:379` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/sanctions/enforcement_tests.rs:406` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/sanctions/export.rs:143` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/sanctions/export_tests.rs:88` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/sanctions/export_tests.rs:213` | implicit SQL write locks; sqlx::query("INSERT INTO account_roles(account_id,role_id) SELECT $1,id FROM roles WHERE key='moderator'").bind(actor).execute(&pool).await? | 7 (effects; owning entry above) | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/sanctions/export_tests.rs:333` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/sanctions/export_tests.rs:335` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/sanctions/export_tests.rs:491` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/sanctions/export_tests.rs:756` | test fixture/blocker: FOR UPDATE; query tables support_sanction_export_counter; sqlx::query("SELECT position FROM support_sanction_export_counter WHERE singleton FOR UPDATE") | 4 / owning domain resource | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/sanctions/export_tests.rs:885` | test fixture/blocker: LOCK TABLE support_sanction_revisions IN ACCESS EXCLUSIVE MODE; sqlx::query("LOCK TABLE support_sanction_revisions IN ACCESS EXCLUSIVE MODE") | 4 / owning domain resource | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/sanctions/export_tests.rs:926` | test fixture/blocker: LOCK TABLE support_sanction_revisions IN ACCESS EXCLUSIVE MODE; sqlx::query("LOCK TABLE support_sanction_revisions IN ACCESS EXCLUSIVE MODE") | 4 / owning domain resource | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/sanctions/mod.rs:3` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/sanctions/mod.rs:4` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/sanctions/mod.rs:122` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/sanctions/mod.rs:136` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/sanctions/store.rs:4` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/sanctions/store.rs:13` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/sanctions/store.rs:19` | FOR SHARE; query tables accounts; sqlx::query_scalar("SELECT status FROM accounts WHERE id=$1 FOR SHARE") | 3 | yes | yes | yes |
| `portal-rs/crates/support/src/sanctions/store.rs:54` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/sanctions/store.rs:58` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/sanctions/store.rs:68` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/sanctions/store.rs:69` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/sanctions/store.rs:77` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/sanctions/store.rs:80` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/sanctions/store.rs:87` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/sanctions/store.rs:93` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/sanctions/store.rs:95` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/sanctions/store.rs:97` | FOR SHARE; query tables accounts; sqlx::query_scalar("SELECT status FROM accounts WHERE id=$1 FOR SHARE") | 3 | yes | yes | yes |
| `portal-rs/crates/support/src/sanctions/store.rs:104` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/sanctions/store.rs:107` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/sanctions/store.rs:121` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/sanctions/store.rs:124` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/sanctions/store.rs:125` | canonical account/resource/RBAC/durable-proof acquisition + live authority | 3→4→5→6 | yes | yes | yes |
| `portal-rs/crates/support/src/sanctions/store.rs:130` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/sanctions/store.rs:133` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/sanctions/store.rs:135` | FOR UPDATE; query tables accounts; sqlx::query_scalar("SELECT status FROM accounts WHERE id=$1 FOR UPDATE") | 3 | yes | yes | yes |
| `portal-rs/crates/support/src/sanctions/store.rs:199` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/sanctions/store.rs:208` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/sanctions/store.rs:209` | canonical account/resource/RBAC/durable-proof acquisition + live authority | 3→4→5→6 | yes | yes | yes |
| `portal-rs/crates/support/src/sanctions/store.rs:214` | FOR UPDATE; query tables sanctions; sqlx::query("SELECT id FROM sanctions WHERE id=$1 FOR UPDATE") | 4 / owning domain resource | yes | yes | yes |
| `portal-rs/crates/support/src/sanctions/store.rs:223` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/sanctions/store.rs:226` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/sanctions/store.rs:228` | FOR UPDATE; query tables accounts; sqlx::query_scalar("SELECT status FROM accounts WHERE id=$1 FOR UPDATE") | 3 | yes | yes | yes |
| `portal-rs/crates/support/src/sanctions/store.rs:236` | FOR UPDATE; query tables sanctions; "SELECT status,created_by,lock_version FROM sanctions WHERE id=$1 FOR UPDATE", | 4 / owning domain resource | yes | yes | yes |
| `portal-rs/crates/support/src/sanctions/store.rs:242` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/sanctions/store.rs:306` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/sanctions/store.rs:308` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/sanctions/store.rs:347` | FOR UPDATE; query tables accounts; sqlx::query_scalar("SELECT id FROM accounts WHERE id=$1 FOR UPDATE") | 3 | yes | yes | yes |
| `portal-rs/crates/support/src/sanctions/store.rs:359` | FOR UPDATE; query tables support_sanction_counters; "SELECT revision FROM support_sanction_counters WHERE account_id=$1 FOR UPDATE", | 4 / owning domain resource | yes | yes | yes |
| `portal-rs/crates/support/src/sanctions/store.rs:510` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/sanctions/store.rs:519` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/sanctions/store.rs:521` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/sanctions/store.rs:527` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/sanctions/store.rs:532` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/sanctions/store.rs:536` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/sanctions/store.rs:547` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/sanctions/store.rs:548` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/sanctions/store.rs:550` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/sanctions/store.rs:554` | canonical account/resource/RBAC/durable-proof acquisition + live authority | 3→4→5→6 | yes | yes | yes |
| `portal-rs/crates/support/src/sanctions/store.rs:557` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/sanctions/store.rs:559` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/sanctions/store.rs:563` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/sanctions/store.rs:564` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/sanctions/store.rs:566` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/sanctions/store.rs:567` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/sanctions/store.rs:568` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/sanctions/store.rs:571` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/sanctions/store.rs:572` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/sanctions/store.rs:576` | FOR UPDATE; query tables accounts; sqlx::query_scalar("SELECT status FROM accounts WHERE id=$1 FOR UPDATE") | 3 | yes | yes | yes |
| `portal-rs/crates/support/src/sanctions/store.rs:580` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/sanctions/store.rs:582` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/sanctions/store.rs:584` | FOR UPDATE; query tables effective_at, expires_at, sanctions; let current:Option&lt;(String,i64,Uuid,String,i64,Option&lt;i64&gt;)&gt;=sqlx::query_as("SELECT status,lock_version,account_id,category,(extract(epoch F | 4 / owning domain resource | yes | yes | yes |
| `portal-rs/crates/support/src/sanctions/store.rs:585` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/sanctions/store.rs:587` | FOR UPDATE; query tables support_appeal_workflows; let workflow: Option&lt;(String,i64)&gt; = sqlx::query_as("SELECT status,version FROM support_appeal_workflows WHERE sanction_id=$1 AND owner_id=$ | 4 / owning domain resource | yes | yes | yes |
| `portal-rs/crates/support/src/sanctions/store.rs:588` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/sanctions/store.rs:593` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/sanctions/store.rs:595` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/sanctions/store.rs:611` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/sanctions/store.rs:615` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/sanctions/store.rs:618` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/sanctions/store.rs:619` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/sanctions/store.rs:621` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/sanctions/store.rs:624` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/sanctions/store.rs:627` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/sanctions/store.rs:631` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/sanctions/store.rs:635` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/sanctions/store.rs:637` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/sanctions/store.rs:648` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/sanctions/store.rs:658` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/sanctions/store.rs:660` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/sanctions/store.rs:662` | implicit SQL write locks; sqlx::query("UPDATE sanctions SET status=$2,public_reason=$3,moderator_notes=nullif(btrim($4),''),lock_version=lock_version+1,updated_at=clo | 7 (effects; owning entry above) | yes | yes | yes |
| `portal-rs/crates/support/src/sanctions/store.rs:670` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/sanctions/store.rs:671` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/sanctions/store.rs:677` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/sanctions/store.rs:692` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/sanctions/store.rs:693` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/sanctions/store.rs:694` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/sanctions/store.rs:701` | implicit SQL write locks; sqlx::query("UPDATE sanctions SET status=$2,lock_version=lock_version+1,updated_at=clock_timestamp() WHERE id=$1").bind(id).bind(next).execu | 7 (effects; owning entry above) | yes | yes | yes |
| `portal-rs/crates/support/src/sanctions/store.rs:704` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/sanctions/store.rs:712` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/sanctions/store.rs:714` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/sanctions/store.rs:726` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/sanctions/store.rs:727` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/sanctions/store.rs:730` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/sanctions/tests.rs:79` | implicit SQL write locks; sqlx::query("INSERT INTO account_roles(account_id,role_id) SELECT $1,id FROM roles WHERE key='moderator'") | 7 (effects; owning entry above) | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/sanctions/tests.rs:145` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/sanctions/tests.rs:148` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/sanctions/tests.rs:150` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/sanctions/tests.rs:192` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/sanctions/tests.rs:202` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/sanctions/tests.rs:211` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/sanctions/tests.rs:221` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/sanctions/tests.rs:242` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/sanctions/tests.rs:250` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/sanctions/tests.rs:260` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/sanctions/tests.rs:271` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/sanctions/tests.rs:281` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/sanctions/tests.rs:293` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/sanctions/tests.rs:305` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/sanctions/tests.rs:315` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/sanctions/tests.rs:319` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/sanctions/tests.rs:327` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/sanctions/tests.rs:328` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/sanctions/tests.rs:335` | implicit SQL write locks; sqlx::query("DELETE FROM account_roles WHERE account_id=$1") | 7 (effects; owning entry above) | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/sanctions/tests.rs:342` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/sanctions/tests.rs:351` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/sanctions/tests.rs:373` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/sanctions/tests.rs:379` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/sanctions/tests.rs:384` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/sanctions/tests.rs:391` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/sanctions/tests.rs:397` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/sanctions/tests.rs:428` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/sanctions/tests.rs:430` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/sanctions/tests.rs:448` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/sanctions/tests.rs:500` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/sanctions/tests.rs:554` | test fixture/blocker: FOR UPDATE; query tables accounts; sqlx::query("SELECT id FROM accounts WHERE id=$1 FOR UPDATE") | 3 | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/sanctions/tests.rs:564` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/sanctions/tests.rs:618` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/sanctions/tests.rs:625` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/sanctions/tests.rs:644` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/sanctions/tests.rs:652` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/sanctions/tests.rs:878` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/sanctions/tests.rs:1071` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/sanctions/tests.rs:1097` | test fixture/blocker: FOR UPDATE; query tables accounts; sqlx::query("SELECT id FROM accounts WHERE id=$1 FOR UPDATE") | 3 | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/sanctions/tests.rs:1105` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/sanctions/tests.rs:1122` | test fixture/blocker: pg_try_advisory_xact_lock; let free: bool = sqlx::query_scalar("SELECT pg_try_advisory_xact_lock(71010)") | 5 | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/README.md:11` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/README.md:14` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/README.md:15` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/README.md:16` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/README.md:29` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/en.ftl:21` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/en.ftl:25` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/mod.rs:4` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/mod.rs:12` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/mod.rs:14` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/mod.rs:15` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/mod.rs:24` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/mod.rs:72` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/mod.rs:88` | FOR UPDATE; query tables accounts; sqlx::query_as("SELECT status,locale FROM accounts WHERE id=$1 FOR UPDATE") | 3 | yes | yes | yes |
| `portal-rs/crates/support/src/tickets/mod.rs:162` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/mod.rs:166` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/mod.rs:172` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/mod.rs:180` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/mod.rs:181` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/mod.rs:182` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/mod.rs:187` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/mod.rs:193` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/mod.rs:199` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/mod.rs:201` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/mod.rs:205` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/mod.rs:210` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/mod.rs:213` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/mod.rs:220` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/mod.rs:224` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/mod.rs:227` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/mod.rs:230` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/mod.rs:232` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/mod.rs:242` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/mod.rs:248` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/mod.rs:249` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/mod.rs:250` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/mod.rs:254` | canonical account/resource/RBAC/durable-proof acquisition + live authority | 3→4→5→6 | yes | yes | yes |
| `portal-rs/crates/support/src/tickets/mod.rs:259` | FOR UPDATE; query tables tickets; sqlx::query("SELECT id FROM tickets WHERE id=$1 FOR UPDATE") | 4 / owning domain resource | yes | yes | yes |
| `portal-rs/crates/support/src/tickets/mod.rs:268` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/mod.rs:285` | FOR UPDATE; query tables tickets; "SELECT * FROM tickets WHERE id=$1 AND ($2 OR account_id=$3) FOR UPDATE", | 4 / owning domain resource | yes | yes | yes |
| `portal-rs/crates/support/src/tickets/mod.rs:288` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/mod.rs:293` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/mod.rs:298` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/mod.rs:302` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/mod.rs:303` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/mod.rs:320` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/mod.rs:321` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/mod.rs:325` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/mod.rs:329` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/mod.rs:332` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/mod.rs:334` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/mod.rs:339` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/mod.rs:345` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/mod.rs:350` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/mod.rs:355` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/mod.rs:372` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/mod.rs:380` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/mod.rs:387` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/mod.rs:390` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/mod.rs:395` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/mod.rs:398` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/mod.rs:403` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/mod.rs:410` | FOR UPDATE; query tables tickets; "SELECT * FROM tickets WHERE id=$1 AND ($2 OR account_id=$3) FOR UPDATE", | 4 / owning domain resource | yes | yes | yes |
| `portal-rs/crates/support/src/tickets/mod.rs:413` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/mod.rs:422` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/mod.rs:426` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/mod.rs:430` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/mod.rs:433` | implicit SQL write locks; sqlx::query("UPDATE tickets SET unread_reply=false WHERE id=$1 AND account_id=$2 AND NOT EXISTS (SELECT 1 FROM ticket_messages WHERE ticket_ | 7 (effects; owning entry above) | yes | yes | yes |
| `portal-rs/crates/support/src/tickets/mod.rs:459` | FOR UPDATE SKIP LOCKED; query tables tickets; sqlx::query("DELETE FROM tickets WHERE id IN (SELECT id FROM tickets WHERE status IN ('resolved','closed') AND closed_at &lt; now()-interval '1 | 4 / owning domain resource | yes | yes | yes |
| `portal-rs/crates/support/src/tickets/mod.rs:465` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/page.html:5` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/page.html:12` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/pl.ftl:21` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/pl.ftl:25` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/staff.rs:1` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/staff.rs:4` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/staff.rs:9` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/staff.rs:18` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/staff.rs:84` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/staff.rs:92` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/staff.rs:97` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/staff.rs:100` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/staff.rs:102` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/staff.rs:103` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/staff.rs:109` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/staff.rs:113` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/staff.rs:120` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/staff.rs:129` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/staff.rs:130` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/staff.rs:137` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/staff.rs:138` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/staff.rs:149` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/staff.rs:150` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/staff.rs:152` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/staff.rs:158` | FOR UPDATE; query tables tickets; "SELECT id FROM tickets WHERE id=$1 FOR UPDATE" | 4 / owning domain resource | yes | yes | yes |
| `portal-rs/crates/support/src/tickets/staff.rs:160` | FOR UPDATE; query tables player_reports; Command::Moderate { .. } =&gt; "SELECT id FROM player_reports WHERE id=$1 FOR UPDATE", | 4 / owning domain resource | yes | yes | yes |
| `portal-rs/crates/support/src/tickets/staff.rs:162` | FOR UPDATE; query tables sanctions; "SELECT id FROM sanctions WHERE false AND id=$1 FOR UPDATE" | 4 / owning domain resource | yes | yes | yes |
| `portal-rs/crates/support/src/tickets/staff.rs:164` | FOR UPDATE; query tables sanctions; _ =&gt; "SELECT id FROM sanctions WHERE id=$1 FOR UPDATE", | 4 / owning domain resource | yes | yes | yes |
| `portal-rs/crates/support/src/tickets/staff.rs:167` | canonical account/resource/RBAC/durable-proof acquisition + live authority | 3→4→5→6 | yes | yes | yes |
| `portal-rs/crates/support/src/tickets/staff.rs:177` | FOR UPDATE; query tables support_appeal_workflows; sqlx::query("SELECT sanction_id FROM support_appeal_workflows WHERE sanction_id=$1 AND owner_id=$2 FOR UPDATE") | 4 / owning domain resource | yes | yes | yes |
| `portal-rs/crates/support/src/tickets/staff.rs:186` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/staff.rs:187` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/staff.rs:192` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/staff.rs:193` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/staff.rs:201` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/staff.rs:216` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/staff.rs:233` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/staff.rs:251` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/staff.rs:254` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/staff.rs:270` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/staff.rs:290` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/staff.rs:291` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/staff.rs:296` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/staff.rs:312` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/staff.rs:321` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/staff.rs:324` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/staff.rs:327` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/staff.rs:336` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/staff.rs:346` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/staff.rs:351` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/staff.rs:357` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/staff.rs:359` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/staff.rs:360` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/staff.rs:361` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/staff.rs:362` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/staff.rs:363` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/staff.rs:375` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/staff.rs:382` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/staff.rs:387` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/staff.rs:392` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/staff.rs:396` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/staff.rs:400` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/staff.rs:401` | implicit SQL write locks; sqlx::query("INSERT INTO notifications(account_id,category,subject,body) VALUES($1,'support',$2,$3)").bind(recipient).bind(rendered.subject) | 7 (effects; owning entry above) | yes | yes | yes |
| `portal-rs/crates/support/src/tickets/staff.rs:415` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/staff_tests.rs:11` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/staff_tests.rs:12` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/staff_tests.rs:16` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/staff_tests.rs:17` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/staff_tests.rs:20` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/staff_tests.rs:21` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/staff_tests.rs:25` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/staff_tests.rs:32` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/staff_tests.rs:35` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/staff_tests.rs:39` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/staff_tests.rs:42` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/staff_tests.rs:44` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/staff_tests.rs:56` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/staff_tests.rs:65` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/staff_tests.rs:85` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/staff_tests.rs:87` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/staff_tests.rs:91` | implicit SQL write locks; sqlx::query("INSERT INTO account_roles(account_id,role_id) SELECT $1,id FROM roles WHERE key IN ('support','moderator')").bind(staff).execut | 7 (effects; owning entry above) | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/staff_tests.rs:93` | implicit SQL write locks; let generation: Uuid = sqlx::query_scalar("INSERT INTO account_mfa_totp(account_id,secret,confirmed) VALUES($1,decode('01','hex'),true) RETU | 7 (effects; owning entry above) | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/staff_tests.rs:95` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/staff_tests.rs:99` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/staff_tests.rs:103` | implicit SQL write locks; let sanction: Uuid = sqlx::query_scalar("INSERT INTO sanctions(account_id,category,status,public_reason,effective_at,expires_at,created_by)  | 7 (effects; owning entry above) | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/staff_tests.rs:106` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/staff_tests.rs:119` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/staff_tests.rs:130` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/staff_tests.rs:134` | test fixture/blocker: pg_advisory_xact_lock; sqlx::query("SELECT pg_advisory_xact_lock(71010)") | 5 | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/staff_tests.rs:156` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/staff_tests.rs:157` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/staff_tests.rs:164` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/staff_tests.rs:177` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/staff_tests.rs:215` | implicit SQL write locks; sqlx::query("INSERT INTO ticket_messages(id,ticket_id,author_account_id,author_kind,body,internal) VALUES($1,$2,$3,'staff',$4,$5)") | 7 (effects; owning entry above) | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/staff_tests.rs:216` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/staff_tests.rs:272` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/staff_tests.rs:278` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/staff_tests.rs:445` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/staff_tests.rs:446` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/staff_tests.rs:464` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/staff_tests.rs:500` | implicit SQL write locks; let permanent:Uuid=sqlx::query_scalar("INSERT INTO sanctions(account_id,category,status,public_reason,effective_at,created_by,approved_by) V | 7 (effects; owning entry above) | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/staff_tests.rs:534` | test fixture/blocker: FOR UPDATE; query tables sanctions; sqlx::query("SELECT id FROM sanctions WHERE id=$1 FOR UPDATE") | 4 / owning domain resource | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/staff_tests.rs:566` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/staff_tests.rs:588` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/staff_tests.rs:602` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/staff_tests.rs:651` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/staff_tests.rs:674` | implicit SQL write locks; "INSERT INTO account_roles(account_id,role_id) SELECT $1,id FROM roles WHERE key='support'", | 7 (effects; owning entry above) | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/staff_tests.rs:689` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/staff_tests.rs:695` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/staff_tests.rs:697` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/staff_tests.rs:701` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/tests.rs:99` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/tests.rs:103` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/tests.rs:106` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/tests.rs:108` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/tests.rs:232` | implicit SQL write locks; sqlx::query("INSERT INTO account_roles(account_id,role_id) SELECT $1,id FROM roles WHERE key='platform_admin'").bind(actor).execute(&db.pool | 7 (effects; owning entry above) | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/tests.rs:268` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/tests.rs:283` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/tests.rs:322` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/tests.rs:339` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/tests.rs:348` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/tests.rs:385` | implicit SQL write locks; sqlx::query("INSERT INTO account_roles(account_id,role_id) SELECT $1,id FROM roles WHERE key='platform_admin'").bind(actor).execute(&db.pool | 7 (effects; owning entry above) | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/tests.rs:401` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/tests.rs:421` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/tests.rs:609` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/tests.rs:613` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/tests.rs:617` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/tests.rs:621` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/tests.rs:627` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/tests.rs:629` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/tests.rs:635` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/tests.rs:637` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/tests.rs:652` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/tests.rs:665` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/tests.rs:666` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/tests.rs:670` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/tests.rs:687` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/tests.rs:693` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/tests.rs:699` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/tests.rs:713` | implicit SQL write locks; sqlx::query("INSERT INTO account_roles(account_id,role_id) SELECT $1,id FROM roles WHERE key='platform_admin'").bind(actor).execute(&db.pool | 7 (effects; owning entry above) | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/tests.rs:719` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/tests.rs:740` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/tests.rs:877` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/tests.rs:893` | implicit SQL write locks; sqlx::query("INSERT INTO role_permissions(role_id,permission) VALUES ($1,'admin.access'),($1,'support.tickets.manage')").bind(role).execute( | 7 (effects; owning entry above) | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/tests.rs:894` | implicit SQL write locks; sqlx::query("INSERT INTO account_roles(account_id,role_id) VALUES ($1,$2)") | 7 (effects; owning entry above) | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/tests.rs:904` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/tests.rs:908` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/tests.rs:912` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/tests.rs:918` | test fixture/blocker: FOR UPDATE; query tables accounts; sqlx::query("SELECT id FROM accounts WHERE id=$1 FOR UPDATE") | 3 | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/tests.rs:930` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/tests.rs:942` | observer SQL; no lock acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/tests.rs:956` | implicit SQL write locks; sqlx::query("DELETE FROM account_roles WHERE account_id=$1") | 7 (effects; owning entry above) | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/tests.rs:962` | implicit SQL write locks; sqlx::query("DELETE FROM role_permissions WHERE role_id=$1 AND permission=$2") | 7 (effects; owning entry above) | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/tests.rs:999` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/tests.rs:1004` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/tests.rs:1014` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/tests.rs:1015` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/tests.rs:1016` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/tests.rs:1017` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/tests.rs:1022` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/tests.rs:1023` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/tests.rs:1028` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/tests.rs:1029` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/tests.rs:1034` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/tests.rs:1035` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/tests.rs:1040` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/tests.rs:1041` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/tests.rs:1046` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/tests.rs:1047` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/tests.rs:1053` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/tests.rs:1054` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/tests.rs:1059` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/tests.rs:1060` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/tests.rs:1076` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/tests.rs:1077` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/tests.rs:1078` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/tests.rs:1083` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/tests.rs:1086` | implicit SQL write locks; sqlx::query("INSERT INTO account_roles(account_id,role_id) SELECT $1,id FROM roles WHERE key='platform_admin'").bind(account).execute(&db.po | 7 (effects; owning entry above) | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/tests.rs:1087` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/tests.rs:1091` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/tests.rs:1095` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/tests.rs:1115` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/tests.rs:1119` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/tests.rs:1137` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/tests.rs:1146` | implicit SQL write locks; sqlx::query("INSERT INTO account_roles(account_id,role_id) SELECT $1,id FROM roles WHERE key='platform_admin'").bind(actor).execute(&db.pool | 7 (effects; owning entry above) | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/tests.rs:1148` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/tests.rs:1149` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/tests.rs:1169` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/tests.rs:1176` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/tests.rs:1192` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/tests.rs:1206` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/tests.rs:1210` | implicit SQL write locks; sqlx::query("INSERT INTO ticket_messages(id,ticket_id,author_account_id,author_kind,body,internal) VALUES($1,$2,$3,$4,$5,false)").bind(Uuid: | 7 (effects; owning entry above) | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/tests.rs:1243` | implicit SQL write locks; sqlx::query("INSERT INTO ticket_messages(id,ticket_id,author_account_id,author_kind,body,internal) VALUES($1,$2,$3,'staff','New reply',false | 7 (effects; owning entry above) | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/tests.rs:1266` | implicit SQL write locks; "INSERT INTO account_roles(account_id,role_id) SELECT $1,id FROM roles WHERE key='support'", | 7 (effects; owning entry above) | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/tests.rs:1281` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/tests.rs:1283` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/tests.rs:1284` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/tests.rs:1292` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/web.rs:49` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/web.rs:83` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/web.rs:84` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/web.rs:86` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/web.rs:89` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/web.rs:118` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/web.rs:136` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/web.rs:137` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/web.rs:138` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/web.rs:148` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/web.rs:153` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/web.rs:174` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/web.rs:191` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/web.rs:194` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/web.rs:196` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/web.rs:209` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/web.rs:275` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/web.rs:308` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/web.rs:324` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/web.rs:330` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/web.rs:347` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/web.rs:375` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/web.rs:380` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/web.rs:384` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/web.rs:388` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/web.rs:392` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/web.rs:458` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/web.rs:463` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/web.rs:472` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/web.rs:478` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/web.rs:489` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/web.rs:584` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/web.rs:591` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/web.rs:601` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/web.rs:609` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/web.rs:615` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/web.rs:623` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/web.rs:629` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/web.rs:637` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/web.rs:658` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/web.rs:664` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/support/src/tickets/web.rs:667` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/ui/i18n/en.ftl:139` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/ui/src/lib.rs:244` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/ui/src/lib.rs:430` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/ui/src/lib.rs:434` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/wallet/README.md:3` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/wallet/README.md:10` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/wallet/README.md:11` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/wallet/README.md:41` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/wallet/README.md:74` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/wallet/README.md:93` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/wallet/src/debt.rs:45` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/wallet/src/debt.rs:66` | FOR UPDATE; query tables coin_purchase_reversals; sqlx::query("SELECT id FROM coin_purchase_reversals WHERE id=ANY($1) ORDER BY created_at,id LIMIT 128 FOR UPDATE") | 4 / owning domain resource | no | n.a. | yes |
| `portal-rs/crates/wallet/src/debt.rs:100` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/wallet/src/debt.rs:102` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/wallet/src/debt.rs:111` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/wallet/src/domain.rs:24` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/wallet/src/domain.rs:50` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/wallet/src/domain.rs:54` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/wallet/src/domain.rs:127` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/wallet/src/domain.rs:307` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/wallet/src/domain.rs:313` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/wallet/src/eligibility.rs:7` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/wallet/src/eligibility.rs:15` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/wallet/src/eligibility.rs:17` | FOR SHARE; query tables accounts; "SELECT status IN ('active','suspended') FROM accounts WHERE id=$1 FOR SHARE", | 3 | no | n.a. | yes |
| `portal-rs/crates/wallet/src/ledger.rs:58` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/wallet/src/ledger.rs:59` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/wallet/src/ledger.rs:60` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/wallet/src/ledger.rs:61` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/wallet/src/ledger.rs:69` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/wallet/src/ledger.rs:74` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/wallet/src/ledger.rs:141` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/wallet/src/ledger.rs:201` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/wallet/src/ledger.rs:334` | FOR UPDATE; query tables wallet_reservations; let row = sqlx::query("SELECT amount-COALESCE((to_jsonb(wallet_reservations)-&gt;&gt;'consumed')::bigint,0) AS amount, status FROM wallet_reservat | 7 / domain namespace | no | n.a. | yes |
| `portal-rs/crates/wallet/src/ledger.rs:455` | FOR UPDATE; query tables wallet_accounts; sqlx::query("SELECT account_id FROM wallet_accounts WHERE account_id=$1 FOR UPDATE") | 7 / domain namespace | no | n.a. | yes |
| `portal-rs/crates/wallet/src/ledger.rs:538` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/wallet/src/ledger.rs:552` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/wallet/src/ledger.rs:568` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/wallet/src/ledger.rs:616` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/wallet/src/ledger.rs:652` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/wallet/src/ledger.rs:664` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/wallet/src/ledger.rs:665` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/wallet/src/ledger.rs:701` | FOR UPDATE; query tables wallet_reservations; let row = sqlx::query("SELECT amount-COALESCE((to_jsonb(wallet_reservations)-&gt;&gt;'consumed')::bigint,0) AS amount, status FROM wallet_reservat | 7 / domain namespace | no | n.a. | yes |
| `portal-rs/crates/wallet/src/ledger.rs:750` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/wallet/src/ledger.rs:911` | FOR UPDATE; query tables wallet_reservations; let row = sqlx::query("SELECT amount-consumed AS remaining,status FROM wallet_reservations WHERE id=$1 FOR UPDATE").bind(reservation).fetch_ | 7 / domain namespace | no | n.a. | yes |
| `portal-rs/crates/wallet/src/lib.rs:3` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/wallet/src/wire.rs:103` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/wallet/src/wire.rs:530` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/wallet/src/wire.rs:538` | FOR SHARE; query tables accounts; match sqlx::query_scalar::&lt;_,bool&gt;("SELECT status='active' AND email_verified_at IS NOT NULL FROM accounts WHERE id=$1 FOR SHARE").bind(acco | 3 | no | n.a. | yes |
| `portal-rs/crates/wallet/src/wire.rs:544` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/wallet/src/wire.rs:548` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/wallet/src/wire.rs:549` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/wallet/src/wire.rs:550` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/wallet/src/wire.rs:551` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/wallet/src/wire.rs:552` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/wallet/tests/coin_http/mod.rs:75` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/wallet/tests/coin_http/mod.rs:476` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/wallet/tests/coin_http/mod.rs:527` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/wallet/tests/coin_http/mod.rs:772` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/wallet/tests/coin_http/mod.rs:812` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/wallet/tests/debt.rs:20` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/wallet/tests/debt.rs:21` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/wallet/tests/debt.rs:162` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/wallet/tests/debt.rs:191` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/wallet/tests/debt.rs:192` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/wallet/tests/debt.rs:206` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/wallet/tests/debt.rs:210` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/wallet/tests/postgres.rs:26` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/wallet/tests/postgres.rs:28` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/wallet/tests/postgres.rs:30` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/wallet/tests/postgres.rs:32` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/wallet/tests/postgres.rs:389` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/wallet/tests/postgres.rs:650` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/wallet/tests/postgres.rs:668` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/wallet/tests/postgres.rs:684` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/wallet/tests/postgres.rs:690` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/wallet/tests/postgres.rs:706` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/wallet/tests/postgres.rs:713` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/wallet/tests/postgres.rs:720` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/wallet/tests/postgres.rs:734` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/wallet/tests/postgres.rs:752` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/wallet/tests/postgres.rs:1045` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/wallet/tests/postgres.rs:1050` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/wallet/tests/postgres.rs:1066` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/wallet/tests/postgres.rs:1125` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/web/README.md:14` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/web/src/chrome.rs:72` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/web/src/chrome.rs:95` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/web/src/lib.rs:80` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/web/src/lib.rs:104` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/web/src/lib.rs:198` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/web/src/ratelimit.rs:442` | FOR UPDATE SKIP LOCKED; LIMIT 1000 FOR UPDATE SKIP LOCKED)", | 4 / owning domain resource | no | n.a. | yes |
| `portal-rs/crates/web/src/session.rs:222` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/web/src/session.rs:293` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/web/src/session.rs:311` | FOR UPDATE; query tables accounts; sqlx::query("SELECT id FROM accounts WHERE id=ANY($1) ORDER BY id FOR UPDATE") | 3 | yes | n.a. | fixed in this commit |
| `portal-rs/crates/web/src/session.rs:325` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/web/src/session.rs:485` | FOR UPDATE SKIP LOCKED; query tables web_sessions; sqlx::query("DELETE FROM web_sessions WHERE id_hash IN (SELECT id_hash FROM web_sessions WHERE idle_expires_at&lt;=now() OR absolute_expires_at | 6 | no | n.a. | yes |
| `portal-rs/crates/web/src/session_tests.rs:557` | test fixture/blocker: FOR UPDATE; query tables accounts; sqlx::query("SELECT id FROM accounts WHERE id=$1 FOR UPDATE") | 3 | n.a. | n.a. | yes |
| `portal-rs/crates/web/src/session_tests.rs:574` | test fixture/blocker: FOR UPDATE; query tables web_sessions; let session_lock = sqlx::query("SELECT id_hash FROM web_sessions WHERE id_hash=$1 FOR UPDATE") | 6 | n.a. | n.a. | yes |
| `portal-rs/crates/web/src/surface.rs:295` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/web/src/surface.rs:359` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/wiki/README.md:6` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/wiki/README.md:8` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/wiki/README.md:10` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/wiki/README.md:12` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/wiki/README.md:13` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/wiki/src/entities.rs:565` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/wiki/src/entities/README.md:33` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/wiki/src/entities/tests.rs:18` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/wiki/src/entities/tests.rs:24` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/wiki/src/entities/tests.rs:26` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/wiki/src/entities/tests.rs:150` | implicit SQL write locks; sqlx::query("INSERT INTO account_roles(account_id,role_id) SELECT $1,id FROM roles WHERE key='platform_admin'").bind(actor).execute(&db.pool | 7 (effects; owning entry above) | n.a. | n.a. | yes |
| `portal-rs/crates/wiki/src/entities/tests.rs:153` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/wiki/src/entities/tests.rs:371` | implicit SQL write locks; sqlx::query("INSERT INTO account_roles(account_id,role_id) SELECT $1,id FROM roles WHERE key='platform_admin'").bind(actor).execute(&db.pool | 7 (effects; owning entry above) | n.a. | n.a. | yes |
| `portal-rs/crates/wiki/src/entities/tests.rs:374` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/wiki/src/lib.rs:15` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/wiki/src/lib.rs:16` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/wiki/src/places/README.md:23` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/wiki/src/repository.rs:1` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/wiki/src/repository.rs:5` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/wiki/src/repository.rs:37` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/wiki/src/repository.rs:42` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/wiki/src/repository.rs:46` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/wiki/src/repository.rs:49` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/wiki/src/repository.rs:51` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/wiki/src/repository.rs:54` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/wiki/src/repository.rs:79` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/wiki/src/repository.rs:83` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/wiki/src/repository.rs:85` | canonical account/resource/RBAC/durable-proof acquisition + live authority | 3→4→5→6 | yes | yes | yes |
| `portal-rs/crates/wiki/src/repository.rs:90` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/wiki/src/repository.rs:93` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/wiki/src/repository.rs:94` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/wiki/src/repository.rs:99` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/wiki/src/repository.rs:107` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/wiki/src/repository.rs:120` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/wiki/src/repository.rs:131` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/wiki/src/repository.rs:138` | canonical account/resource/RBAC/durable-proof acquisition + live authority | 3→4→5→6 | yes | yes | yes |
| `portal-rs/crates/wiki/src/repository.rs:143` | pg_advisory_xact_lock; sqlx::query("SELECT pg_advisory_xact_lock(20261105220206)") | domain namespace (README inventory) | yes | yes | yes |
| `portal-rs/crates/wiki/src/repository.rs:152` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/wiki/src/repository.rs:153` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/wiki/src/repository.rs:154` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/wiki/src/repository.rs:159` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/wiki/src/repository.rs:188` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/wiki/src/repository.rs:195` | canonical account/resource/RBAC/durable-proof acquisition + live authority | 3→4→5→6 | yes | yes | yes |
| `portal-rs/crates/wiki/src/repository.rs:200` | pg_advisory_xact_lock; sqlx::query("SELECT pg_advisory_xact_lock(20261105220206)") | domain namespace (README inventory) | yes | yes | yes |
| `portal-rs/crates/wiki/src/repository.rs:203` | FOR UPDATE; query tables wiki_guides; sqlx::query("SELECT id FROM wiki_guides WHERE id=$1 FOR UPDATE") | 4 / owning domain resource | yes | yes | yes |
| `portal-rs/crates/wiki/src/repository.rs:213` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/wiki/src/repository.rs:214` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/wiki/src/repository.rs:215` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/wiki/src/repository.rs:222` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/wiki/src/repository.rs:249` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/wiki/src/repository.rs:251` | canonical account/resource/RBAC/durable-proof acquisition + live authority | 3→4→5→6 | yes | yes | yes |
| `portal-rs/crates/wiki/src/repository.rs:256` | pg_advisory_xact_lock; sqlx::query("SELECT pg_advisory_xact_lock(20261105220206)") | domain namespace (README inventory) | yes | yes | yes |
| `portal-rs/crates/wiki/src/repository.rs:259` | FOR UPDATE; query tables wiki_guides; sqlx::query("SELECT id FROM wiki_guides WHERE id=$1 FOR UPDATE") | 4 / owning domain resource | yes | yes | yes |
| `portal-rs/crates/wiki/src/repository.rs:269` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/wiki/src/repository.rs:270` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/wiki/src/repository.rs:271` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/wiki/src/repository.rs:277` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/wiki/src/repository.rs:305` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/wiki/src/repository.rs:308` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/wiki/src/repository.rs:310` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/wiki/src/repository.rs:315` | canonical account/resource/RBAC/durable-proof acquisition + live authority | 3→4→5→6 | yes | yes | yes |
| `portal-rs/crates/wiki/src/repository.rs:316` | FOR UPDATE; query tables wiki_guides; sqlx::query("SELECT id,status,version,revision_version,entity_key FROM wiki_guides WHERE id=$1 FOR UPDATE").bind(id).execute(connection).awa | 4 / owning domain resource | yes | yes | yes |
| `portal-rs/crates/wiki/src/repository.rs:318` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/wiki/src/repository.rs:321` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/wiki/src/repository.rs:322` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/wiki/src/repository.rs:324` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/wiki/src/repository.rs:326` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/wiki/src/repository.rs:329` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/wiki/src/repository.rs:364` | FOR UPDATE; query tables wiki_guides; sqlx::query_as("SELECT id,status,version,revision_version,entity_key FROM wiki_guides WHERE id=$1 FOR UPDATE").bind(id).fetch_optional(&mut  | 4 / owning domain resource | yes | yes | yes |
| `portal-rs/crates/wiki/src/repository.rs:374` | pg_advisory_xact_lock; sqlx::query("SELECT pg_advisory_xact_lock(20261105220206)") | domain namespace (README inventory) | yes | yes | yes |
| `portal-rs/crates/wiki/src/web.rs:1` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/wiki/src/web.rs:16` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/wiki/src/web.rs:17` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/wiki/src/web.rs:27` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/wiki/src/web.rs:31` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/wiki/src/web.rs:32` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/wiki/src/web.rs:33` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/wiki/src/web.rs:47` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/wiki/src/web.rs:48` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/wiki/src/web.rs:54` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/wiki/src/web.rs:61` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/wiki/src/web.rs:64` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/wiki/src/web.rs:65` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/wiki/src/web.rs:262` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/wiki/src/web.rs:298` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/wiki/src/web.rs:306` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/wiki/src/web.rs:325` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/wiki/src/web.rs:391` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/wiki/src/web.rs:410` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/wiki/src/web.rs:445` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/wiki/src/web.rs:492` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/wiki/src/web.rs:536` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/wiki/src/web.rs:555` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/wiki/src/web.rs:563` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/wiki/src/web.rs:600` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/wiki/src/web/tests.rs:9` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/wiki/src/web/tests.rs:19` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/wiki/src/web/tests.rs:20` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/wiki/src/web/tests.rs:21` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/wiki/src/web/tests.rs:29` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/wiki/src/web/tests.rs:35` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/wiki/src/web/tests.rs:37` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/wiki/src/web/tests.rs:82` | implicit SQL write locks; sqlx::query("INSERT INTO account_roles(account_id,role_id) SELECT $1,id FROM roles WHERE key='platform_admin'").bind(identity.actor).execute | 7 (effects; owning entry above) | n.a. | n.a. | yes |
| `portal-rs/crates/wiki/src/web/tests.rs:93` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/wiki/src/web/tests.rs:102` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/wiki/src/web/tests.rs:106` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/wiki/src/web/tests.rs:148` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/wiki/src/web/tests.rs:161` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/wiki/src/web/tests.rs:162` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/wiki/src/web/tests.rs:342` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/wiki/src/web/tests.rs:350` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/wiki/src/web/tests.rs:351` | implicit SQL write locks; sqlx::query("INSERT INTO role_permissions(role_id,permission) VALUES($1,$2)") | 7 (effects; owning entry above) | n.a. | n.a. | yes |
| `portal-rs/crates/wiki/src/web/tests.rs:353` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/wiki/src/web/tests.rs:358` | implicit SQL write locks; sqlx::query("INSERT INTO account_roles(account_id,role_id) VALUES($1,$2)") | 7 (effects; owning entry above) | n.a. | n.a. | yes |
| `portal-rs/crates/wiki/src/web/tests.rs:369` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/wiki/src/web/tests.rs:374` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/wiki/src/web/tests.rs:412` | implicit SQL write locks; sqlx::query("DELETE FROM account_roles WHERE account_id=$1") | 7 (effects; owning entry above) | n.a. | n.a. | yes |
| `portal-rs/crates/wiki/src/web/tests.rs:617` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/wiki/src/web/tests.rs:627` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/wiki/src/web/tests.rs:631` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/wiki/src/web/tests.rs:640` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/wiki/src/web/tests.rs:644` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/wiki/src/web/tests.rs:750` | implicit SQL write locks; sqlx::query("INSERT INTO account_roles(account_id,role_id) SELECT $1,id FROM roles WHERE key='platform_admin' ON CONFLICT DO NOTHING").bind( | 7 (effects; owning entry above) | n.a. | n.a. | yes |
| `portal-rs/crates/wiki/src/web/tests.rs:759` | test fixture/blocker: pg_advisory_xact_lock; sqlx::query("SELECT pg_advisory_xact_lock(20261105220206)") | domain namespace (README inventory) | n.a. | n.a. | yes |
| `portal-rs/crates/wiki/src/web/tests.rs:782` | observer SQL; no lock acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/wiki/src/web/tests.rs:788` | test fixture/blocker: pg_advisory_xact_lock; sqlx::query("SELECT pg_advisory_xact_lock(71010)") | 5 | n.a. | n.a. | yes |
| `portal-rs/crates/wiki/src/web/tests.rs:792` | implicit SQL write locks; sqlx::query("DELETE FROM account_roles WHERE account_id=$1") | 7 (effects; owning entry above) | n.a. | n.a. | yes |
| `portal-rs/crates/wiki/src/web/tests.rs:877` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/wiki/src/web/tests.rs:886` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/wiki/src/web/tests.rs:918` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/wiki/src/web/tests.rs:932` | test fixture/blocker: FOR UPDATE; query tables wiki_guides; sqlx::query("SELECT id FROM wiki_guides WHERE id=$1 FOR UPDATE") | 4 / owning domain resource | n.a. | n.a. | yes |
| `portal-rs/crates/wiki/src/web/tests.rs:961` | test fixture/blocker: pg_try_advisory_xact_lock; sqlx::query_scalar::&lt;_, bool&gt;("SELECT pg_try_advisory_xact_lock(71010)") | 5 | n.a. | n.a. | yes |
| `portal-rs/crates/wiki/tests/transaction_authorization.rs:2` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/wiki/tests/transaction_authorization.rs:3` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/wiki/tests/transaction_authorization.rs:12` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/wiki/tests/transaction_authorization.rs:18` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/wiki/tests/transaction_authorization.rs:20` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/wiki/tests/transaction_authorization.rs:64` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/wiki/tests/transaction_authorization.rs:68` | implicit SQL write locks; sqlx::query("INSERT INTO account_roles(account_id,role_id) SELECT $1,id FROM roles WHERE key='platform_admin'").bind(actor.account).execute( | 7 (effects; owning entry above) | n.a. | n.a. | yes |
| `portal-rs/crates/wiki/tests/transaction_authorization.rs:93` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/wiki/tests/transaction_authorization.rs:100` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/wiki/tests/transaction_authorization.rs:101` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/wiki/tests/transaction_authorization.rs:102` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/wiki/tests/transaction_authorization.rs:103` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/wiki/tests/transaction_authorization.rs:104` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/wiki/tests/transaction_authorization.rs:106` | implicit SQL write locks; sqlx::query("INSERT INTO role_permissions(role_id,permission) VALUES($1,$2)") | 7 (effects; owning entry above) | n.a. | n.a. | yes |
| `portal-rs/crates/wiki/tests/transaction_authorization.rs:108` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/wiki/tests/transaction_authorization.rs:112` | implicit SQL write locks; sqlx::query("UPDATE account_roles SET role_id=$2 WHERE account_id=$1") | 7 (effects; owning entry above) | n.a. | n.a. | yes |
| `portal-rs/crates/wiki/tests/transaction_authorization.rs:133` | test fixture/blocker: FOR UPDATE; query tables wiki_guides; sqlx::query("SELECT id FROM wiki_guides WHERE id=$1 FOR UPDATE") | 4 / owning domain resource | n.a. | n.a. | yes |
| `portal-rs/crates/wiki/tests/transaction_authorization.rs:150` | implicit SQL write locks; sqlx::query("INSERT INTO account_roles(account_id,role_id) SELECT $1,id FROM roles WHERE key='security_admin'").bind(manager.account).execut | 7 (effects; owning entry above) | n.a. | n.a. | yes |
| `portal-rs/crates/wiki/tests/transaction_authorization.rs:151` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/wiki/tests/transaction_authorization.rs:155` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/wiki/tests/transaction_authorization.rs:157` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/wiki/tests/transaction_authorization.rs:158` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/wiki/tests/transaction_authorization.rs:159` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/wiki/tests/transaction_authorization.rs:160` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/wiki/tests/transaction_authorization.rs:177` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/world/CATALOG_WIKI_REQUIREMENTS.md:33` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/world/CATALOG_WIKI_REQUIREMENTS.md:57` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/world/CATALOG_WIKI_REQUIREMENTS.md:100` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/world/README.md:63` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/world/README.md:87` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/world/README.md:137` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/world/src/admin.rs:1` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/world/src/admin.rs:15` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/world/src/admin.rs:16` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/world/src/admin.rs:27` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/world/src/admin.rs:29` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/world/src/admin.rs:43` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/world/src/admin.rs:46` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/world/src/admin.rs:47` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/world/src/admin.rs:63` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/world/src/admin.rs:72` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/world/src/admin.rs:185` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/world/src/admin.rs:306` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/world/src/admin.rs:307` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/world/src/admin.rs:308` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/world/src/admin.rs:309` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/world/src/admin.rs:312` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/world/src/admin.rs:313` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/world/src/admin.rs:318` | canonical account/resource/RBAC/durable-proof acquisition + live authority | 3→4→5→6 | yes | yes | yes |
| `portal-rs/crates/world/src/admin.rs:327` | FOR UPDATE; query tables world_active_version; "SELECT version_id FROM world_active_version WHERE singleton FOR UPDATE", | 4 / owning domain resource | yes | yes | yes |
| `portal-rs/crates/world/src/admin.rs:332` | FOR UPDATE; query tables world_catalog_publication; "SELECT snapshot_id FROM world_catalog_publication WHERE singleton FOR UPDATE", | 4 / owning domain resource | yes | yes | yes |
| `portal-rs/crates/world/src/admin.rs:342` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/world/src/admin.rs:343` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/world/src/admin.rs:344` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/world/src/admin.rs:345` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/world/src/admin.rs:354` | FOR UPDATE; query tables world_active_version; sqlx::query_as("SELECT version_id FROM world_active_version WHERE singleton FOR UPDATE") | 4 / owning domain resource | yes | yes | yes |
| `portal-rs/crates/world/src/admin.rs:377` | FOR UPDATE; query tables world_catalog_publication; "SELECT snapshot_id FROM world_catalog_publication WHERE singleton FOR UPDATE", | 4 / owning domain resource | yes | yes | yes |
| `portal-rs/crates/world/src/admin.rs:384` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/world/src/admin.rs:385` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/world/src/admin.rs:386` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/world/src/admin.rs:387` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/world/src/admin.rs:390` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/world/src/admin.rs:394` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/world/src/admin.rs:500` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/world/src/admin.rs:503` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/world/src/admin.rs:505` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/world/src/admin.rs:520` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/world/src/admin_pool_tests.rs:2` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/world/src/admin_pool_tests.rs:3` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/world/src/admin_pool_tests.rs:12` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/world/src/admin_pool_tests.rs:18` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/world/src/admin_pool_tests.rs:20` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/world/src/admin_pool_tests.rs:64` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/world/src/admin_pool_tests.rs:68` | implicit SQL write locks; sqlx::query("INSERT INTO account_roles(account_id,role_id) SELECT $1,id FROM roles WHERE key='platform_admin'").bind(actor.account).execute( | 7 (effects; owning entry above) | n.a. | n.a. | yes |
| `portal-rs/crates/world/src/admin_pool_tests.rs:120` | observer SQL; no lock acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/world/src/admin_pool_tests.rs:135` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/world/src/admin_pool_tests.rs:139` | test fixture/blocker: pg_advisory_xact_lock; sqlx::query("SELECT pg_advisory_xact_lock(71010)") | 5 | n.a. | n.a. | yes |
| `portal-rs/crates/world/src/admin_pool_tests.rs:153` | observer SQL; no lock acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/world/src/admin_pool_tests.rs:178` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/world/src/admin_pool_tests.rs:183` | implicit SQL write locks; sqlx::query("INSERT INTO account_roles(account_id,role_id) SELECT $1,id FROM roles WHERE key='security_admin'").bind(manager.account).execut | 7 (effects; owning entry above) | n.a. | n.a. | yes |
| `portal-rs/crates/world/src/admin_pool_tests.rs:190` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/world/src/admin_pool_tests.rs:191` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/world/src/admin_pool_tests.rs:192` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/world/src/admin_pool_tests.rs:193` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/world/src/admin_pool_tests.rs:217` | test fixture/blocker: pg_advisory_xact_lock; sqlx::query("SELECT pg_advisory_xact_lock(71010)") | 5 | n.a. | n.a. | yes |
| `portal-rs/crates/world/src/admin_pool_tests.rs:243` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/world/src/admin_pool_tests.rs:265` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/world/src/admin_pool_tests.rs:272` | implicit SQL write locks; sqlx::query("INSERT INTO account_roles(account_id,role_id) SELECT $1,id FROM roles WHERE key='security_admin'").bind(manager.account).execut | 7 (effects; owning entry above) | n.a. | n.a. | yes |
| `portal-rs/crates/world/src/admin_pool_tests.rs:279` | test fixture/blocker: FOR UPDATE; "SELECT singleton FROM {table} WHERE singleton FOR UPDATE" | 4 / owning domain resource | n.a. | n.a. | yes |
| `portal-rs/crates/world/src/admin_pool_tests.rs:294` | observer SQL; no lock acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/world/src/admin_pool_tests.rs:300` | implicit SQL write locks; sqlx::query("INSERT INTO account_roles(account_id,role_id) VALUES($1,$2)") | 7 (effects; owning entry above) | n.a. | n.a. | yes |
| `portal-rs/crates/world/src/admin_pool_tests.rs:318` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/world/src/admin_pool_tests.rs:338` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/world/src/admin_pool_tests.rs:339` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/world/src/admin_pool_tests.rs:343` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/world/src/admin_pool_tests.rs:344` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/world/src/admin_pool_tests.rs:351` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/world/src/admin_pool_tests.rs:367` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/world/src/admin_pool_tests.rs:370` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/world/src/admin_pool_tests.rs:377` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/world/src/admin_pool_tests.rs:405` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/world/src/admin_pool_tests.rs:406` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/world/src/admin_pool_tests.rs:419` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/world/src/admin_pool_tests.rs:440` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/world/src/admin_pool_tests.rs:454` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/world/src/admin_pool_tests.rs:458` | test fixture/blocker: pg_advisory_xact_lock; sqlx::query("SELECT pg_advisory_xact_lock(71010)") | 5 | n.a. | n.a. | yes |
| `portal-rs/crates/world/src/admin_pool_tests.rs:472` | test fixture/blocker: FOR UPDATE; query tables world_active_version; sqlx::query("SELECT singleton FROM world_active_version WHERE singleton FOR UPDATE") | 4 / owning domain resource | n.a. | n.a. | yes |
| `portal-rs/crates/world/src/admin_pool_tests.rs:478` | test fixture/blocker: FOR UPDATE; query tables world_catalog_publication; sqlx::query("SELECT singleton FROM world_catalog_publication WHERE singleton FOR UPDATE") | 4 / owning domain resource | n.a. | n.a. | yes |
| `portal-rs/crates/world/src/admin_tests.rs:43` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/world/src/admin_tests.rs:44` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/world/src/admin_tests.rs:45` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/world/src/admin_tests.rs:53` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/world/src/admin_tests.rs:59` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/world/src/admin_tests.rs:61` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/world/src/admin_tests.rs:77` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/world/src/admin_tests.rs:88` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/world/src/admin_tests.rs:100` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/world/src/admin_tests.rs:101` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/world/src/admin_tests.rs:113` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/world/src/admin_tests.rs:150` | implicit SQL write locks; sqlx::query("INSERT INTO account_roles(account_id,role_id) SELECT $1,id FROM roles WHERE key='platform_admin'").bind(identity.account).execu | 7 (effects; owning entry above) | n.a. | n.a. | yes |
| `portal-rs/crates/world/src/admin_tests.rs:433` | implicit SQL write locks; sqlx::query("DELETE FROM role_permissions WHERE permission='game_catalog.snapshots.activate'") | 7 (effects; owning entry above) | n.a. | n.a. | yes |
| `portal-rs/crates/world/src/admin_tests.rs:448` | implicit SQL write locks; sqlx::query("DELETE FROM account_roles WHERE account_id=$1") | 7 (effects; owning entry above) | n.a. | n.a. | yes |
| `portal-rs/crates/world/src/catalog/cache.rs:2` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/world/src/catalog/cache.rs:38` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/world/src/catalog/mod.rs:593` | FOR UPDATE; query tables world_catalog_candidates; "SELECT artifact_sha256,artifact FROM world_catalog_candidates WHERE snapshot_id=$1 FOR UPDATE", | 4 / owning domain resource | no | n.a. | yes |
| `portal-rs/crates/world/src/catalog/publication.rs:78` | FOR UPDATE; query tables world_catalog_publication; let (epoch, snapshot): (Option&lt;String&gt;, Option&lt;String&gt;) = sqlx::query_as("SELECT authority_epoch,COALESCE(configured_snapshot_id,snapshot_id | 4 / owning domain resource | no | n.a. | yes |
| `portal-rs/crates/world/src/catalog/publication.rs:136` | FOR UPDATE; query tables world_catalog_publication; sqlx::query("SELECT singleton FROM world_catalog_publication WHERE singleton FOR UPDATE") | 4 / owning domain resource | no | n.a. | yes |
| `portal-rs/crates/world/src/lib.rs:41` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/world/src/lib.rs:42` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/world/src/sprites/mod.rs:370` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/world/src/store.rs:13` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/world/src/store.rs:281` | FOR UPDATE; query tables world_active_version; sqlx::query_as("SELECT version_id, previous_version_id FROM world_active_version WHERE singleton FOR UPDATE") | 4 / owning domain resource | no | n.a. | yes |
| `portal-rs/crates/world/tests/fixtures/game/tools/content-schema/monster-authoring/monster.schema.json:1025` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/world/tests/fixtures/game/tools/content-schema/monster-authoring/monster.schema.json:1052` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/crates/world/tests/postgres.rs:245` | test fixture/blocker: FOR UPDATE; query tables world_active_version; sqlx::query("SELECT version_id FROM world_active_version WHERE singleton FOR UPDATE") | 4 / owning domain resource | n.a. | n.a. | yes |
| `portal-rs/crates/world/tests/postgres.rs:494` | test reference / authority adapter / fixture definition | n.a. | n.a. | n.a. | yes |
| `portal-rs/e2e/COVERAGE.md:9` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/e2e/COVERAGE.md:10` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/e2e/COVERAGE.md:11` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/e2e/COVERAGE.md:12` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/e2e/COVERAGE.md:13` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/e2e/COVERAGE.md:14` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/e2e/COVERAGE.md:15` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/e2e/COVERAGE.md:16` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/e2e/COVERAGE.md:17` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/e2e/COVERAGE.md:18` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/e2e/COVERAGE.md:19` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/e2e/COVERAGE.md:20` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/e2e/COVERAGE.md:21` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/e2e/COVERAGE.md:22` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/e2e/COVERAGE.md:23` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/e2e/COVERAGE.md:24` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/e2e/COVERAGE.md:25` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/e2e/COVERAGE.md:26` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/e2e/COVERAGE.md:27` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/e2e/COVERAGE.md:28` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/e2e/COVERAGE.md:29` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/e2e/COVERAGE.md:30` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/e2e/COVERAGE.md:31` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/e2e/COVERAGE.md:32` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/e2e/COVERAGE.md:33` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/e2e/COVERAGE.md:75` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/e2e/COVERAGE.md:76` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/e2e/COVERAGE.md:94` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/e2e/COVERAGE.md:95` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/e2e/COVERAGE.md:96` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/e2e/COVERAGE.md:97` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/e2e/COVERAGE.md:98` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/e2e/COVERAGE.md:99` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/e2e/COVERAGE.md:100` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/e2e/COVERAGE.md:101` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/e2e/COVERAGE.md:102` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/e2e/COVERAGE.md:103` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/e2e/COVERAGE.md:108` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/e2e/COVERAGE.md:109` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/e2e/COVERAGE.md:110` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/e2e/COVERAGE.md:111` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/e2e/COVERAGE.md:112` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/e2e/COVERAGE.md:113` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/e2e/COVERAGE.md:114` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/e2e/COVERAGE.md:123` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/e2e/COVERAGE.md:254` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/e2e/COVERAGE.md:255` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/e2e/COVERAGE.md:256` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/e2e/COVERAGE.md:257` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/e2e/COVERAGE.md:294` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/e2e/COVERAGE.md:295` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/e2e/COVERAGE.md:296` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/e2e/COVERAGE.md:297` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/e2e/COVERAGE.md:298` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/e2e/COVERAGE.md:299` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/e2e/COVERAGE.md:300` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/e2e/COVERAGE.md:301` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/e2e/COVERAGE.md:302` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/e2e/COVERAGE.md:303` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/e2e/COVERAGE.md:317` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/e2e/COVERAGE.md:318` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/e2e/COVERAGE.md:319` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/e2e/COVERAGE.md:320` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/e2e/COVERAGE.md:321` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/e2e/COVERAGE.md:322` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/e2e/COVERAGE.md:323` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/e2e/COVERAGE.md:324` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/e2e/COVERAGE.md:325` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/e2e/COVERAGE.md:326` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/e2e/COVERAGE.md:385` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/e2e/ENVIRONMENT.md:525` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/e2e/README.md:35` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/e2e/README.md:276` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/e2e/REPORT.md:59` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/e2e/REPORT.md:63` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/e2e/REPORT.md:80` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/e2e/inventory.json:1460` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/e2e/inventory.json:1474` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/e2e/inventory.json:1566` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/e2e/inventory.py:108` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/e2e/qualification.json:1488` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/e2e/qualification.json:1497` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/e2e/qualification.json:1506` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/e2e/qualification.json:1515` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/e2e/qualification.json:5052` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/e2e/qualification.json:7914` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/e2e/qualification.json:10776` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/e2e/qualification.json:13638` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/e2e/qualification.json:15795` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/e2e/qualification.json:15818` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/e2e/qualification.json:15841` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/e2e/qualification.json:15864` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/e2e/qualification.json:15887` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/e2e/qualification.json:15909` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/e2e/qualification.json:15929` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/e2e/qualification.json:15952` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/e2e/qualification.json:15975` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/e2e/qualification.json:15998` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/e2e/qualification.json:16018` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/e2e/qualification.json:16038` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/e2e/qualification.json:16058` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/e2e/qualification.json:16078` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/e2e/qualification.json:16099` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/e2e/qualification.json:16120` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/e2e/qualification.json:16141` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/e2e/qualification.json:16162` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/e2e/qualification.json:16182` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/e2e/qualification.json:16202` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/e2e/qualification.json:16221` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/e2e/qualification.json:16241` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/e2e/qualification.json:16260` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/e2e/qualification.json:16279` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/e2e/qualification.json:16299` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/e2e/qualification.json:17140` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/e2e/qualification.json:17160` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/e2e/qualification.json:17505` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/e2e/qualification.json:17524` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/e2e/qualification.json:17543` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/e2e/qualification.json:17563` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/e2e/qualification.json:17583` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/e2e/qualification.json:17603` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/e2e/qualification.json:17622` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/e2e/qualification.json:17641` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/e2e/qualification.json:17660` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/e2e/qualification.json:17679` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/e2e/qualification.json:17767` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/e2e/qualification.json:17786` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/e2e/qualification.json:17805` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/e2e/qualification.json:17826` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/e2e/qualification.json:17845` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/e2e/qualification.json:17862` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/e2e/qualification.json:17864` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/e2e/qualification.json:17882` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/e2e/qualification.json:17884` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/e2e/qualification.json:18028` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/e2e/qualification.json:18042` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/e2e/qualification.json:20568` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/e2e/qualification.json:20588` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/e2e/qualification.json:20608` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/e2e/qualification.json:20629` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/e2e/qualification.json:21320` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/e2e/qualification.json:21339` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/e2e/qualification.json:21359` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/e2e/qualification.json:21378` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/e2e/qualification.json:21397` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/e2e/qualification.json:21416` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/e2e/qualification.json:21435` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/e2e/qualification.json:21454` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/e2e/qualification.json:21473` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/e2e/qualification.json:21493` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/e2e/qualification.json:21733` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/e2e/qualification.json:21752` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/e2e/qualification.json:21771` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/e2e/qualification.json:21790` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/e2e/qualification.json:21810` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/e2e/qualification.json:21830` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/e2e/qualification.json:21850` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/e2e/qualification.json:21870` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/e2e/qualification.json:21890` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/e2e/qualification.json:21910` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/e2e/route-expectations.json:5` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/e2e/route-expectations.json:13` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/e2e/route-expectations.json:21` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/e2e/route-expectations.json:29` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/e2e/route-expectations.json:37` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/e2e/route-expectations.json:45` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/e2e/route-expectations.json:53` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/e2e/route-expectations.json:61` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/e2e/route-expectations.json:69` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/e2e/route-expectations.json:77` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/e2e/route-expectations.json:85` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/e2e/route-expectations.json:93` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/e2e/route-expectations.json:101` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/e2e/route-expectations.json:109` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/e2e/route-expectations.json:117` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/e2e/route-expectations.json:125` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/e2e/route-expectations.json:133` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/e2e/route-expectations.json:141` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/e2e/route-expectations.json:149` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/e2e/route-expectations.json:157` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/e2e/route-expectations.json:165` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/e2e/route-expectations.json:173` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/e2e/route-expectations.json:181` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/e2e/route-expectations.json:189` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/e2e/route-expectations.json:197` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/e2e/route-expectations.json:517` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/e2e/route-expectations.json:525` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/e2e/route-expectations.json:669` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/e2e/route-expectations.json:677` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/e2e/route-expectations.json:685` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/e2e/route-expectations.json:693` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/e2e/route-expectations.json:701` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/e2e/route-expectations.json:709` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/e2e/route-expectations.json:717` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/e2e/route-expectations.json:725` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/e2e/route-expectations.json:733` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/e2e/route-expectations.json:741` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/e2e/route-expectations.json:781` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/e2e/route-expectations.json:789` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/e2e/route-expectations.json:797` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/e2e/route-expectations.json:805` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/e2e/route-expectations.json:813` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/e2e/route-expectations.json:821` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/e2e/route-expectations.json:829` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/e2e/route-expectations.json:900` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/e2e/route-expectations.json:1765` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/e2e/route-expectations.json:1773` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/e2e/route-expectations.json:1781` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/e2e/route-expectations.json:1789` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/e2e/route-expectations.json:2085` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/e2e/route-expectations.json:2093` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/e2e/route-expectations.json:2101` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/e2e/route-expectations.json:2109` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/e2e/route-expectations.json:2117` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/e2e/route-expectations.json:2125` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/e2e/route-expectations.json:2133` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/e2e/route-expectations.json:2141` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/e2e/route-expectations.json:2149` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/e2e/route-expectations.json:2157` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/e2e/route-expectations.json:2269` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/e2e/route-expectations.json:2277` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/e2e/route-expectations.json:2285` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/e2e/route-expectations.json:2293` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/e2e/route-expectations.json:2301` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/e2e/route-expectations.json:2309` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/e2e/route-expectations.json:2317` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/e2e/route-expectations.json:2325` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/e2e/route-expectations.json:2333` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/e2e/route-expectations.json:2341` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/e2e/specs/admin-analytics.spec.mjs:11` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/e2e/specs/admin-analytics.spec.mjs:14` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/e2e/specs/admin-analytics.spec.mjs:16` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/e2e/specs/admin-analytics.spec.mjs:19` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/e2e/specs/admin-analytics.spec.mjs:22` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/e2e/specs/admin-analytics.spec.mjs:24` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/e2e/specs/admin-analytics.spec.mjs:45` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/e2e/specs/admin.spec.mjs:8` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/e2e/specs/admin.spec.mjs:11` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/e2e/specs/admin.spec.mjs:14` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/e2e/specs/admin.spec.mjs:15` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/e2e/specs/admin.spec.mjs:17` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/e2e/specs/admin.spec.mjs:20` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/e2e/specs/admin.spec.mjs:23` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/e2e/specs/admin.spec.mjs:26` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/e2e/specs/admin.spec.mjs:31` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/e2e/specs/admin.spec.mjs:35` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/e2e/specs/admin.spec.mjs:37` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/e2e/specs/admin.spec.mjs:47` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/e2e/specs/admin.spec.mjs:52` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/e2e/specs/admin.spec.mjs:63` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/e2e/specs/content.spec.mjs:9` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/e2e/specs/content.spec.mjs:19` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/e2e/specs/content.spec.mjs:23` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/e2e/specs/content.spec.mjs:27` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/e2e/specs/content.spec.mjs:39` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/e2e/specs/content.spec.mjs:55` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/e2e/specs/oauth.spec.mjs:13` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/e2e/specs/routes.spec.mjs:15` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/e2e/specs/routes.spec.mjs:16` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/e2e/specs/routes.spec.mjs:17` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/e2e/specs/routes.spec.mjs:26` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/e2e/summarize.mjs:32` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/migrations/20261105170000_t10_rbac_audit.sql:9` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/migrations/20261105170000_t10_rbac_audit.sql:11` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/migrations/20261105170000_t10_rbac_audit.sql:45` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/migrations/20261105170000_t10_rbac_audit.sql:47` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/migrations/20261105170000_t10_rbac_audit.sql:54` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/migrations/20261105170000_t10_rbac_audit.sql:78` | implicit SQL write locks; INSERT INTO role_permissions (role_id, permission) | 7 (effects; owning entry above) | n.a. | n.a. | yes |
| `portal-rs/migrations/20261105170000_t10_rbac_audit.sql:79` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/migrations/20261105170000_t10_rbac_audit.sql:110` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/migrations/20261105170000_t10_rbac_audit.sql:113` | implicit SQL write locks; INSERT INTO role_permissions (role_id, permission) | 7 (effects; owning entry above) | n.a. | n.a. | yes |
| `portal-rs/migrations/20261105170000_t10_rbac_audit.sql:114` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/migrations/20261105170000_t10_rbac_audit.sql:118` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/migrations/20261105170000_t10_rbac_audit.sql:121` | implicit SQL write locks; INSERT INTO role_permissions (role_id, permission) | 7 (effects; owning entry above) | n.a. | n.a. | yes |
| `portal-rs/migrations/20261105170000_t10_rbac_audit.sql:122` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/migrations/20261105170000_t10_rbac_audit.sql:136` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/migrations/20261105170000_t10_rbac_audit.sql:139` | implicit SQL write locks; INSERT INTO role_permissions (role_id, permission) | 7 (effects; owning entry above) | n.a. | n.a. | yes |
| `portal-rs/migrations/20261105170000_t10_rbac_audit.sql:140` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/migrations/20261105170000_t10_rbac_audit.sql:144` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/migrations/20261105170000_t10_rbac_audit.sql:147` | implicit SQL write locks; INSERT INTO role_permissions (role_id, permission) | 7 (effects; owning entry above) | n.a. | n.a. | yes |
| `portal-rs/migrations/20261105170000_t10_rbac_audit.sql:148` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/migrations/20261105170000_t10_rbac_audit.sql:152` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/migrations/20261105170000_t10_rbac_audit.sql:155` | implicit SQL write locks; INSERT INTO role_permissions (role_id, permission) | 7 (effects; owning entry above) | n.a. | n.a. | yes |
| `portal-rs/migrations/20261105170000_t10_rbac_audit.sql:156` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/migrations/20261105170000_t10_rbac_audit.sql:168` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/migrations/20261105220212_t05_password_resets.sql:18` | runtime SQL function reset_guard_session_authentication: FOR UPDATE; query tables accounts; PERFORM id FROM accounts WHERE id=NEW.account_id FOR UPDATE; | 3 | no | n.a. | yes |
| `portal-rs/migrations/20261105220218_t33_tickets.sql:7` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/migrations/20261105220218_t33_tickets.sql:22` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/migrations/20261105220218_t33_tickets.sql:27` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/migrations/20261105220244_t49_players.sql:1` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/migrations/20261105220245_t50_commerce.sql:1` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/migrations/20261105230107_t64_moderation.sql:82` | runtime SQL function bazaar_moderation_fence: pg_advisory_xact_lock_shared; PERFORM pg_advisory_xact_lock_shared(64002); | 1 | no | n.a. | yes |
| `portal-rs/migrations/20261105230107_t64_moderation.sql:90` | runtime SQL function bazaar_moderation_fence: FOR UPDATE; query tables bazaar_listings; PERFORM 1 FROM bazaar_listings WHERE id = listing FOR UPDATE; | 2 | no | n.a. | yes |
| `portal-rs/migrations/20261105231209_t89_season_pass.sql:1` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/migrations/20261105231403_adm-p2a_game_roles.sql:1` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/migrations/20261105231403_adm-p2a_game_roles.sql:2` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/migrations/20261105231403_adm-p2a_game_roles.sql:3` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/migrations/20261105231403_adm-p2a_game_roles.sql:4` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/migrations/20261105231403_adm-p2a_game_roles.sql:60` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/migrations/20261105231403_adm-p2a_game_roles.sql:70` | implicit SQL write locks; INSERT INTO role_permissions (role_id, permission) | 7 (effects; owning entry above) | n.a. | n.a. | yes |
| `portal-rs/migrations/20261105231403_adm-p2a_game_roles.sql:71` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/migrations/20261105231403_adm-p2a_game_roles.sql:81` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/migrations/20261105231403_adm-p2a_game_roles.sql:84` | implicit SQL write locks; INSERT INTO role_permissions (role_id, permission) | 7 (effects; owning entry above) | n.a. | n.a. | yes |
| `portal-rs/migrations/20261105231403_adm-p2a_game_roles.sql:85` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/migrations/20261105231403_adm-p2a_game_roles.sql:98` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/migrations/20261105231403_adm-p2a_game_roles.sql:101` | implicit SQL write locks; INSERT INTO role_permissions (role_id, permission) | 7 (effects; owning entry above) | n.a. | n.a. | yes |
| `portal-rs/migrations/20261105231403_adm-p2a_game_roles.sql:102` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/migrations/20261105231403_adm-p2a_game_roles.sql:120` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/migrations/20261105231403_adm-p2a_game_roles.sql:123` | implicit SQL write locks; INSERT INTO role_permissions (role_id, permission) | 7 (effects; owning entry above) | n.a. | n.a. | yes |
| `portal-rs/migrations/20261105231403_adm-p2a_game_roles.sql:124` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/migrations/20261105231403_adm-p2a_game_roles.sql:139` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/migrations/20261105231403_adm-p2a_game_roles.sql:142` | implicit SQL write locks; INSERT INTO role_permissions (role_id, permission) | 7 (effects; owning entry above) | n.a. | n.a. | yes |
| `portal-rs/migrations/20261105231403_adm-p2a_game_roles.sql:143` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/migrations/20261105231403_adm-p2a_game_roles.sql:161` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/migrations/20261105231403_adm-p2a_game_roles.sql:162` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/migrations/20261105231403_adm-p2a_game_roles.sql:165` | implicit SQL write locks; INSERT INTO role_permissions (role_id, permission) | 7 (effects; owning entry above) | n.a. | n.a. | yes |
| `portal-rs/migrations/20261105231403_adm-p2a_game_roles.sql:166` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/migrations/20261105231403_adm-p2a_game_roles.sql:171` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/migrations/20261105231403_adm-p2a_game_roles.sql:174` | implicit SQL write locks; INSERT INTO role_permissions (role_id, permission) | 7 (effects; owning entry above) | n.a. | n.a. | yes |
| `portal-rs/migrations/20261105231403_adm-p2a_game_roles.sql:175` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/migrations/20261105231403_adm-p2a_game_roles.sql:180` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/migrations/20261105231403_adm-p2a_game_roles.sql:183` | implicit SQL write locks; INSERT INTO role_permissions (role_id, permission) | 7 (effects; owning entry above) | n.a. | n.a. | yes |
| `portal-rs/migrations/20261105231403_adm-p2a_game_roles.sql:184` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/migrations/20261105231403_adm-p2a_game_roles.sql:192` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/migrations/20261105231403_adm-p2a_game_roles.sql:195` | implicit SQL write locks; INSERT INTO role_permissions (role_id, permission) | 7 (effects; owning entry above) | n.a. | n.a. | yes |
| `portal-rs/migrations/20261105231403_adm-p2a_game_roles.sql:196` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/migrations/20261105231403_adm-p2a_game_roles.sql:198` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/migrations/20261105231404_adm-p2b_controls.sql:6` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/migrations/20261107140001_admin_analytics.down.sql:1` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/migrations/20261107140001_admin_analytics.down.sql:2` | implicit SQL write locks; DELETE FROM role_permissions WHERE permission IN ('admin.analytics.view','admin.analytics.export'); | 7 (effects; owning entry above) | n.a. | n.a. | yes |
| `portal-rs/migrations/20261107140001_admin_analytics.down.sql:3` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/migrations/20261107140001_admin_analytics.down.sql:4` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/migrations/20261107140001_admin_analytics.down.sql:5` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/migrations/20261107140001_admin_analytics.down.sql:61` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/migrations/20261107140001_admin_analytics.up.sql:2` | applied migration DDL: pg_advisory_xact_lock; SELECT pg_advisory_xact_lock(71140002); | domain namespace (README inventory) | n.a. | n.a. | yes |
| `portal-rs/migrations/20261107140001_admin_analytics.up.sql:58` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/migrations/20261107140001_admin_analytics.up.sql:59` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/migrations/20261107140001_admin_analytics.up.sql:60` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/migrations/20261107140001_admin_analytics.up.sql:118` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/migrations/20261107140001_admin_analytics.up.sql:126` | implicit SQL write locks; INSERT INTO role_permissions(role_id,permission) | 7 (effects; owning entry above) | n.a. | n.a. | yes |
| `portal-rs/migrations/20261107140001_admin_analytics.up.sql:127` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/migrations/20261107140001_admin_analytics.up.sql:132` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/migrations/20261107140001_admin_analytics.up.sql:134` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/migrations/20261107140001_admin_analytics.up.sql:138` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/migrations/20261107170001_admin_analytics_economy.down.sql:4` | applied migration DDL: pg_advisory_xact_lock; SELECT pg_advisory_xact_lock(71140001); | domain namespace (README inventory) | n.a. | n.a. | yes |
| `portal-rs/migrations/20261107170001_admin_analytics_economy.up.sql:2` | applied migration DDL: pg_advisory_xact_lock; SELECT pg_advisory_xact_lock(71140001); | domain namespace (README inventory) | n.a. | n.a. | yes |
| `portal-rs/migrations/20261107180001_wallet_topup.down.sql:2` | applied migration DDL: LOCK TABLE wallet_operations IN ACCESS EXCLUSIVE MODE; LOCK TABLE wallet_operations IN ACCESS EXCLUSIVE MODE; | 7 / domain namespace | n.a. | n.a. | yes |
| `portal-rs/migrations/20261109071943_t125_coin_classes.sql:13` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/migrations/20261111140002_n009_sanction_export.sql:38` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/migrations/20261111140002_n009_sanction_export.sql:130` | runtime SQL function support_sanction_export_typed: FOR UPDATE; query tables support_sanction_export_counter; PERFORM 1 FROM support_sanction_export_counter WHERE singleton FOR UPDATE; | 4 / owning domain resource | no | n.a. | yes |
| `portal-rs/migrations/20261111140002_n009_sanction_export.sql:150` | runtime SQL function support_sanction_export_reason: FOR UPDATE; WHERE sanction_id=NEW.sanction_id) FOR UPDATE; | 4 / owning domain resource | no | n.a. | yes |
| `portal-rs/migrations/20261111140002_n009_sanction_export.sql:168` | runtime SQL function support_sanction_export_appeal: FOR UPDATE; query tables support_sanction_export_counter; PERFORM 1 FROM support_sanction_export_counter WHERE singleton FOR UPDATE; | 4 / owning domain resource | no | n.a. | yes |
| `portal-rs/migrations/20261111140002_n009_sanction_export.sql:209` | runtime SQL function support_sanction_export_expire_namelocks: FOR UPDATE SKIP LOCKED; query tables support_sanction_export_counter; PERFORM 1 FROM support_sanction_export_counter WHERE singleton FOR UPDATE SKIP LOCKED; | 4 / owning domain resource | no | n.a. | yes |
| `portal-rs/migrations/20261111140004_n012_campaign.sql:102` | runtime SQL function referral_campaign_prepare: FOR UPDATE NOWAIT; query tables accounts; PERFORM id FROM accounts WHERE id=account FOR UPDATE NOWAIT; | 3 | no | n.a. | yes |
| `portal-rs/migrations/20261111140004_n012_campaign.sql:104` | runtime SQL function referral_campaign_prepare: FOR UPDATE; query tables accounts; PERFORM id FROM accounts WHERE id=account FOR UPDATE; | 3 | no | n.a. | yes |
| `portal-rs/migrations/20261111140004_n012_campaign.sql:109` | runtime SQL function referral_campaign_prepare: FOR UPDATE NOWAIT; ORDER BY account_id FOR UPDATE NOWAIT; | 4 / owning domain resource | no | n.a. | yes |
| `portal-rs/migrations/20261111140004_n012_campaign.sql:112` | runtime SQL function referral_campaign_prepare: FOR UPDATE; ORDER BY account_id FOR UPDATE; | 4 / owning domain resource | no | n.a. | yes |
| `portal-rs/migrations/20261111140004_n012_campaign.sql:123` | runtime SQL function referral_campaign_lock: pg_advisory_xact_lock; PERFORM pg_advisory_xact_lock(5600); | domain namespace (README inventory) | no | n.a. | yes |
| `portal-rs/migrations/20261111140004_n012_campaign.sql:135` | runtime SQL function referral_campaign_purchase: FOR UPDATE OF i; WHERE i.invitee_id=a AND i.created_at&lt;=paid_at FOR UPDATE OF i; | 4 / owning domain resource | no | n.a. | yes |
| `portal-rs/migrations/20261111140004_n012_campaign.sql:161` | runtime SQL function referral_campaign_loyalty: FOR UPDATE OF i; WHERE i.invitee_id=a AND i.kind='new' FOR UPDATE OF i; | 4 / owning domain resource | no | n.a. | yes |
| `portal-rs/migrations/20261111140004_n012_campaign.sql:177` | runtime SQL function referral_campaign_login: FOR UPDATE OF i; WHERE i.invitee_id=NEW.account_id AND i.kind='returning' AND i.created_at&lt;=to_timestamp(NEW.entered_at/1000.0) FOR UPDATE OF i; | 4 / owning domain resource | no | n.a. | yes |
| `portal-rs/migrations/20261111140004_n012_campaign.sql:273` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/migrations/20261111231501_security_revocations.sql:16` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/migrations/20261111231501_security_revocations.sql:47` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/migrations/20261111231501_security_revocations.sql:48` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/migrations/20261111231501_security_revocations.sql:63` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/migrations/20261111231501_security_revocations.sql:98` | runtime SQL function identity_security_revocation_suspension_publish: FOR UPDATE; query tables accounts, generation; SELECT native_security_generation INTO generation FROM accounts WHERE id=account FOR UPDATE; | 3 | no | n.a. | yes |
| `portal-rs/migrations/20261111231501_security_revocations.sql:99` | runtime SQL function identity_security_revocation_suspension_publish: FOR UPDATE; query tables identity_security_revocation_counter; PERFORM 1 FROM identity_security_revocation_counter WHERE singleton FOR UPDATE; | 4 / owning domain resource | no | n.a. | yes |
| `portal-rs/migrations/20261111231501_security_revocations.sql:141` | runtime SQL function identity_security_revocation_ban: FOR UPDATE; query tables accounts, generation; SELECT native_security_generation INTO generation FROM accounts WHERE id=NEW.account_id FOR UPDATE; | 3 | no | n.a. | yes |
| `portal-rs/migrations/20261111231501_security_revocations.sql:154` | runtime SQL function identity_security_revocation_reconcile: FOR UPDATE SKIP LOCKED; query tables identity_security_revocation_counter; PERFORM 1 FROM identity_security_revocation_counter WHERE singleton FOR UPDATE SKIP LOCKED; | 4 / owning domain resource | no | n.a. | yes |
| `portal-rs/migrations/20261111231501_security_revocations.sql:173` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/migrations/20261111231501_security_revocations.sql:174` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/migrations/20261111231501_security_revocations.sql:183` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/migrations/20261111231501_security_revocations.sql:184` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/migrations/20261111231501_security_revocations.sql:185` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/migrations/20261111231501_security_revocations.sql:186` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/migrations/20261111231501_security_revocations.sql:244` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/migrations/20261111231501_security_revocations.sql:253` | implicit SQL write locks; INSERT INTO role_permissions(role_id,permission) | 7 (effects; owning entry above) | n.a. | n.a. | yes |
| `portal-rs/migrations/20261112090001_t19c_grant_issuance.sql:6` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/migrations/20261112100004_store_offers_coins_only.sql:5` | applied migration DDL: LOCK TABLE store_offers IN ACCESS EXCLUSIVE MODE; LOCK TABLE store_offers IN ACCESS EXCLUSIVE MODE; | 4 / owning domain resource | n.a. | n.a. | yes |
| `portal-rs/migrations/20261112100010_coin_native_reversals.sql:36` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/migrations/20261112100013_coin_hold_release.sql:1` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/migrations/20261112100013_coin_hold_release.sql:2` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/migrations/20261112100013_coin_hold_release.sql:3` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/migrations/20261112100013_coin_hold_release.sql:22` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/migrations/20261112100013_coin_hold_release.sql:23` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/migrations/20261112100013_coin_hold_release.sql:24` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/migrations/20261112100013_coin_hold_release.sql:25` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/migrations/20261112100013_coin_hold_release.sql:83` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/migrations/20261112100013_coin_hold_release.sql:93` | implicit SQL write locks; INSERT INTO role_permissions(role_id,permission) | 7 (effects; owning entry above) | n.a. | n.a. | yes |
| `portal-rs/migrations/20261112100014_coin_hold_release_debt.sql:1` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/migrations/20261114100001_native_recovery_hardening.sql:25` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/migrations/20261114100004_coin_debt_settlement.sql:13` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/migrations/20261115100004_t28_ops_permissions.sql:2` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/migrations/20261115100004_t28_ops_permissions.sql:3` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/migrations/20261115100004_t28_ops_permissions.sql:4` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/migrations/20261115100004_t28_ops_permissions.sql:62` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/migrations/20261115100004_t28_ops_permissions.sql:74` | implicit SQL write locks; INSERT INTO role_permissions (role_id, permission) | 7 (effects; owning entry above) | n.a. | n.a. | yes |
| `portal-rs/migrations/20261115110002_fix_02_identity.sql:19` | runtime SQL function reset_guard_session_authentication: FOR UPDATE; query tables accounts, current_generation; SELECT native_security_generation INTO current_generation FROM accounts WHERE id=NEW.account_id FOR UPDATE; | 3 | no | n.a. | yes |
| `portal-rs/migrations/20261115110004_fix_04_scheduling.sql:24` | runtime SQL function bazaar_moderation_statement_gate: pg_advisory_xact_lock_shared; PERFORM pg_advisory_xact_lock_shared(64002); | 1 | no | n.a. | yes |
| `portal-rs/migrations/20261115110004_fix_04_scheduling.sql:35` | runtime SQL function bazaar_moderation_fence: pg_advisory_xact_lock_shared; PERFORM pg_advisory_xact_lock_shared(64002); | 1 | no | n.a. | yes |
| `portal-rs/migrations/20261115110004_fix_04_scheduling.sql:41` | runtime SQL function bazaar_moderation_fence: FOR UPDATE; query tables bazaar_listings, item_active; SELECT kind='item' AND state='active' INTO item_active FROM bazaar_listings WHERE id=listing FOR UPDATE; | 2 | no | n.a. | yes |
| `portal-rs/migrations/20261115110004_fix_04_scheduling.sql:76` | runtime SQL function bazaar_character_publication_barrier: pg_advisory_xact_lock; PERFORM pg_advisory_xact_lock(22005160000); | domain namespace (README inventory) | no | n.a. | yes |
| `portal-rs/migrations/20261115110023_fix_w2c_support_evidence.sql:1` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/migrations/20261115120002_fix_a2-01b-admin_hold_identity.sql:8` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/migrations/20261115120009_fix_a2-08-store-wallet_codes.sql:2` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/migrations/20261115120009_fix_a2-08-store-wallet_codes.sql:3` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/migrations/20261115120009_fix_a2-08-store-wallet_codes.sql:4` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/migrations/20261115120009_fix_a2-08-store-wallet_codes.sql:62` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/migrations/20261115120009_fix_a2-08-store-wallet_codes.sql:75` | implicit SQL write locks; INSERT INTO role_permissions (role_id, permission) | 7 (effects; owning entry above) | n.a. | n.a. | yes |
| `portal-rs/migrations/20261115120014_fix_a2-13-world-atlas-wiki_authority.sql:71` | runtime SQL function world_catalog_recover_authority: FOR UPDATE; query tables previous_epoch, world_catalog_publication; INTO previous_epoch,previous_snapshot FROM world_catalog_publication WHERE singleton FOR UPDATE; | 4 / owning domain resource | no | n.a. | yes |
| `portal-rs/migrations/20261115120016_fix_a2-15-migrations_reminders_and_expiry.sql:11` | runtime SQL function pd_reminder_legacy_canonical: pg_advisory_xact_lock; PERFORM pg_advisory_xact_lock(hashtextextended(jsonb_build_array( | domain namespace (README inventory) | no | n.a. | yes |
| `portal-rs/migrations/20261115120016_fix_a2-15-migrations_reminders_and_expiry.sql:48` | applied migration DDL: LOCK TABLE support_sanction_export_revisions IN SHARE ROW EXCLUSIVE MODE; LOCK TABLE support_sanction_export_revisions IN SHARE ROW EXCLUSIVE MODE; | 4 / owning domain resource | n.a. | n.a. | yes |
| `portal-rs/migrations/20261115120016_fix_a2-15-migrations_reminders_and_expiry.sql:99` | runtime SQL function support_sanction_export_expire_namelocks: FOR UPDATE SKIP LOCKED; query tables support_sanction_export_counter; PERFORM 1 FROM support_sanction_export_counter WHERE singleton FOR UPDATE SKIP LOCKED; | 4 / owning domain resource | no | n.a. | yes |
| `portal-rs/migrations/20261115120017_fix_a2-l6_value_eligibility.sql:9` | runtime SQL function account_value_restriction: FOR SHARE NOWAIT; query tables accounts; PERFORM id FROM accounts WHERE id=target FOR SHARE NOWAIT; | 3 | no | n.a. | yes |
| `portal-rs/migrations/20261115120017_fix_a2-l6_value_eligibility.sql:11` | runtime SQL function account_value_restriction: FOR SHARE; query tables accounts; PERFORM id FROM accounts WHERE id=target FOR SHARE; | 3 | no | n.a. | yes |
| `portal-rs/migrations/20261115120017_fix_a2-l6_value_eligibility.sql:42` | reference / declaration / authorization read | n.a. | n.a. | n.a. | yes |
| `portal-rs/migrations/20261115120017_fix_a2-l6_value_eligibility.sql:66` | runtime SQL function referral_campaign_purchase: FOR UPDATE OF i; WHERE i.invitee_id=a AND i.created_at&lt;=paid_at FOR UPDATE OF i; | 4 / owning domain resource | no | n.a. | yes |
| `portal-rs/migrations/20261115120017_fix_a2-l6_value_eligibility.sql:91` | runtime SQL function referral_campaign_loyalty: FOR UPDATE OF i; WHERE i.invitee_id=a AND i.kind='new' FOR UPDATE OF i; | 4 / owning domain resource | no | n.a. | yes |
| `portal-rs/migrations/20261115120017_fix_a2-l6_value_eligibility.sql:108` | runtime SQL function referral_campaign_login: FOR UPDATE OF i; WHERE i.invitee_id=NEW.account_id AND i.kind='returning' AND i.created_at&lt;=to_timestamp(NEW.entered_at/1000.0) FOR UPDATE OF i; | 4 / owning domain resource | no | n.a. | yes |
| `portal-rs/static/design/ICON-LICENSE:5` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/static/design/ICON-LICENSE:7` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/static/fonts/OFL-cinzel.txt:47` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/static/fonts/OFL-cinzel.txt:48` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/static/fonts/OFL-cinzel.txt:64` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/static/fonts/OFL-cinzel.txt:72` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/static/fonts/OFL-inter.txt:47` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/static/fonts/OFL-inter.txt:48` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/static/fonts/OFL-inter.txt:64` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/static/fonts/OFL-inter.txt:72` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/static/fonts/OFL-ptsans.txt:47` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/static/fonts/OFL-ptsans.txt:48` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/static/fonts/OFL-ptsans.txt:64` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/static/fonts/OFL-ptsans.txt:72` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/static/fonts/OFL-spectral.txt:47` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/static/fonts/OFL-spectral.txt:48` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/static/fonts/OFL-spectral.txt:64` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/static/fonts/OFL-spectral.txt:72` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/templates/admin/en.ftl:1` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/templates/admin/en.ftl:3` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/templates/admin/roles.html:6` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/templates/admin/roles.html:7` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/templates/admin/roles.html:12` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/templates/admin/roles.html:13` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/templates/admin/roles.html:20` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/templates/admin/roles.html:21` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/templates/admin/roles.html:22` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/templates/bazaar/en.ftl:15` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/templates/components/shell-header.html:11` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/templates/oauth/consent.html:7` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/templates/oauth/en.ftl:1` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/templates/pages/home.html:12` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |
| `portal-rs/templates/stats/world/en.ftl:6` | comment/document/template reference; no acquisition | n.a. | n.a. | n.a. | yes |

### Post-wait revalidation correction

Authorization after last wait: admin bazaar moderation; players notes, logout, kick, reset evidence, reset requests and MFA approval; console query admission; RBAC role changes; and panel role revocation now recheck the existing actor permissions on the same transaction after the final awaited write and before commit. MFA approval also rechecks request expiry. This supersedes the matching inventory cells above.
