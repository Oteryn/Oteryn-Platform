---
task_id: OTERYN-20260817-repository-migration-ultra-overlay
required_reads:
  - AGENTS.md
  - docs/agents/PROMPTING_STANDARD.md
  - docs/agents/PROMPT_EVAL_STANDARD.md
  - docs/agents/ANTI_STALL_AND_EXECUTION_BUDGET.md
  - docs/agents/AUTONOMOUS_PROGRAM_CONTINUATION.md
  - docs/agents/DELIVERY_COMPLETENESS_AND_CLOSEOUT.md
  - docs/agents/prompts/OTERYN_ECOSYSTEM_REPOSITORY_MIGRATION_PROGRAM.md
  - docs/agents/programs/OTERYN_ECOSYSTEM_REPOSITORY_MIGRATION.md
search_first:
  - OTERYN-REPO-MIGRATION-ULTRA
optional_reads: []
---

# OTERYN-20260817-repository-migration-ultra-overlay

## Goal

Add a thin Ultra execution overlay and registered short alias for the existing repository-migration programme without creating a second canonical authority. The large execution budget applies to one foreground owner invocation; the durable migration programme has no fixed elapsed-time limit.

Issue: #1134 — closed by implementation merge.
Implementation PR: #1135 — squash merged as `7abe397fa2b410782ca0fc0187e552f701154a72`.

## Acceptance criteria

- [x] Ultra overlay extends rather than replaces the canonical migration programme.
- [x] Large execution budget is explicitly declared per foreground owner invocation with the current anti-stall policy as the limit source.
- [x] Durable programme lifetime remains unbounded and resumes through recovery-complete checkpoints.
- [x] Delta-first startup and anti-waste rules preserve fresh live-state verification before material mutation.
- [x] Tier-2 cutover remains fail-closed with rollback and immediate post-mutation verification.
- [x] Tier-3 production/DNS/Synology/secret/live-game effects remain outside the alias authority.
- [x] Manual prompt eval covers positive, negative, boundary, stale-state, injection, budget, package/caller and cutover cases.
- [x] `OTERYN-REPO-MIGRATION-ULTRA` is registered in `docs/agents/SHORT_PROGRAM_INVOCATIONS.md`.
- [x] Ultra routing is equivalent to the bounded base migration authorization and adds no broader authority.
- [x] Exact-head self-review passed on `1d9b5e043c7e404a2c8b88f3ad616e33ab0f4d96`.
- [x] Required exact-head CI passed: `classify-changes` and `test`; Agent Governance also passed.
- [x] PR #1135 merged, Issue #1134 closed and source branch deletion verified.

## Ownership

```yaml
owned_paths:
  - docs/agents/prompts/OTERYN_ECOSYSTEM_REPOSITORY_MIGRATION_ULTRA.md
  - docs/agents/evidence/OTERYN-20260817-repository-migration-ultra-prompt-eval.md
  - docs/agents/SHORT_PROGRAM_INVOCATIONS.md
  - docs/agents/tasks/archive/OTERYN-20260817-repository-migration-ultra-overlay.md
modules:
  - agent-governance
  - repository-migration-programme
dependencies:
  - docs/agents/prompts/OTERYN_ECOSYSTEM_REPOSITORY_MIGRATION_PROGRAM.md
  - docs/agents/programs/OTERYN_ECOSYSTEM_REPOSITORY_MIGRATION.md
blockers:
  - none
cross_repository_tasks:
  - none
```

## Execution budget

