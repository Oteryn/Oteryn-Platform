---
task_id: OTERYN-20261011-native-login-e2e-1
governing_issue: 1474
required_reads: []
search_first:
  - deploy/synology/scripts/lib.sh
optional_reads: []
---

# OTERYN-20261011 Native login end to end, deploy route publication

## Goal

Governing GitHub Issue: #1474 — canonical lifecycle authority for this task. The top-level `governing_issue` frontmatter field is machine-required while this packet remains active.

Make the Synology preproduction deploy publish the Game world's native route record automatically from repository variables, failing closed on a missing or malformed variable, and give the owner a numbered checklist for the whole native login path. The publication step is opt-in and ships switched off because the route commands are fenced to the disposable store and refuse the Synology database; that decision is an owner question. The whole-chain Platform feature test is a follow-up.

## Acceptance criteria

- [x] Deploy publishes topology and route record from repository variables when enabled, and fails closed with a named variable otherwise.
- [x] Owner checklist exists with variable names, formats, workflows and verification.
- [ ] Whole-chain Platform feature test (follow-up).
- [ ] Exact-head CI and Agent Governance gates pass.

## Ownership

```yaml
owned_paths:
  - .github/workflows/deploy-synology-staging.yml
  - deploy/synology/scripts/lib.sh
  - deploy/synology/scripts/deploy.sh
  - tests/ci/test_synology_native_route_publication.py
  - docs/operations/NATIVE_LOGIN_OWNER_STEPS.md
  - docs/agents/tasks/active/OTERYN-20261011-native-login-e2e-1.md
  - docs/agents/tasks/archive/OTERYN-20261011-native-login-e2e-1.md
modules:
  - deploy
  - game-auth
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
updated_at: 2026-10-11T09:00:00Z
head: UNKNOWN
branch: claude/native-login-e2e-1-20261011
pr: none
status: validating
context_routes:
  - ci-workflows
owned_paths:
  - .github/workflows/deploy-synology-staging.yml
  - deploy/synology/scripts/lib.sh
  - deploy/synology/scripts/deploy.sh
  - tests/ci/test_synology_native_route_publication.py
  - docs/operations/NATIVE_LOGIN_OWNER_STEPS.md
  - docs/agents/tasks/active/OTERYN-20261011-native-login-e2e-1.md
proven:
  - game-auth:native-topology:issue and native-route:publish are fenced to the disposable store and refuse the Synology MariaDB
derived:
  - publication stays opt-in until the owner decides how the fence admits the preproduction database
unknown:
  - behaviour on the live Synology deploy
conflicts: []
first_failure:
  marker: none
  evidence: none
rejected_hypotheses: []
changed_paths:
  - .github/workflows/deploy-synology-staging.yml
  - deploy/synology/scripts/lib.sh
  - deploy/synology/scripts/deploy.sh
  - tests/ci/test_synology_native_route_publication.py
  - docs/operations/NATIVE_LOGIN_OWNER_STEPS.md
validation:
  - command: python tests/ci/test_synology_native_route_publication.py
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

Follow-up: whole-chain feature test (OAuth ticket to admission to route record) drafted but unexecuted for lack of PHP dependencies in the authoring environment.
