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
updated_at: 2026-08-05T15:15:00Z
status: ready
current_cycle: 1
current_domain: none
active_task: none
branch: none
pull_request: none
last_merged_audit_head: 75ce5c8c39be35c7271049d6deb7ee733c5f35f2
last_completed_domain: main-integrity-policy
coverage_inventory:
  baseline: docs/agents/evidence/OTERYN-20260803-portal-exhaustive-current-main-audit/
  baseline_merge: cbbd7613cee13cf01931a0ba0f7ac089122132e0
  latest_audited_main: a7eb03d49e328e8115adb54e772c9c8366b737d3
  audited_delta_commits: 39
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
open_material_findings: existing_owner_packages_plus_two_current_cycle_findings
ready_remediation_issues:
  - 547
blocked_findings:
  - 552
proven:
  - PR #483 and its merged evidence are the authoritative existing module and observable-surface inventory.
  - OPA-SEC-0001 is proven and deduplicated in Issue #547; its audit task is archived.
  - OPA-GOV-0001 is proven and deduplicated in Issue #552.
  - PR #553 passed all six exact-head workflows and merged as 75ce5c8c39be35c7271049d6deb7ee733c5f35f2.
  - The main-integrity policy audit task is archived and all audit ownership is released by the lifecycle closeout PR.
derived:
  - Payment provider activation remains blocked until Issue #547 is remediated and independently verified.
  - The documented PR and exact-head validation process remains advisory until Issue #552 is resolved.
unknown:
  - The owner-approved main ruleset, emergency bypass and stable required-check list.
  - The exact count and terminal validity of historical files still retained under docs/agents/tasks/active.
conflicts:
  - ADR 0021 protects payment amount/currency integrity while the verified-event contract cannot carry or validate those facts.
  - Repository governance requires exact-head CI, audit, E2E and PR closeout while GitHub applies no main-branch enforcement.
blockers: []
next_action: Refresh active-task and related PR state, then audit whether completed or superseded historical tasks remain falsely active without live ownership.
```

## Programme rules

- Keep this file compact; detailed evidence belongs in bounded task records, Issues and evidence indexes.
- Update it after a completed audit package, a material queue change, a new blocker, or before rotation.
- Never store secrets, full logs or copied Issue bodies here.
- Exactly one `next_action` is required while the programme is not terminal.
- A completed audit package is not the end of the programme; refresh the queue and continue within the bounded invocation budget.
