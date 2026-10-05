---
task_id: OTERYN-20261005-php-coverage-enforcement
governing_issue: 1010
required_reads:
  - docs/agents/CI_COVERAGE_POLICY.json
search_first:
  - php coverage policy
optional_reads: []
---

# OTERYN-20261005-php-coverage-enforcement

## Goal

Governing GitHub Issue: #1010 — https://github.com/Oteryn/Oteryn-Platform/issues/1010.

Promote the already-delivered PHP coverage observability lane from report-only evidence to a bounded fail-closed regression guard using directly verified main-branch coverage evidence. Keep ordinary pull requests free of the expensive coverage job; require coverage on relevant Merge Queue candidates and relevant protected-main pushes.

## Acceptance criteria

- [ ] Record the verified stable baseline from qualifying protected-main coverage runs.
- [ ] Switch the policy to enforce mode with a reviewed floor below, but close to, the stable baseline.
- [ ] Run the coverage job on relevant Merge Queue candidates before integration and on relevant main pushes for durable evidence.
- [ ] Do not run the expensive coverage job on ordinary pull_request events.
- [ ] Preserve existing integration/E2E/security/concurrency gates as authoritative behavioral evidence.
- [ ] Exact-head policy tests, workflow routing tests and required CI pass.

## Ownership

```yaml
owned_paths:
  - docs/agents/CI_COVERAGE_POLICY.json
  - .github/workflows/ci.yml
  - tools/validation/test_php_coverage_policy.py
  - tests/ci/test_workflow_trigger_economy.py
  - docs/agents/tasks/active/OTERYN-20261005-php-coverage-enforcement.md
modules:
  - CI
  - test coverage observability
dependencies:
  - GitHub Actions coverage evidence on protected main
blockers:
  - none
cross_repository_tasks:
  - none
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-10-05T10:30:00Z
head: 7c906877f0cc10d6c37ad8b31a4c74e8ed30eab8
branch: test/issue-1010-php-coverage-enforcement
pr: 1445
status: validating
context_routes:
  - ci
owned_paths:
  - docs/agents/CI_COVERAGE_POLICY.json
  - .github/workflows/ci.yml
  - tools/validation/test_php_coverage_policy.py
  - tests/ci/test_workflow_trigger_economy.py
  - docs/agents/tasks/active/OTERYN-20261005-php-coverage-enforcement.md
proven:
  - Coverage lane exists on current main with PCOV, Clover output and durable artifacts.
  - Five consecutive qualifying protected-main CI coverage runs from 2026-10-01 reported identical 82.69 percent statement coverage and 55.86 percent method coverage over 20669 statements.
  - Current policy remains report_only with baseline_state UNKNOWN.
derived:
  - A reviewed 82.0 percent statement floor leaves 0.69 percentage points of tolerance while failing a material regression from the verified stable baseline.
unknown:
  - Exact branch-head CI result after implementation.
conflicts: []
first_failure:
  marker: none
  evidence: none
rejected_hypotheses:
  - Coverage observability is missing entirely; disproven by current ci.yml and five successful main-branch coverage runs.
changed_paths:
  - docs/agents/CI_COVERAGE_POLICY.json
  - tools/validation/php_coverage_policy.py
  - tools/validation/test_php_coverage_policy.py
  - .github/workflows/ci.yml
  - tests/ci/test_workflow_trigger_economy.py
  - docs/agents/tasks/active/OTERYN-20261005-php-coverage-enforcement.md
validation:
  - command: PR #1445 required GitHub checks
    result: NOT_RUN
    evidence: waiting for exact-head CI after task/PR binding
blockers:
  - none
next_action: wait for PR #1445 exact-head checks, inspect any first failure, and repair before readiness
```

## Source branch closeout

```yaml
source_branch_disposition: auto_delete_after_merge
source_branch_reason: ordinary same-repository CI policy task
source_branch_evidence: pending
```
