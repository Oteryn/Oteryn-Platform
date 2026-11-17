---
task_id: OTERYN-20261116-r4-lockctl
governing_issue: 530
required_reads: []
search_first: []
optional_reads: []
---

# OTERYN-20261116-r4-lockctl

## Goal

Governing GitHub Issue: https://github.com/Oteryn/Oteryn-Platform/issues/530.

Round 4 claim lockctl: control ingress `hello` and `results` took HTTP
ownership before control ownership, the reverse of the documented hierarchy
"Control ownership, then HTTP ownership, then scope/request serialization"
(`crates/admin/src/rbac/README.md`), so a control writer and an ingress call
could wait on each other.

## Acceptance criteria

- [x] `hello` and `results` take control ownership before HTTP ownership
- [x] `postgres_hello_waits_for_control_before_http_ownership` is red without the fix and green with it

Closeout of merged PR #1892: its packet `OTERYN-20261116-portal-rs-ci-shards` is archived as completed (same archive as #1893).

## Ownership

```yaml
owned_paths:
  - portal-rs/crates/admin/src/control/http.rs
  - portal-rs/crates/admin/src/control/http_tests.rs
  - docs/agents/tasks/active/OTERYN-20261116-r4-lockctl.md
  - docs/agents/tasks/archive/OTERYN-20261116-portal-rs-ci-shards.md
  - docs/agents/tasks/active/OTERYN-20261116-portal-rs-ci-shards.md
modules:
  - admin
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
updated_at: 2026-11-16T23:59:00Z
head: 08b3dace01d08b96f4b62472b48a8ea3a7f97e73
branch: claude/happy-davinci-ijz3wy-r4rp-lockctl
pr: 1895
status: completed
context_routes: []
owned_paths:
  - portal-rs/crates/admin/src/control/http.rs
  - portal-rs/crates/admin/src/control/http_tests.rs
  - docs/agents/tasks/active/OTERYN-20261116-r4-lockctl.md
  - docs/agents/tasks/archive/OTERYN-20261116-portal-rs-ci-shards.md
  - docs/agents/tasks/active/OTERYN-20261116-portal-rs-ci-shards.md
proven:
  - The red test was written on 85336fc6 and the branch is merged with main e9b0bcaaacf61d29593e38454fd77977d86335d2
  - No migration, contract, fixture, flag or wire byte change
  - No Game repository, production service or production data accessed
  - Without the fix the test fails with "hello acquired HTTP ownership while waiting for control ownership"
derived: []
unknown:
  - Coordinator Jira synchronization
conflicts: []
first_failure:
  marker: none
  evidence: none
rejected_hypotheses: []
changed_paths:
  - portal-rs/crates/admin/src/control/http.rs
  - portal-rs/crates/admin/src/control/http_tests.rs
  - docs/agents/tasks/active/OTERYN-20261116-r4-lockctl.md
  - docs/agents/tasks/archive/OTERYN-20261116-portal-rs-ci-shards.md
validation:
  - command: cargo test --locked -p oteryn-admin --lib postgres_hello_waits_for_control_before_http_ownership (fix absent)
    result: PASS
    evidence: expected red at http_tests.rs, hello acquired HTTP ownership while waiting for control ownership
  - command: cargo test --locked -p oteryn-admin --no-fail-fast (merged tree)
    result: PASS
    evidence: 180 passed, 0 failed, PostgreSQL 18, no SKIPPED line
  - command: cargo clippy --locked -p oteryn-admin --all-targets -- -D warnings; cargo fmt --all --check
    result: PASS
    evidence: exit 0
blockers:
  - none
next_action: none; delivered by PR 1895 and archived
```

## Source branch closeout

```yaml
source_branch_disposition: auto_delete_after_merge
source_branch_reason: PR 1895 squash-merged through the merge queue; GitHub deletes the head branch after merge
source_branch_evidence: PR 1895 merged as 08b3dace01d08b96f4b62472b48a8ea3a7f97e73
```
