---
task_id: PLAT3-ACLOSE-LIFECYCLE-READ
governing_issue: 1755
required_reads: []
search_first: []
optional_reads: []
---
# PLAT3-ACLOSE-LIFECYCLE-READ

Governing Issue: https://github.com/Oteryn/Oteryn-Platform/issues/1755
Owner: current Codex worker. Branch: feat/plat3-aclose-lifecycle-read.
Scope: lifecycle HTTP/wire/runtime, additive observation sequence, pinned fixture,
regression tests and E2E producer. Game inspected read-only at 9668e0a04e1a631ebfc8752772102a1fd98ce8fa.
No PR or deployment authorized. Owner integrates the committed candidate.

Validation: all required Rust checks PASS; E2E 8/8 PASS and final binary recheck 2/2 PASS.

## Acceptance

- Exact request/response vectors and hashes; pinned source bytes.
- Current locked lifecycle, independent observation counter, reconciled recovery.
- Character mTLS/no kid, no-store, bounded request/response.
- Refusal regressions, full Rust checks and account-closure E2E.

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-11-07T12:30:00Z
head: 960874eb0d9026a0166f40d70571d915e3896b77
branch: claude/portal-rs-acl-lifecycle
pr: 1762
status: completed
context_routes:
  - api
owned_paths:
  - portal-rs/crates/identity/src/termination/game_closure/**
  - portal-rs/crates/identity/src/termination/fixtures/**
  - portal-rs/bin/oteryn-portal/src/account_closure.rs
  - portal-rs/migrations/20261107090002_plat3_closure_lifecycle_observation.sql
  - portal-rs/e2e/game-closure-producer.mjs
  - portal-rs/e2e/ENVIRONMENT.md
proven:
  - lifecycle fixture matches pinned Game bytes
  - formatting and JavaScript syntax pass
derived: []
unknown: []
conflicts: []
first_failure:
  marker: none
  evidence: none
rejected_hypotheses: []
changed_paths:
  - portal-rs
validation:
  - command: cargo fmt --all --check
    result: PASS
    evidence: final candidate; /tmp/aclose-final-checks.log
  - command: cargo clippy -q --workspace --all-targets -- -D warnings
    result: PASS
    evidence: final candidate exit 0
  - command: cargo test -q --workspace
    result: PASS
    evidence: 1096 passed, 0 failed, 3 ignored; /tmp/aclose-final-checks.log
  - command: account-closure Playwright E2E
    result: PASS
    evidence: 8 passed across all projects; final binary recheck 2 passed; /tmp/aclose-final-e2e.log
blockers:
  - none
next_action: none; PR 1762 merged and Issue 1755 closed.
```

## Source branch closeout

```yaml
source_branch_disposition: auto_delete_after_merge
source_branch_reason: ordinary same-repository PR; head branch deleted after merge
source_branch_evidence: PR 1762 merged as 960874eb0d9026a0166f40d70571d915e3896b77
```

## Validation environment

Use only PORTAL_TEST_DATABASE_URL from the task-local E2E database and
RUST_TEST_THREADS=1 for workspace tests. Do not inherit E2E feature flags:
the suite asserts default-disabled behavior. The prepared shared database has
pre-existing wallet tables without matching migration history and is unsuitable.
Initial attempts exposed these environment issues; final acceptance uses the
isolated migrated database, with serve/worker stopped. Logs remain in /tmp.

Final source review: all 11 changed files inspected; fixture cmp PASS; git diff --check PASS; no .github/workflows changes. PR 1762 opened; no deployment performed.
