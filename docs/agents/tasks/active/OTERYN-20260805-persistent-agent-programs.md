---
task_id: OTERYN-20260805-persistent-agent-programs
required_reads:
  - AGENTS.md
  - AGENTS.override.md
  - docs/agents/AGENTS.md
  - docs/agents/PROMPTING_STANDARD.md
  - docs/agents/PROMPTING_HANDOVER.md
  - docs/agents/AUTONOMOUS_PROGRAM_CONTINUATION.md
  - docs/agents/TRUST_AND_CONTEXT_BOUNDARIES.md
  - docs/agents/END_TO_END_FEATURE_COMPLETENESS.md
  - docs/agents/TASK_CLOSEOUT_AUDIT_E2E.md
  - docs/agents/ANTI_STALL_AND_EXECUTION_BUDGET.md
  - docs/agents/SESSION_RECOVERY_AND_ORPHANED_EXECUTION.md
  - docs/agents/TERMINAL_ONLY_COMMUNICATION.md
search_first:
  - existing audit, remediation, architecture and short-command prompts
  - active tasks and open pull requests touching docs/agents prompts, programme registries or claim ownership
optional_reads:
  - docs/agents/PROMPT_EVAL_STANDARD.md
---

# OTERYN-20260805-persistent-agent-programs

## Goal

Create three durable, non-overlapping Oteryn Platform agent programmes for continuous product audit, audit remediation, and architecture/structure/CI review, plus short owner commands, repository-backed continuation state, classified parallel work queues and collision-safe remediation claims.

## Acceptance criteria

- [x] Three versioned programme prompts exist and follow the current prompting, trust, completeness, audit, E2E, closeout and anti-stall contracts.
- [x] Auditor, remediator and architecture adviser have explicit non-overlapping mutation authority.
- [x] Each programme has a durable state record that allows continuation without chat history.
- [x] One short-command registry maps Polish owner invocations to canonical prompts and state files, including bounded parallel remediation commands.
- [x] Missing modules found by the auditor produce a confirmed Issue and a Proposed documentation/contract PR, while runtime implementation remains owned by remediation.
- [x] Audit-created Issues carry labels and machine-readable workstream, risk, path, dependency, delivery and parallelization metadata.
- [x] Remediation uses a dual lock: globally visible Issue claim plus active task ownership for exact paths, branch, PR and lease.
- [x] Claim races, stale takeover, release and shared-path serialization are defined without relying on chat or assignees.
- [x] Documentation paths and internal references were verified on the branch head and a manual positive/negative/boundary scenario matrix was recorded.

## Ownership