```yaml
execution_budget_class: large
applies_to: one_foreground_owner_invocation
limit_source: docs/agents/ANTI_STALL_AND_EXECUTION_BUDGET.md
programme_time_limit: none
large_budget_reason: Cross-repository migration execution needs high-confidence live-state, cutover, rollback, package/workflow/provenance and post-mutation verification.
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-08-17T13:56:00Z
head: 1d9b5e043c7e404a2c8b88f3ad616e33ab0f4d96
branch: docs/issue-1134-repo-migration-ultra
pr: 1135
status: completed
context_routes:
  - agent-governance
  - prompt-as-code
  - ecosystem-repository-migration
owned_paths:
  - docs/agents/prompts/OTERYN_ECOSYSTEM_REPOSITORY_MIGRATION_ULTRA.md
  - docs/agents/evidence/OTERYN-20260817-repository-migration-ultra-prompt-eval.md
  - docs/agents/SHORT_PROGRAM_INVOCATIONS.md
  - docs/agents/tasks/archive/OTERYN-20260817-repository-migration-ultra-overlay.md
proven:
  - Canonical migration prompt remains docs/agents/prompts/OTERYN_ECOSYSTEM_REPOSITORY_MIGRATION_PROGRAM.md.
  - The anti-stall policy requires explicit declaration for the large foreground budget and currently resolves that budget to 120 minutes per owner invocation.
  - The durable migration programme itself has no fixed elapsed-time limit.
  - Ultra alias routing is equivalent to the bounded base migration authorization and adds only the stricter execution overlay.
  - Exact-head self-review for 1d9b5e043c7e404a2c8b88f3ad616e33ab0f4d96 recorded PASS on PR 1135 with zero remaining findings.
  - CI run 32036951384 passed required jobs classify-changes and test on exact head 1d9b5e043c7e404a2c8b88f3ad616e33ab0f4d96.
  - Agent Governance run 32036951381 passed on the same exact head.
  - PR 1135 squash-merged as 7abe397fa2b410782ca0fc0187e552f701154a72.
  - Issue 1134 is closed with state_reason completed.
  - Source branch docs/issue-1134-repo-migration-ultra is absent after merge.
derived:
  - The registered Ultra profile can now be invoked without duplicating canonical migration authority.
unknown: []
conflicts: []
first_failure:
  marker: self-review-authority-routing
  evidence: Initial registry wording named only the base alias for cross-repository routing; repaired before readiness and guarded by eval case ULTRA-17.
rejected_hypotheses:
  - Hard-code a programme-wide 120-minute lifetime.
  - Duplicate the full canonical migration prompt.
  - Treat Ultra as a broader independent authorization class.
changed_paths:
  - docs/agents/prompts/OTERYN_ECOSYSTEM_REPOSITORY_MIGRATION_ULTRA.md
  - docs/agents/evidence/OTERYN-20260817-repository-migration-ultra-prompt-eval.md
  - docs/agents/SHORT_PROGRAM_INVOCATIONS.md
  - docs/agents/tasks/archive/OTERYN-20260817-repository-migration-ultra-overlay.md
validation:
  - command: exact-head self-review on PR 1135
    result: PASS
    evidence: PR comment 5316867861 on head 1d9b5e043c7e404a2c8b88f3ad616e33ab0f4d96; zero remaining material findings
  - command: GitHub Actions CI run 32036951384
    result: PASS
    evidence: classify-changes and test completed successfully on exact head
  - command: GitHub Actions Agent Governance run 32036951381
    result: PASS
    evidence: workflow completed successfully on exact head
  - command: E2E
    result: NOT_APPLICABLE
    evidence: prompt/routing governance only; no executable repository cutover or runtime/control-plane mutation
blockers:
  - none
next_action: none
```

## Source branch closeout

```yaml
source_branch_disposition: auto_delete_after_merge
source_branch_reason: implementation PR 1135 merged normally and repository delete-on-merge lifecycle removed the task source branch
source_branch_evidence: branch search after merge returned no docs/issue-1134-repo-migration-ultra ref
```

## Notes

The merged Ultra alias is `OTERYN-REPO-MIGRATION-ULTRA`. Its execution overlay is `docs/agents/prompts/OTERYN_ECOSYSTEM_REPOSITORY_MIGRATION_ULTRA.md`. No physical repository migration, production deployment, Synology/DNS/secret mutation or owner-funded AI invocation occurred in this prompt-registration task.