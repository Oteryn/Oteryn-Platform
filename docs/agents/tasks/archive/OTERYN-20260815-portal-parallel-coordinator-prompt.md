---
task_id: OTERYN-20260815-portal-parallel-coordinator-prompt
project_lane: oteryn-platform-core
implementation_authorized: true
required_reads:
  - AGENTS.md
  - AGENTS.override.md
  - docs/agents/AGENTS.md
  - docs/agents/PROMPT_EVAL_STANDARD.md
  - docs/agents/DELIVERY_COMPLETENESS_AND_CLOSEOUT.md
search_first:
  - Issue #1082
  - PR #1083
optional_reads: []
---

# OTERYN-20260815-portal-parallel-coordinator-prompt

## Goal

Add a standalone repository-owned prompt and executable dedicated evaluation for a parallel `OTERYN_PORTAL_COMPLETION` coordinator/auditor/integrator without modifying the canonical portal execution prompt or shared prompt-eval ownership.

## Terminal outcome

The task is complete.

Delivered on protected `main` by PR #1083:

- `docs/agents/prompts/OTERYN-PORTAL-COMPLETION-PARALLEL-COORDINATOR-PROMPT.md`;
- `docs/agents/evals/oteryn-portal-parallel-coordinator-prompt-v1.json`;
- `.github/workflows/parallel-coordinator-prompt-eval.yml`.

The final prompt enforces independent-task-only parallelism, one Issue/task/branch/PR per lane, non-overlapping ownership, canonical selector precedence, candidate handoff with allowed checkpoint `status: ready`, coordinator audit/takeover after worker writes stop, dependency-safe integration, terminal closeout, and unchanged external/production/owner-funded-AI authority boundaries.

## Acceptance criteria

- [x] Parallel execution is limited to genuinely independent, path-disjoint tasks.
- [x] Existing ownership is reused rather than duplicated.
- [x] Candidate handoff uses checkpoint `status: ready` and separate `HANDOFF_STATE: CANDIDATE_READY_FOR_AUDIT`.
- [x] Coordinator audit verifies exact outcome rather than trusting worker summaries.
- [x] Canonical selector ordering remains authoritative.
- [x] External/server repositories, production/protected environments, credentials, signing, payments and owner-funded AI remain unauthorized absent separate permission.
- [x] Prompt contract is versioned and has a schema-valid deterministic evaluation inventory.
- [x] Dedicated prompt-eval workflow executes the suite through the repository evaluator.
- [x] Canonical prompt/shared eval were not authored by this task; later bytes were inherited only from synchronized protected `main`.
- [x] Both material review findings on PR #1083 were repaired and both threads resolved.
- [x] Required exact-head validation passed on final implementation head.
- [x] Runtime/browser E2E is `NOT_APPLICABLE` because no product runtime/user path changed.
- [x] PR #1083 merged, Issue #1082 closed `completed`, implementation source branch was auto-deleted, and ownership is released by this archive.

## Ownership

```yaml
owned_paths:
  - docs/agents/prompts/OTERYN-PORTAL-COMPLETION-PARALLEL-COORDINATOR-PROMPT.md
  - docs/agents/evals/oteryn-portal-parallel-coordinator-prompt-v1.json
  - .github/workflows/parallel-coordinator-prompt-eval.yml
  - docs/agents/tasks/archive/OTERYN-20260815-portal-parallel-coordinator-prompt.md
modules:
  - agent-governance
  - portal-completion
dependencies:
  - none
blockers:
  - none
cross_repository_tasks:
  - none
```

## Context checkpoint

