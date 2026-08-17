---
task_id: OTERYN-20260817-ecosystem-repository-migration-prompt
required_reads:
  - AGENTS.md
  - docs/agents/AGENTS.md
  - docs/agents/DELIVERY_COMPLETENESS_AND_CLOSEOUT.md
  - docs/agents/ANTI_STALL_AND_EXECUTION_BUDGET.md
  - docs/agents/GITHUB_ONLY_EXECUTION.md
  - docs/agents/EXECUTION_PROTOCOL.md
  - docs/agents/PROJECT_LANES.json
---

# OTERYN-20260817-ecosystem-repository-migration-prompt

## Goal

Persist the owner-requested high-reasoning Oteryn ecosystem repository-migration programme as one canonical prompt plus durable short alias `OTERYN-REPO-MIGRATION`, initial programme state and explicit manual prompt-evaluation matrix, without changing runtime/product behaviour or consuming owner-funded AI review.

## Acceptance criteria

- [x] The canonical migration prompt is registered at `docs/agents/prompts/OTERYN_ECOSYSTEM_REPOSITORY_MIGRATION_PROGRAM.md`.
- [x] Durable programme state exists at `docs/agents/programs/OTERYN_ECOSYSTEM_REPOSITORY_MIGRATION.md`.
- [x] `docs/agents/SHORT_PROGRAM_INVOCATIONS.md` registers `OTERYN-REPO-MIGRATION`.
- [x] The manual prompt-evaluation matrix is recorded without claiming automated model evaluation.
- [x] Final PR changed exactly the five declared documentation/governance paths.
- [x] Exact-head CI and Agent Governance passed on the final PR head.
- [x] Whole-diff self-review passed with no material findings.
- [x] PR #1124 squash-merged through protected `main`.
- [x] Source branch was auto-deleted after merge.
- [x] Runtime/component/browser E2E is `NOT_APPLICABLE`: the delivery changes prompt/programme/task/registry documentation only.
- [x] No owner-funded Codex/OpenAI/API invocation was used by this task.

## Ownership

```yaml
owned_paths:
  - docs/agents/prompts/OTERYN_ECOSYSTEM_REPOSITORY_MIGRATION_PROGRAM.md
  - docs/agents/programs/OTERYN_ECOSYSTEM_REPOSITORY_MIGRATION.md
  - docs/agents/SHORT_PROGRAM_INVOCATIONS.md
  - docs/agents/evidence/OTERYN-20260817-ecosystem-repository-migration-prompt-eval.md
  - docs/agents/tasks/archive/OTERYN-20260817-ecosystem-repository-migration-prompt.md
modules:
  - agent-governance
  - prompt-routing
project_lane: oteryn-platform-core
blockers: []
cross_repository_tasks: []
```

## Context checkpoint

