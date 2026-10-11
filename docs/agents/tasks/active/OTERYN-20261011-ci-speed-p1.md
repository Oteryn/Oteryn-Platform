---
task_id: OTERYN-20261011-ci-speed-p1
governing_issue: 1515
required_reads: []
search_first:
  - .github/workflows/ci.yml
optional_reads: []
---

# OTERYN-20261011 CI speed P1

## Goal

Governing GitHub Issue: #1515 — canonical lifecycle authority for this task. The top-level `governing_issue` frontmatter field is machine-required while this packet remains active.

Measure why Platform CI got slower and fix what the evidence proves. Measured: the `runtime-tests` job grew from about 113 s (mid-September) to about 198 s (2026-10-11) because the PHPUnit step grew from about 41 s to about 119 s, and bursts of concurrent pull requests add runner queue time (up to 318 s) because every push fans out to many workflows. This slice overlaps the independent formatting and static-analysis checks with the test run inside the existing `runtime-tests` job. No required check is renamed or weakened.

## Acceptance criteria

- [x] `runtime-tests` still fails when formatting, static analysis or tests fail.
- [x] No job or check name changes; workflow count unchanged.
- [ ] Exact-head CI and Agent Governance gates pass.

## Ownership

```yaml
owned_paths:
  - .github/workflows/ci.yml
  - docs/agents/tasks/active/OTERYN-20261011-ci-speed-p1.md
  - docs/agents/tasks/archive/OTERYN-20261011-ci-speed-p1.md
modules:
  - ci
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
updated_at: 2026-10-11T07:30:00Z
head: UNKNOWN
branch: claude/ci-speed-p1-20261011
pr: 1516
status: validating
context_routes:
  - ci-workflows
owned_paths:
  - .github/workflows/ci.yml
  - docs/agents/tasks/active/OTERYN-20261011-ci-speed-p1.md
proven:
  - runtime-tests job median 113 s (2026-09-12) to 198 s (2026-10-11); PHPUnit step 41 s to 119 s
  - composer install is 6 s, so dependency caching has no measurable gain
derived:
  - overlapping pint and phpstan with tests saves about 25-45 s per runtime-tests run
unknown:
  - actual saving until CI runs on this head
conflicts: []
first_failure:
  marker: none
  evidence: none
rejected_hypotheses:
  - composer or npm install without cache is the cause (6 s install)
changed_paths:
  - .github/workflows/ci.yml
validation:
  - command: python tests/ci/test_*.py, github_actions_pinning, workflow_inventory, legacy_token_ratchet
    result: PASS
    evidence: local run
blockers:
  - none
next_action: Archive this record once the PR is closed.
terminal_pr_policy: archive_pending
```

## Source branch closeout

```yaml
source_branch_disposition: auto_delete_after_merge
source_branch_reason: ordinary same-repository PR
source_branch_evidence: pending
```

## Notes

Follow-ups named in the PR body: Playwright browser caching (needs a pinned actions/cache), Announcements acceptance 403/409 console-error failure on main, test-suite growth.
