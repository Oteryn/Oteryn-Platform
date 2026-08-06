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
updated_at: 2026-08-05T22:01:00Z
status: ready
current_review_domain: none
active_task: none
issue: none
branch: none
pull_request: none
last_completed_domain: architecture-decision-backlog-authority
last_completed_issue: 602
last_completed_pull_request: 604
last_completed_merge: 2cb10c7a916fff670ce1ec7f813ae75d95fb9f3e
accepted_decision:
  option: B
  accepted_on: 2026-08-05
  adr: docs/architecture/adr/0022-architecture-authority-index-and-focused-canonical-documents.md
  authority_index: docs/architecture/ARCHITECTURE_AUTHORITY.md
latest_accepted_decision:
  option: B
  accepted_on: 2026-08-05
  issue: 602
  pull_request: 604
  merge: 2cb10c7a916fff670ce1ec7f813ae75d95fb9f3e
  adr: docs/architecture/adr/0023-machine-readable-architecture-decision-backlog.md
  summary: Use one dedicated canonical JSON inventory for unresolved architecture decision obligations, subordinate to accepted ADR authority.
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
    state: completed
    issue: 577
    pull_request: 581
    merge: 2a9715f89a38d2e8e441d34813f03bc0ad6dd707
    summary: Enforce ADR registry integrity with a closed exact-path legacy collision allowlist.
  - id: ARCH-AUTH-004
    severity: high
    type: documentation_drift
    state: completed
    issue: 593
    pull_request: 594
    merge: 4cd3c6daf8fcd152743db34f214abb531e1e2d01
    summary: Reconcile current system and module architecture using PR 453 and later exact merged evidence.
  - id: ARCH-AUTH-005
    severity: medium
    type: missing_decision
    state: completed
    issue: 602
    pull_request: 604
    merge: 2cb10c7a916fff670ce1ec7f813ae75d95fb9f3e
    accepted_adr: docs/architecture/adr/0023-machine-readable-architecture-decision-backlog.md
    summary: Accept the authority, schema, lifecycle and validation boundary for one machine-readable architecture decision backlog.
architecture_conflicts:
  - Historical duplicate ADR prefixes remain for 0008, 0010, 0011, 0015, 0016, 0017, 0018 and 0021, but the exact accepted path sets are machine-enforced and cannot expand silently.
ci_architecture_findings:
  - Existing PHPUnit CI executes the ADR registry validator without workflow changes.
  - PR 626 repaired the protected documentation-only gate by separating conditional runtime-tests from an always-emitted aggregate test context.
  - Runtime/code changes still require the complete MariaDB/PHP suite before the aggregate test gate can pass.
  - PR 604 proved documentation-only changes can merge only after fail-closed classification reports NOT_APPLICABLE and the aggregate protected test context succeeds.
accepted_handoffs_ready_for_remediation:
  - Issues 365, 488, 489 and 490 remain the exact owners for retained completeness, recovery, applicability and environment evidence gaps.
  - ADR 0023 is ready for one separate bounded implementation package covering the JSON backlog, validator, tests, initial unresolved records, programme projection and authority-routing documentation.
proven:
  - Option B from ADR 0022 remains accepted and durable.
  - PR 581 established fail-closed ADR registry integrity without renumbering accepted history.
  - PR 594 merged as 4cd3c6daf8fcd152743db34f214abb531e1e2d01 and Issue 593 is closed.
  - Repository, Issue and PR searches found no prior machine-readable architecture decision backlog owner.
  - The repository owner accepted Option B for ARCH-AUTH-005 on Issue 602.
  - ADR 0023 records the accepted dedicated JSON backlog authority boundary and separate implementation handoff.
  - PR 626 merged as 8c0c19253bdc938876cdeeae24455b27e91c4049 without weakening runtime test enforcement.
  - PR 604 CI run 31050673929 proved classify-changes success, runtime-tests skipped and aggregate test success before protected auto-merge.
unknown: []
conflicts: []
blockers: []
next_action: Start one separate bounded remediation package implementing accepted ADR 0023, deduplicating unresolved records against current ADRs, Issues, PRs and programme state before seeding the JSON backlog.
```

## Programme rules

- Proposed ADRs remain proposed until accepted by authoritative owner/repository state.
- Runtime, workflow and infrastructure implementation belongs to remediation after acceptance.
- Do not create a duplicate architecture registry when an existing canonical source can be corrected or extended.
- Keep this file compact and update it after a material decision, handoff, blocker, completed review package or rotation.
- Exactly one `next_action` is required while the programme is not terminal.
