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
updated_at: 2026-08-05T22:20:00Z
status: validating
current_review_domain: architecture-decision-backlog-implementation
active_task: OTERYN-20260805-architecture-decision-backlog-implementation
issue: 642
branch: repair/issue-642-architecture-decision-backlog
pull_request: 650
last_completed_domain: architecture-decision-backlog-authority
last_completed_issue: 602
last_completed_pull_request: 604
last_completed_merge: 2cb10c7a916fff670ce1ec7f813ae75d95fb9f3e
accepted_authority:
  authority_index: docs/architecture/ARCHITECTURE_AUTHORITY.md
  authority_adr: docs/architecture/adr/0022-architecture-authority-index-and-focused-canonical-documents.md
  backlog_adr: docs/architecture/adr/0023-machine-readable-architecture-decision-backlog.md
  backlog_registry: docs/architecture/ARCHITECTURE_DECISION_BACKLOG.json
active_architecture_decision_ids: ["ARCH-DEC-0001","ARCH-DEC-0002","ARCH-DEC-0003"]
architecture_conflicts:
  - Historical duplicate ADR prefixes remain for 0008, 0010, 0011, 0015, 0016, 0017, 0018 and 0021, but the exact accepted path sets are machine-enforced and cannot expand silently.
ci_architecture_findings:
  - PR 626 separates conditional runtime-tests from an always-emitted aggregate protected test context.
  - Runtime/code changes require the complete MariaDB/PHP suite before the aggregate test gate can pass.
  - Documentation-only changes pass only after fail-closed classification proves runtime tests are NOT_APPLICABLE.
proven:
  - ADR 0023 authorizes one repository-owned JSON inventory for unresolved architecture decision obligations.
  - Duplicate searches found no competing registry implementation Issue, PR or active task owner.
  - Issues 586, 587 and 588 contain unresolved owner policy decisions suitable for initial seeding.
  - Current repository metadata reports automatic merged-branch deletion enabled, while no canonical exception and recovery policy was found.
  - Completed ARCH-AUTH history and implementation-only work are excluded from the active registry.
derived:
  - Three initial records exercise the non-empty schema while preserving a narrow unresolved-decision boundary.
unknown:
  - Exact-head validation and merge outcome for Issue 642.
conflicts: []
blockers: []
next_action: Validate and merge the ADR 0023 registry implementation, archive Issue 642 ownership, then route the highest-priority active decision through its linked Issue without inferring owner acceptance.
```

## Programme rules

- Proposed ADRs remain proposed until accepted by authoritative owner/repository state.
- Runtime, workflow and infrastructure implementation belongs to remediation after acceptance.
- Do not create a duplicate architecture registry when an existing canonical source can be corrected or extended.
- Keep only the compact `active_architecture_decision_ids` projection here; full record data belongs to `ARCHITECTURE_DECISION_BACKLOG.json`.
- Keep this file compact and update it after a material decision, handoff, blocker, completed review package or rotation.
- Exactly one `next_action` is required while the programme is not terminal.
