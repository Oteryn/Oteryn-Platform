---
task_id: OTERYN-20260822-parallel-agent-prompts
required_reads:
  - AGENTS.md
  - docs/agents/AGENTS.md
  - docs/agents/PLATFORM_AGENT_BOOTSTRAP.md
  - docs/agents/PROMPTING_HANDOVER.md
  - docs/agents/PROMPTING_STANDARD.md
  - docs/agents/PROMPT_EVAL_STANDARD.md
  - docs/agents/EXECUTION_PROTOCOL.md
  - docs/agents/DELIVERY_COMPLETENESS_AND_CLOSEOUT.md
search_first:
  - docs/agents/prompts
  - docs/agents/SHORT_PROGRAM_INVOCATIONS.md
optional_reads: []
---

# OTERYN-20260822-parallel-agent-prompts

## Goal

Create four repository-owned launch prompts and short aliases for a safe parallel Platform completion wave after Portal Docker E2E PR #1223 merged.

## Acceptance criteria

- [ ] Four lane prompts are standalone, live-state-first, path/authority bounded, and safe to run in parallel.
- [ ] Character, Game Catalog, and Payments prompts preserve their current dependency/authority gates instead of inventing missing producer or provider facts.
- [ ] A coordination-only wave prompt cannot steal sibling product ownership.
- [ ] Short invocation registry maps each alias to its exact canonical prompt.
- [ ] Deterministic prompt-contract evaluation covers success, overlap, external-authority, payment, stale-state and closeout safety cases.
- [ ] Documentation/governance validation and full exact-head self-review pass.

## Ownership

```yaml
owned_paths:
  - docs/agents/prompts/OTERYN-PLATFORM-PARALLEL-WAVE-COORDINATOR.md
  - docs/agents/prompts/OTERYN-CHARACTER-LIFECYCLE-BARRIER-AGENT.md
  - docs/agents/prompts/OTERYN-GAME-CATALOG-COMPLETION-AGENT.md
  - docs/agents/prompts/OTERYN-PAYMENTS-FOUNDATION-AGENT.md
  - docs/agents/evals/oteryn-platform-parallel-wave-prompts-v1.json
  - docs/agents/SHORT_PROGRAM_INVOCATIONS.md
  - docs/agents/tasks/active/OTERYN-20260822-parallel-agent-prompts.md
modules:
  - agent-governance
  - prompt-routing
dependencies:
  - OTERYN_PORTAL_COMPLETION live selector
  - Issues #301, #317, #319, #320, #321
blockers:
  - none
cross_repository_tasks:
  - none
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-08-22T15:40:00Z
head: 0291b95acb8d7de01952878aa002a9fe6e7d3d91
branch: docs/parallel-agent-prompts-20260822
pr: 1224
status: ready
context_routes:
  - agent-governance
owned_paths:
  - docs/agents/prompts/OTERYN-PLATFORM-PARALLEL-WAVE-COORDINATOR.md
  - docs/agents/prompts/OTERYN-CHARACTER-LIFECYCLE-BARRIER-AGENT.md
  - docs/agents/prompts/OTERYN-GAME-CATALOG-COMPLETION-AGENT.md
  - docs/agents/prompts/OTERYN-PAYMENTS-FOUNDATION-AGENT.md
  - docs/agents/evals/oteryn-platform-parallel-wave-prompts-v1.json
  - docs/agents/SHORT_PROGRAM_INVOCATIONS.md
  - docs/agents/tasks/active/OTERYN-20260822-parallel-agent-prompts.md
proven:
  - PR #1223 merged to main as 8b307a1e5ba2dea02d644147dc1841059588cd7c before prompt construction.
  - No active task record currently claims Issues #301, #317, #319, #320, #321 or #322.
  - Open PR #338 owns the inactive Game Catalog schema 1.3 NPC/shop consumer and remains held on producer compatibility.
derived:
  - The prior Portal Closeout worker prompt would be stale as a dedicated implementation lane; the wave needs a coordination-only post-merge role instead.
unknown: []
conflicts: []
first_failure:
  marker: none
  evidence: none
rejected_hypotheses: []
changed_paths:
  - docs/agents/SHORT_PROGRAM_INVOCATIONS.md
  - docs/agents/evals/oteryn-platform-parallel-wave-prompts-v1.json
  - docs/agents/prompts/OTERYN-PLATFORM-PARALLEL-WAVE-COORDINATOR.md
  - docs/agents/prompts/OTERYN-CHARACTER-LIFECYCLE-BARRIER-AGENT.md
  - docs/agents/prompts/OTERYN-GAME-CATALOG-COMPLETION-AGENT.md
  - docs/agents/prompts/OTERYN-PAYMENTS-FOUNDATION-AGENT.md
  - docs/agents/tasks/active/OTERYN-20260822-parallel-agent-prompts.md
validation:
  - command: python tools/validation/prompt_eval.py --suite docs/agents/evals/oteryn-platform-parallel-wave-prompts-v1.json
    result: PASS
    evidence: 11 cases, 11 categories, 4 safety-critical deterministic checks; model trials not claimed
  - command: python tools/validation/test_prompt_eval.py
    result: PASS
    evidence: 8 unit tests passed
  - command: python tools/agents/checkpoint.py docs/agents/tasks/active/OTERYN-20260822-parallel-agent-prompts.md --require-checkpoint
    result: PASS
    evidence: checkpoint contract v1 validated
  - command: git diff --check
    result: PASS
    evidence: no whitespace errors
  - command: runtime/browser E2E
    result: NOT_APPLICABLE
    evidence: documentation and agent-routing only; no executable product behavior changed
blockers:
  - none
next_action: verify exact-head PR #1224 CI and merge only after all required gates pass
```

## Validation gate

```yaml
self_review:
  result: PASS
  exact_head: 0291b95acb8d7de01952878aa002a9fe6e7d3d91
  acceptance_checked: true
  full_diff_checked: true
  negative_paths_checked: true
  rollback_checked: true
  compatibility_checked: true
  related_prs_checked: true
  findings: []
  evidence:
    - seven task-owned changed paths only
    - no external-repository or production authority expansion
    - PR #338 hold preserved
    - payment test adapter remains non-production and #321 remains open on provider gates
```

## Source branch closeout

```yaml
source_branch_disposition: auto_delete_after_merge
source_branch_reason: ordinary documentation/governance task branch
source_branch_evidence: pending
```

## Notes

This task changes agent routing only. Runtime/browser E2E is expected to be `NOT_APPLICABLE` because no executable product code is changed.
