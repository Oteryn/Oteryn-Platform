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
updated_at: 2026-08-05T14:30:00Z
status: ready
current_review_domain: none
active_task: none
branch: none
pull_request: none
exact_head: main-at-next-invocation
decision_backlog: not_reconciled
architecture_conflicts: unknown
ci_architecture_findings: unknown
accepted_handoffs_ready_for_remediation: unknown
proven:
  - The canonical architecture review prompt is repository-backed.
  - This programme is immutably scoped to blakinio/Oteryn-Platform by docs/agents/OTERYN_PLATFORM_PROGRAM_SCOPE.md.
derived:
  - The first review must reconcile existing ADRs, canonical architecture documents, active implementation work and CI evidence before proposing a new decision registry.
unknown:
  - Whether a complete canonical decision backlog/global architecture registry currently exists without contradictions.
conflicts: []
blockers: []
next_action: Reconcile current canonical architecture sources, ADR status and numbering, active tasks and CI structure, then select the highest-risk unresolved decision or contradiction for one bounded review package.
```

## Programme rules

- Proposed ADRs remain proposed until accepted by authoritative owner/repository state.
- Runtime, workflow and infrastructure implementation belongs to remediation after acceptance.
- Do not create a duplicate architecture registry when an existing canonical source can be corrected or extended.
- Keep this file compact and update it after a material decision, handoff, blocker, completed review package or rotation.
- Exactly one `next_action` is required while the programme is not terminal.
