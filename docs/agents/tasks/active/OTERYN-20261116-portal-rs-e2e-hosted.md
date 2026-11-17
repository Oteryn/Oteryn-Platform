---
task_id: OTERYN-20261116-portal-rs-e2e-hosted
governing_issue: 530
required_reads: []
search_first: []
optional_reads: []
---

# OTERYN-20261116-portal-rs-e2e-hosted

## Goal

Governing GitHub Issue: https://github.com/Oteryn/Oteryn-Platform/issues/530.

The full Playwright corpus in `portal-rs/e2e` ran only on the owner's PC. Run it
on hosted runners: one job builds the portal binary once, one job per
Playwright project starts the disposable stack (`local.mjs`) against its own
PostgreSQL 18.6 and runs every spec. Triggers: nightly, manual dispatch
(projects, shards) and pull requests touching `portal-rs/e2e/**` or the
workflow. Owner consent for the new workflow and its lifecycle entry was
relayed by the coordinator.

## Acceptance criteria

- [ ] Every project in `playwright.config.mjs` runs unless a dispatch names a subset; an unknown project fails the plan
- [ ] Each matrix job lists more than zero tests and reports its counts and seconds in the step summary
- [ ] No test, timeout, retry or expected count in `portal-rs/e2e` changes; `local.mjs` only skips its own build when `PORTAL_E2E_BINARY` is set
- [ ] The scratch directory (env.json, keys) is never uploaded; log tails redact connection strings
- [ ] Workflow registered in `CI_WORKFLOW_LIFECYCLE.json` with its review/retirement condition in the workflow header
- [ ] Every action pinned by full SHA

## Ownership

```yaml
owned_paths:
  - .github/workflows/portal-rs-e2e.yml
  - docs/agents/CI_WORKFLOW_LIFECYCLE.json
  - portal-rs/e2e/local.mjs
  - docs/agents/tasks/active/OTERYN-20261116-portal-rs-e2e-hosted.md
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
updated_at: 2026-11-17T00:55:00Z
head: 73f0de7a7775d04443c043656a46a13b98323e84
branch: claude/happy-davinci-ijz3wy-e2e-hosted
pr: 1896
status: ready
context_routes: []
owned_paths:
  - .github/workflows/portal-rs-e2e.yml
  - docs/agents/CI_WORKFLOW_LIFECYCLE.json
  - portal-rs/e2e/local.mjs
  - docs/agents/tasks/active/OTERYN-20261116-portal-rs-e2e-hosted.md
proven:
  - Base is origin/main e9b0bcaa
  - setup-node v7.1.0 and download-artifact v8.0.2 SHAs read from the action repositories' tags
derived: []
unknown:
  - Coordinator Jira synchronization
  - Hosted-runner build and per-project times until the first run
conflicts: []
first_failure:
  marker: none
  evidence: none
rejected_hypotheses: []
changed_paths:
  - .github/workflows/portal-rs-e2e.yml
  - docs/agents/CI_WORKFLOW_LIFECYCLE.json
  - portal-rs/e2e/local.mjs
  - docs/agents/tasks/active/OTERYN-20261116-portal-rs-e2e-hosted.md
validation:
  - command: python3 tools/validation/workflow_inventory.py; github_actions_pinning.py; legacy_token_ratchet.py; prompt_eval.py; test_workflow_inventory.py; test_github_actions_pinning.py
    result: PASS
    evidence: lifecycle PASS actual=28 budget=55; 113 uses references validated; every script exit 0
  - command: python3 tests/ci/test_workflow_trigger_economy.py, test_github_actions_storage_hygiene.py, test_stable_platform_gate_policy.py, test_required_test_gate.py, test_push_change_routing.py, test_classify_changes.py
    result: PASS
    evidence: every script exit 0
blockers:
  - none
next_action: First run 43980291709 built in 2m22s; e2e jobs failed on ubuntu-latest 26.04 (no Chromium for the pinned Playwright); pinned both jobs to ubuntu-24.04, read per-project times from the next run
```

## Source branch closeout

```yaml
source_branch_disposition: pending
source_branch_reason: task is still active; branch not yet pushed
source_branch_evidence: pending
```
