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

Add a thin Ultra execution overlay and registered short alias for the existing repository-migration programme. Preserve the canonical programme as authority, make the large execution budget explicitly per-invocation rather than programme-wide, enforce delta-first continuation and blocker decomposition, and align terminal reporting with the canonical anti-stall contract.

Issue: #1134

## Acceptance criteria

- [x] Add an Ultra overlay that extends rather than duplicates/replaces the canonical migration programme.
- [x] Explicitly declare the repository-required `large` foreground invocation budget and its reason.
- [x] State that the durable migration programme has no fixed elapsed-time limit and rotates through recovery-complete checkpoints.
- [x] Add delta-first startup and anti-waste rules that preserve live-state refresh before material mutation.
- [x] Preserve fail-closed Tier-2 cutover, rollback, provenance, package, workflow, ownership and post-mutation verification requirements.
- [x] Add a documented manual prompt-evaluation matrix covering positive, negative, boundary, stale-state, injection, budget and cutover cases.
- [ ] Register `OTERYN-REPO-MIGRATION-ULTRA` in `docs/agents/SHORT_PROGRAM_INVOCATIONS.md`.
- [ ] Open/update the draft PR and verify repository-selected exact-head checks.
- [ ] Complete exact-head self-review and terminal closeout when merge gates permit.

## Ownership

```yaml
owned_paths:
  - docs/agents/prompts/OTERYN_ECOSYSTEM_REPOSITORY_MIGRATION_ULTRA.md
  - docs/agents/evidence/OTERYN-20260817-repository-migration-ultra-prompt-eval.md
  - docs/agents/SHORT_PROGRAM_INVOCATIONS.md
  - docs/agents/tasks/active/OTERYN-20260817-repository-migration-ultra-overlay.md
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
large_budget_reason: Cross-repository migration execution needs high-confidence live-state, cutover, rollback, package/workflow/provenance and post-mutation verification; the overlay itself must encode the same bounded invocation semantics.
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-08-17T13:47:00Z
head: 94338e1cb231765d783dcb69f9b9b77268a2d469
branch: docs/issue-1134-repo-migration-ultra
pr: none
status: implementing
context_routes:
  - agent-governance
  - prompt-as-code
  - ecosystem-repository-migration
owned_paths:
  - docs/agents/prompts/OTERYN_ECOSYSTEM_REPOSITORY_MIGRATION_ULTRA.md
  - docs/agents/evidence/OTERYN-20260817-repository-migration-ultra-prompt-eval.md
  - docs/agents/SHORT_PROGRAM_INVOCATIONS.md
  - docs/agents/tasks/active/OTERYN-20260817-repository-migration-ultra-overlay.md
proven:
  - Canonical migration prompt remains docs/agents/prompts/OTERYN_ECOSYSTEM_REPOSITORY_MIGRATION_PROGRAM.md.
  - Current anti-stall policy requires an explicit task declaration for the large foreground budget and currently defines that large budget as 120 minutes.
  - The large budget limits one owner invocation; durable programme continuation persists across rotations.
  - Issue 1134 owns this bounded prompt/routing change.
derived:
  - A thin execution overlay avoids creating a second canonical migration authority while allowing an Ultra-specific execution profile.
unknown:
  - Exact repository-selected CI checks for the final prompt/routing diff until a PR is opened.
conflicts: []
first_failure:
  marker: none
  evidence: none
rejected_hypotheses:
  - Hard-code a programme-wide 120-minute lifetime; rejected because the anti-stall budget applies to one foreground invocation.
  - Duplicate the full canonical migration prompt; rejected because it would create drift and competing behavioural authority.
changed_paths:
  - docs/agents/prompts/OTERYN_ECOSYSTEM_REPOSITORY_MIGRATION_ULTRA.md
  - docs/agents/evidence/OTERYN-20260817-repository-migration-ultra-prompt-eval.md
  - docs/agents/tasks/active/OTERYN-20260817-repository-migration-ultra-overlay.md
validation:
  - command: manual prompt-contract review against PROMPTING_STANDARD.md, PROMPT_EVAL_STANDARD.md and ANTI_STALL_AND_EXECUTION_BUDGET.md
    result: PASS
    evidence: overlay preserves canonical authority, explicitly declares large per-invocation budget, keeps programme_time_limit none, adds fail-closed cutover and canonical final response
blockers:
  - none
next_action: Register OTERYN-REPO-MIGRATION-ULTRA in docs/agents/SHORT_PROGRAM_INVOCATIONS.md and open the draft PR.
```

## Source branch closeout

```yaml
source_branch_disposition: pending
source_branch_reason: task is still active
source_branch_evidence: pending
```

## Notes

This task changes prompting/routing only. It performs no physical repository migration, repository rename/transfer/create, production deployment, Synology/DNS/secret mutation or owner-funded AI invocation.