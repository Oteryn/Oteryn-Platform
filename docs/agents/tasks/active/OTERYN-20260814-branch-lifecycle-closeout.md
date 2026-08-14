---
task_id: OTERYN-20260814-branch-lifecycle-closeout
issue: 1050
status: active
project_lane: oteryn-platform-core
phase: repair
execution_mode: github_connector
required_reads:
  - AGENTS.md
  - AGENTS.override.md
  - docs/agents/AGENTS.md
  - docs/agents/DELIVERY_COMPLETENESS_AND_CLOSEOUT.md
  - docs/agents/ANTI_STALL_AND_EXECUTION_BUDGET.md
  - docs/agents/SESSION_RECOVERY_AND_ORPHANED_EXECUTION.md
  - docs/agents/GITHUB_ONLY_EXECUTION.md
  - docs/architecture/adr/0024-merged-source-branch-lifecycle-policy.md
  - docs/architecture/adr/0037-terminal-source-branch-lifecycle.md
---

# OTERYN-20260814-branch-lifecycle-closeout

## Goal

Finish the reviewed historical branch cleanup and leave terminal source-branch cleanup deterministic and fail-closed.

## Acceptance criteria

- [x] PR #1056 merged the accepted branch-lifecycle governance into protected `main`.
- [x] PR #1056 source branch auto-deleted and immutable PR/head provenance remains available.
- [ ] Post-merge deletion verification tolerates REST read-after-delete lag without weakening exact-SHA deletion.
- [ ] Reviewed 8 `TERMINAL_MERGED` and 105 `TERMINAL_CLOSED_UNMERGED` refs are deleted with verified evidence.
- [ ] One-time approval files are removed and final dry-runs have zero authorized pending deletion candidates.
- [ ] Task record is archived and Issue #1050 closes `completed`.

## Ownership

