---
programme_id: OTERYN_PLATFORM_CONTINUOUS_AUDIT
programme_version: 2
canonical_prompt: docs/agents/prompts/OTERYN_PLATFORM_CONTINUOUS_AUDIT_PROGRAM.md
required_reads:
  - docs/agents/OTERYN_PLATFORM_PROGRAM_SCOPE.md
  - docs/agents/AUDIT_REMEDIATION_ISSUE_TAXONOMY.md
  - docs/agents/REMEDIATION_WORK_CLAIM_PROTOCOL.md
  - docs/agents/SHORT_PROGRAM_INVOCATIONS.md
repository: blakinio/Oteryn-Platform
---

# Oteryn Platform Continuous Audit — Programme State

## Mission

Continuously audit every delivered or declared Platform module and surface for technical correctness, security, completeness, frontend/backend integration, operability and evidence quality. Persist findings as deduplicated, classified Issues that can be safely routed to remediation agents.

## Durable queue

```yaml
programme_state_version: 2
updated_at: 2026-08-05T16:25:00Z
status: validating
current_cycle: 1
current_domain: security-content-contract-lifecycle
active_task: docs/agents/tasks/active/OTERYN-20260805-security-content-contract-lifecycle-audit.md
branch: audit/20260805-security-content-contract-lifecycle
pull_request: pending
exact_head: pending-branch-head
last_merged_audit_head: 3f79987f47e5c7593daccdf1136e09d6641017de
last_completed_domain: implementation-ownership-lifecycle
coverage_inventory:
  baseline: docs/agents/evidence/OTERYN-20260803-portal-exhaustive-current-main-audit/
  baseline_merge: cbbd7613cee13cf01931a0ba0f7ac089122132e0
  latest_audited_main: 9635bf15f15ea4ab5fb229fd78f3312baad412bf
  selected_delta_domain: security-content-contract-lifecycle
finding_ledger:
  baseline_owners:
    - 486
    - 487
    - 488
    - 489
    - 490
    - 491
  current_cycle_findings:
    - OPA-SEC-0001: 547
    - OPA-GOV-0001: 552
    - OPA-GOV-0002: 555
    - OPA-GOV-0003: 558
    - OPA-GOV-0004: 561
    - OPA-GOV-0005: 562
    - OPA-GOV-0006: 565
    - OPA-GOV-0007: 566
    - OPA-GOV-0008: 567
    - OPA-GOV-0009: 570
    - OPA-GOV-0010: 571
    - OPA-GOV-0011: 573
    - OPA-GOV-0012: 574
    - OPA-GOV-0013: 575
    - OPA-GOV-0014: 576
    - OPA-GOV-0015: 579
open_material_findings: existing_owner_packages_plus_sixteen_current_cycle_findings
ready_remediation_issues:
  - 547
  - 555
  - 561
  - 562
  - 565
  - 566
  - 567
  - 570
  - 571
  - 573
  - 574
  - 575
  - 576
  - 579
blocked_findings:
  - 552
  - 558
proven:
  - PR #483 and its merged evidence are the authoritative existing module and observable-surface inventory.
  - OPA-SEC-0001 through OPA-GOV-0005 are proven and their audit tasks are archived.
  - OPA-GOV-0006 through OPA-GOV-0010 are proven and their audit task is archived.
  - OPA-GOV-0011 through OPA-GOV-0015 are proven in Issues #573, #574, #575, #576 and #579.
  - Wiki foundation, MFA QR enrollment, route-view-navigation inventory, content-scale evidence and public-endpoint contract tasks remain active despite terminal PRs, missing archives and retained branches.
  - Each concrete Issue owns only its historical task/archive pair and forbids product, workflow, operational and external-repository mutation.
derived:
  - Payment provider activation remains blocked until Issue #547 is remediated and independently verified.
  - The documented PR and exact-head validation process remains advisory until Issue #552 is resolved.
  - Stale task records remain schema-valid until Issue #558 adds live-state governance; concrete records can be repaired independently through ready Issues.
  - Completed bounded slices must be archived with their future-feature, parent-programme, staging and reachability nonclaims rather than retaining broad ownership.
unknown:
  - The owner-approved main ruleset, emergency bypass and stable required-check list.
  - The full count of historical active tasks whose PRs are already terminal or whose implementation and later proof phases are conflated.
conflicts:
  - ADR 0021 protects payment amount/currency integrity while the verified-event contract cannot carry or validate those facts.
  - Repository governance requires exact-head CI, audit, E2E and PR closeout while GitHub applies no main-branch enforcement.
  - Repository coordination treats task and Git state as authoritative while Agent Governance proves only local text validity.
  - Five current records claim completed security, Wiki, acceptance-harness or canonical contract ownership despite terminal PR state.
blockers: []
next_action: Validate and merge the security-content-contract lifecycle audit PR, archive its audit task, then continue the remaining active-task inventory against live PR, branch, archive and blocker state.
```

## Programme rules

- Keep this file compact; detailed evidence belongs in bounded task records, Issues and evidence indexes.
- Update it after a completed audit package, a material queue change, a new blocker, or before rotation.
- Never store secrets, full logs or copied Issue bodies here.
- Exactly one `next_action` is required while the programme is not terminal.
- A completed audit package is not the end of the programme; refresh the queue and continue within the bounded invocation budget.
