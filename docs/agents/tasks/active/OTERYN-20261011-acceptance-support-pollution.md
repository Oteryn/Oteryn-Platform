---
task_id: OTERYN-20261011-acceptance-support-pollution
governing_issue: 1535
required_reads: []
search_first:
  - scripts/acceptance/tests/portal-visual-review.spec.mjs
optional_reads: []
---

# OTERYN-20261011 Acceptance support page pollution

## Goal

Governing GitHub Issue: #1535 (https://github.com/Oteryn/Oteryn-Platform/issues/1535) — canonical lifecycle authority for this task. The top-level `governing_issue` frontmatter field is machine-required while this packet remains active.

The full-profile Acceptance E2E fails on main because `portal-visual-review.spec.mjs` seeds the published support/legal editorial pages (`seed-browser-support-legal.php seed-public`) and never resets them. The later `portal-487-strictness` resilience test expects `/support` to be unconfigured (404) and gets 200. Reset the fixture in `afterAll`, which runs even when a test fails.

## Acceptance criteria

- [x] portal-visual-review resets the support-legal fixture after its tests.
- [ ] Exact-head CI and the full-profile Acceptance E2E pass.

## Ownership

```yaml
owned_paths:
  - scripts/acceptance/tests/portal-visual-review.spec.mjs
  - docs/agents/tasks/active/OTERYN-20261011-acceptance-support-pollution.md
  - docs/agents/tasks/archive/OTERYN-20261011-acceptance-support-pollution.md
modules:
  - acceptance
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
updated_at: 2026-10-11T10:00:00Z
head: UNKNOWN
branch: claude/acceptance-support-pollution-20261011
pr: 1536
status: validating
context_routes:
  - acceptance
owned_paths:
  - scripts/acceptance/tests/portal-visual-review.spec.mjs
  - docs/agents/tasks/active/OTERYN-20261011-acceptance-support-pollution.md
proven:
  - SupportPageController returns 404 only when the support editorial page is unpublished
  - portal-visual-review beforeAll runs seed-public and has no reset; chromium-primary runs before resilience-chromium in the full profile on one database
derived:
  - resetting in afterAll removes the inherited published /support page
unknown:
  - full-profile result until CI runs on this head
conflicts: []
first_failure:
  marker: "Expected unconfigured support editorial state (Expected 404, Received 200)"
  evidence: Acceptance E2E full profile on main 8e132b88 and Dependabot PRs 1510 and 1524
rejected_hypotheses:
  - support-legal-acceptance.spec.mjs pollutes (it resets in afterEach and is excluded from chromium-primary in the full profile)
changed_paths:
  - scripts/acceptance/tests/portal-visual-review.spec.mjs
validation:
  - command: node --check scripts/acceptance/tests/portal-visual-review.spec.mjs
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
