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
updated_at: 2026-11-07T12:20:00Z
head: 541af097dd9e0c0031afb4fee269050ac70096b3
branch: claude/portal-rs-acl-lifecycle
pr: 1762
status: ready
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
next_action: land PR 1762 through auto-merge and archive this task after merge
```

## Source branch closeout

```yaml
source_branch_disposition: retain
source_branch_reason: same-repository PR 1762 is open; disposition becomes terminal at merge
source_branch_evidence: PR 1762 for governing Issue 1755
```

## Validation environment

Use only PORTAL_TEST_DATABASE_URL from the task-local E2E database and
RUST_TEST_THREADS=1 for workspace tests. Do not inherit E2E feature flags:
the suite asserts default-disabled behavior. The prepared shared database has
pre-existing wallet tables without matching migration history and is unsuitable.
Initial attempts exposed these environment issues; final acceptance uses the
isolated migrated database, with serve/worker stopped. Logs remain in /tmp.

Final source review: all 11 changed files inspected; fixture cmp PASS; git diff --check PASS; no .github/workflows changes. PR 1762 opened; no deployment performed.
