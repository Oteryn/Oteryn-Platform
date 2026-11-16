---
task_id: OTERYN-20261116-pr-a-delta
governing_issue: 536
required_reads:
  - docs/architecture/SECURITY_ARCHITECTURE.md
search_first:
  - existing RBAC mutation helper and bootstrap permission-revocation regression
  - active packets and live open PRs for affected paths
optional_reads: []
---

# PR-A delta review

Owner-authorized base f7773311, branch claude/happy-davinci-ijz3wy-fix-a3-dra.
Programme #536 confirmed open. Single writer; owned paths: admin Bazaar adapter,
community party moderation, catalog CLI and their tests. No overlapping open PR
found. No PR, merge, deployment or external messages requested. Jira mapping
pending coordinator. This task explicitly authorizes fixes across these crates.

Fix account/listing inversion, stale party session/MFA/permission authorization,
and catalog CLI permission revocation. Reuse the canonical RBAC helper and one
three-second transaction budget. Record PostgreSQL 18 red/green regressions and
full touched-crate validation below. Browser E2E: party regression exercises the
real guarded POST route; Bazaar/catalog exercise their transactional services.

## Implementation and controlled regression evidence

Global Bazaar order is the existing moderation gate/listing, then accounts,
then RBAC. Repository `locked()` (including bid, offer, settle, cancel and
complete-delivery), item service `locked()` (including buy-now), and character
publication already acquire listings before account authority. Staff moderation
now follows that order. Ordered account locking, the RBAC fence and live session,
MFA and permission checks remain in the canonical helper. The helper's existing
bounded resource callback was extracted for reuse before Bazaar account locking;
it shares the original transaction deadline and adds no lock/fence mechanism.

Party POST takes CheckContext and MutationAuthorization from the staff guard;
the canonical helper locks actor, post, then RBAC and revalidates on the same
transaction before changing post, reports and audit. Queue reads retain RBAC
checks. Direct service tests now supply their existing trusted staff adapter and
explicit grants to the secured service API; assertions remain intact.

Catalog CLI locks account and RBAC before state/permission reads on the library
transaction, retaining locks across active-pointer/import waits. Locking before
FOR SHARE also prevents concurrent CLI issuers upgrading competing shared locks.

All races use backend-specific pg_blocking_pids barriers and yield, without
sleeps. PostgreSQL server_version_num=180006, Rust 1.97.0, database passed through
PORTAL_TEST_DATABASE_URL without printing it. Durations below cover whole tests,
including fixture migrations and cleanup, rather than just transaction waits.

Combined baseline command (six failures, valid-session control passes):
`cargo test --locked -p oteryn-admin -p oteryn-community -p oteryn-portal delta_ --no-fail-fast -- --nocapture`.
Local evidence: /tmp/delta-red.log. Earlier isolated reproduction logs are
/tmp/delta-red-bazaar.log, /tmp/delta-red-party.log and /tmp/delta-red-catalog.log.

| Regression test (unique suffix) | Red seconds | Green seconds | Red observation |
| --- | ---: | ---: | --- |
| delta_bazaar_listing_before_staff_account_avoids_deadlock | 3.418 | 4.782 | 40P01 deadlock in staff listing lock |
| delta_party_revalidates_session_after_post_wait | 3.620 | 7.087 | guarded POST returned 303 after committed session revocation |
| delta_party_revalidates_mfa_after_post_wait | 6.646 | 7.459 | guarded POST returned 303 after committed MFA grant revocation |
| delta_party_revalidates_permission_after_post_wait | 5.350 | 7.611 | guarded POST returned 303 after Change::SetPermissions committed |
| delta_catalog_activation_serializes_permission_revocation | 2.539 | 7.134 | Change::SetPermissions committed ahead of activation blocked on active pointer |
| delta_catalog_permission_read_retains_rbac_fence | 3.312 | 6.218 | another transaction acquired the RBAC fence after import/activation permission reads |

`delta_party_valid_session_commits_after_post_wait` is the positive control:
2.375 seconds on baseline; a valid password-bound MFA/session permits removal,
report resolution and exactly one audit. Revocation tests require 403, active
post, open report and zero moderation audit rows. Catalog uses a real durable
manager session and the production RBAC Change::SetPermissions writer, observes
its fence wait and verifies a subsequent activation is denied.

