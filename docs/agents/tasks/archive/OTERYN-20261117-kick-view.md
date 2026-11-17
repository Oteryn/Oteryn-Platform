---
task_id: OTERYN-20261117-kick-view
governing_issue: 1928
required_reads: []
search_first: []
optional_reads: []
---

# OTERYN-20261117-kick-view

## Goal

Governing GitHub Issue: https://github.com/Oteryn/Oteryn-Platform/issues/1928
(review 5 findings). A moderator holding only `KickAccountSessions` reaches
the player detail route and sees only the kick form: no account details,
notes, sessions, characters, ticket or reset actions. The players router
guard admits any staff member; every handler keeps its own permission check.

## Acceptance criteria

- [x] Support staff keep the full view; a kick-only moderator gets only the kick form, a `players.kick.view` audit entry, 403 on search and notes, and can kick (`postgres_http_kick_only_moderator_sees_only_the_kick_view`)
- [x] Portal RS CI green on the PR head

## Ownership

```yaml
owned_paths:
  - docs/agents/tasks/active/OTERYN-20261117-kick-view.md
  - portal-rs/crates/admin/src/players/http.rs
  - portal-rs/crates/admin/src/players/mod.rs
  - portal-rs/crates/admin/src/players/page.html
  - portal-rs/crates/admin/src/players/tests.rs
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
updated_at: 2026-11-17T14:00:00Z
head: 052dfdb53f73aabb586348f99e89f4e8ddb42f66
branch: claude/happy-davinci-ijz3wy-kick-view
pr: 1932
status: completed
terminal_pr_policy: archive_pending
context_routes: []
owned_paths:
  - docs/agents/tasks/active/OTERYN-20261117-kick-view.md
  - portal-rs/crates/admin/src/players/http.rs
  - portal-rs/crates/admin/src/players/mod.rs
  - portal-rs/crates/admin/src/players/page.html
  - portal-rs/crates/admin/src/players/tests.rs
proven:
  - The new test is red without the fix (403 where 200 expected at players/tests.rs:1970) and green with it (PostgreSQL 18)
  - No migration, contract, fixture, flag or workflow change
derived:
  - Every players handler checks its own permission in players/mod.rs, so the Access guard admits nothing new beyond the kick view
unknown:
  - Coordinator Jira synchronization
conflicts: []
first_failure:
  marker: none
  evidence: none
rejected_hypotheses: []
changed_paths:
  - docs/agents/tasks/active/OTERYN-20261117-kick-view.md
  - portal-rs/crates/admin/src/players/http.rs
  - portal-rs/crates/admin/src/players/mod.rs
  - portal-rs/crates/admin/src/players/page.html
  - portal-rs/crates/admin/src/players/tests.rs
validation:
  - command: cargo test -p oteryn-admin --lib (PostgreSQL 18)
    result: PASS
    evidence: 198 passed, 0 failed; the new test fails without the fix
  - command: cargo fmt --all --check; cargo clippy -p oteryn-admin --all-targets -- -D warnings
    result: PASS
    evidence: both exit 0
blockers:
  - none
next_action: none; delivered by PR 1932 and archived
```

## Source branch closeout

```yaml
source_branch_disposition: auto_delete_after_merge
source_branch_reason: PR 1932 squash-merged through the merge queue; GitHub deletes the head branch after merge
source_branch_evidence: PR 1932 merged as 052dfdb53f73aabb586348f99e89f4e8ddb42f66
```
