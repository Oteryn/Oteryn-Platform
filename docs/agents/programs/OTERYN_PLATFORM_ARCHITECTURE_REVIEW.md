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
updated_at: 2026-08-05T16:55:00+02:00
status: blocked
current_review_domain: canonical-architecture-authority
active_task: OTERYN-20260805-architecture-authority
issue: 548
branch: task/OTERYN-20260805-architecture-authority
pull_request: 550
exact_base: 3ab77c072dce796b09004c54b649db009a75d524
decision_backlog:
  - id: ARCH-AUTH-001
    severity: high
    type: missing_decision
    state: owner_decision_required
    summary: Select the canonical architecture authority and precedence model.
    issue: 548
    pull_request: 550
  - id: ARCH-AUTH-002
    severity: medium
    type: defect
    state: ready_after_ARCH-AUTH-001
    summary: Repair repository architecture routing to live canonical entry points.
  - id: ARCH-AUTH-003
    severity: high
    type: defect
    state: ready_after_ARCH-AUTH-001
    summary: Reconcile the incomplete ADR index and duplicate 0008 identifiers with compatibility preserved.
  - id: ARCH-AUTH-004
    severity: high
    type: documentation_drift
    state: ready_after_ARCH-AUTH-001
    summary: Reconcile current system and module architecture using PR 453 and later merged evidence.
  - id: ARCH-AUTH-005
    severity: medium
    type: improvement
    state: ready_after_ARCH-AUTH-001
    summary: Add one validated machine-readable architecture decision backlog.
architecture_conflicts:
  - SYSTEM_ARCHITECTURE initial-phase/non-goal text conflicts with ROADMAP, MODULE_CATALOG, contracts and merged implementation.
  - REPOSITORY_MAP references missing docs/architecture/overview.md.
  - ADR README is non-exhaustive and the ADR directory contains duplicate 0008 numbering.
ci_architecture_findings:
  - Future architecture-link and ADR-inventory validation should be governance-scoped and fail closed without weakening runtime gates.
accepted_handoffs_ready_for_remediation: []
proven:
  - The canonical architecture review prompt is repository-backed and immutably scoped to blakinio/Oteryn-Platform.
  - Issue 548 is the deduplicated decision boundary for the first review package.
  - Draft PR 550 contains only the bounded architecture task, report and programme-state paths.
  - PR 453 is the existing authoritative audit evidence for module-catalogue drift.
  - Active PR 542 owns native-protocol implementation and related contracts and is excluded from this task.
derived:
  - An explicit authority model is required before safely repairing routing, ADR allocation and module/current-state documentation.
  - Option B, an authority index plus focused canonical documents, has the strongest correctness and maintainability trade-off.
unknown:
  - Whether the owner accepts Option B or selects A/C.
conflicts:
  - Initial target documentation can currently be mistaken for present implementation truth.
blockers:
  - decision: Accept Option B, or select Option A/C with rationale in Issue 548.
next_action: Record the owner decision in Issue 548; if B is accepted, continue PR 550 with deterministic ADR inventory and documentation Slice 1.
```

## Programme rules

- Proposed ADRs remain proposed until accepted by authoritative owner/repository state.
- Runtime, workflow and infrastructure implementation belongs to remediation after acceptance.
- Do not create a duplicate architecture registry when an existing canonical source can be corrected or extended.
- Keep this file compact and update it after a material decision, handoff, blocker, completed review package or rotation.
- Exactly one `next_action` is required while the programme is not terminal.
