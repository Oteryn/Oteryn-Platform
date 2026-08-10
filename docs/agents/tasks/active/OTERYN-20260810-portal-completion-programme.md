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

- [x] The dated portal review is persisted and refreshed against live `main`, current open PRs and current P1 findings.
- [x] A canonical portal-completion delivery plan is persisted without superseding accepted ADRs/contracts.
- [x] A durable programme state and short-command alias `PORTAL-CLOSEOUT` are registered on the task branch.
- [x] The execution prompt conforms to `PROMPTING_STANDARD.md`, includes connector-first GitHub routing, does not infer access failure from local `git`/`gh`, and preserves Platform-only authority.
- [x] The material prompt change records a prompt contract plus a documented baseline/candidate positive/negative/boundary manual scenario matrix without claiming automated model-trial evidence.
- [x] The repository map links the delivery plan and programme.
- [x] Exact-head self-review confirms only the declared documentation/governance paths changed and no live/runtime authority was expanded.
- [x] Runtime/browser E2E is recorded `NOT_APPLICABLE` because this task changes documentation/governance/prompt routing only.
- [ ] PR #962 reaches terminal merge state and this task record is archived with ownership released.

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
updated_at: 2026-08-10T18:59:00Z
head: d4f2a0931ee06f8331db48dbed27e1891863b7c0
branch: docs/OTERYN-20260810-portal-completion-programme
pr: 962
status: ready
terminal_pr_policy: archive_pending
phase: merge_gate
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
  - Protected main was re-read immediately before first mutation at dc9adc7d9246e83c7299d8cf9c161524fb85b2c9.
  - PR #962 is the authoritative draft PR for branch docs/OTERYN-20260810-portal-completion-programme.
  - The branch diff contains exactly the seven declared documentation/governance paths and no application, schema, workflow, deployment or external-repository change.
  - Current unrelated open PRs at the live refresh are #541 and #338; their changed paths do not overlap this task.
  - Issues #941, #944, #948 and #490 remain open in the live refresh and are recorded as live priority/evidence owners rather than duplicated repair tasks.
  - GitHub connector exposes and successfully executed branch, blob/tree/commit, file, PR and comparison operations for this task.
  - The candidate prompt uses current Prompting Standard enum values and states connector-first GitHub routing without expanding repository authority.
  - All eight GitHub Actions workflow runs on implementation head d4f2a0931ee06f8331db48dbed27e1891863b7c0 completed successfully, including Agent Governance, CI, production-like classification, database-outage, edge-security and native-contract checks.
derived:
  - The persisted 2026-08-10 report must treat dependency PRs #952-#958 as historical rather than current open work.
  - Runtime/browser E2E is not applicable because the complete branch diff changes only documentation, governance and prompt routing.
unknown:
  - Exact final repository-required CI outcome for this terminal-transition checkpoint head until GitHub Actions completes.
conflicts:
  - ACTIVE_WORK.md says no active tasks while live docs/agents/tasks/active contains two pre-existing blocked records; the review records this source-of-truth drift but this registration task does not repair their separate lifecycle.
first_failure:
  marker: none
  evidence: none
rejected_hypotheses:
  - GitHub write access is unavailable; disproven by successful connected repository writes and PR #962 creation.
  - Dependency-update PRs #952-#958 remain current open work; disproven by the live open-PR refresh before this branch was created.
changed_paths:
  - docs/agents/REPOSITORY_MAP.md
  - docs/agents/SHORT_PROGRAM_INVOCATIONS.md
  - docs/agents/programs/OTERYN_PORTAL_COMPLETION.md
  - docs/agents/prompts/OTERYN-PORTAL-COMPLETION-EXECUTION-PROMPT.md
  - docs/agents/reports/OTERYN-20260810-portal-architecture-product-review.md
  - docs/architecture/PORTAL_COMPLETION_DELIVERY_PLAN.md
  - docs/agents/tasks/active/OTERYN-20260810-portal-completion-programme.md
validation:
  - command: exact-head changed-path and full-diff self-review for PR #962 implementation head d4f2a0931ee06f8331db48dbed27e1891863b7c0
    result: PASS
    evidence: GitHub compare and PR file inspection show exactly seven declared documentation/governance paths with no runtime, schema, workflow, deployment or external-repository mutation.
  - command: manual baseline/candidate prompt scenario matrix review against PROMPT_EVAL_STANDARD.md
    result: PASS
    evidence: Candidate prompt records the same representative baseline/candidate scenarios, includes positive, negative, boundary, stale-state, injection, missing-layer and closeout cases, and explicitly states that automated model trials were not run.
  - command: runtime/browser E2E
    result: NOT_APPLICABLE
    evidence: The complete task diff is documentation/governance/prompt-routing only and changes no executable application or integration path.
  - command: repository-required GitHub Actions on implementation head d4f2a0931ee06f8331db48dbed27e1891863b7c0
    result: PASS
    evidence: Eight pull-request workflow runs completed successfully, including Agent Governance run 31421586629 and CI run 31421586483.
  - command: repository-required exact-head GitHub Actions on terminal-transition checkpoint head
    result: NOT_RUN
    evidence: This checkpoint transition itself must complete exact-head workflow validation before PR #962 is made terminal.
blockers: []
next_action: Archive this task record under docs/agents/tasks/archive and release ownership once PR #962 is terminal.
```

## Notes

This task registers and validates the durable programme; it does not execute the first portal-remediation/product slice. Existing implementation-authorized security findings remain owned by their Issue-based remediation flow.
