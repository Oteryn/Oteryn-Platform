---
task_id: OTERYN-20260822-organization-runner-bootstrap-closeout
required_reads:
  - AGENTS.md
  - docs/agents/PLATFORM_AGENT_BOOTSTRAP.md
  - docs/agents/DELIVERY_COMPLETENESS_AND_CLOSEOUT.md
  - docs/agents/ANTI_STALL_AND_EXECUTION_BUDGET.md
  - docs/agents/SESSION_RECOVERY_AND_ORPHANED_EXECUTION.md
  - docs/agents/TERMINAL_ONLY_COMMUNICATION.md
  - docs/agents/GITHUB_ONLY_EXECUTION.md
  - docs/agents/AUTONOMOUS_PROGRAM_CONTINUATION.md
  - docs/agents/REPOSITORY_MAP.md
  - docs/agents/CONTEXT_ROUTING.md
search_first:
  - organization runner bootstrap
  - deploy/synology/runner
optional_reads: []
---

# OTERYN-20260822-organization-runner-bootstrap-closeout

## Goal

Rebase the canonical organization-capable Synology runner bootstrap/hardening from stale PR #1200 onto current Platform `main`, without restoring superseded activation workflows, then terminally reconcile Platform #1199 and PR #1200 after exact-head validation.

## Acceptance criteria

- [ ] Current Platform `main` receives the reviewed organization/repository registration entrypoint, immutable runner image inputs, self-tests and no-secret organization Compose/runbook.
- [ ] Existing repository-scoped `oteryn-staging` restart compatibility remains intact.
- [ ] Organization mode requires explicit group/name/custom labels, uses `--runnergroup` and `--no-default-labels`, and rejects generic default labels.
- [ ] Current main-only runner Compose retains the existing Platform state mount while gaining the generic scope/group/token-file inputs.
- [ ] Superseded activation/repair workflows from stale PR #1200 are not reintroduced.
- [ ] Exact-head Platform CI/governance/security/build validation passes and no material review finding remains.
- [ ] PR #1200 is intentionally closed as superseded; #1199 closes only after current-main successor merge and resulting source inspection.

## Ownership

```yaml
owned_paths:
  - deploy/synology/runner/.env.example
  - deploy/synology/runner/Dockerfile
  - deploy/synology/runner/compose.yml
  - deploy/synology/runner/compose.organization.example.yml
  - deploy/synology/runner/entrypoint.sh
  - deploy/synology/runner/organization.env.example
  - deploy/synology/runner/test-entrypoint.sh
  - docs/operations/SYNOLOGY_ORGANIZATION_RUNNERS.md
  - docs/agents/tasks/active/OTERYN-20260822-organization-runner-bootstrap-closeout.md
modules:
  - synology-runner-bootstrap
dependencies:
  - Oteryn/Oteryn-Platform#1199
  - Oteryn/Oteryn#34
blockers:
  - none
cross_repository_tasks:
  - Oteryn/Oteryn-Atlas#35
  - Oteryn/Oteryn-Game#34
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-08-22T02:28:00Z
head: a8d46c91571a8df5193d6ddd2c28b35db2e85934
branch: infra/issue-1199-runner-bootstrap-current-main
pr: none
status: implementing
context_routes:
  - testing
  - ci-repair
  - agent-governance
owned_paths:
  - deploy/synology/runner/.env.example
  - deploy/synology/runner/Dockerfile
  - deploy/synology/runner/compose.yml
  - deploy/synology/runner/compose.organization.example.yml
  - deploy/synology/runner/entrypoint.sh
  - deploy/synology/runner/organization.env.example
  - deploy/synology/runner/test-entrypoint.sh
  - docs/operations/SYNOLOGY_ORGANIZATION_RUNNERS.md
  - docs/agents/tasks/active/OTERYN-20260822-organization-runner-bootstrap-closeout.md
proven:
  - current Platform main is a8d46c91571a8df5193d6ddd2c28b35db2e85934 after Atlas scaffold terminal cleanup
  - stale PR 1200 head 7f5b77361cb8719e163704b528845d1f3463e1a6 is 11 commits behind current main and must not merge as-is
  - all canonical runner-core files except deploy/synology/runner/compose.yml are untouched on current main since PR 1200 merge base 3f1a0eeb42a777106bef466dbcb4150d8a1bb818
  - current live organization runners previously proved Actions Runner 2.336.0 and immutable Oteryn image digest sha256:f0c452798a17df09006a12d437e83a72d681dcd338ef22ed01fca329d1bbab8d
derived:
  - the safe current-main closeout is a bounded carry-forward of reviewed runner-core files only, excluding stale task-owned activation workflows
unknown:
  - exact-head validation result of the current-main successor candidate
conflicts: []
first_failure:
  marker: none
  evidence: none
rejected_hypotheses:
  - merge PR 1200 directly; rejected because its base diverged and it contains superseded activation workflows
changed_paths:
  - docs/agents/tasks/active/OTERYN-20260822-organization-runner-bootstrap-closeout.md
validation:
  - command: not-run
    result: NOT_RUN
    evidence: implementation is being reconstructed on current main
blockers:
  - none
next_action: carry forward only the canonical runner bootstrap/image/self-test/runbook files onto this branch
```

## Recovery checkpoint

```yaml
recovery:
  policy_version: 1
  generation: 1
  session_id: runner-bootstrap-closeout-20260822-001
  session_started_at: 2026-08-22T02:20:00Z
  checkpointed_at: 2026-08-22T02:28:00Z
  last_progress_at: 2026-08-22T02:28:00Z
  phase: implementation
  exact_head: a8d46c91571a8df5193d6ddd2c28b35db2e85934
  pull_request: none
  active_operation: none
  external_run_ids: []
  operation_started_at: null
  wait_deadline_at: null
  check_generation: null
  checks_used: 0
  status: active
  safe_to_resume: true
  resume_condition: current task branch still owns the declared paths
  next_action: carry forward only the canonical runner bootstrap/image/self-test/runbook files onto this branch
```

## Source branch closeout

```yaml
source_branch_disposition: pending
source_branch_reason: task is still active
source_branch_evidence: pending
```

## Notes

This task is repository hardening/closeout only. It does not create runner groups, registration tokens, or mutate the live Synology estate.