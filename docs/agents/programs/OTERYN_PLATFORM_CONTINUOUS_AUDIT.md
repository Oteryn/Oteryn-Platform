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
updated_at: 2026-08-05T15:52:00Z
status: validating
current_cycle: 1
current_domain: public-module-stale-task-lifecycle
active_task: docs/agents/tasks/active/OTERYN-20260805-public-modules-stale-tasks-audit.md
branch: audit/20260805-public-modules-stale-tasks
pull_request: 563
head_before_programme_checkpoint: adee1581612c7901b4ac85cb52ec35f84d2b35f0
last_merged_audit_head: bb6d2d86ffe418c20f11995b8abb9ec38c5dc49b
last_completed_domain: agent-governance-live-task-liveness
coverage_inventory:
  baseline: docs/agents/evidence/OTERYN-20260803-portal-exhaustive-current-main-audit/
  baseline_merge: cbbd7613cee13cf01931a0ba0f7ac089122132e0
  latest_audited_main: 86cd5cccb47ebfbe1a77e65c2ba8b6d912acfcc5
  selected_delta_domain: public-module-stale-task-lifecycle
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
open_material_findings: existing_owner_packages_plus_six_current_cycle_findings
ready_remediation_issues:
  - 547
  - 555
  - 561
  - 562
blocked_findings:
  - 552
  - 558
proven:
  - PR #483 and its merged evidence are the authoritative existing module and observable-surface inventory.
  - OPA-SEC-0001 is proven and deduplicated in Issue #547; its audit task is archived.
  - OPA-GOV-0001 is proven and deduplicated in Issue #552; its audit task is archived.
  - OPA-GOV-0002 is proven and deduplicated in Issue #555; its audit task is archived.
  - OPA-GOV-0003 is proven and deduplicated in Issue #558; its audit task is archived.
  - OPA-GOV-0004 is proven and deduplicated in Issue #561.
  - OPA-GOV-0005 is proven and deduplicated in Issue #562.
  - Announcements/Events PR #157 and Download Center PR #161 are terminal while their task records remain active, archives are absent and branches remain.
  - Issues #561 and #562 are independent concrete cleanup owners and do not overlap product modules or systemic Issue #558 tooling paths.
  - Draft PR #563 contains only the bounded audit task, evidence, report and programme-state paths.
derived:
  - Payment provider activation remains blocked until Issue #547 is remediated and independently verified.
  - The documented PR and exact-head validation process remains advisory until Issue #552 is resolved.
  - Stale task records remain schema-valid until Issue #558 adds live-state governance; concrete false-active records can be cleaned independently through Issues #555, #561 and #562.
unknown:
  - The owner-approved main ruleset, emergency bypass and stable required-check list.
  - The full count of historical active tasks whose PRs are already terminal.
conflicts:
  - ADR 0021 protects payment amount/currency integrity while the verified-event contract cannot carry or validate those facts.
  - Repository governance requires exact-head CI, audit, E2E and PR closeout while GitHub applies no main-branch enforcement.
  - Repository coordination treats task and Git state as authoritative while Agent Governance proves only local text validity.
  - Announcements/Events and Download Center claim active ownership and actions contradicted by terminal PR state.
blockers: []
next_action: Verify all emitted workflows and final review hygiene on PR #563, squash-merge it, archive the audit task, then continue the remaining active-task inventory against live PR, branch and archive state.
```

## Programme rules

- Keep this file compact; detailed evidence belongs in bounded task records, Issues and evidence indexes.
- Update it after a completed audit package, a material queue change, a new blocker, or before rotation.
- Never store secrets, full logs or copied Issue bodies here.
- Exactly one `next_action` is required while the programme is not terminal.
- A completed audit package is not the end of the programme; refresh the queue and continue within the bounded invocation budget.