No new allows, ignores, sleeps, retry loops around mutations or raised production
timeouts. Existing test assertions are retained. Source branch is retained for
owner/coordinator review; no PR or merge requested.

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-11-16T01:27:09Z
head: f7773311523833657505c60974fe2270befa40c2
branch: claude/happy-davinci-ijz3wy-fix-a3-dra
pr: none
status: waiting
context_routes: []
owned_paths:
  - portal-rs/crates/admin/src/bazaar.rs
  - portal-rs/crates/admin/src/bazaar_tests.rs
  - portal-rs/crates/admin/src/rbac/mod.rs
  - portal-rs/crates/community/src/party.rs
  - portal-rs/crates/community/src/party/party_store.rs
  - portal-rs/crates/community/src/party/party_tests.rs
  - portal-rs/bin/oteryn-portal/src/catalog_cli.rs
  - portal-rs/bin/oteryn-portal/src/catalog_cli_tests.rs
  - docs/agents/tasks/active/OTERYN-20261116-pr-a-delta.md
proven:
  - PostgreSQL 18.6 red regressions for all three P1 findings
  - Focused green regressions and valid-session positive control
  - Workspace formatting and Clippy all-targets passed
derived: []
unknown: []
conflicts: []
first_failure:
  marker: PostgreSQL 40P01 and committed stale authorization
  evidence: /tmp/delta-red.log
rejected_hypotheses: []
changed_paths:
  - portal-rs/crates/admin/src/bazaar.rs
  - portal-rs/crates/admin/src/bazaar_tests.rs
  - portal-rs/crates/admin/src/rbac/mod.rs
  - portal-rs/crates/community/src/party.rs
  - portal-rs/crates/community/src/party/party_store.rs
  - portal-rs/crates/community/src/party/party_tests.rs
  - portal-rs/bin/oteryn-portal/src/catalog_cli.rs
  - portal-rs/bin/oteryn-portal/src/catalog_cli_tests.rs
  - docs/agents/tasks/active/OTERYN-20261116-pr-a-delta.md
validation:
  - command: cargo fmt --all --check
    result: PASS
    evidence: /tmp/delta-fmt-final.log
  - command: cargo clippy --locked --all-targets -- -D warnings
    result: PASS
    evidence: /tmp/delta-clippy-final.log
  - command: cargo test --locked -p oteryn-admin -p oteryn-community -p oteryn-portal --no-fail-fast -- --nocapture
    result: FAIL
    evidence: /tmp/delta-full-verified.log; admin passed; missing-MFA success fixture corrected; unrelated league deadline diagnosed
  - command: cargo test --locked -p oteryn-community -p oteryn-portal --no-fail-fast -- --include-ignored --nocapture
    result: FAIL
    evidence: /tmp/delta-community-portal-final.log; community fully passed; portal 186 passed and only unchanged large-league read deadline failed
  - command: cargo test --locked -p oteryn-portal --no-fail-fast -- --include-ignored --nocapture --test-threads=1
    result: PASS
    evidence: /tmp/delta-portal-isolated-harness.log; all 187 unit and 4 integration tests, including Chromium; one harness worker
blockers:
  - Default parallel portal suite still has an unchanged large-league 503; full one-worker suite passed
