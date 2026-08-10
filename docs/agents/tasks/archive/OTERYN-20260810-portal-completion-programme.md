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
- [x] A durable programme state and short-command alias `PORTAL-CLOSEOUT` are merged to protected `main`.
- [x] The execution prompt conforms to `PROMPTING_STANDARD.md`, includes connector-first GitHub routing, does not infer access failure from local `git`/`gh`, and preserves Platform-only authority.
- [x] The material prompt change records a prompt contract plus a documented baseline/candidate positive/negative/boundary manual scenario matrix without claiming automated model-trial evidence.
- [x] The repository map links the delivery plan and programme.
- [x] Exact-head self-review confirms only the declared documentation/governance paths changed and no live/runtime authority was expanded.
- [x] Runtime/browser E2E is recorded `NOT_APPLICABLE` because this task changes documentation/governance/prompt routing only.
- [x] Repository-required exact-head CI passed and PR #962 merged as squash commit `d1d7159f43c815f39db55cb86f518fc85df8d454`.
- [x] Ownership is released by archiving this record.

## Ownership

```yaml
project_lane: oteryn-platform-core
owned_paths: []
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
updated_at: 2026-08-10T19:03:00Z
head: d1d7159f43c815f39db55cb86f518fc85df8d454
branch: docs/OTERYN-20260810-portal-completion-programme
pr: 962
status: completed
phase: archived
session_id: chat-20260810-portal-completion
session_role: implementer
execution_mode: chat
execution_reason: GitHub connector completed repository writes, exact-head validation, squash merge and lifecycle archival without requiring a local checkout
context_routes:
  - agent-governance
  - architecture
owned_paths: []
proven:
  - Protected main remained dc9adc7d9246e83c7299d8cf9c161524fb85b2c9 through the final pre-merge revalidation of PR #962.
  - PR #962 was the authoritative PR for branch docs/OTERYN-20260810-portal-completion-programme and merged successfully by squash.
  - Protected main now contains squash commit d1d7159f43c815f39db55cb86f518fc85df8d454 with the seven declared documentation/governance paths and no application, schema, workflow, deployment or external-repository mutation.
  - All eight pull-request workflow runs on final PR head bbe12f107051e2aa89dcee11fc74d585bbb6e749 completed successfully, including Agent Governance and CI.
  - PR #962 had no unresolved review threads or submitted review conflicts before merge.
  - Corrected post-merge evidence shows open PRs #961, #541 and #338 had no changed-path overlap with this task at closeout; #961 was separate synthetic/no-network research work with red exact-head validation.
  - Issues #941, #944, #948 and #490 remain separate live work owners and were not duplicated by this registration task.
  - Runtime/browser E2E is not applicable because the merged task diff changes only documentation, governance and prompt routing.
derived:
  - The durable alias PORTAL-CLOSEOUT can now route the next eligible portal-completion slice from live repository state.
unknown: []
conflicts:
  - ACTIVE_WORK.md still says no active tasks while two pre-existing blocked active task records remain; the persisted portal review records this source-of-truth drift for later bounded reconciliation.
  - The first persisted open-PR inventory omitted PR #961 even though GitHub records #961 as created before PR #962; post-merge reconciliation corrected the report and delivery plan rather than preserving the inaccurate inventory.
first_failure:
  marker: none
  evidence: none
rejected_hypotheses:
  - GitHub write access is unavailable; disproven by successful connected repository writes, PR creation, exact-head validation and merge.
  - Dependency-update PRs #952-#958 remain current open work; disproven by the live open-PR refresh used for the persisted report.
  - PR #961 opened only after the portal-completion review; disproven by GitHub creation timestamps during post-merge reconciliation.
changed_paths:
  - docs/agents/REPOSITORY_MAP.md
  - docs/agents/SHORT_PROGRAM_INVOCATIONS.md
  - docs/agents/programs/OTERYN_PORTAL_COMPLETION.md
  - docs/agents/prompts/OTERYN-PORTAL-COMPLETION-EXECUTION-PROMPT.md
  - docs/agents/reports/OTERYN-20260810-portal-architecture-product-review.md
  - docs/architecture/PORTAL_COMPLETION_DELIVERY_PLAN.md
  - docs/agents/tasks/archive/OTERYN-20260810-portal-completion-programme.md
validation:
  - command: exact-head changed-path and full-diff self-review for PR #962
    result: PASS
    evidence: GitHub compare and PR file inspection showed exactly seven declared documentation/governance paths with no runtime, schema, workflow, deployment or external-repository mutation.
  - command: manual baseline/candidate prompt scenario matrix review against PROMPT_EVAL_STANDARD.md
    result: PASS
    evidence: The merged candidate prompt records representative baseline/candidate positive, negative, boundary, stale-state, injection, missing-layer and closeout scenarios and explicitly states that automated model trials were not run.
  - command: runtime/browser E2E
    result: NOT_APPLICABLE
    evidence: The merged task diff is documentation/governance/prompt-routing only and changes no executable application or integration path.
  - command: repository-required exact-head GitHub Actions on PR #962 head bbe12f107051e2aa89dcee11fc74d585bbb6e749
    result: PASS
    evidence: Eight pull-request workflow runs completed successfully, including Agent Governance run 31421767423 and CI run 31421767328.
  - command: PR #962 terminal merge verification
    result: PASS
    evidence: GitHub merge returned merged=true with squash commit d1d7159f43c815f39db55cb86f518fc85df8d454 and protected main resolved to that commit immediately after merge.
  - command: post-merge open-PR reconciliation
    result: PASS
    evidence: GitHub proved #961 was created at 2026-08-10T18:45:31Z before #962 at 2026-08-10T18:50:31Z; the persisted review/plan were corrected in the dedicated reconciliation task.
blockers: []
next_action: Invoke PORTAL-CLOSEOUT from live repository state to select or resume the next eligible portal-completion slice.
```

## Notes

The programme registration task is terminal. Future `PORTAL-CLOSEOUT` invocations must re-read live state rather than inheriting this dated task checkpoint.
