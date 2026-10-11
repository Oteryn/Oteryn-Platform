---
task_id: OTERYN-20261011-announcements-lifecycle-fix
governing_issue: 1518
required_reads: []
search_first:
  - scripts/acceptance/runtime-diagnostics.mjs
optional_reads: []
---

# OTERYN-20261011 announcements lifecycle fix

## Goal

Governing GitHub Issue: #1518 (https://github.com/Oteryn/Oteryn-Platform/issues/1518) — canonical lifecycle authority for this task. The top-level `governing_issue` frontmatter field is machine-required while this packet remains active.

Make the `Complete Announcements lifecycle` check green. The admin Announcements acceptance spec deliberately provokes HTTP 403 and 409 responses, which the strict runtime diagnostics (#1242) report as unexpected console errors. Declare exactly those responses per URL and status in the spec. Diagnostics stay strict and no test is skipped.

## Acceptance criteria

- [x] The 403 on `/admin/announcements` (count 3) and the 409 on `/admin/announcements/{id}` are declared with `allowExpectedHttpFailure` and a comment.
- [x] Diagnostics helpers are unchanged; no test is skipped or retried.
- [x] Exact-head `Complete Announcements lifecycle` check passes. Evidence: PR 1519 integrated as 7a7ff356 with the check green.

## Ownership

```yaml
owned_paths:
  - scripts/acceptance/tests/announcements-admin-acceptance.spec.mjs
  - docs/agents/tasks/archive/OTERYN-20261011-announcements-lifecycle-fix.md
modules:
  - Announcements
dependencies:
  - Issue #1518
blockers:
  - none
cross_repository_tasks:
  - none
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-10-11T08:00:00Z
head: 7a7ff356 (PR 1519 integrated)
branch: claude/announcements-lifecycle-fix-20261011
pr: 1519
status: completed
context_routes:
  - testing
owned_paths:
  - scripts/acceptance/tests/announcements-admin-acceptance.spec.mjs
  - docs/agents/tasks/archive/OTERYN-20261011-announcements-lifecycle-fix.md
proven:
  - Main run 38110180091 (head 8006fc8f) failed only on 6 admin-spec tests with console errors for 403 (3x) and 409 (1x); 5 other tests passed.
  - The spec has no allowExpectedHttpFailure declarations while runtime-diagnostics.mjs rejects undeclared console errors since #1242.
derived:
  - Not a product regression; the path-filtered workflow rarely runs, so the spec was never updated for the stricter diagnostics.
unknown:
  - Whether the post-login redirect accounts for the third 403 (count 3 follows the observed console errors).
conflicts: []
first_failure:
  marker: Unexpected browser/runtime failures
  evidence: consoleErrors 403 /admin/announcements and 409 /admin/announcements/{id} in run 38110180091
rejected_hypotheses:
  - Product or seed regression from a merged commit; the failing lines are asserted expected statuses.
changed_paths:
  - scripts/acceptance/tests/announcements-admin-acceptance.spec.mjs
  - docs/agents/tasks/archive/OTERYN-20261011-announcements-lifecycle-fix.md
validation:
  - command: node --check scripts/acceptance/tests/announcements-admin-acceptance.spec.mjs
    result: PASS
    evidence: syntax check clean
  - command: python tools/agents/checkpoint.py docs/agents/tasks/archive/OTERYN-20261011-announcements-lifecycle-fix.md --require-checkpoint
    result: PASS
    evidence: Validated 1 task checkpoint against contract v1
blockers:
  - none
next_action: none; PR 1519 integrated as 7a7ff356 and this record archived by its post-merge closeout
terminal_pr_policy: archive_pending
```

## Source branch closeout

```yaml
source_branch_disposition: auto_delete_after_merge
source_branch_reason: ordinary same-repository PR path
source_branch_evidence: PR 1519 integrated as 7a7ff35601b396565c223f69e877f2261dd253d4
```