```yaml
checkpoint_version: 1
policy_version: 2
updated_at: 2026-08-15T07:45:30Z
head: 5c01b24a76058311138868ab68eb3062b53744a9
branch: docs/portal-parallel-coordinator-prompt
pr: 1083
status: completed
phase: close
project_lane: oteryn-platform-core
session_id: chatgpt-20260815-portal-parallel-prompt
session_role: implementation_owner
execution_mode: chat_github
execution_reason: terminal archive after verified implementation merge
context_routes:
  - agent-governance
  - portal-completion
context_pressure: medium
context_growth: stable
decomposition_decision: single
validation_level: full
owned_paths:
  - docs/agents/tasks/archive/OTERYN-20260815-portal-parallel-coordinator-prompt.md
proven:
  - PR #1083 final source head 5c01b24a76058311138868ab68eb3062b53744a9 squash-merged as 939fc95b8b86959371aad372ad17a2bff2c4da66
  - Issue #1082 is closed with state_reason completed
  - final Parallel Coordinator Prompt Eval run 31872607124 completed SUCCESS
  - final Agent Governance run 31872607135 completed SUCCESS
  - final CI run 31872607134 completed SUCCESS
  - final Phase 7 run 31872607162, Platform DB Outage run 31872607122, Game Auth Ticket Concurrency run 31872607140 and Edge Security Emulation run 31872607120 all completed SUCCESS
  - both material review threads PRRT_kwDOTcsYjs6Ze-4h and PRRT_kwDOTcsYjs6Ze-4i are resolved
  - repository metadata proves delete_branch_on_merge=true
  - exact Git ref lookup for heads/docs/portal-parallel-coordinator-prompt returns 404 after merge, proving implementation source branch removal
  - protected main contains merge 939fc95b8b86959371aad372ad17a2bff2c4da66
  - runtime/browser E2E is NOT_APPLICABLE because the task changes agent-governance prompt/eval/workflow behavior only, not executable product behavior
derived:
  - implementation ownership is fully released once this archive PR merges and the active task record is removed
unknown: []
conflicts: []
first_failure:
  marker: none
  evidence: terminal state has no unresolved failure
rejected_hypotheses:
  - merge alone is sufficient without task archival and branch verification
changed_paths:
  - docs/agents/tasks/active/OTERYN-20260815-portal-parallel-coordinator-prompt.md
  - docs/agents/tasks/archive/OTERYN-20260815-portal-parallel-coordinator-prompt.md
validation:
  - command: final implementation exact-head checks
    result: PASS
    evidence: runs 31872607124 / 31872607135 / 31872607134 and additionally triggered runs all SUCCESS on 5c01b24a76058311138868ab68eb3062b53744a9
  - command: PR review hygiene
    result: PASS
    evidence: both material review threads resolved before merge
  - command: Issue closeout
    result: PASS
    evidence: Issue #1082 closed completed at implementation merge
  - command: implementation source branch closeout
    result: PASS
    evidence: exact Git ref lookup returns 404 after merge; delete_branch_on_merge=true
  - command: runtime/browser E2E
    result: NOT_APPLICABLE
    evidence: no product runtime/user journey changed
blockers:
  - none
next_action: none
```

## Self-review

```yaml
self_review:
  result: PASS
  exact_head: 5c01b24a76058311138868ab68eb3062b53744a9
  acceptance_checked: true
  full_diff_checked: true
  negative_paths_checked: true
  rollback_checked: true
  compatibility_checked: true
  related_prs_checked: true
  findings: []
  evidence:
    - PR #1083 review repairs and resolved threads
    - final exact-head validation runs listed above
    - merge commit 939fc95b8b86959371aad372ad17a2bff2c4da66
```

## Source branch closeout

```yaml
source_branch_disposition: auto_delete_after_merge
source_branch_reason: the dedicated implementation branch has no continuing ownership or recovery purpose after PR #1083 merged
source_branch_evidence: repository delete_branch_on_merge=true and exact Git ref lookup for heads/docs/portal-parallel-coordinator-prompt returns 404 after merge; immutable PR #1083 and this archive retain provenance
```

## Closeout

This archive move removes the stale active record and releases the task's remaining documentation ownership. No production, protected-environment, external-repository, credential, signing, payment or owner-funded AI operation is performed by archive closeout.
