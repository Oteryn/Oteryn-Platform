---
task_id: OTERYN-20260806-issue-owned-remediation-audit-gate
project_lane: oteryn-platform-core
task_kind: implementation
implementation_authorized: true
issue: 753
status: implementing
base_head: 1b737574851453e950fa485c26f1a322b8e8ddd2
branch: docs/issue-owned-remediation-audit-gate-20260806
---

# OTERYN-20260806-issue-owned-remediation-audit-gate

## Goal

Replace mandatory independent audit for every Platform repair with a fail-closed risk gate while making one implementation agent the durable owner of one Issue from claim through implementation, validation, PR, merge, Issue closure and task archival.

## Acceptance criteria

- [ ] Default repair lifecycle is one Issue, one implementation owner, one deterministic branch and one delivery PR from claim through terminal closeout.
- [ ] Every repair records self-review and exact-head validation.
- [ ] A canonical machine-readable audit gate selects `NOT_REQUIRED`, `OPTIONAL`, or `REQUIRED` from explicit evidence.
- [ ] Mandatory triggers cover high/critical risk, security/auth/payment/data-integrity/migration/protocol/CI/deployment/architecture/cross-repository/irreversible boundaries and material uncertainty.
- [ ] An implementation agent cannot downgrade a mandatory trigger or describe self-review as independent audit.
- [ ] When audit is required, the same implementation owner remains responsible for findings and terminal closeout; the auditor remains read-only.
- [ ] Parallel repair commands allocate implementation owners by default; audit roles are created only for valid required handoffs.
- [ ] Active repair trains are exceptional, opt-in and restricted to homogeneous low-risk mechanical/governance work; ordinary product repairs stay one Issue/one PR.
- [ ] Taxonomy, claim protocol, programme prompt/state, short-command registry and closeout contracts use coherent versions and semantics.
- [ ] Static adversarial evaluation has no safety-critical regression and records repeated model trials truthfully.
- [ ] Runtime E2E is `NOT_APPLICABLE` with a governance-only reason.
- [ ] This policy change receives the fresh independent audit required by the trusted base, exact-head CI, terminal PR/Issue state, archival and ownership release.

## Ownership

```yaml
owned_paths:
  - docs/agents/AGENTS.md
  - docs/agents/AUDIT_REMEDIATION_ISSUE_TAXONOMY.md
  - docs/agents/DELIVERY_COMPLETENESS_AND_CLOSEOUT.md
  - docs/agents/REMEDIATION_AUDIT_RISK_GATE.md
  - docs/agents/REMEDIATION_WORK_CLAIM_PROTOCOL.md
  - docs/agents/REPAIR_PR_ECONOMY.md
  - docs/agents/SHORT_PROGRAM_INVOCATIONS.md
  - docs/agents/TASK_CLOSEOUT_AUDIT_E2E.md
  - docs/agents/evidence/OTERYN-20260806-issue-owned-remediation-audit-gate/prompt-eval.md
  - docs/agents/programs/OTERYN_PLATFORM_REMEDIATION.md
  - docs/agents/prompts/OTERYN_PLATFORM_REMEDIATION_PROGRAM.md
  - docs/agents/tasks/active/OTERYN-20260806-issue-owned-remediation-audit-gate.md
modules:
  - agent-governance
  - remediation-programme
dependencies:
  - PR #743 merged repair PR economy and claim protocol version 3
  - current trusted-base docs/agents/AGENTS.md requires fresh independent audit before archival
blockers:
  - none
cross_repository_tasks:
  - none
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-08-06T14:03:00Z
head: 1b737574851453e950fa485c26f1a322b8e8ddd2
branch: docs/issue-owned-remediation-audit-gate-20260806
pr: none
status: implementing
context_routes:
  - agent-governance
  - testing
owned_paths:
  - docs/agents/AGENTS.md
  - docs/agents/AUDIT_REMEDIATION_ISSUE_TAXONOMY.md
  - docs/agents/DELIVERY_COMPLETENESS_AND_CLOSEOUT.md
  - docs/agents/REMEDIATION_AUDIT_RISK_GATE.md
  - docs/agents/REMEDIATION_WORK_CLAIM_PROTOCOL.md
  - docs/agents/REPAIR_PR_ECONOMY.md
  - docs/agents/SHORT_PROGRAM_INVOCATIONS.md
  - docs/agents/TASK_CLOSEOUT_AUDIT_E2E.md
  - docs/agents/evidence/OTERYN-20260806-issue-owned-remediation-audit-gate/prompt-eval.md
  - docs/agents/programs/OTERYN_PLATFORM_REMEDIATION.md
  - docs/agents/prompts/OTERYN_PLATFORM_REMEDIATION_PROGRAM.md
  - docs/agents/tasks/active/OTERYN-20260806-issue-owned-remediation-audit-gate.md
proven:
  - Trusted base is main commit 1b737574851453e950fa485c26f1a322b8e8ddd2.
  - Issue #753 authorizes the bounded governance change.
  - Current policy requires a distinct final auditor for every remediation delivery and permanently recommends audit-role slots.
  - Current closeout contracts require fresh independent audit before every substantial task archive.
  - No open PR was found that owns REPAIR_PR_ECONOMY.md or this selective-audit change.
derived:
  - A central audit-risk gate is required to avoid contradictory trigger logic across documents.
  - Claim ownership and final independent validation must be separate concepts.
unknown:
  - Final candidate head and PR number.
  - Independent auditor identity and final audit generation.
conflicts: []
first_failure:
  marker: none
  evidence: none
rejected_hypotheses:
  - Allowing the implementation owner to waive an explicit mandatory audit trigger.
  - Reserving one audit slot even when no required audit handoff exists.
  - Using repair trains as the normal product-remediation delivery mode.
changed_paths:
  - docs/agents/tasks/active/OTERYN-20260806-issue-owned-remediation-audit-gate.md
validation:
  - command: live governance and ownership preflight
    result: PASS
    evidence: current main, governing contracts, active tasks and open PRs inspected
  - command: runtime E2E classification
    result: NOT_APPLICABLE
    evidence: repository agent-governance and remediation routing only; no executable runtime journey changes
audit_gate:
  version: trusted-base-predecessor
  requirement: REQUIRED
  triggers:
    - risk:high
    - changes independent-audit and task-closeout policy
  self_review: PENDING
  independent_audit: PENDING
blockers:
  - none
next_action: Add the canonical remediation audit-risk gate and update the controlling remediation and closeout contracts coherently.
```

## Notes

The current task cannot use its own unmerged policy to waive the trusted-base requirement for a fresh independent audit. The candidate must therefore rotate to a distinct auditor before merge.