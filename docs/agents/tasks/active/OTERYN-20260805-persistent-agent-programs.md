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
  - active tasks and open pull requests touching agent prompts, programme registries or claim ownership
optional_reads:
  - docs/agents/PROMPT_EVAL_STANDARD.md
---

# OTERYN-20260805-persistent-agent-programs

## Goal

Create three durable, non-overlapping Oteryn Platform agent programmes for continuous product audit, audit remediation, and architecture/structure/CI review, plus short owner commands, repository-backed continuation state, classified parallel queues and collision-safe remediation claims.

## Acceptance criteria

- [x] Three versioned programme prompts follow current prompting, trust, completeness, audit, E2E, closeout and anti-stall contracts.
- [x] Auditor, remediator and architecture adviser have non-overlapping mutation authority.
- [x] Every programme has durable state allowing continuation without chat history.
- [x] A short-command registry maps Polish invocations to prompts/state, including bounded parallel remediation.
- [x] Missing required modules produce a confirmed Issue and Proposed documentation/contract PR; runtime implementation remains remediation work.
- [x] Audit Issues carry existing labels and machine-readable workstream, risk, path, dependency, delivery and parallelization metadata.
- [x] Remediation ownership uses deterministic branch `repair/issue-<number>` as the atomic lock, Issue comments for visibility, and active task/PR for detailed ownership.
- [x] Claim race, losing claimant, activation, lease renewal, stale takeover, release and shared-path serialization are defined.
- [x] Documentation paths/references were verified and 21 positive, negative and boundary scenarios passed manual static review.

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
updated_at: 2026-08-05T13:45:00Z
head: 43c2eccd246da6b1d33590c7ec2b0148b845a766
branch: docs/persistent-agent-programs-20260805
pr: 543
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
  - Three canonical prompts and three mutable programme-state files exist on the branch.
  - Existing repository labels cover programme, type, risk, priority, state, readiness and managed governance.
  - GitHub unique ref creation for repair/issue-<number> is the atomic race arbiter.
  - Issue comments alone are visibility, and labels/assignees alone are not ownership.
  - The branch winner must activate through an active task and one draft/open PR before product edits.
  - A losing claimant may not create a suffix branch or mutate product paths.
  - Shared manifests, lockfiles, route registries, shells, migration chains, generated contracts and CI workflows are serialized by default.
derived:
  - Atomic deterministic branch acquisition plus Issue visibility and task path ownership is safer than any one mechanism alone, including when several agents share one GitHub identity.
  - Immutable prompts plus mutable programme state keep continuation durable without corrupting behavioral contracts.
unknown:
  - Required exact-head GitHub checks after PR #543 becomes ready for review.
conflicts: []
first_failure:
  marker: none
  evidence: none
rejected_hypotheses:
  - Historical narrow prompts do not cover continuous whole-platform scope, architecture/CI review or parallel remediation.
  - Comment chronology cannot be the atomic race arbiter.
  - Labels or assignees cannot safely lock work for agents sharing one GitHub identity.
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
  - command: GitHub branch contents inventory for prompts and programme state
    result: PASS
    evidence: all three canonical prompts and all three mutable programme files exist
  - command: changed-file inventory for PR #543
    result: PASS
    evidence: exactly 11 declared governance/documentation paths; no runtime application paths
  - command: manual static prompt and claim-protocol scenario review
    result: PASS
    evidence: docs/agents/evidence/OTERYN-20260805-persistent-agent-programs/prompt-eval.md; 21 scenarios
  - command: repeated runtime/model trials
    result: NOT_RUN
    evidence: no repository multi-session agent execution harness identified; limitation recorded
  - command: runtime end-to-end validation
    result: NOT_APPLICABLE
    evidence: agent-governance/documentation only; no application runtime behavior changed
  - command: exact-head required GitHub CI
    result: NOT_RUN
    evidence: PR #543 remains draft pending ready-state check generation
blockers:
  - none
next_action: Mark PR #543 ready for review, inspect aggregate exact-head required checks, and complete documentation closeout.
```

## Notes

The deterministic Git ref is the atomic exclusion mechanism. The Issue is the global visibility surface. The active task is the detailed path, lease, recovery and continuation source of truth. None may override repository authority, accepted architecture, security, production or cross-repository restrictions.