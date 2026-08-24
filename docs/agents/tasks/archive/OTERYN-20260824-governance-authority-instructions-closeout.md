---
task_id: OTERYN-20260824-governance-authority-instructions-closeout
issue: 1254
status: completed
project_lane: oteryn-platform-core
execution_mode: github_connector
required_reads: []
search_first:
  - GitHub Issue #1254
optional_reads: []
---

# OTERYN-20260824-governance-authority-instructions-closeout

## Goal

Governing GitHub Issue: #1254 — canonical lifecycle authority for this task.

Close only original audit v3.9 findings P1.1 lifecycle authority and P1.2 bounded instruction loading with the smallest compatible governance-document patch.

## Acceptance criteria

- [x] GitHub Issue/task is canonical for task lifecycle and live PR is authoritative for PR state.
- [x] Task/context records are durable context/evidence/handoff mirrors and stale fields cannot override newer GitHub state.
- [x] Root `AGENTS.md` remains the entry point and mandatory instruction loading is bounded, deterministic and non-recursive.
- [x] Material safety, repository, merge, concurrency, execution-resource and owner-funded-AI protections remain intact.
- [x] Only authorized P1.1/P1.2 paths change; exact-head required checks must pass before squash merge.

## Ownership

```yaml
owned_paths:
  - AGENTS.md
  - docs/agents/PLATFORM_AGENT_BOOTSTRAP.md
  - docs/agents/REPOSITORY_MAP.md
  - docs/agents/CONTEXT_ROUTING.md
  - docs/agents/CONTEXT_HANDOFF.md
  - docs/agents/tasks/TASK_TEMPLATE.md
  - docs/agents/tasks/archive/OTERYN-20260824-governance-authority-instructions-closeout.md
modules:
  - agent-governance
dependencies:
  - GitHub Issue #1254
blockers:
  - none
cross_repository_tasks:
  - none
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-08-24T12:10:00Z
head: e649e9e007c6d1876ac8ee36d0165faa4d39e1e3
branch: docs/issue-1254-governance-authority-instructions
pr: 1257
status: completed
context_routes:
  - agent-governance
owned_paths:
  - AGENTS.md
  - docs/agents/PLATFORM_AGENT_BOOTSTRAP.md
  - docs/agents/REPOSITORY_MAP.md
  - docs/agents/CONTEXT_ROUTING.md
  - docs/agents/CONTEXT_HANDOFF.md
  - docs/agents/tasks/TASK_TEMPLATE.md
  - docs/agents/tasks/archive/OTERYN-20260824-governance-authority-instructions-closeout.md
proven:
  - Task continuation live main is 557c08e72497ecd6a1b07fe8f282a1754763a4ff.
  - GitHub Issue #1254 is the governing lifecycle authority for this task and PR #1257 is the live PR-state authority.
  - Branch protection requires only platform-gate and repository policy allows squash merge.
  - PR #1257 diff is bounded to the six authorized governance documents plus this one task-specific record.
  - Full PR diff review confirms no P1.3, P1.4, P1.5, P1.6, workflow, application, dependency, migration, runner, security-setting, branch-protection, product or cross-repository changes.
  - The previous exact head passed 95 policy-consistency regression tests, policy_consistency.py, checkpoint validation and git diff --check.
  - Agent Governance run 32724726090 failed for this task only because its checkpoint still recorded pr none after PR #1257 opened; this archive candidate reconciles that live PR identity.
  - Agent Governance run 32724726090 also failed on pre-existing task OTERYN-20260823-platform-transfer-terminal-reconciliation because terminal PR #1243 remains represented as active; that task is outside P1.1/P1.2 and was not modified.
derived:
  - The P1.1/P1.2 documentation patch can be rebased unchanged onto current main because intervening P1.4 merge #1256 changed only its separate archive task record.
unknown: []
conflicts: []
first_failure:
  marker: agent-governance-live-task-liveness
  evidence: run 32724726090; own stale pr mirror corrected here, unrelated #1243 liveness finding recorded out of scope.
rejected_hypotheses:
  - Agent Governance failure proves the P1.1/P1.2 governance-document semantics are invalid.
changed_paths:
  - AGENTS.md
  - docs/agents/PLATFORM_AGENT_BOOTSTRAP.md
  - docs/agents/REPOSITORY_MAP.md
  - docs/agents/CONTEXT_ROUTING.md
  - docs/agents/CONTEXT_HANDOFF.md
  - docs/agents/tasks/TASK_TEMPLATE.md
  - docs/agents/tasks/archive/OTERYN-20260824-governance-authority-instructions-closeout.md
validation:
  - command: python tools/agents/test_policy_consistency.py
    result: PASS
    evidence: 95 of 95 policy-consistency regressions passed on prior patch head; final exact-head CI will re-run applicable policy validation.
  - command: python tools/agents/policy_consistency.py
    result: PASS
    evidence: Agent governance policy consistency PASS on prior patch head; final exact-head CI will re-run applicable policy validation.
  - command: python tools/agents/checkpoint.py docs/agents/tasks/active/OTERYN-20260824-governance-authority-instructions-closeout.md --require-checkpoint
    result: PASS
    evidence: prior active checkpoint validated against contract v1.
  - command: git diff --check
    result: PASS
    evidence: prior patch head had no whitespace errors; final PR diff will be re-inspected.
  - command: runtime/browser E2E
    result: NOT_APPLICABLE
    evidence: governance documentation only; no executable product or integration journey changed.
blockers: []
next_action: rebase the bounded patch onto current main, verify final exact-head platform-gate and full diff/review state, then squash-merge PR #1257.
```

## Source branch closeout

```yaml
source_branch_disposition: auto_delete_after_merge
source_branch_reason: same-repository squash PR #1257 is the sole delivery path and this source ref has no retention purpose
source_branch_evidence: repository delete_branch_on_merge is enabled; final ref absence must be verified immediately after merge
```

## Out-of-scope finding

OUT_OF_SCOPE_FINDING: Agent Governance run 32724726090 also failed because pre-existing task `OTERYN-20260823-platform-transfer-terminal-reconciliation` still represents terminal PR #1243 as active. That task/path is outside P1.1/P1.2 and was not modified.

## Notes

Runtime/browser E2E is `NOT_APPLICABLE` because this task changes governance documentation only.
