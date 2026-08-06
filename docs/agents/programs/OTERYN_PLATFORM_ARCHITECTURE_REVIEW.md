---
programme_id: OTERYN_PLATFORM_ARCHITECTURE_REVIEW
programme_version: 2
canonical_prompt: docs/agents/prompts/OTERYN_PLATFORM_ARCHITECTURE_REVIEW_PROGRAM.md
required_reads:
  - docs/agents/OTERYN_PLATFORM_PROGRAM_SCOPE.md
  - docs/agents/SHORT_PROGRAM_INVOCATIONS.md
repository: blakinio/Oteryn-Platform
---

# Oteryn Platform Architecture, Structure and CI Review — Programme State

## Mission

Continuously challenge Platform architecture, repository structure and CI/CD; identify contradictions and missing decisions; compare alternatives; and persist accepted decisions and implementation handoffs without changing runtime code.

## Durable queue

```yaml
programme_state_version: 2
updated_at: 2026-08-06T08:55:00Z
status: validating
current_review_domain: repository-licensing-policy
active_task: docs/agents/tasks/active/OTERYN-20260806-platform-licensing-policy.md
issue: 587
branch: docs/OTERYN-20260806-platform-licensing-policy
pull_request: 690
last_completed_domain: merged-source-branch-lifecycle-policy
last_completed_issue: 586
last_completed_pull_request: 653
last_completed_merge: 2abfb961201f7f5d359c5b140dba68be492157be
accepted_authority:
  authority_index: docs/architecture/ARCHITECTURE_AUTHORITY.md
  authority_adr: docs/architecture/adr/0022-architecture-authority-index-and-focused-canonical-documents.md
  backlog_adr: docs/architecture/adr/0023-machine-readable-architecture-decision-backlog.md
  backlog_registry: docs/architecture/ARCHITECTURE_DECISION_BACKLOG.json
  branch_lifecycle_adr: docs/architecture/adr/0024-merged-source-branch-lifecycle-policy.md
implementation_handoffs:
  - issue: 658
    scope: deterministic branch inventory, retention metadata, conservative cleanup and recovery proof
active_architecture_decision_ids: ["ARCH-DEC-0003"]
architecture_conflicts:
  - Historical duplicate ADR prefixes remain for 0008, 0010, 0011, 0015, 0016, 0017, 0018 and 0021, but the exact accepted path sets are machine-enforced and cannot expand silently.
  - Issue 586 retains historical evidence that delete_branch_on_merge was disabled; ADR 0024 and current metadata prove the accepted current state is enabled.
ci_architecture_findings:
  - PR 626 separates conditional runtime-tests from an always-emitted aggregate protected test context.
  - Runtime/code changes require the complete MariaDB/PHP suite before the aggregate test gate can pass.
  - Documentation-only changes pass only after fail-closed classification proves runtime tests are NOT_APPLICABLE.
proven:
  - Repository owner selected Option A for the merged source-branch lifecycle policy.
  - PR 653 merged ADR 0024 as 2abfb961201f7f5d359c5b140dba68be492157be after all eight final workflows passed.
  - Repository owner selected Option A for ARCH-DEC-0002 on 2026-08-06.
  - PR 690 contains ADR 0026, the proprietary notice, third-party boundary and synchronized backlog projection.
derived:
  - After licensing closeout, the next unresolved architecture decision is confidential vulnerability disclosure policy, ARCH-DEC-0003 / Issue 588.
unknown:
  - Final exact-head validation, protected merge and archival outcome for PR 690.
conflicts: []
blockers: []
next_action: Complete PR 690 validation, independent audit, protected merge and task archival, then advance the programme to Issue 588 without inferring its owner decision.
```

## Programme rules

- Proposed ADRs remain proposed until accepted by authoritative owner/repository state.
- Runtime, workflow and infrastructure implementation belongs to remediation after acceptance.
- Do not create a duplicate architecture registry when an existing canonical source can be corrected or extended.
- Keep only the compact `active_architecture_decision_ids` projection here; full record data belongs to `ARCHITECTURE_DECISION_BACKLOG.json`.
- Keep this file compact and update it after a material decision, handoff, blocker, completed review package or rotation.
- Exactly one `next_action` is required while the programme is not terminal.
