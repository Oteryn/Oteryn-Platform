---
task_id: OTERYN-20260815-historical-work-reconciliation
issue: 1072
status: investigating
project_lane: oteryn-platform-core
phase: investigate
execution_mode: github_connector
required_reads:
  - AGENTS.md
  - AGENTS.override.md
  - docs/agents/AGENTS.md
  - docs/agents/REPOSITORY_MAP.md
  - docs/agents/CONTEXT_ROUTING.md
  - docs/agents/PROMPTING_STANDARD.md
  - docs/agents/DELIVERY_COMPLETENESS_AND_CLOSEOUT.md
  - docs/agents/ANTI_STALL_AND_EXECUTION_BUDGET.md
  - docs/agents/SESSION_RECOVERY_AND_ORPHANED_EXECUTION.md
  - docs/agents/TERMINAL_ONLY_COMMUNICATION.md
  - docs/agents/GITHUB_ONLY_EXECUTION.md
  - docs/agents/AUTONOMOUS_PROGRAM_CONTINUATION.md
  - docs/agents/EXECUTION_PROTOCOL.md
  - docs/agents/PROJECT_LANES.json
  - docs/agents/BRANCH_LIFECYCLE_POLICY.json
  - docs/architecture/adr/0037-terminal-source-branch-lifecycle.md
  - docs/architecture/adr/0039-historical-work-canonicalization-and-managed-recovery.md
search_first:
  - Issue #1072 and Issue #1068 terminal evidence
  - current live branches and every open PR
  - current active tasks and path ownership
  - release/tag/ref-triggered workflows before choosing a managed recovery ref mechanism
optional_reads:
  - docs/agents/reports/OTERYN-20260814-remaining-branch-audit.md
  - tools/agents/historical_branch_audit.py
  - tools/agents/historical_branch_policy.py
---

# OTERYN-20260815-historical-work-reconciliation

## Goal

Reconcile every historical branch that survives only because Issue #1068 classified it `RETAIN` or `RECOVERY`, move valuable work/evidence to its correct durable home, introduce managed recovery retention where exact Git reachability is genuinely required, and remove obsolete execution refs using exact-head fail-closed controls.

## Acceptance criteria

- [ ] Reconcile against fresh protected `main`; do not rely on the 2026-08-14/15 seed counts without live verification.
- [ ] Account for every live branch and every open same-repository PR before mutation.
- [ ] Audit each historical `RETAIN` / `RECOVERY` ref for unique commits, changed paths, content value, PR/Issue/task provenance, ancestry/reachability, current-main relevance and recovery purpose.
- [ ] No ordinary historical branch remains solely as an archive convenience.
- [ ] `RETAIN` is treated as reconciliation-required, not terminal.
- [ ] Every valuable source/config/test/governance change selected for current use is intentionally reconstructed/delivered against current `main`; no blind historical branch merge.
- [ ] Historical evidence/context selected for preservation is persisted in the focused canonical report/task/ADR/Issue/PR location with exact branch/SHA provenance before ref deletion.
- [ ] Every long-lived recovery ref that truly requires exact Git reachability has machine-checkable owner, purpose, reachability mechanism, restore procedure, review trigger, and expiry/retention semantics.
- [ ] A SHA in JSON/Markdown alone is never treated as proof that a required Git object will remain reachable.
- [ ] Any tag/custom-ref/other managed recovery mechanism is proven not to trigger release/deploy/publication workflows before adoption.
- [ ] Obsolete/superseded/disposable refs are deleted only under exact-head, liveness, protection, ownership and recovery checks with post-delete verification.
- [ ] Open-PR, protected and genuinely active work is preserved and attached to explicit ownership.
- [ ] Durable policy/validators prevent future unexplained `RETAIN` or unmanaged `RECOVERY` from being accepted as terminal closeout.
- [ ] Final live inventory contains zero unexplained `RETAIN` refs and zero unmanaged `RECOVERY` refs; only protected `main`, live/owned work and the minimum explicitly managed recovery refs remain.
- [ ] Relevant focused validation, exact-head self-review, required CI, PR hygiene and runtime E2E classification are terminal.
- [ ] Issue #1072 closes completed only after implementation merge, task archival, approval/temporary-state cleanup, ownership release and source-branch verification.

## Ownership

```yaml
owned_paths:
  - docs/agents/tasks/active/OTERYN-20260815-historical-work-reconciliation.md
  - docs/agents/tasks/archive/OTERYN-20260815-historical-work-reconciliation.md
  - docs/agents/prompts/OTERYN-HISTORICAL-WORK-RECONCILIATION.md
  - docs/agents/BRANCH_LIFECYCLE_POLICY.json
  - docs/agents/HISTORICAL_WORK_RECONCILIATION_REGISTRY.json
  - docs/architecture/adr/0039-historical-work-canonicalization-and-managed-recovery.md
  - docs/architecture/adr/README.md
  - tools/agents/historical_branch_audit.py
  - tools/agents/historical_branch_policy.py
  - tools/agents/test_historical_branch_audit.py
  - tools/agents/test_historical_branch_policy.py
  - tools/agents/*historical*reconciliation*.py
  - .github/workflows/historical-branch-audit.yml
modules:
  - repository-governance
  - branch-lifecycle
  - historical-work-reconciliation
dependencies:
  - Issue #1050 terminal source-branch lifecycle
  - Issue #1068 historical exact-SHA cleanup and final zero-candidate audit
  - ADR 0037 terminal source-branch lifecycle
blockers:
  - none
cross_repository_tasks:
  - none
```

`HISTORICAL_WORK_RECONCILIATION_REGISTRY.json` is a proposed path, not a mandatory abstraction. Create it only if live design proves a machine-readable registry is the smallest correct enforcement mechanism. If a different focused path is chosen, update this ownership record before editing.

