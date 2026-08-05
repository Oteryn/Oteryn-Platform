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
updated_at: 2026-08-05T15:43:00Z
status: ready
current_cycle: 1
current_domain: none
active_task: none
branch: none
pull_request: none
last_merged_audit_head: bb6d2d86ffe418c20f11995b8abb9ec38c5dc49b
last_completed_domain: agent-governance-live-task-liveness
coverage_inventory:
  baseline: docs/agents/evidence/OTERYN-20260803-portal-exhaustive-current-main-audit/
  baseline_merge: cbbd7613cee13cf01931a0ba0f7ac089122132e0
  latest_audited_main: 968c8adc912beef0119da21a345b0afadc45a494
  audited_delta_commits: 43
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
open_material_findings: existing_owner_packages_plus_four_current_cycle_findings
ready_remediation_issues:
  - 547
  - 555
blocked_findings:
  - 552
  - 558
proven:
  - PR #483 and its merged evidence are the authoritative existing module and observable-surface inventory.
  - OPA-SEC-0001 is proven and deduplicated in Issue #547; its audit task is archived.
  - OPA-GOV-0001 is proven and deduplicated in Issue #552; its audit task is archived.
  - OPA-GOV-0002 is proven and deduplicated in Issue #555; its audit task is archived.
  - OPA-GOV-0003 is proven and deduplicated in Issue #558.
  - PR #559 passed all six exact-head workflows and merged as bb6d2d86ffe418c20f11995b8abb9ec38c5dc49b.
  - The Agent Governance liveness audit is archived and all audit ownership is released by the lifecycle closeout PR.
  - Announcements/Events and Download Center remain proven false-active tasks with merged PRs, missing archives and retained branches.
derived:
  - Payment provider activation remains blocked until Issue #547 is remediated and independently verified.
  - The documented PR and exact-head validation process remains advisory until Issue #552 is resolved.
  - Stale task records remain schema-valid until Issue #558 adds live-state governance; concrete records still require bounded cleanup owners.
unknown:
  - The owner-approved main ruleset, emergency bypass and stable required-check list.
  - The full count of historical active tasks whose PRs are already terminal.
conflicts:
  - ADR 0021 protects payment amount/currency integrity while the verified-event contract cannot carry or validate those facts.
  - Repository governance requires exact-head CI, audit, E2E and PR closeout while GitHub applies no main-branch enforcement.
  - Repository coordination treats task and Git state as authoritative while Agent Governance proves only local text validity.
blockers: []
next_action: Audit and deduplicate concrete cleanup ownership for the proven false-active Announcements/Events and Download Center task records without changing their product modules.
```

## Programme rules

- Keep this file compact; detailed evidence belongs in bounded task records, Issues and evidence indexes.
- Update it after a completed audit package, a material queue change, a new blocker, or before rotation.
- Never store secrets, full logs or copied Issue bodies here.
- Exactly one `next_action` is required while the programme is not terminal.
- A completed audit package is not the end of the programme; refresh the queue and continue within the bounded invocation budget.
