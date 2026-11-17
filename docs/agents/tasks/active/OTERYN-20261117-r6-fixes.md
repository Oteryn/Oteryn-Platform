---
task_id: OTERYN-20261117-r6-fixes
governing_issue: 530
required_reads: []
search_first: []
optional_reads: []
---

# OTERYN-20261117-r6-fixes

## Goal

Governing GitHub Issue: https://github.com/Oteryn/Oteryn-Platform/issues/530.

Review round 6 (59 slice and 5 lens Codex reviews at 1bb15ad3) claims that a
read-only check confirmed, each fixed by Codex Cloud with a red-then-green test
and integrated here:

- f2 admin players: a denied character read no longer turns the staff account page into 403.
- f3 bazaar: a skill level of 0 in a signed sale snapshot is valid.
- f6 identity closure: a new termination waits while an earlier CANCEL restoration is undelivered, so the lifecycle generation does not strand it.
- f7 payments: settlement takes the wallet row lock before reading purchase holds.
- f9 playerdata G-9: transfer tombstones honour their closed_revision cutoff.
- f10 playerdata retention: live G-11 progress streams keep replay digests; erased progress streams follow the 120-day rule.

Claims checked and not real: admin projection Production, profile Production,
referral cooldown, player-api freshness clock, store review race. Packets of
merged #1938, #1939 and #1940 are archived here.

## Acceptance criteria

- [x] Each fix has a test that fails without it (worker report) and passes with it (rerun here)
- [x] Focused crate tests pass on the integrated tree
- [ ] Portal RS CI green on the PR head

## Ownership

```yaml
owned_paths:
  - docs/agents/tasks/active/OTERYN-20261117-r6-fixes.md
  - docs/agents/tasks/archive/OTERYN-20261117-channel-probe.md
  - docs/agents/tasks/archive/OTERYN-20261117-create-vocation.md
  - docs/agents/tasks/archive/OTERYN-20261117-newhaven-vocation.md
  - portal-rs/bin/oteryn-portal/src/coin_purchases_tests.rs
  - portal-rs/crates/admin/src/players/http.rs
  - portal-rs/crates/admin/src/players/tests.rs
  - portal-rs/crates/bazaar/src/game/authority_tests.rs
  - portal-rs/crates/bazaar/src/game/runtime/characters.rs
  - portal-rs/crates/identity/src/termination/game_closure.rs
  - portal-rs/crates/identity/src/termination/game_closure/tests.rs
  - portal-rs/crates/payments/src/coin_store.rs
  - portal-rs/crates/playerdata/src/authority/tests.rs
  - portal-rs/crates/playerdata/src/retention.rs
  - portal-rs/crates/playerdata/src/sessions.rs
  - portal-rs/crates/playerdata/src/store/tests.rs
modules:
  - admin
  - bazaar
  - identity
  - payments
  - playerdata
dependencies:
  - none
blockers:
  - none
cross_repository_tasks:
  - none
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-11-17T17:30:00Z
head: 756c18f14e2a70e5c4545c7a4db71abe471a4f11
branch: claude/happy-davinci-ijz3wy-r6-fixes
pr: 1941
status: validating
terminal_pr_policy: archive_pending
context_routes: []
owned_paths:
  - docs/agents/tasks/active/OTERYN-20261117-r6-fixes.md
  - docs/agents/tasks/archive/OTERYN-20261117-channel-probe.md
  - docs/agents/tasks/archive/OTERYN-20261117-create-vocation.md
  - docs/agents/tasks/archive/OTERYN-20261117-newhaven-vocation.md
  - portal-rs/bin/oteryn-portal/src/coin_purchases_tests.rs
  - portal-rs/crates/admin/src/players/http.rs
  - portal-rs/crates/admin/src/players/tests.rs
  - portal-rs/crates/bazaar/src/game/authority_tests.rs
  - portal-rs/crates/bazaar/src/game/runtime/characters.rs
  - portal-rs/crates/identity/src/termination/game_closure.rs
  - portal-rs/crates/identity/src/termination/game_closure/tests.rs
  - portal-rs/crates/payments/src/coin_store.rs
  - portal-rs/crates/playerdata/src/authority/tests.rs
  - portal-rs/crates/playerdata/src/retention.rs
  - portal-rs/crates/playerdata/src/sessions.rs
  - portal-rs/crates/playerdata/src/store/tests.rs
proven:
  - Six read-only verifications at 1bb15ad3 confirmed the defects with file:line evidence
derived:
  - none
unknown:
  - Coordinator Jira synchronization
conflicts: []
first_failure:
  marker: none
  evidence: none
rejected_hypotheses:
  - Five other round-6 claims; see Goal
changed_paths:
  - docs/agents/tasks/active/OTERYN-20261117-r6-fixes.md
  - docs/agents/tasks/archive/OTERYN-20261117-channel-probe.md
  - docs/agents/tasks/archive/OTERYN-20261117-create-vocation.md
  - docs/agents/tasks/archive/OTERYN-20261117-newhaven-vocation.md
  - portal-rs/bin/oteryn-portal/src/coin_purchases_tests.rs
  - portal-rs/crates/admin/src/players/http.rs
  - portal-rs/crates/admin/src/players/tests.rs
  - portal-rs/crates/bazaar/src/game/authority_tests.rs
  - portal-rs/crates/bazaar/src/game/runtime/characters.rs
  - portal-rs/crates/identity/src/termination/game_closure.rs
  - portal-rs/crates/identity/src/termination/game_closure/tests.rs
  - portal-rs/crates/payments/src/coin_store.rs
  - portal-rs/crates/playerdata/src/authority/tests.rs
  - portal-rs/crates/playerdata/src/retention.rs
  - portal-rs/crates/playerdata/src/sessions.rs
  - portal-rs/crates/playerdata/src/store/tests.rs
validation:
  - command: cargo fmt --all --check; cargo clippy --locked -p oteryn-admin -p oteryn-bazaar -p oteryn-identity -p oteryn-payments -p oteryn-playerdata -p oteryn-portal --all-targets -- -D warnings
    result: PASS
    evidence: both exit 0
  - command: cargo test --locked -p oteryn-admin -p oteryn-bazaar -p oteryn-identity -p oteryn-payments -p oteryn-playerdata -p oteryn-portal (each)
    result: PASS
    evidence: admin 209, bazaar 143, identity 181, payments 49 (1 existing ignore), playerdata 166, portal 223 (1 existing ignore); PostgreSQL 18 set
  - command: follow-up (closure guard, retention batch) cargo test --locked -p oteryn-identity termination; cargo test --locked -p oteryn-playerdata
    result: PASS
    evidence: identity termination 31, playerdata 166; the terminal-REJECTED case of postgres_cancel_before_delivery_and_after_proof_never_issues_finalize fails with the previous guard
  - command: product runtime E2E
    result: NOT_APPLICABLE
    evidence: narrow server-side fixes covered by the PostgreSQL tests through the real entry points
blockers:
  - none
next_action: wait for Portal RS CI on the PR head
```