```yaml
checkpoint_version: 2
policy_version: 2
updated_at: 2026-08-17T14:35:00+02:00
phase: close
execution_mode: chat_github
execution_reason: lifecycle-only documentation closeout is fully supported by the GitHub connector and repository CI
status: completed
head: e26f38ebdc1ebd6d8922a33cb017970da8cca23c
branch: docs/oteryn-ecosystem-repository-migration-prompt
pr: 1124
project_lane: oteryn-platform-core
task_kind: implementation
context_pressure: low
context_growth: stable
context_score: 4
estimate_confidence: high
decomposition_decision: single
decomposition_reason: one prompt-registration delivery and its required lifecycle closeout
invocation_started_at: 2026-08-17T14:21:00+02:00
last_progress_at: 2026-08-17T14:35:00+02:00
ci_checks_for_current_head: 2
ci_check_generation: ready
terminal_ci_wait_started_at: null
terminal_ci_checks_for_current_generation: 0
unchanged_state_checks: 0
identical_failure_retries: 0
repair_cycles_for_current_gate: 1
context_reconstruction_attempts: 1
stall_warnings: 0
proven:
  - PR 1124 final exact head was e26f38ebdc1ebd6d8922a33cb017970da8cca23c.
  - Final PR changed exactly five declared paths: SHORT_PROGRAM_INVOCATIONS, prompt evaluation, programme state, canonical prompt, and this task record.
  - CI run 32029947663 passed on exact head e26f38ebdc1ebd6d8922a33cb017970da8cca23c.
  - Agent Governance run 32029947610 passed on exact head e26f38ebdc1ebd6d8922a33cb017970da8cca23c.
  - Final whole-diff self-review passed with no material findings; later main integration changed ancestry only and preserved the five-file PR diff.
  - PR 1124 had zero review threads at terminal inspection.
  - PR 1124 squash-merged as 7d11777b2a411621b70246970c1225042e1a6367.
  - Source branch docs/oteryn-ecosystem-repository-migration-prompt was absent after merge.
  - No temporary host, container, runner, deployment, Synology, DNS, production, payment, or secret resource was created or mutated by this task.
  - Authenticated GitHub organization membership inspection after merge returned an empty organization set; this is programme discovery evidence, not authority created by this task.
derived:
  - The migration programme is now canonical on protected main and can be invoked from durable repository state.
unknown: []
conflicts: []
first_failure:
  marker: ready_generation_required_status_refresh
  evidence: the first protected merge attempt after Draft-to-Ready was rejected because the required status-check generation had to be refreshed against current main; the branch was merged normally with current main and fresh exact-head checks passed.
rejected_hypotheses:
  - Direct protected-main write was not used.
  - Force-push or branch-protection bypass was not used.
  - Owner-funded Codex/OpenAI review was not used.
changed_paths:
  - docs/agents/prompts/OTERYN_ECOSYSTEM_REPOSITORY_MIGRATION_PROGRAM.md
  - docs/agents/programs/OTERYN_ECOSYSTEM_REPOSITORY_MIGRATION.md
  - docs/agents/SHORT_PROGRAM_INVOCATIONS.md
  - docs/agents/evidence/OTERYN-20260817-ecosystem-repository-migration-prompt-eval.md
  - docs/agents/tasks/archive/OTERYN-20260817-ecosystem-repository-migration-prompt.md
validation:
  - command: CI run 32029947663 on exact PR head e26f38ebdc1ebd6d8922a33cb017970da8cca23c
    result: PASS
    evidence: workflow conclusion success
  - command: Agent Governance run 32029947610 on exact PR head e26f38ebdc1ebd6d8922a33cb017970da8cca23c
    result: PASS
    evidence: workflow conclusion success
  - command: exact final changed-file inspection
    result: PASS
    evidence: five declared paths only
  - command: exact-head whole-diff self-review
    result: PASS
    evidence: no material findings
  - command: pull-request review-thread inspection
    result: PASS
    evidence: zero review threads
  - command: runtime/component/browser E2E
    result: NOT_APPLICABLE
    evidence: prompt/programme/task/registry documentation only; no executable product/runtime path changed
blockers: []
next_action: none; registration task is terminal after this archival closeout merges
```

## Self-review

```yaml
self_review:
  result: PASS
  exact_head: e26f38ebdc1ebd6d8922a33cb017970da8cca23c
  acceptance_checked: true
  full_diff_checked: true
  negative_paths_checked: true
  rollback_checked: true
  compatibility_checked: true
  related_prs_checked: true
  findings: []
  evidence:
    - PR #1124 final changed-file set
    - CI 32029947663
    - Agent Governance 32029947610
    - zero review threads
```

## Source branch closeout

```yaml
source_branch_disposition: auto_delete_after_merge
source_branch_reason: ordinary same-repository prompt/governance delivery with no durable post-merge branch purpose
source_branch_evidence: PR 1124 merged as 7d11777b2a411621b70246970c1225042e1a6367 and branch lookup returned no docs/oteryn-ecosystem-repository-migration-prompt ref afterward
```

## Notes

This task registered the migration executor only. It performed no repository rename/create/transfer/extraction and no production/DNS/Synology/secret/live-game mutation.