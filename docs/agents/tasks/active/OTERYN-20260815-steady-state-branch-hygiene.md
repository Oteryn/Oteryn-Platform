---
task_id: OTERYN-20260815-steady-state-branch-hygiene
issue: 1089
status: validating
project_lane: oteryn-platform-core
phase: validate
execution_mode: github_connector
task_kind: implementation
required_reads:
  - AGENTS.md
  - AGENTS.override.md
  - docs/agents/AGENTS.md
  - docs/agents/REPOSITORY_MAP.md
  - docs/agents/CONTEXT_ROUTING.md
  - docs/agents/DELIVERY_COMPLETENESS_AND_CLOSEOUT.md
  - docs/agents/ANTI_STALL_AND_EXECUTION_BUDGET.md
  - docs/agents/SESSION_RECOVERY_AND_ORPHANED_EXECUTION.md
  - docs/agents/TERMINAL_ONLY_COMMUNICATION.md
  - docs/agents/GITHUB_ONLY_EXECUTION.md
  - docs/agents/AUTONOMOUS_PROGRAM_CONTINUATION.md
  - docs/agents/EXECUTION_PROTOCOL.md
  - docs/agents/PROJECT_LANES.json
  - docs/architecture/adr/0037-terminal-source-branch-lifecycle.md
  - docs/architecture/adr/0039-historical-work-canonicalization-and-managed-recovery.md
search_first:
  - Issue #1089 and terminal Issue #1072 evidence
  - current live branches, active tasks and open same-repository PRs
  - existing Historical Branch Audit implementation and tests
  - overlapping CI/governance PR ownership
optional_reads:
  - docs/agents/HISTORICAL_WORK_RECONCILIATION_REGISTRY.json
---

# OTERYN-20260815-steady-state-branch-hygiene

## Goal

Extend the existing post-#1072 repository governance so new unexplained remote branch debt is detected continuously without reintroducing a second cleanup programme or unnecessary CI load.

## Acceptance criteria

- [x] Hard steady-state invariant is `NEW_UNEXPLAINED_BRANCHES = 0`; raw branch count remains informational and has no fixed cap.
- [x] Protected `main` is the only ordinary long-lived branch; other ordinary remote refs require a live same-repository PR, active task claim, or managed-recovery contract.
- [x] New active ordinary refs using top-level `tmp`, `backup`, `archive`, `recovery`, or `rollback` fail closed; the terminal #1072 registry is provenance, not a future exemption.
- [x] Multiple open same-repository PRs on one branch or multiple active task claims on one branch fail closed.
- [x] Human/agent naming `<type>/issue-<number>-<slug>` is advisory only; bot/system branches are exempt and naming never authorizes deletion.
- [x] Repository setting drift from `main`, squash-only delivery and `delete_branch_on_merge=true` is reported deterministically and never auto-mutated.
- [x] Existing Historical Branch Audit has a read-only trusted PR-lifecycle event and bounded schedule; destructive historical apply remains trusted-main push only.
- [x] Terminal historical destructive tooling/registry remain unchanged; applied-phase ongoing inventory routes through a separate read-only hygiene helper so the historical blob binding remains meaningful if destructive state is ever reintroduced.
- [x] Focused negative-path tests cover the new invariants.
- [ ] Exact-head current-repository live inventory passes with zero hard findings.
- [ ] Exact-head self-review, required CI, PR hygiene and terminal task/branch closeout are complete.

## Ownership

