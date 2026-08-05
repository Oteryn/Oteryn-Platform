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
updated_at: 2026-08-05T21:02:00Z
status: blocked
current_review_domain: architecture-decision-backlog-authority
active_task: OTERYN-20260805-architecture-decision-backlog
issue: 602
branch: task/OTERYN-20260805-architecture-decision-backlog
pull_request: 604
exact_base: aa3ddcd0513708276920cb2734f7be845c3f177a
last_completed_domain: current-system-module-reconciliation
last_completed_issue: 593
last_completed_pull_request: 594
last_completed_merge: 4cd3c6daf8fcd152743db34f214abb531e1e2d01
accepted_architecture_authority:
  option: B
  accepted_on: 2026-08-05
  adr: docs/architecture/adr/0022-architecture-authority-index-and-focused-canonical-documents.md
  authority_index: docs/architecture/ARCHITECTURE_AUTHORITY.md
accepted_decision_backlog_model:
  option: B
  accepted_on: 2026-08-05
  issue: 602
  pull_request: 604
  adr: docs/architecture/adr/0023-machine-readable-architecture-decision-backlog.md
  canonical_backlog: docs/architecture/ARCHITECTURE_DECISION_BACKLOG.json
  implementation_state: ready_after_design_merge
  implementation_authority: repository documentation governance and deterministic validation only
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
    state: accepted_blocked_merge_gate
    issue: 602
    pull_request: 604
    accepted_adr: docs/architecture/adr/0023-machine-readable-architecture-decision-backlog.md
    accepted_option: B
    summary: Use a dedicated canonical JSON backlog for unresolved architecture decision obligations, subordinate to accepted ADR authority.
architecture_conflicts:
  - Historical duplicate ADR prefixes remain for 0008, 0010, 0011, 0015, 0016, 0017, 0018 and 0021, but the exact accepted path sets are machine-enforced and cannot expand silently.
  - The full programme-embedded decision queue remains transitional until the accepted ADR 0023 implementation migrates active obligations to the dedicated JSON registry.
ci_architecture_findings:
  - Existing PHPUnit CI executes the ADR registry validator without workflow changes.
  - Canonical architecture separates repository availability, capability completeness, environment proof and activation authority.
  - The accepted backlog validator must remain reproducible offline; remote Issue and PR liveness belongs to a separate fail-closed reconciliation boundary.
  - Main protection currently requires both classify-changes and test, while the CI workflow skips test for documentation-only changes; PR 604 proves this makes a fully green documentation PR unmergeable.
accepted_handoffs_ready_for_remediation:
  - id: ARCH-AUTH-005-IMPLEMENTATION
    issue: 602
    accepted_adr: docs/architecture/adr/0023-machine-readable-architecture-decision-backlog.md
    state: ready_after_PR_604
    scope:
      - docs/architecture/ARCHITECTURE_DECISION_BACKLOG.json
      - tools/validation architecture-decision-backlog validator and tests
      - existing repository test-suite registration without workflow edits where possible
      - compact programme projection
      - narrow ARCHITECTURE_AUTHORITY routing update
  - id: MAIN-MERGE-GATE-CONSISTENCY
    issue: 552
    state: blocked_owner_admin_repair
    scope:
      - stable always-emitted required check for documentation-only and runtime changes
      - preserve full test enforcement for runtime changes
      - no protection bypass
  - Issues 365, 488, 489 and 490 remain the exact owners for retained completeness, recovery, applicability and environment evidence gaps.
proven:
  - Option B from ADR 0022 remains accepted and durable.
  - PR 581 established fail-closed ADR registry integrity without renumbering accepted history.
  - PR 594 merged as 4cd3c6daf8fcd152743db34f214abb531e1e2d01 and Issue 593 is closed.
  - Repository, Issue and PR searches found no existing machine-readable architecture decision backlog owner.
  - The repository owner accepted Option B for Issue 602 in the current invocation.
  - ADR 0023 records the accepted authority, lifecycle, validation, migration and rollback model without runtime authorization.
  - PR 604 head 0c3845b0a7a9a30f81fe42fcb2825693aacc20c4 passed all eight emitted workflows and has zero unresolved review threads.
  - CI run 31046599415 reports classify-changes success and test skipped; protected-main merge rejects the exact head because both contexts are required.
derived:
  - A dedicated JSON backlog gives unresolved decisions stable, reproducible identities while allowing the programme to become a compact execution projection.
  - PR 604 cannot merge safely until Issue 552 reconciles the required-check policy with conditional CI execution.
unknown:
  - final PR 604 merge commit and design-task archive commit
conflicts:
  - Protected-main policy requires a test success context that the workflow intentionally does not emit for documentation-only changes.
blockers:
  - Issue 552 must reconcile the always-required merge-gate context without bypassing protection or weakening runtime test enforcement.
next_action: Repair Issue 552 so documentation-only changes emit a successful required gate while runtime changes still require full tests, then merge PR 604, archive the design task and start the separate ADR 0023 implementation package.
```

## Programme rules

- Proposed ADRs remain proposed until accepted by authoritative owner/repository state.
- Runtime, workflow and infrastructure implementation belongs to remediation after acceptance.
- Do not create a duplicate architecture registry when an existing canonical source can be corrected or extended.
- Keep this file compact and update it after a material decision, handoff, blocker, completed review package or rotation.
- Exactly one `next_action` is required while the programme is not terminal.