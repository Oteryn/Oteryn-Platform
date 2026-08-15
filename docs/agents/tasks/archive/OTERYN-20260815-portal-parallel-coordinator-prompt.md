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
  - PR #1087
optional_reads: []
---

# OTERYN-20260815-portal-parallel-coordinator-prompt

## Goal

Add a standalone repository-owned prompt and executable dedicated evaluation for a parallel `OTERYN_PORTAL_COMPLETION` coordinator/auditor/integrator without modifying the canonical portal execution prompt or shared prompt-eval ownership.

## Terminal outcome

The implementation outcome is merged and accepted. This lifecycle archive becomes terminal only after PR #1087 merges and its closeout branch is verified deleted or explicitly reconciled.

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
- [x] PR #1083 merged, Issue #1082 closed `completed`, and the implementation source branch was auto-deleted.
- [ ] Archive PR #1087 passes exact-final-head self-review and CI, merges, and its archive-closeout source branch is verified absent or explicitly reconciled.

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
  - archive PR #1087 terminal closeout
blockers:
  - none
cross_repository_tasks:
  - none
```

## Context checkpoint

```yaml
checkpoint_version: 1
policy_version: 2
updated_at: 2026-08-15T07:54:30Z
head: 71d8c4fda1baeeec425930b56697dfe57c2f6c41
branch: docs/archive-1082-parallel-coordinator-prompt
pr: 1087
status: completed
phase: close
project_lane: oteryn-platform-core
session_id: chatgpt-20260815-portal-parallel-prompt
session_role: implementation_owner
execution_mode: chat_github
execution_reason: lifecycle archive after verified implementation merge
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
  - exact Git ref lookup for heads/docs/portal-parallel-coordinator-prompt returns 404 after implementation merge
  - protected main contains implementation merge 939fc95b8b86959371aad372ad17a2bff2c4da66
  - runtime/browser E2E is NOT_APPLICABLE because the task changes agent-governance prompt/eval/workflow behavior only, not executable product behavior
  - PR #1087 review identified three closeout defects and the repair now keeps a concrete next_action, separates implementation review from archive review, and explicitly tracks the archive closeout branch
  - archive exact-head self-review for head 71d8c4fda1baeeec425930b56697dfe57c2f6c41 is retained as PR #1087 review 4943299858, but this source-closeout schema repair requires a fresh self-review on its resulting final head
derived:
  - implementation ownership is released; archive closeout remains responsible only for moving the task record and verifying its own lifecycle branch
unknown:
  - exact final archive PR #1087 head after this source-closeout schema repair
  - archive-closeout branch deletion result until after PR #1087 merges
conflicts: []
first_failure:
  marker: archive-closeout-review-and-governance-findings
  evidence: initial PR #1087 review found invalid placeholder next_action, wrong self-review head and missing archive-branch tracking; first repair then exposed nested source-branch closeout fields rejected by Agent Governance; this repair uses the required top-level scalar schema
rejected_hypotheses:
  - merge alone is sufficient without task archival and branch verification
  - implementation-head self-review can certify the archive diff
  - nested source-branch closeout YAML is accepted by the validator
changed_paths:
  - docs/agents/tasks/active/OTERYN-20260815-portal-parallel-coordinator-prompt.md
  - docs/agents/tasks/archive/OTERYN-20260815-portal-parallel-coordinator-prompt.md
validation:
  - command: final implementation exact-head checks
    result: PASS
    evidence: runs 31872607124 / 31872607135 / 31872607134 and additionally triggered runs all SUCCESS on 5c01b24a76058311138868ab68eb3062b53744a9
  - command: PR #1083 review hygiene
    result: PASS
    evidence: both material implementation review threads resolved before merge
  - command: Issue closeout
    result: PASS
    evidence: Issue #1082 closed completed at implementation merge
  - command: implementation source branch closeout
    result: PASS
    evidence: exact Git ref lookup returns 404 after merge; delete_branch_on_merge=true
  - command: archive PR #1087 initial CI
    result: PASS
    evidence: Agent Governance 31872842300 and CI 31872842322 passed on initial archive head 4c324dd2b26a495aab9bc49499358cb9b221b658 before review repair
  - command: archive PR #1087 first review-repair Agent Governance
    result: FAIL
    evidence: run 31873042124 rejected nested source-branch closeout fields; exact failure repaired in this commit
  - command: runtime/browser E2E
    result: NOT_APPLICABLE
    evidence: archive closeout changes no product runtime/user journey
blockers:
  - none
next_action: verify this repaired exact final head of archive PR #1087, record a fresh exact-head self-review in the PR, resolve all review threads, merge after required CI passes, then verify heads/docs/archive-1082-parallel-coordinator-prompt is absent or reconcile it through Branch Lifecycle and record terminal evidence on PR #1087 / Issue #1082
```

## Implementation self-review evidence

The implementation self-review belongs to PR #1083 and exact implementation head `5c01b24a76058311138868ab68eb3062b53744a9`. It is retained as implementation evidence only and does **not** certify the archive diff.

## Archive PR self-review requirement

Before PR #1087 merges, perform a fresh full-diff self-review on its exact final archive head and record that review durably in the PR conversation. The archive PR may not merge if that exact-head review or required CI is missing or non-terminal.

## Source branch closeout

```yaml
source_branch_disposition: auto_delete_after_merge
source_branch_reason: implementation branch docs/portal-parallel-coordinator-prompt was auto-deleted after merged PR #1083; lifecycle branch docs/archive-1082-parallel-coordinator-prompt has no retention purpose and must auto-delete after PR #1087 merges
source_branch_evidence: implementation ref lookup is 404 and repository delete_branch_on_merge=true; after PR #1087 merges the archive ref must also return 404 or be explicitly reconciled through Branch Lifecycle, with terminal evidence recorded in PR #1087 and Issue #1082
```

## Closeout

This archive move removes the stale active record and releases the task's remaining documentation ownership. After archive merge, verify the archive branch is absent. If it is not absent, reconcile the exact ref through the repository Branch Lifecycle mechanism before reporting `DONE`. Record the post-merge branch result durably in PR #1087 and/or Issue #1082.

No production, protected-environment, external-repository, credential, signing, payment or owner-funded AI operation is performed by archive closeout.
