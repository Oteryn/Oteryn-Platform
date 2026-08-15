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
- [x] Separate coordinator execution mode from selected-slice execution mode while preserving the standard `execution_mode` field.
- [x] Reconcile entry-slice wording with the anti-stall allowance for at most one additional task.
- [x] Put focused v1.2 regression cases in the repository's canonical deterministic eval suite so required CI actually executes them.
- [x] Preserve connector-first routing and false-GitHub-blocker protections.
- [x] Preserve Platform-only, non-production, non-protected, non-payment, non-external-repository and no-standing-Codex authority.
- [x] Record runtime/browser E2E as `NOT_APPLICABLE` with the concrete docs-only reason.
- [ ] Verify the repaired exact branch diff and deterministic eval on the new exact head.
- [ ] Complete final exact-head self-review and required GitHub CI with zero unresolved review findings.
- [ ] Merge/close Issue #1075 and archive this task when all gates pass.

## Ownership

```yaml
project_lane: oteryn-platform-core
owned_paths:
  - docs/agents/prompts/OTERYN-PORTAL-COMPLETION-EXECUTION-PROMPT.md
  - docs/agents/evals/prompt-contract-v1.json
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
updated_at: 2026-08-15T07:06:14Z
head: LIVE_PR_HEAD
branch: docs/issue-1075-portal-completion-prompt-hardening
pr: 1076
status: implementing
phase: repair_review_findings
execution_mode: chat
execution_reason: bounded agent-governance repair through GitHub connector; no owner-funded AI invocation required
project_lane: oteryn-platform-core
context_routes:
  - agent-governance
owned_paths:
  - docs/agents/prompts/OTERYN-PORTAL-COMPLETION-EXECUTION-PROMPT.md
  - docs/agents/evals/prompt-contract-v1.json
  - docs/agents/tasks/active/OTERYN-20260815-portal-completion-prompt-hardening.md
proven:
  - protected main at task start remained 5000f271db49215c93432b78397dd3560b49e7e7 through the first readiness attempt
  - Issue #1075 and PR #1076 own this bounded prompt-hardening task
  - prompt v1.2 keeps OTERYN_PORTAL_COMPLETION.md as the sole selector authority and short-circuits only after all siblings in the current mixed entry are classified
  - prompt v1.2 preserves the standard execution_mode field and resolves selected-slice execution mode only after canonical selection
  - prompt v1.2 preserves connector-first, Platform-only, production/protected/payment/external-repository and owner-funded-AI boundaries
  - runtime/browser E2E is NOT_APPLICABLE because only agent-governance text/JSON changes
  - first exact-head self-review on 9071e6288e8fee3e9f672b34ec9d0ac416bef361 found no prompt-content defects and Agent Governance/CI passed there
  - later PR review identified two material P2 eval-integration findings despite green CI
  - tools/validation/prompt_eval.py requires mode deterministic_text_contract, eval_policy, required_categories, and per-case source/must_contain schema
  - required Agent Governance and CI invoke tools/validation/prompt_eval.py with its default docs/agents/evals/prompt-contract-v1.json suite
  - therefore a separate unregistered focused suite would not prove the prompt contract in required CI
derived:
  - the smallest coherent repair is to add portal-v1.2 cases to the already-required canonical prompt-contract-v1.json suite and remove the unregistered focused suite rather than changing validator/workflow infrastructure
unknown:
  - repaired exact-head deterministic evaluator and required CI result
conflicts: []
first_failure:
  marker: PR #1076 review thread PRRT_kwDOTcsYjs6Ze3O1
  evidence: focused eval suite was not compatible with tools/validation/prompt_eval.py schema
rejected_hypotheses:
  - keep the separate focused suite and rely on its manual PASS claim
  - change prompt_eval.py/workflows solely to support a second suite when the canonical required suite can own the focused cases directly
  - weaken or resolve review findings without executable deterministic coverage
changed_paths:
  - docs/agents/prompts/OTERYN-PORTAL-COMPLETION-EXECUTION-PROMPT.md
  - docs/agents/evals/prompt-contract-v1.json
  - docs/agents/tasks/active/OTERYN-20260815-portal-completion-prompt-hardening.md
validation:
  - command: prior focused static prompt review
    result: PASS
    evidence: prompt content invariants remain accepted; review defects concern eval schema/integration rather than the selector/authority design
  - command: runtime/browser E2E
    result: NOT_APPLICABLE
    evidence: governance Markdown/JSON only; no executable browser/runtime journey
  - command: repaired deterministic prompt contract suite
    result: NOT_RUN
    evidence: run required on repaired exact head after this coherent repair commit
blockers:
  - none
next_action: persist the coherent repair that points prompt v1.2 at the canonical required eval suite, adds focused portal cases to that suite, deletes the unregistered suite, then run exact-head review/CI and resolve both PR findings only after evidence passes
```

## Review findings

```yaml
findings:
  - id: PRRT_kwDOTcsYjs6Ze3O1
    severity: P2
    status: repairing
    summary: separate focused eval suite used an unsupported schema and could not pass prompt_eval.py
    repair: move portal-v1.2 cases into schema-valid canonical docs/agents/evals/prompt-contract-v1.json
  - id: PRRT_kwDOTcsYjs6Ze3O3
    severity: P2
    status: repairing
    summary: required CI did not invoke the separate focused suite
    repair: use the canonical default suite already executed by both Agent Governance and CI rather than adding redundant workflow plumbing
```

## Anti-stall state

```yaml
invocation_started_at: 2026-08-15T06:52:51Z
last_progress_at: 2026-08-15T07:06:14Z
ci_checks_for_current_head: 0
ci_check_generation: repair
terminal_ci_wait_started_at: null
terminal_ci_checks_for_current_generation: 0
unchanged_state_checks: 0
identical_failure_retries: 0
repair_cycles_for_current_gate: 1
context_reconstruction_attempts: 0
stall_warnings: 0
```

## Recovery checkpoint

```yaml
recovery:
  policy_version: 1
  generation: 2
  session_id: portal-prompt-hardening-20260815T065251Z
  session_started_at: 2026-08-15T06:52:51Z
  checkpointed_at: 2026-08-15T07:06:14Z
  last_progress_at: 2026-08-15T07:06:14Z
  phase: repair_review_findings
  exact_head: LIVE_PR_HEAD
  pull_request: 1076
  active_operation: coherent repository repair
  external_run_ids: []
  operation_started_at: 2026-08-15T07:06:14Z
  wait_deadline_at: null
  check_generation: repair
  checks_used: 0
  status: active
  safe_to_resume: true
  resume_condition: repaired branch head exists with canonical eval-suite integration
  next_action: Verify the repaired exact head, inspect required CI and review threads, and resolve findings only after deterministic evidence passes.
```

## Source branch closeout

```yaml
source_branch_disposition: pending
source_branch_reason: task is active while review findings are repaired
source_branch_evidence: pending
```

## Notes

No Codex, OpenAI API or other owner-funded AI service was invoked. The deterministic prompt evaluator is repository tooling and does not execute an LLM.
