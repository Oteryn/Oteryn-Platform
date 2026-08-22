---
task_id: OTERYN-20260823-terminal-branch-read-permission-fix
status: completed
phase: closeout
project_lane: oteryn-platform-core
pull_request: 1234
merge_sha: e145f7c03bd0b15f0b0fecc0f6fae7884fe3e0db
issue: 1233
---

# Terminal branch read-permission repair — terminal closeout

## Result

PR #1234 repaired the reusable workflow permission chain by adding `.github/workflows/terminal-branch-lifecycle-read-reusable.yml` with read-only permissions and limiting `.github/workflows/terminal-branch-lifecycle-reusable.yml` to `close` and `apply`. The repair squash-merged as `e145f7c03bd0b15f0b0fecc0f6fae7884fe3e0db` and Issue #1233 is closed completed.

META #52, Game #66 and Atlas #95 were repinned to that exact merged SHA, requalified with real GitHub Actions caller runs, and subsequently merged. The repair source branch is absent.

## Acceptance criteria

- [x] Initial caller startup failure was reproduced on META, Game and Atlas.
- [x] A dedicated read-only reusable workflow contains no `contents: write` surface.
- [x] The write-capable reusable workflow accepts only `close` and `apply`.
- [x] Deterministic reusable/cleanup/guard/approval and workflow-inventory tests passed.
- [x] Platform PR #1234 merged and Issue #1233 closed completed.
- [x] META #52, Game #66 and Atlas #95 were repinned, requalified and merged.
- [x] Repair source branch is absent after merge.
- [x] This archived record releases all repair ownership.

## Ownership release

```yaml
owned_paths: []
modules:
  - agent-governance
  - ci
blockers: []
```

## Terminal evidence

```yaml
pull_request: 1234
exact_head: 2b15fb201e183bc485891e384162cae2cc0aa6a3
merge_sha: e145f7c03bd0b15f0b0fecc0f6fae7884fe3e0db
issue_1233_state: completed
source_branch: fix/org-terminal-read-permissions
source_branch_absent: true
provider_merges:
  meta_52: 64371aa0d1548e92376c8e8952def4a9332bd3e7
  game_66: 8fedbee08b7e3b0621aae52980958362bc0f36c5
  atlas_95: 890802e549029ac4aa985649131a2c2cea9cc025
```

## Validation

```yaml
self_review:
  result: PASS
  acceptance_checked: true
  full_diff_checked: true
  negative_paths_checked: true
  rollback_checked: true
  compatibility_checked: true
  related_prs_checked: true
  findings: []
validation:
  - result: PASS
    evidence: reusable contract 5/5, cleanup 7/7, guarded 8/8, approval 10/10 and workflow inventory 18/18 passed on the repair candidate.
  - result: PASS
    evidence: Platform exact-head CI run 32604729381, Agent Governance 32604729358, Terminal Branch Lifecycle 32604729365 and CodeQL 32604729385 all succeeded.
  - result: PASS
    evidence: all three repository callers subsequently executed the split read-only workflow successfully before merge.
  - result: NOT_APPLICABLE
    evidence: browser/runtime E2E is not applicable to a GitHub Actions permission-chain repair; cross-repository workflow execution is the real integration path and passed.
```

## Source branch closeout

```yaml
source_branch_disposition: auto_delete_after_merge
source_branch_reason: permission-chain repair branch became terminal after PR #1234 squash-merged
source_branch_evidence: refs/heads/fix/org-terminal-read-permissions is absent and PR #1234 merged as e145f7c03bd0b15f0b0fecc0f6fae7884fe3e0db
```

## Next action

No action; repair is integrated and its provider rollout is complete.
