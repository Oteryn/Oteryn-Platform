---
task_id: OTERYN-20260810-portal-completion-programme
required_reads:
  - AGENTS.md
  - AGENTS.override.md
  - docs/agents/AGENTS.md
  - docs/agents/EXECUTION_PROTOCOL.md
  - docs/agents/CONTEXT_HANDOFF.md
  - docs/agents/DELIVERY_COMPLETENESS_AND_CLOSEOUT.md
  - docs/agents/ANTI_STALL_AND_EXECUTION_BUDGET.md
  - docs/agents/GITHUB_ONLY_EXECUTION.md
  - docs/agents/PROMPTING_HANDOVER.md
  - docs/agents/PROMPTING_STANDARD.md
  - docs/agents/PROMPT_EVAL_STANDARD.md
search_first:
  - current main and open PRs
  - docs/agents/tasks/active/**
  - PORTAL-CLOSEOUT / portal completion overlap
optional_reads:
  - docs/architecture/PORTAL_COMPLETENESS_ARCHITECTURE.md
  - docs/architecture/PLAYER_COMPANION_ARCHITECTURE.md
---

# OTERYN-20260810-portal-completion-programme

## Goal

Persist the 2026-08-10 portal architecture/product review, risk-first delivery plan, durable completion programme and refreshed execution prompt, then register the short alias `PORTAL-CLOSEOUT` without changing runtime, production, payment or external-repository state.

## Acceptance criteria

- [ ] The dated portal review is persisted and refreshed against live `main`, current open PRs and current P1 findings.
- [ ] A canonical portal-completion delivery plan is persisted without superseding accepted ADRs/contracts.
- [ ] A durable programme state and short-command alias `PORTAL-CLOSEOUT` are registered.
- [ ] The execution prompt conforms to `PROMPTING_STANDARD.md`, includes connector-first GitHub routing, does not infer access failure from local `git`/`gh`, and preserves Platform-only authority.
- [ ] The material prompt change records a prompt contract plus a documented positive/negative/boundary manual scenario matrix without claiming automated model-trial evidence.
- [ ] The repository map links the delivery plan and programme.
- [ ] Exact-head self-review confirms only the declared documentation/governance paths changed and no live/runtime authority was expanded.
- [ ] Runtime/browser E2E is recorded `NOT_APPLICABLE` because this task changes documentation/governance/prompt routing only.
- [ ] Repository-required exact-head CI passes, related PR state is intentional, the PR is merged when gates permit, and this task record is archived with ownership released.

## Ownership

```yaml
project_lane: oteryn-platform-core
owned_paths:
  - docs/agents/REPOSITORY_MAP.md
  - docs/agents/SHORT_PROGRAM_INVOCATIONS.md
  - docs/agents/programs/OTERYN_PORTAL_COMPLETION.md
  - docs/agents/prompts/OTERYN-PORTAL-COMPLETION-EXECUTION-PROMPT.md
  - docs/agents/reports/OTERYN-20260810-portal-architecture-product-review.md
  - docs/architecture/PORTAL_COMPLETION_DELIVERY_PLAN.md
  - docs/agents/tasks/active/OTERYN-20260810-portal-completion-programme.md
modules:
  - agent-governance
  - architecture
  - portal-completeness
dependencies:
  - none
blockers:
  - none
cross_repository_tasks:
  - none
```

## Feature scope

```yaml
feature_scope:
  type: infrastructure
  user_facing: false
  backend_required: false
  frontend_required: false
  integration_required: false
  e2e_required: false
  completion_claim: internal_only
```

## Context checkpoint

```yaml
checkpoint_version: 1
policy_version: 2
updated_at: 2026-08-10T18:41:22Z
head: dc9adc7d9246e83c7299d8cf9c161524fb85b2c9
branch: docs/OTERYN-20260810-portal-completion-programme
pr: none
status: implementing
phase: persist_documentation_package
session_id: chat-20260810-portal-completion
session_role: implementer
execution_mode: chat
execution_reason: GitHub connector supports exact repository reads/writes and documentation-only change does not require a local checkout
context_routes:
  - agent-governance
  - architecture
owned_paths:
  - docs/agents/REPOSITORY_MAP.md
  - docs/agents/SHORT_PROGRAM_INVOCATIONS.md
  - docs/agents/programs/OTERYN_PORTAL_COMPLETION.md
  - docs/agents/prompts/OTERYN-PORTAL-COMPLETION-EXECUTION-PROMPT.md
  - docs/agents/reports/OTERYN-20260810-portal-architecture-product-review.md
  - docs/architecture/PORTAL_COMPLETION_DELIVERY_PLAN.md
  - docs/agents/tasks/active/OTERYN-20260810-portal-completion-programme.md
proven:
  - Protected main was re-read immediately before mutation at dc9adc7d9246e83c7299d8cf9c161524fb85b2c9.
  - Current open PR search returns only #541 and #338; no PORTAL-CLOSEOUT or portal-completion PR exists.
  - Current active task files are public-domain repair and native-auth production verification; neither owns this task's documentation paths.
  - Issues #941, #944, #948 and #490 remain open in the live refresh.
  - GitHub connector exposes branch, file/blob/tree/commit, PR and merge operations; local git/gh availability is not a blocker.
derived:
  - The persisted 2026-08-10 report must remove the stale claim that dependency PRs #952-#958 remain open.
  - The dedicated prompt must use current Prompting Standard enum values and connector-first routing.
unknown:
  - Exact repository-required check set for the new PR until GitHub creates the PR check generation.
conflicts:
  - ACTIVE_WORK.md says no active tasks while live docs/agents/tasks/active contains two blocked records; the review records this source-of-truth drift but this registration task does not repair their separate lifecycle.
first_failure:
  marker: none
  evidence: none
rejected_hypotheses:
  - GitHub write access is unavailable; disproven by the connected write operation surface.
changed_paths: []
validation:
  - command: manual prompt-structure and Markdown checks on prepared package
    result: NOT_RUN
    evidence: run after coherent package is assembled
  - command: repository-required exact-head GitHub Actions
    result: NOT_RUN
    evidence: PR not yet created
  - command: runtime/browser E2E
    result: NOT_APPLICABLE
    evidence: documentation/governance/prompt-routing only; no executable application or integration path changes
blockers:
  - none
next_action: Create the dedicated branch from dc9adc7d9246e83c7299d8cf9c161524fb85b2c9 and persist the coherent documentation package.
```

## Notes

This task registers and validates the durable programme; it does not execute the first portal-remediation/product slice. Existing implementation-authorized security findings remain owned by their Issue-based remediation flow.
