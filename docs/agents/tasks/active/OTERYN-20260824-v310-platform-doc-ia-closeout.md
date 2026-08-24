---
task_id: OTERYN-20260824-v310-platform-doc-ia-closeout
governing_issue: 1260
required_reads:
  - docs/agents/DOCUMENTATION_IA_LIFECYCLE.md
search_first:
  - live Issues, PRs, active tasks, prompt inventory, handover inventory, effective AGENTS.md chain
optional_reads: []
---

# OTERYN-20260824-v310-platform-doc-ia-closeout

## Goal

Governing GitHub Issue: #1260 — canonical lifecycle authority for this task.

Close the bounded v3.10 Platform Documentation/Agent IA gaps without touching runtime, migration, runner, release, production, or external-repository surfaces.

## Acceptance criteria

- [x] Exact current-head Documentation/Agent IA inventory is deterministic.
- [x] Every retained prompt is classified reusable or one-shot/historical with deterministic lifecycle metadata.
- [x] Terminal one-shot prompts are non-executable while provenance remains resolvable.
- [x] Active task packets reconcile to live Issue/PR authority and terminal Issues cannot leave stale active tasks.
- [x] Retained handovers are non-authoritative and expire/supersede on live lifecycle transitions.
- [x] Effective instruction chain is measured and any verified live defect is repaired.
- [ ] Focused governance validation and exact-head required CI pass.
- [ ] PR is squash-merged, Issue is terminal, task is archived, source branch is removed, merged main is read back.

## Ownership

```yaml
owned_paths:
  - docs/agents/DOCUMENTATION_IA_CATALOG.json
  - docs/agents/DOCUMENTATION_IA_LIFECYCLE.md
  - docs/agents/CONTEXT_HANDOFF.md
  - docs/agents/GOVERNANCE_CONTRACT.json
  - docs/agents/tasks/TASK_TEMPLATE.md
  - docs/agents/tasks/active/OTERYN-20260801-public-domain-repair.md
  - docs/agents/tasks/archive/OTERYN-20260805-native-auth-production-verification.md
  - docs/agents/tasks/active/OTERYN-20260824-v310-platform-doc-ia-closeout.md
  - tools/agents/documentation_ia.py
  - tools/agents/test_documentation_ia.py
  - tools/agents/task_issue_liveness.py
  - tools/agents/test_task_issue_liveness.py
  - .github/workflows/agent-governance.yml
modules:
  - agent-governance
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
updated_at: 2026-08-24T17:31:00Z
head: 495de3657c5d62d8f79df3b7dcb17431eb8e09d7
branch: agent/v310-platform-doc-ia-closeout
pr: 1261
status: validating
context_routes:
  - agent-governance
owned_paths:
  - docs/agents/**
  - tools/agents/**
  - .github/workflows/agent-governance.yml
proven:
  - protected main baseline audited at b930d2782e1d2fe01f66cde5c28b1c2486541cec
  - exact prompt inventory contains 22 Markdown files and is classified 10 reusable plus 12 historical non-executable entries
  - exact handover inventory contains 3 Markdown files and all are cataloged authoritative false
  - Issue #864 is terminal and its stale active task cache is moved to archive with Git-history provenance
  - public-domain active task is governed by open Issue #91 and now records governing_issue 91
  - effective instruction chain is AGENTS.md plus docs/agents/AGENTS.md with four measured absent nearer overrides
  - policy revision mismatch is repaired by synchronized revision 4 governing-Issue liveness documentation and contract
  - product runtime browser E2E is not applicable because this change only modifies Documentation/Agent IA governance surfaces
derived:
  - deterministic catalog and live governing-Issue checks prevent the verified prompt, handover and stale-task recurrence classes
unknown: []
conflicts: []
first_failure:
  marker: none
  evidence: none
rejected_hypotheses:
  - runtime, migration, runner, package, release or production changes are required for this bounded closeout
changed_paths:
  - .github/workflows/agent-governance.yml
  - docs/agents/CONTEXT_HANDOFF.md
  - docs/agents/DOCUMENTATION_IA_CATALOG.json
  - docs/agents/DOCUMENTATION_IA_LIFECYCLE.md
  - docs/agents/GOVERNANCE_CONTRACT.json
  - docs/agents/tasks/TASK_TEMPLATE.md
  - docs/agents/tasks/active/OTERYN-20260801-public-domain-repair.md
  - docs/agents/tasks/active/OTERYN-20260805-native-auth-production-verification.md
  - docs/agents/tasks/archive/OTERYN-20260805-native-auth-production-verification.md
  - docs/agents/tasks/active/OTERYN-20260824-v310-platform-doc-ia-closeout.md
  - tools/agents/documentation_ia.py
  - tools/agents/task_issue_liveness.py
  - tools/agents/test_documentation_ia.py
  - tools/agents/test_task_issue_liveness.py
validation:
  - command: isolated Documentation/Agent IA validator test suite
    result: PASS
    evidence: 5 unit tests passed before repository commit
  - command: isolated governing Issue liveness validator test suite
    result: PASS
    evidence: 5 unit tests passed before repository commit
  - command: product runtime/browser E2E
    result: NOT_APPLICABLE
    evidence: bounded governance/documentation-only change modifies no product runtime path
blockers:
  - none
next_action: Run exact-head Agent Governance and required platform-gate for PR #1261, remediate any failure, then archive this task in the same PR and squash merge after a final exact-head pass.
```

## Source branch closeout

```yaml
source_branch_disposition: auto_delete_after_merge
source_branch_reason: ordinary same-repository bounded documentation/governance repair
source_branch_evidence: repository setting delete_branch_on_merge=true; verify source ref disappearance after merge
```

## Notes

This task is intentionally bounded to Platform Documentation/Agent IA. Product runtime/browser E2E is `NOT_APPLICABLE` because no product behavior is in scope.
