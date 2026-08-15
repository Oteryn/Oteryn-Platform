---
task_id: OTERYN-20260815-portal-completion-prompt-hardening
required_reads:
  - AGENTS.md
  - AGENTS.override.md
  - docs/agents/AGENTS.md
  - docs/agents/REPOSITORY_MAP.md
  - docs/agents/CONTEXT_ROUTING.md
  - docs/agents/PROMPTING_STANDARD.md
  - docs/agents/PROMPTING_HANDOVER.md
  - docs/agents/PROMPT_EVAL_STANDARD.md
  - docs/agents/DELIVERY_COMPLETENESS_AND_CLOSEOUT.md
  - docs/agents/ANTI_STALL_AND_EXECUTION_BUDGET.md
  - docs/agents/AUTONOMOUS_PROGRAM_CONTINUATION.md
  - docs/agents/programs/OTERYN_PORTAL_COMPLETION.md
search_first:
  - open Issues/PRs and branches owning OTERYN_PORTAL_COMPLETION prompt/governance paths
optional_reads:
  - docs/agents/programs/OTERYN_PORTAL_COMPLETION_WORK_ALLOCATION.md
---

# OTERYN-20260815-portal-completion-prompt-hardening

## Goal

Harden the canonical `OTERYN_PORTAL_COMPLETION` execution prompt so it is deterministic, context-efficient and fully aligned with current prompting, anti-stall, validation and closeout governance without weakening repository or authority boundaries.

## Acceptance criteria

- [x] Replace the prompt-local shortened terminal response with delegation to the canonical anti-stall terminal response.
- [x] Separate normal fresh independent validation routing from the one-owner remediation self-review model.
- [x] Make selector traversal ordered and short-circuit after the first canonical `READY` candidate while still classifying all siblings in the current mixed entry.
- [x] Preserve mandatory startup reads and move selected-slice context to just-in-time retrieval.
- [x] Separate coordinator execution mode from selected-slice execution mode.
- [x] Reconcile entry-slice wording with the anti-stall allowance for at most one additional task.
- [x] Externalize focused prompt-evaluation cases under `docs/agents/evals/` and state model-trial limitations explicitly.
- [x] Preserve Platform-only, non-production, non-protected, non-payment, non-external-repository and no-standing-Codex authority.
- [ ] Verify exact branch diff contains only declared governance paths.
- [ ] Complete exact-head self-review and applicable repository CI.
- [ ] Record runtime/browser E2E as `NOT_APPLICABLE` with the concrete docs-only reason.
- [ ] Merge/close Issue #1075 and archive this task when all gates pass.

## Ownership

```yaml
project_lane: oteryn-platform-core
owned_paths:
  - docs/agents/prompts/OTERYN-PORTAL-COMPLETION-EXECUTION-PROMPT.md
  - docs/agents/evals/oteryn-portal-completion-prompt-v1.2.json
  - docs/agents/tasks/active/OTERYN-20260815-portal-completion-prompt-hardening.md
modules:
  - agent-governance
  - portal-completion-programme
dependencies:
  - Issue #1075
  - docs/agents/programs/OTERYN_PORTAL_COMPLETION.md
blockers:
  - none
cross_repository_tasks:
  - none
```

## Context checkpoint

```yaml
checkpoint_version: 1
policy_version: 2
updated_at: 2026-08-15T06:55:00Z
head: 5000f271db49215c93432b78397dd3560b49e7e7
branch: docs/issue-1075-portal-completion-prompt-hardening
pr: none
status: implementing
phase: implement
execution_mode: chat
execution_reason: narrow governance/documentation edit through GitHub connector; no owner-funded AI invocation required
project_lane: oteryn-platform-core
context_routes:
  - agent-governance
owned_paths:
  - docs/agents/prompts/OTERYN-PORTAL-COMPLETION-EXECUTION-PROMPT.md
  - docs/agents/evals/oteryn-portal-completion-prompt-v1.2.json
  - docs/agents/tasks/active/OTERYN-20260815-portal-completion-prompt-hardening.md
proven:
  - protected main at task start is 5000f271db49215c93432b78397dd3560b49e7e7
  - Issue #1075 owns this bounded prompt-hardening task
  - no open Issue or matching branch was found owning the canonical portal-completion execution prompt
  - prompt version 1.1 duplicated a shorter terminal response than the controlling anti-stall contract
  - current governance requires mandatory startup context plus just-in-time task-specific expansion
  - current autonomous programme contract permits at most one additional task after a terminal entry task under explicit anti-stall conditions
derived:
  - prompt version 1.2 can remove ambiguity and context waste without changing the canonical programme queue
unknown:
  - exact required CI result on the final PR head
conflicts: []
first_failure:
  marker: none
  evidence: none
rejected_hypotheses:
  - rewriting the programme queue inside the worker prompt
  - removing PROJECT_STATE.md or BUILD_TEST_MATRIX.md from mandatory startup despite CONTEXT_ROUTING.md
changed_paths:
  - docs/agents/prompts/OTERYN-PORTAL-COMPLETION-EXECUTION-PROMPT.md
  - docs/agents/evals/oteryn-portal-completion-prompt-v1.2.json
  - docs/agents/tasks/active/OTERYN-20260815-portal-completion-prompt-hardening.md
validation:
  - command: focused static specification review against current governance
    result: PASS
    evidence: dedicated v1.2 eval artifact records zero identified safety regressions and explicit model-trial limitations
  - command: runtime/browser E2E
    result: NOT_APPLICABLE
    evidence: task changes only agent-governance Markdown/JSON and has no executable browser/runtime behavior
blockers:
  - none
next_action: persist the coherent three-file prompt-hardening commit, open the authoritative PR, and verify exact-head governance CI
```

## Anti-stall state

```yaml
invocation_started_at: 2026-08-15T06:52:51Z
last_progress_at: 2026-08-15T06:55:00Z
ci_checks_for_current_head: 0
ci_check_generation: draft
terminal_ci_wait_started_at: null
terminal_ci_checks_for_current_generation: 0
unchanged_state_checks: 0
identical_failure_retries: 0
repair_cycles_for_current_gate: 0
context_reconstruction_attempts: 0
stall_warnings: 0
```

## Source branch closeout

```yaml
source_branch_disposition: pending
source_branch_reason: task is active and awaiting exact-head validation/merge
source_branch_evidence: pending
```

## Notes

The focused eval is intentionally static/deterministic plus manual specification review. It must not be presented as repeated LLM/model-trial evidence. No Codex, OpenAI API or other owner-funded AI service is authorized or used for this task.
