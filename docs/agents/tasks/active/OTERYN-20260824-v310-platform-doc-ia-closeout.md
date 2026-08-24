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

- [ ] Exact current-head Documentation/Agent IA inventory is deterministic.
- [ ] Every retained prompt is classified reusable or one-shot/historical with deterministic lifecycle metadata.
- [ ] Terminal one-shot prompts are non-executable while provenance remains resolvable.
- [ ] Active task packets reconcile to live Issue/PR authority and terminal Issues cannot leave stale active tasks.
- [ ] Retained handovers are non-authoritative and expire/supersede on live lifecycle transitions.
- [ ] Effective instruction chain is measured and any verified live defect is repaired.
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
  - docs/agents/tasks/active/OTERYN-20260805-native-auth-production-verification.md
  - docs/agents/tasks/archive/OTERYN-20260805-native-auth-production-verification.md
  - docs/agents/tasks/active/OTERYN-20260824-v310-platform-doc-ia-closeout.md
  - docs/agents/tasks/archive/OTERYN-20260824-v310-platform-doc-ia-closeout.md
  - docs/agents/evidence/OTERYN-20260824-v310-platform-doc-ia-closeout.md
  - tools/agents/task_liveness.py
  - tools/agents/test_task_liveness.py
  - tools/agents/documentation_ia.py
  - tools/agents/test_documentation_ia.py
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
updated_at: 2026-08-24T17:08:00Z
head: b930d2782e1d2fe01f66cde5c28b1c2486541cec
branch: agent/v310-platform-doc-ia-closeout
pr: none
status: investigating
context_routes:
  - agent-governance
owned_paths:
  - docs/agents/**
  - tools/agents/**
  - .github/workflows/agent-governance.yml
proven:
  - protected main audited at b930d2782e1d2fe01f66cde5c28b1c2486541cec
  - prompt inventory contains 22 Markdown files
  - active task inventory contains 2 task packets
  - handover inventory contains 3 Markdown files
  - Issue #864 is closed while its native-auth verification task remains active
  - CONTEXT_HANDOFF.md names policy revision 2 while GOVERNANCE_CONTRACT.json is revision 3
derived:
  - deterministic prompt and handover classification needs a machine-validated catalog
  - active task liveness must include governing Issue state
unknown:
  - final prompt classification until every prompt is inspected
conflicts: []
first_failure:
  marker: none
  evidence: none
rejected_hypotheses:
  - runtime or migration repair is required for this bounded IA closeout
changed_paths:
  - docs/agents/tasks/active/OTERYN-20260824-v310-platform-doc-ia-closeout.md
validation:
  - command: pending
    result: NOT_RUN
    evidence: implementation not yet complete
blockers:
  - none
next_action: inspect and classify every retained prompt, then implement the deterministic IA catalog and liveness checks
```

## Source branch closeout

```yaml
source_branch_disposition: auto_delete_after_merge
source_branch_reason: ordinary same-repository bounded documentation/governance repair
source_branch_evidence: repository setting delete_branch_on_merge=true; verify source ref disappearance after merge
```

## Notes

This task is intentionally bounded to Platform Documentation/Agent IA. Product runtime/browser E2E is `NOT_APPLICABLE` because no product behavior is in scope.
