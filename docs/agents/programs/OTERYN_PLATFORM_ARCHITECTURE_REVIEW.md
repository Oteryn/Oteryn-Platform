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
updated_at: 2026-08-06T09:45:00Z
status: validating
current_review_domain: confidential-vulnerability-disclosure-policy
active_task: docs/agents/tasks/active/OTERYN-20260806-vulnerability-disclosure-policy.md
issue: 588
branch: docs/OTERYN-20260806-vulnerability-disclosure-policy
pull_request: 702
last_completed_domain: repository-licensing-policy
last_completed_issue: 587
last_completed_pull_request: 690
last_completed_merge: d353235a3c7d4b7b34f35a745871c10a71192cc6
accepted_authority:
  authority_index: docs/architecture/ARCHITECTURE_AUTHORITY.md
  authority_adr: docs/architecture/adr/0022-architecture-authority-index-and-focused-canonical-documents.md
  backlog_adr: docs/architecture/adr/0023-machine-readable-architecture-decision-backlog.md
  backlog_registry: docs/architecture/ARCHITECTURE_DECISION_BACKLOG.json
  branch_lifecycle_adr: docs/architecture/adr/0024-merged-source-branch-lifecycle-policy.md
  licensing_adr: docs/architecture/adr/0026-proprietary-repository-licensing-policy.md
  vulnerability_disclosure_adr: docs/architecture/adr/0027-confidential-vulnerability-disclosure-policy.md
implementation_handoffs:
  - issue: 658
    scope: deterministic branch inventory, retention metadata, conservative cleanup and recovery proof
active_architecture_decision_ids: []
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
  - PR 690 merged ADR 0026 and the proprietary/no-permission policy as d353235a3c7d4b7b34f35a745871c10a71192cc6 after all exact-head workflows and independent audit passed.
  - Repository owner selected Option A for ARCH-DEC-0003 on 2026-08-06.
  - GitHub API proves private vulnerability reporting is enabled for blakinio/Oteryn-Platform.
  - PR 702 contains ADR 0027, canonical SECURITY.md routing, Issue-form diversion and an empty active decision backlog.
derived:
  - Repository security maintainers and administrators form the role-based contact while GitHub private advisories provide the confidential transport.
unknown:
  - Final exact-head validation, protected merge and archival outcome for PR 702.
conflicts: []
blockers: []
next_action: Complete exact-head validation and independent audit for PR 702, merge through the protected route, archive the task and close Issue 588.
```

## Programme rules

- Proposed ADRs remain proposed until accepted by authoritative owner/repository state.
- Runtime, workflow and infrastructure implementation belongs to remediation after acceptance.
- Do not create a duplicate architecture registry when an existing canonical source can be corrected or extended.
- Keep only the compact `active_architecture_decision_ids` projection here; full record data belongs to `ARCHITECTURE_DECISION_BACKLOG.json`.
- Keep this file compact and update it after a material decision, handoff, blocker, completed review package or rotation.
- Exactly one `next_action` is required while the programme is not terminal.