next_action: Publish the authorized P1 commit; owner reviews it with the parallel-suite limitation retained; no merge authorized
```

## Local PostgreSQL capacity correction

The first full default-concurrency run encountered PostgreSQL 53200 during
fixture migration/cleanup with max_locks_per_transaction=64. That incomplete
run is retained at /tmp/delta-full-tests-capacity64.log. The local setup-owned
PostgreSQL 18.6 container now has max_locks_per_transaction=1024 (ALTER SYSTEM
and service restart); no repository/test assertions, runner concurrency or
application/test timeouts changed. A restart-overlapping run was discarded;
its remaining test processes were terminated before the fresh complete run.
Final full-suite evidence is /tmp/delta-full-verified.log, launched only after
PostgreSQL readiness/version/capacity readback and no live earlier test runner.

Admin full suite passed: 171 tests, zero failures/ignores/filtered tests,
189.30 seconds. Its existing target/gate budget regression passed all four cases
in 3.087, 3.093, 3.014 and 3.014 seconds, including immediate pool reuse.
The final Bazaar regression passed in 4.782 seconds (fixtures included).

The first complete community run exposed an existing success fixture with no
durable staff MFA grant: audit_w2_c05_production_authority_owner_report_and_staff_flow
received the correct new 403 rather than its required 303. Its assertion is
unchanged. Database::staff_session now seeds a real password-bound grant and
session generation for that existing production-authority test and the new
revocation tests. The full community suite and final fmt/Clippy are rerun after
this fixture correction; admin product/test source is unchanged from its pass.

The first full portal run had one unrelated large-page deadline failure:
l5_league_fourteen_full_seasons_are_complete returned 503 rather than 200;
the unchanged isolated test passed in 14.512 seconds. League::read retains its
three-second deadline. A fresh complete community/portal run uses default runner
concurrency and --include-ignored, including the existing portal Chromium test.
No test assertions, fixture sizes or read budgets were changed for that failure.
Native Node and the cached Chromium run against the real local test server;
CI-pinned playwright-core 1.63.0 and @axe-core/playwright 4.14.0 are installed
only under /tmp/pr-a-browser, outside the repository. Final run evidence is
/tmp/delta-community-portal-final.log.

Corrected full community results: 114 unit tests in 141.48 seconds and 47
integration tests (13 online, 2 privacy authority, 8 rankings, 15 Today, 9 worlds)
passed. No failures or ignored tests. The native guarded production-authority
party flow passed in 7.223 seconds with its real session/MFA grant fixture.
Final party revocation durations: session 7.087 s, MFA 7.459 s, permission 7.611 s;
valid-session control 6.956 s. All three denied without post/report/audit changes.
Final format check passed in 3.477 s; workspace all-targets Clippy passed in
13.294 s after the fixture correction.

The second default-concurrency full portal run passed 186 tests, including the
existing Chromium checkout test, with zero ignored tests. Only the unchanged
large-league test failed again (503). All four portal integration binaries
passed. Default-concurrency community was fully green. The complete portal
suite is additionally run with --test-threads=1 to avoid cross-fixture CPU
contention on the three-second league read. This changes no assertion, fixture
size, timeout or internal mutation concurrency; the P1 races already passed at
default harness concurrency. The default parallel portal-suite limitation is
retained explicitly, not relabelled as a passing gate. Single-harness evidence:
/tmp/delta-portal-isolated-harness.log.

## Final validation and publication scope

All six P1 regressions passed at default harness concurrency, with the green
fixture-inclusive durations in the table above. The valid-session positive
control also passed. Full admin: 171 passed (189.30 s). Full community: 114 unit
and 47 integration tests passed (141.48 s for the unit binary). Both used default
harness concurrency. Full portal with one harness worker: all 187 unit tests,
including the existing Chromium checkout test, and all 4 integration binaries
passed; no ignored tests remain. The unchanged large-league test passed in
14.806 s in that complete run. The two default parallel portal failures remain
recorded above and are not a passing CI/readiness claim.

No league source, test size, assertion, timeout or test attribute changed. No
Rust harness concurrency is committed to repository configuration; the explicit
one-worker command is additional validation. Actual mutation interleaving tests
still run their concurrent transactions/tasks, and passed under default harness
concurrency as recorded separately. Formatting, workspace all-targets Clippy,
checkpoint validation, and complete diff review passed.

Publish exactly one commit with the owner-requested subject/author on the
allocated task branch. If its remote ref is absent, allocate that exact branch
at the authorized f7773311523833657505c60974fe2270befa40c2 predecessor before
using the bound META publication helper to fast-forward the exact candidate.
Preserve a verified recovery bundle outside the checkout and verify remote head.
No PR, merge, main push, deployment or external message is authorized/requested.

Portal one-worker complete output summaries:

```text
test result: ok. 187 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 450.61s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.73s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.83s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.55s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.26s
duration=461.689 seconds
```
