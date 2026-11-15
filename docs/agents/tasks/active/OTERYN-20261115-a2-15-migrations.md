---
task_id: OTERYN-20261115-a2-15-migrations
governing_issue: 536
required_reads: []
search_first: []
optional_reads: []
---

# Audit wave 2 migration repairs

Governing issue: https://github.com/Oteryn/Oteryn-Platform/issues/536 (OPEN, verified 2026-11-15).
Owner authorizes the exact base, branch, two findings, local PostgreSQL validation,
commit and push; no PR, merge or deployment. Single agent, single writer.

Base: c6fdc5438a4afa91f32d547b7cfc61f22bf21139.
Branch: claude/happy-davinci-ijz3wy-fix-a2-15-migrations.
Ownership: portal-rs reminder and sanction-export regression tests and additive
20261115120016 migration. Existing migrations and contract bytes remain immutable.
The dedicated requested branch isolates this work. Jira mapping/synchronization remains with
the coordinator. Local prepared PostgreSQL only; no external game access.

W2-M01 confirmed: legacy AFTER trigger only handles ID conflicts, whereas the
new API writes exclusively to scoped storage. Canonicalize legacy writes against
the scoped unique intent before returning IDs, preserving terminal states.
W2-M02 confirmed: foreground expiry scans the permanent journal under the global
counter. Maintain current expiry candidates in publication transactions and
reconcile at most 500 indexed due candidates, each with one journal PK lookup.

Acceptance: regression failures on base, passes after repair, real PostgreSQL
execution with names/durations, fmt, touched-crate Clippy/tests, requested commit
and non-force push. Runtime external E2E is NOT_APPLICABLE: database integration
seams have no production reminder scheduler or outbound game operation.

Status: ready for coordinator integration. Rust 1.97 validation passed: fmt,
both touched-crate all-targets Clippy checks, playerdata 147/147 and support 86/86
(including real PostgreSQL), with no ignored or skipped tests. Playerdata was
rerun serially after the first parallel run exhausted the local PostgreSQL lock
table during schema cleanup (146 passed, one cleanup failure); the serial run
passed all 147 tests. No test or assertion was disabled or weakened.

Final PG regression durations: postgres_reminder_identity_includes_character_and_cut
1.439256215 s; postgres_expiry_candidates_bound_work_with_permanent_history
2.044779654 s. The latter covers 20,000 revoked journal entries, a 500-candidate
batch from 503 due candidates, authoritative-history lock independence, a
concurrent publisher deadline and a caught-up one-item poll. Restoring the old
reconciliation function makes this same regression time out (3.56 s total).
Before/after and full-check logs remain in /tmp/a215-*, /tmp/m01-before.log and
/tmp/m02-before.log. The entire changed-file list and diff were reviewed.

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-11-15T17:21:45Z
head: c6fdc5438a4afa91f32d547b7cfc61f22bf21139
branch: claude/happy-davinci-ijz3wy-fix-a2-15-migrations
pr: none
status: ready
context_routes: []
owned_paths:
  - portal-rs/crates/playerdata/src/reminders/tests.rs
  - portal-rs/crates/support/src/sanctions/export_tests.rs
  - portal-rs/migrations/20261115120016_fix_a2-15-migrations_reminders_and_expiry.sql
proven:
  - W2-M01 regression fails on base with scoped intent uniqueness violation
  - W2-M02 regression fails with old reconciliation under authoritative history lock
  - Both focused PostgreSQL regressions pass with the additive migration
  - Existing migrations, feature gates and contract bytes are unchanged
derived: []
unknown:
  - Coordinator Jira mapping and synchronization
conflicts: []
first_failure:
  marker: none
  evidence: expected before-fix regression failures retained locally
rejected_hypotheses: []
changed_paths:
  - portal-rs/crates/playerdata/src/reminders/tests.rs
  - portal-rs/crates/support/src/sanctions/export_tests.rs
  - portal-rs/migrations/20261115120016_fix_a2-15-migrations_reminders_and_expiry.sql
  - docs/agents/tasks/active/OTERYN-20261115-a2-15-migrations.md
validation:
  - command: local PostgreSQL regressions
    result: PASS
    evidence: final full-suite PG durations 1.439256215 s and 2.044779654 s
  - command: cargo fmt --all --check
    result: PASS
    evidence: /tmp/a215-fmt-final.log
  - command: cargo clippy --locked --all-targets for oteryn-playerdata and oteryn-support with -D warnings
    result: PASS
    evidence: /tmp/a215-clippy-playerdata-final.log and /tmp/a215-clippy-support-final.log
  - command: cargo test --locked -p oteryn-playerdata -- --nocapture --test-threads=1
    result: PASS
    evidence: 147 passed, 0 failed, 0 ignored; /tmp/a215-test-playerdata-serial.log
  - command: cargo test --locked -p oteryn-support -- --nocapture --test-threads=2
    result: PASS
    evidence: 86 passed, 0 failed, 0 ignored; /tmp/a215-test-support.log
  - command: external runtime E2E
    result: NOT_APPLICABLE
    evidence: local PostgreSQL API-to-database integration seams; no production reminder scheduler or outbound game operation
blockers:
  - none
next_action: coordinator review and integration of the requested published fix branch
```

## Source branch closeout

```yaml
source_branch_disposition: retain
source_branch_reason: owner requests this branch for coordinator audit integration, without a PR
source_branch_evidence: explicit task instruction; coordinator review/integration is the release trigger
```