```yaml
owned_paths:
  - docs/agents/tasks/active/OTERYN-20260815-steady-state-branch-hygiene.md
  - docs/agents/tasks/archive/OTERYN-20260815-steady-state-branch-hygiene.md
  - docs/architecture/adr/0039-historical-work-canonicalization-and-managed-recovery.md
  - .github/workflows/historical-branch-audit.yml
  - tools/agents/branch_hygiene.py
  - tools/agents/test_branch_hygiene.py
modules:
  - repository-governance
  - branch-lifecycle
  - historical-work-reconciliation
dependencies:
  - Issue #1072 terminal historical reconciliation
  - ADR 0037
  - ADR 0039
blockers:
  - none
cross_repository_tasks:
  - none
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-08-15T08:53:30Z
head: LIVE_PR_1090_HEAD
branch: repair/issue-1089-steady-state-branch-hygiene
pr: 1090
status: validating
context_routes:
  - agent-governance
  - architecture
  - testing
  - ci-repair
owned_paths:
  - docs/agents/tasks/active/OTERYN-20260815-steady-state-branch-hygiene.md
  - docs/agents/tasks/archive/OTERYN-20260815-steady-state-branch-hygiene.md
  - docs/architecture/adr/0039-historical-work-canonicalization-and-managed-recovery.md
  - .github/workflows/historical-branch-audit.yml
  - tools/agents/branch_hygiene.py
  - tools/agents/test_branch_hygiene.py
proven:
  - Issue #1072 is terminal completed; 37 reviewed historical refs are absent and its final approval-free audit reports 11 fully accounted live refs with zero unexplained refs.
  - Repository settings at task start use default main, squash merge enabled, merge/rebase merge disabled and delete_branch_on_merge=true.
  - Current Historical Branch Audit before this task was path-triggered plus workflow_dispatch only; it had no scheduled or trusted PR-lifecycle steady-state trigger.
  - PR #1086 explicitly does not own .github/workflows/historical-branch-audit.yml and does not claim this task's helper/ADR paths.
  - The task owns Issue #1089, branch repair/issue-1089-steady-state-branch-hygiene and draft PR #1090.
  - Focused helper/test construction validation passes nine deterministic negative/positive cases in isolated Python execution, including REST-to-GraphQL merge-setting fallback.
derived:
  - The smallest correct design extends the existing Historical Branch Audit rather than creating another governance workflow or programme.
  - Keeping the terminal #1072 destructive script and registry unchanged preserves immutable deletion provenance; a new read-only helper is safer than repurposing the destructive historical implementation.
  - pull_request_target is limited to opened/reopened and checks out trusted main, avoiding transient branch-creation failures and untrusted PR execution; daily schedule catches ownerless refs with no PR.
unknown:
  - Exact required CI results on the coherent implementation head.
  - Whether exact-head self-review or GitHub review reveals an additional material finding.
conflicts: []
first_failure:
  marker: REPOSITORY_SETTING_DRIFT_FROM_REST_FIELD_OMISSION
  evidence: Historical Branch Audit run 31875483710 job 94990473867 produced artifact 9244613114; all four merge-setting fields were null while default_branch was present, and live branch ownership itself had zero hard findings
rejected_hypotheses:
  - impose a fixed maximum raw branch count
  - delete branches by name or age
  - create a second branch-hygiene workflow/programme
  - mutate repository merge settings automatically from the audit
  - modify the terminal historical registry merely to permit future read-only maintenance
changed_paths:
  - .github/workflows/historical-branch-audit.yml
  - docs/agents/tasks/active/OTERYN-20260815-steady-state-branch-hygiene.md
  - docs/architecture/adr/0039-historical-work-canonicalization-and-managed-recovery.md
  - tools/agents/branch_hygiene.py
  - tools/agents/test_branch_hygiene.py
validation:
  - command: live repository and overlap preflight
    result: PASS
    evidence: terminal #1072 state, current live refs, current repo settings and open PR ownership were verified through GitHub before implementation
  - command: exact-head Historical Branch Audit 31875483710
    result: FAIL
    evidence: focused validate job passed; live inventory found 12 accounted refs and zero unexplained refs but REST repository metadata omitted four merge-setting fields, producing false REPOSITORY_SETTING_DRIFT
  - command: isolated Python compile + focused branch-hygiene repair tests
    result: PASS
    evidence: 9 focused tests pass, including deterministic GraphQL fallback when REST omits merge settings
  - command: runtime/browser E2E
    result: NOT_APPLICABLE
    evidence: this is repository GitHub branch/PR/task governance; executable integration evidence is the real live GitHub inventory workflow, not browser behavior
blockers:
  - none
next_action: Push the REST-to-GraphQL merge-setting fallback repair and inspect the new exact-head Historical Branch Audit before any readiness transition.
```

## Recovery checkpoint

```yaml
recovery:
  policy_version: 1
  generation: 3
  session_id: github-20260815-0838
  session_started_at: 2026-08-15T08:33:00Z
  checkpointed_at: 2026-08-15T08:53:30Z
  last_progress_at: 2026-08-15T08:53:30Z
  phase: validate
  exact_head: LIVE_PR_1090_HEAD
  pull_request: 1090
  active_operation: validate REST-to-GraphQL merge-setting fallback after first exact-head failure
  external_run_ids:
    - 31875483710
  operation_started_at: null
  wait_deadline_at: null
  check_generation: draft
  checks_used: 0
  status: active
  safe_to_resume: true
  resume_condition: PR #1090 head remains the dedicated Issue #1089 branch and no ownership conflict appears
  next_action: Inspect exact-head validation and repair only a concrete failing gate.
```

## Source branch closeout

```yaml
source_branch_disposition: pending
source_branch_reason: Issue #1089 is active on repair/issue-1089-steady-state-branch-hygiene and PR #1090
source_branch_evidence: pending
```

## Notes

No production, staging, external-repository, credential or owner-funded AI operation is in scope. Runtime/browser E2E is `NOT_APPLICABLE`; the real integration path is the read-only GitHub branch/PR/task inventory audit.