```yaml
owned_paths:
  - docs/agents/prompts/OTERYN_PLATFORM_CONTINUOUS_AUDIT_PROGRAM.md
  - docs/agents/prompts/OTERYN_PLATFORM_REMEDIATION_PROGRAM.md
  - docs/agents/prompts/OTERYN_PLATFORM_ARCHITECTURE_REVIEW_PROGRAM.md
  - docs/agents/programs/OTERYN_PLATFORM_CONTINUOUS_AUDIT.md
  - docs/agents/programs/OTERYN_PLATFORM_REMEDIATION.md
  - docs/agents/programs/OTERYN_PLATFORM_ARCHITECTURE_REVIEW.md
  - docs/agents/SHORT_PROGRAM_INVOCATIONS.md
  - docs/agents/AUDIT_REMEDIATION_ISSUE_TAXONOMY.md
  - docs/agents/REMEDIATION_WORK_CLAIM_PROTOCOL.md
  - docs/agents/evidence/OTERYN-20260805-persistent-agent-programs/prompt-eval.md
  - docs/agents/tasks/active/OTERYN-20260805-persistent-agent-programs.md
modules:
  - agent-governance
  - programme-coordination
  - audit-remediation-routing
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
updated_at: 2026-08-05T13:40:00Z
head: efa37a5bdf76539d8f92cda5f7db3d5d63e198c4
branch: docs/persistent-agent-programs-20260805
pr: none
status: validating
context_routes:
  - agent-governance
  - architecture
  - testing
owned_paths:
  - docs/agents/prompts/OTERYN_PLATFORM_CONTINUOUS_AUDIT_PROGRAM.md
  - docs/agents/prompts/OTERYN_PLATFORM_REMEDIATION_PROGRAM.md
  - docs/agents/prompts/OTERYN_PLATFORM_ARCHITECTURE_REVIEW_PROGRAM.md
  - docs/agents/programs/OTERYN_PLATFORM_CONTINUOUS_AUDIT.md
  - docs/agents/programs/OTERYN_PLATFORM_REMEDIATION.md
  - docs/agents/programs/OTERYN_PLATFORM_ARCHITECTURE_REVIEW.md
  - docs/agents/SHORT_PROGRAM_INVOCATIONS.md
  - docs/agents/AUDIT_REMEDIATION_ISSUE_TAXONOMY.md
  - docs/agents/REMEDIATION_WORK_CLAIM_PROTOCOL.md
  - docs/agents/evidence/OTERYN-20260805-persistent-agent-programs/prompt-eval.md
  - docs/agents/tasks/active/OTERYN-20260805-persistent-agent-programs.md
proven:
  - Current prompting standard supports short programme invocations resolved from live repository state.
  - Three canonical prompts and three mutable programme-state files exist on the branch.
  - The repository already provides programme, type, risk, priority, state, ready and governance labels needed by the taxonomy.
  - The claim protocol requires a provisional Issue marker before product edits and activation through task, branch and draft/open PR ownership.
  - The earliest valid unexpired Issue claim wins a race; later claimants release without product mutation.
  - Shared manifests, lockfiles, route registries, shells, migration chains, generated contracts and CI workflows are serialized by default.
derived:
  - Combining the global Issue claim with detailed active-task ownership is safer than either mechanism alone, especially when several agents share one GitHub identity.
  - Separate immutable prompts and mutable programme-state records prevent continuation state from corrupting the behavioural contract.
unknown:
  - Required GitHub checks and final exact-head result for the pull request that will be opened from this branch.
conflicts: []
first_failure:
  marker: none
  evidence: none
rejected_hypotheses:
  - Reusing only historical narrow prompts would not cover continuous whole-platform scope, architecture/CI review or parallel remediation.
  - Labels or assignees alone are insufficient ownership because multiple agents may share the same GitHub identity and label changes are not an atomic work lock.
changed_paths:
  - docs/agents/AUDIT_REMEDIATION_ISSUE_TAXONOMY.md
  - docs/agents/REMEDIATION_WORK_CLAIM_PROTOCOL.md
  - docs/agents/SHORT_PROGRAM_INVOCATIONS.md
  - docs/agents/evidence/OTERYN-20260805-persistent-agent-programs/prompt-eval.md
  - docs/agents/programs/OTERYN_PLATFORM_ARCHITECTURE_REVIEW.md
  - docs/agents/programs/OTERYN_PLATFORM_CONTINUOUS_AUDIT.md
  - docs/agents/programs/OTERYN_PLATFORM_REMEDIATION.md
  - docs/agents/prompts/OTERYN_PLATFORM_ARCHITECTURE_REVIEW_PROGRAM.md
  - docs/agents/prompts/OTERYN_PLATFORM_CONTINUOUS_AUDIT_PROGRAM.md
  - docs/agents/prompts/OTERYN_PLATFORM_REMEDIATION_PROGRAM.md
  - docs/agents/tasks/active/OTERYN-20260805-persistent-agent-programs.md
validation:
  - command: GitHub contents API branch path inventory for docs/agents/prompts and docs/agents/programs
    result: PASS
    evidence: all three canonical prompts and all three programme state files exist on branch efa37a5bdf76539d8f92cda5f7db3d5d63e198c4
  - command: manual static prompt scenario review
    result: PASS
    evidence: docs/agents/evidence/OTERYN-20260805-persistent-agent-programs/prompt-eval.md; 18 positive, negative and boundary scenarios passed static review
  - command: repeated runtime/model trials
    result: NOT_RUN
    evidence: no repository harness capable of independent multi-session agent execution was identified; limitation recorded in the evaluation artifact
  - command: runtime end-to-end validation
    result: NOT_APPLICABLE
    evidence: deliverable changes only agent prompts, governance, programme state and documentation; no application runtime behavior changes
  - command: exact-head required GitHub CI
    result: NOT_RUN
    evidence: pull request not yet opened
blockers:
  - none
next_action: Open the documentation pull request, inspect its exact-head required checks and complete documentation closeout.
```

## Notes

The issue claim is the global race arbiter. The active task record remains the detailed path/branch/PR/lease source of truth. Neither may override repository authority, accepted architecture, security, production or cross-repository restrictions.