```yaml
owned_paths:
  - docs/agents/tasks/active/OTERYN-20260814-branch-lifecycle-closeout.md
  - docs/agents/tasks/archive/OTERYN-20260814-branch-lifecycle-closeout.md
  - docs/agents/BRANCH_DELETION_APPROVAL.json
  - docs/agents/TERMINAL_BRANCH_DELETION_APPROVAL.json
  - tools/agents/branch_lifecycle.py
  - tools/agents/branch_lifecycle_legacy.py
  - tools/agents/test_terminal_branch_guarded.py
  - tools/agents/test_terminal_branch_guarded_legacy.py
  - .github/workflows/branch-lifecycle.yml
  - .github/workflows/terminal-branch-lifecycle.yml
modules:
  - repository-governance
  - branch-lifecycle
dependencies:
  - Issue #1050
  - merged PR #1056
blockers: []
cross_repository_tasks:
  - none
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-08-14T19:11:00Z
head: 238a535cd7a9d2cf5fe34092069308af2be471d9
branch: repair/issue-1050-cleanup-verification
pr: 1064
status: validating
context_routes:
  - agent-governance
  - testing
owned_paths:
  - docs/agents/tasks/active/OTERYN-20260814-branch-lifecycle-closeout.md
  - tools/agents/branch_lifecycle.py
  - tools/agents/branch_lifecycle_legacy.py
  - tools/agents/test_terminal_branch_guarded.py
  - tools/agents/test_terminal_branch_guarded_legacy.py
  - .github/workflows/branch-lifecycle.yml
  - .github/workflows/terminal-branch-lifecycle.yml
proven:
  - PR #1056 merged as e4498ba9856a3779c8ae3a6f5bed608256a35fef and refs/heads/repair/issue-1050 is absent.
  - Terminal Branch Lifecycle push run 31823855258 rebuilt and validated the exact 105-entry reviewed manifest before apply.
  - The first exact deletion push removed agent/production-go-live-gate at SHA a170072ac5567a153e44a1442568eddf2f3e904e, but the immediate REST ref read remained stale and caused verification failure.
  - GitHub branch search subsequently proved agent/production-go-live-gate absent; the ref was restored at the exact immutable SHA so the reviewed 105-entry approval remains reproducible for a clean retry.
  - Branch Lifecycle push run 31823855101 independently reproduced the same stale-REST verification race after an exact Git deletion; affected reviewed refs were restored to their exact approved SHAs before retry.
  - Exact-head b9da9145836d0e7be42c67529e4d24b7cede26ed passed CI 31824773967, Agent Governance 31824773823, Terminal Branch Lifecycle 31824773809, Branch Lifecycle 31824773849 and all other associated validation workflows.
  - Repository owner explicitly authorized one Codex review for PR #1064; marking the PR ready triggered Codex review on b9da9145836d0e7be42c67529e4d24b7cede26ed, consuming that authorization. No further owner-funded AI invocation is authorized.
  - Codex found one P2: delegated implementation/test files were missing from lifecycle workflow path filters. Both lifecycle workflows now include the delegated implementation path, and Terminal Branch Lifecycle also includes the delegated guarded-test path.
derived:
  - Post-delete absence must be reconciled through Git transport after the exact lease-guarded Git push instead of trusting an immediately consistent REST read.
  - Future changes to delegated branch-lifecycle implementation/test files must trigger the relevant lifecycle workflows.
unknown:
  - Exact-final-head CI/lifecycle result after the Codex P2 path-filter repair and this checkpoint update.
  - Final post-merge deletion evidence for both reviewed candidate sets.
conflicts: []
first_failure:
  marker: post-delete-rest-read-after-write
  evidence: Terminal Branch Lifecycle run 31823855258 failed apply-reviewed-manifest after deleting agent/production-go-live-gate because immediate REST verification still returned the deleted ref.
rejected_hypotheses:
  - Retry the unchanged destructive workflow; it would reproduce the same verification race.
  - Weaken exact-SHA lease deletion or skip verification; both would reduce safety.
  - Leave delegated files outside workflow path filters; destructive lifecycle changes must not bypass their proving workflows.
changed_paths:
  - docs/agents/tasks/active/OTERYN-20260814-branch-lifecycle-closeout.md
  - tools/agents/branch_lifecycle.py
  - tools/agents/branch_lifecycle_legacy.py
  - tools/agents/test_terminal_branch_guarded.py
  - tools/agents/test_terminal_branch_guarded_legacy.py
  - .github/workflows/branch-lifecycle.yml
  - .github/workflows/terminal-branch-lifecycle.yml
validation:
  - command: Terminal Branch Lifecycle run 31823855258
    result: BLOCKED
    evidence: validate and live-dry-run passed; apply failed only at immediate post-delete REST verification after the exact deletion push
  - command: exact-head validation on b9da9145836d0e7be42c67529e4d24b7cede26ed
    result: PASS
    evidence: CI 31824773967, Agent Governance 31824773823, Terminal Branch Lifecycle 31824773809 and Branch Lifecycle 31824773849 succeeded with original reviewed approval sets reproducible
  - command: owner-authorized Codex review on PR #1064
    result: BLOCKED
    evidence: one P2 path-filter finding repaired on the current branch; no second Codex invocation is authorized
blockers: []
next_action: Wait for exact-final-head CI, verify the repaired Codex finding against the final diff, resolve its thread, then squash-merge PR #1064 and verify both reviewed cleanup applies.
```

## Source branch closeout

```yaml
source_branch_disposition: auto_delete_after_merge
source_branch_reason: bounded same-repository repair PR #1064 will merge only after exact-head gates pass
source_branch_evidence: pending merge and post-merge source-ref verification
```

## Notes

Issue #1050 was reopened because cleanup acceptance was not yet terminal after PR #1056. One owner-authorized Codex review for PR #1064 was consumed; no additional Codex or owner-funded AI invocation is authorized.