## Accepted owner decision

The repository owner has accepted the durable policy recorded in ADR 0039:

- branches are execution resources, not the default historical archive;
- `RETAIN` is transitional;
- historical value must be canonicalized to `main`, focused documentation/evidence, PR provenance, or a purpose-built managed recovery mechanism;
- managed recovery must preserve actual required reachability, not merely textual SHA provenance;
- branch deletion remains exact-head and fail-closed under ADR 0037.

The implementation owner may choose the concrete managed recovery mechanism after inspecting repository automation, but may not weaken those invariants.

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-08-15T06:46:00Z
head: 77719cd8c725c0bb6f1cf35efa670ee63416c124
branch: repair/issue-1072-historical-work-reconciliation
pr: 1074
status: investigating
phase: investigate
context_routes:
  - agent-governance
  - architecture
  - testing
owned_paths:
  - docs/agents/tasks/active/OTERYN-20260815-historical-work-reconciliation.md
  - docs/agents/tasks/archive/OTERYN-20260815-historical-work-reconciliation.md
  - docs/agents/prompts/OTERYN-HISTORICAL-WORK-RECONCILIATION.md
  - docs/agents/BRANCH_LIFECYCLE_POLICY.json
  - docs/agents/HISTORICAL_WORK_RECONCILIATION_REGISTRY.json
  - docs/architecture/adr/0039-historical-work-canonicalization-and-managed-recovery.md
  - docs/architecture/adr/README.md
  - tools/agents/historical_branch_audit.py
  - tools/agents/historical_branch_policy.py
  - tools/agents/test_historical_branch_audit.py
  - tools/agents/test_historical_branch_policy.py
  - tools/agents/*historical*reconciliation*.py
  - .github/workflows/historical-branch-audit.yml
proven:
  - Issue #1068 is closed completed after deleting 33 exact-reviewed historical refs and removing its one-time approval.
  - Protected main at task creation is 5000f271db49215c93432b78397dd3560b49e7e7.
  - Issue #1068 final approval-free audit reported 45 refs with zero DELETE candidates under the prior policy: 7 OPEN_PR, 1 PROTECTED, 15 RECOVERY, 22 RETAIN.
  - Current main has two pre-existing active tasks unrelated to this repository-governance path set: public-domain repair and native-auth production verification.
  - Open PR #1065 already allocates proposed ADR prefix 0038; this task therefore allocates ADR 0039 to avoid collision.
  - Repository owner accepted that ordinary branches are not the default long-term archive and that RETAIN must not remain an unexplained terminal state.
  - Bootstrap commit 77719cd8c725c0bb6f1cf35efa670ee63416c124 persists ADR 0039, this active task, the execution prompt and ADR inventory update on the dedicated Issue #1072 branch.
  - Draft PR #1074 owns the bootstrap branch against main; it is intentionally draft so no Ready-triggered owner-funded review is invoked during handoff.
derived:
  - The 37 seed RETAIN/RECOVERY refs require content-level reconciliation, not another name/age-based deletion sweep.
  - Some exact recovery histories may need a purpose-built reachable Git ref mechanism; PR/Issue/document provenance alone is not automatically equivalent to object reachability.
unknown:
  - Current exact branch count and classification when the implementation owner begins execution.
  - Which historical refs contain still-useful current code versus evidence-only or obsolete work.
  - Which existing PR refs provide sufficient durable provenance/recovery authority for deletion of their source branches.
  - Whether annotated tags, custom refs, PR refs or another mechanism are operationally safe for managed recovery under current workflows.
conflicts: []
first_failure:
  marker: none
  evidence: none
rejected_hypotheses:
  - keep all unique historical branches indefinitely because they are safer than deciding
  - delete by age, name, prefix or apparent inactivity
  - blindly merge stale branches so GitHub will auto-delete them
  - store only commit SHAs in a registry and assume that guarantees future object reachability
changed_paths:
  - docs/agents/prompts/OTERYN-HISTORICAL-WORK-RECONCILIATION.md
  - docs/agents/tasks/active/OTERYN-20260815-historical-work-reconciliation.md
  - docs/architecture/adr/0039-historical-work-canonicalization-and-managed-recovery.md
  - docs/architecture/adr/README.md
validation:
  - command: live main / active task / open PR preflight
    result: PASS
    evidence: main and repository governance state were read before task bootstrap; implementation must repeat mutable live-state checks before destructive work
  - command: draft PR #1074 bootstrap ownership
    result: PASS
    evidence: PR #1074 is draft, base main@5000f271db49215c93432b78397dd3560b49e7e7, head repair/issue-1072-historical-work-reconciliation@77719cd8c725c0bb6f1cf35efa670ee63416c124, with exactly the four declared bootstrap paths before this checkpoint update
  - command: runtime/browser E2E
    result: NOT_APPLICABLE
    evidence: bootstrap phase is repository-governance documentation/prompt setup only; the implementation task must define the real Git-ref reconciliation E2E path and verify it directly
blockers:
  - none
next_action: Rebuild a fresh live branch/PR/task inventory from protected main and produce a per-branch content/provenance/reachability reconciliation matrix before any historical ref mutation.
```

## Source branch closeout

```yaml
source_branch_disposition: pending
source_branch_reason: Issue #1072 is active on dedicated branch repair/issue-1072-historical-work-reconciliation and draft PR #1074
source_branch_evidence: pending
```

## Notes

Seed counts from Issue #1068 are discovery hints only. The implementation owner must distrust stale branch classifications, preserve all live/open/protected work, and keep every destructive step recoverable and exact-head guarded.
