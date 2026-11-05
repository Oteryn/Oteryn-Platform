---
task_id: OTERYN-20261105-portalrs-local-e2e
governing_issue: 530
required_reads:
  - docs/agents/EXECUTION_RESOURCE_HYGIENE.md
  - docs/agents/BUILD_TEST_MATRIX.md
  - docs/architecture/SECURITY_ARCHITECTURE.md
  - docs/contracts/AUTH_GAME_LOGIN_CONTRACT.md
search_first:
  - portal-rs/crates/identity
optional_reads: []
---

# Local portal-rs end-to-end qualification

Governing Issue: https://github.com/Oteryn/Oteryn-Platform/issues/530 (live-read open).
Owner task in chat authorizes local test data, isolated portalrs containers and local fix commits only; forbids GitHub writes, publication and existing-stack changes.

## Goal and acceptance

Exercise registration, email verification, login/logout, MFA, recovery, abuse/CSRF and account reads in the isolated local environment. Produce `/home/mole/oteryn-cache/portalrs-e2e-report.md` and reproducible scripts under `portalrs-e2e/`.

## Ownership

Owned: this task record, bounded fixes within `portal-rs/`, external test scripts and `oteryn-testenv-portalrs*` resources only. One writer, local branch `portalrs-e2e-fixes`. Existing pkg6 packet is a ready integration record, not an active writer; live open PRs cover architecture draft and retained salvage, with no active implementation overlap found. No external repository inspection. Jira synchronization pending; this worker does not load Jira.

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-11-05T23:30:00Z
head: c75f1cea567f
branch: portalrs-e2e-fixes
pr: none
status: investigating
context_routes:
  - testing
  - auth-identity
  - execution-resources
owned_paths:
  - portal-rs/**
  - docs/agents/tasks/active/OTERYN-20261105-portalrs-local-e2e.md
proven:
  - isolated portal and local PostgreSQL run
  - live Issue 530 remains open
  - user prohibits all GitHub writes
unknown:
  - full browser flow outcomes
conflicts: []
derived: []
first_failure:
  marker: none
  evidence: none
rejected_hypotheses: []
changed_paths: []
validation:
  - command: browser E2E
    result: NOT_RUN
    evidence: setup in progress
blockers: []
next_action: execute browser E2E and fix confirmed bounded defects
```

## Source branch closeout

```yaml
source_branch_disposition: retain
source_branch_reason: owner requests local fix commits for review; no push or PR authorized
source_branch_evidence: task instruction in chat
```
