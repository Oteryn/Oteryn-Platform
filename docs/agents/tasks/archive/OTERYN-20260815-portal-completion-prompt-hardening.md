---
task_id: OTERYN-20260815-portal-completion-prompt-hardening
issue: 1075
status: completed
programme: OTERYN_PORTAL_COMPLETION
project_lane: oteryn-platform-core
phase: closeout
execution_mode: github_connector
---

# OTERYN-20260815-portal-completion-prompt-hardening

## Goal

Harden the canonical `OTERYN_PORTAL_COMPLETION` execution prompt so it is deterministic, context-efficient and aligned with current prompting, anti-stall, validation and closeout governance without weakening repository or authority boundaries.

## Final result

- Canonical execution prompt advanced to prompt contract `1.2` at `docs/agents/prompts/OTERYN-PORTAL-COMPLETION-EXECUTION-PROMPT.md`.
- `docs/agents/programs/OTERYN_PORTAL_COMPLETION.md` remains the sole selector authority; the worker prompt contains no duplicate queue.
- Selector traversal is ordered and short-circuits only after every sibling in the current mixed entry has been classified.
- Mandatory startup context is preserved while selected-slice architecture/contracts/code are loaded just in time.
- Standard coordinator `execution_mode: chat` is preserved and the selected-slice execution mode resolves only after canonical selection.
- Normal material/user-facing validation is distinguished from the one-owner remediation self-review model.
- Terminal reporting delegates to the canonical anti-stall response rather than maintaining a weaker duplicate.
- Focused portal-v1.2 regression cases are integrated into canonical `docs/agents/evals/prompt-contract-v1.json`, which required CI executes.
- Connector-first, Platform-only, production/protected/payment/external-repository and owner-funded-AI restrictions remain explicit.
- Runtime/browser E2E is `NOT_APPLICABLE`: this delivery changes agent-governance Markdown/JSON only.

## Review and repair evidence

Two material P2 findings identified that the first focused eval artifact used an unsupported schema and was not executed by required CI. The separate artifact was removed, focused portal cases were moved into the canonical schema-valid suite, and both findings were resolved only after required validation passed.

```yaml
review_threads:
  - PRRT_kwDOTcsYjs6Ze3O1: RESOLVED
  - PRRT_kwDOTcsYjs6Ze3O3: RESOLVED
repair_head: ba8487ef6cd3e5ca08d93c686c8f4cee889b34f5
repair_validation:
  agent_governance_run: 31871134618
  ci_run: 31871134611
  result: PASS
```

## Exact implementation evidence

```yaml
implementation_issue: 1075
implementation_pr: 1076
implementation_final_head: 83e15c2d0b22c120976f14b7f7aeed1ad7d4db5b
implementation_merge_commit: 841793be68d4c52aff14bfc51d6f291a24b74477
issue_state: CLOSED_COMPLETED
final_exact_head_ci:
  agent_governance_run: 31872089772
  ci_run: 31872089774
  result: PASS
full_diff_self_review:
  pr_comment: 5301156573
  current_base_gate_comment: 5301173760
  result: PASS
review_threads: RESOLVED
runtime_browser_e2e: NOT_APPLICABLE
changed_paths:
  - docs/agents/prompts/OTERYN-PORTAL-COMPLETION-EXECUTION-PROMPT.md
  - docs/agents/evals/prompt-contract-v1.json
  - docs/agents/tasks/active/OTERYN-20260815-portal-completion-prompt-hardening.md
```

## Source branch closeout

```yaml
source_branch_disposition: auto_delete_after_merge
source_branch_reason: the dedicated implementation branch had no continuing ownership or recovery purpose after PR #1076 merged; this archive closeout branch is likewise lifecycle-only and must not be retained after merge
source_branch_evidence: live Git ref lookup after PR #1076 merge returned 404 for `refs/heads/docs/issue-1075-portal-completion-prompt-hardening`, proving the implementation source ref is absent; repository branch lifecycle policy requires the archive closeout branch to auto-delete after its own merge or be reconciled explicitly
```

## Closeout

PR #1076 was squash-merged only after exact current-base Agent Governance and CI passed on unchanged head `83e15c2d0b22c120976f14b7f7aeed1ad7d4db5b`, all review threads were resolved, and the PR diff was verified to contain exactly the three declared governance paths. Issue #1075 closed as completed. This archive move removes the task from `docs/agents/tasks/active/`, releases prompt/task path ownership, and preserves durable implementation, validation, review and branch-lifecycle evidence. No production, protected-environment, external-repository, payment, secret or owner-funded AI operation was performed.
