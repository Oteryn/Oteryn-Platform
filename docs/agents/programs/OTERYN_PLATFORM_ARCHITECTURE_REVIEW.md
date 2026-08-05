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
updated_at: 2026-08-05T19:55:00Z
status: ready
current_review_domain: none
active_task: none
issue: none
branch: none
pull_request: none
last_completed_domain: current-system-module-reconciliation
last_completed_issue: 593
last_completed_pull_request: 594
last_completed_merge: 4cd3c6daf8fcd152743db34f214abb531e1e2d01
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
    type: improvement
    state: ready
    summary: Add one validated machine-readable architecture decision backlog after its schema and ownership are accepted.
architecture_conflicts:
  - Historical duplicate ADR prefixes remain for 0008, 0010, 0011, 0015, 0016, 0017, 0018 and 0021, but the exact accepted path sets are machine-enforced and cannot expand silently.
ci_architecture_findings:
  - Existing PHPUnit CI executes the ADR registry validator without workflow changes.
  - All eight exact-head workflows passed on a3cf245b5b0eafff00a87ba97878adcc8154a8df before PR 594 merged.
  - Canonical architecture now separates repository availability, capability completeness, environment proof and activation authority.
accepted_handoffs_ready_for_remediation:
  - Issues 365, 488, 489 and 490 remain the exact owners for retained completeness, recovery, applicability and environment evidence gaps.
proven:
  - Option B remains accepted and durable in ADR 0022.
  - PR 581 established fail-closed ADR registry integrity without renumbering accepted history.
  - PR 594 merged as 4cd3c6daf8fcd152743db34f214abb531e1e2d01 and Issue 593 is closed.
  - EditorialMedia, Wiki, Wallet, Marketplace and GameCatalog now have evidence-aligned current availability boundaries.
  - ProductsEntitlements, LegalCommerce, OperationsObservability, PublicEdge and QualityE2E now have explicit ownership boundaries.
  - Wallet/Marketplace, provider Payments and product fulfilment remain separate domains.
  - Open gaps and environment gates remain explicit and are not converted into production claims.
derived:
  - The next highest-value bounded review is the authority, schema and validation model for one machine-readable architecture decision backlog.
unknown: []
conflicts: []
blockers: []
next_action: Start a bounded review of the machine-readable architecture decision backlog schema and ownership, deduplicating against ADRs, Issues, PRs and the programme queue before creating a task.
```

## Programme rules

- Proposed ADRs remain proposed until accepted by authoritative owner/repository state.
- Runtime, workflow and infrastructure implementation belongs to remediation after acceptance.
- Do not create a duplicate architecture registry when an existing canonical source can be corrected or extended.
- Keep this file compact and update it after a material decision, handoff, blocker, completed review package or rotation.
- Exactly one `next_action` is required while the programme is not terminal.
