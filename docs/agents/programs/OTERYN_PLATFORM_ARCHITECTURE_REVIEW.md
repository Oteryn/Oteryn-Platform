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
updated_at: 2026-08-05T16:40:00Z
status: validating
current_review_domain: adr-registry-integrity
active_task: OTERYN-20260805-adr-registry-validator
issue: 577
branch: task/OTERYN-20260805-adr-registry-validator
pull_request: 581
exact_base: 3f79987f47e5c7593daccdf1136e09d6641017de
failed_head: 2d1d59fffe8d0163ff49a42afb7c0c18d7521655
last_completed_domain: canonical-architecture-authority
last_completed_issue: 548
last_completed_pull_request: 550
last_completed_merge: 05c7695149117e9cdb8e34937217033357175619
accepted_decision:
  option: B
  accepted_on: 2026-08-05
  adr: docs/architecture/adr/0022-architecture-authority-index-and-focused-canonical-documents.md
  authority_index: docs/architecture/ARCHITECTURE_AUTHORITY.md
decision_backlog:
  - id: ARCH-AUTH-001
    severity: high
    type: missing_decision
    state: completed
    summary: Select the canonical architecture authority and precedence model.
  - id: ARCH-AUTH-002
    severity: medium
    type: routing
    state: completed
    summary: Route architecture-wide work through the accepted authority index and focused canonical owners.
  - id: ARCH-AUTH-003
    severity: high
    type: defect
    state: validating
    issue: 577
    pull_request: 581
    summary: Add a fail-closed ADR registry validator using a closed exact-path legacy collision allowlist.
  - id: ARCH-AUTH-004
    severity: high
    type: documentation_drift
    state: ready
    summary: Reconcile current system and module architecture using PR 453 and later exact merged evidence.
  - id: ARCH-AUTH-005
    severity: medium
    type: improvement
    state: queued
    summary: Add one validated machine-readable architecture decision backlog after its schema and ownership are accepted.
architecture_conflicts:
  - Historical duplicate ADR prefixes remain for 0008, 0010, 0011, 0015, 0016, 0017, 0018 and 0021; Issue 577 converts them into a closed compatibility allowlist.
ci_architecture_findings:
  - Existing PHPUnit CI enforces the ADR validator without editing workflow files.
  - Failed head 2d1d59fffe8d0163ff49a42afb7c0c18d7521655 proved three established lifecycle declaration formats; the parser repair preserves all historical ADR bytes.
accepted_handoffs_ready_for_remediation:
  - Reconcile module and system current-state documentation from PR 453 and later exact merged evidence after Issue 577 reaches terminal state.
proven:
  - No exact existing Issue, PR or implementation owner existed before Issue 577.
  - ADR 0022 requires fail-closed validation while preserving existing accepted paths.
  - Issue 558 and tools/agents scope remain excluded.
  - CI run 31025277136 failed only because the first parser accepted bullet status metadata but not established plain and section forms.
  - The repaired focused suite passes 10 tests and rejects ambiguous lifecycle declarations.
derived:
  - A closed exact-path legacy allowlist is the smallest compatibility-safe collision model.
  - Supporting established lifecycle syntax is safer than rewriting accepted ADR history.
unknown:
  - Repaired exact final-head validation result.
conflicts: []
blockers: []
next_action: Run exact-head GitHub Actions on the repaired parser and complete the fresh diff and invariant audit.
```

## Programme rules

- Proposed ADRs remain proposed until accepted by authoritative owner/repository state.
- Runtime, workflow and infrastructure implementation belongs to remediation after acceptance.
- Do not create a duplicate architecture registry when an existing canonical source can be corrected or extended.
- Keep this file compact and update it after a material decision, handoff, blocker, completed review package or rotation.
- Exactly one `next_action` is required while the programme is not terminal.
