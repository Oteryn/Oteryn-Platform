---
programme_id: OTERYN_PLATFORM_CONTINUOUS_AUDIT
programme_version: 1
canonical_prompt: docs/agents/prompts/OTERYN_PLATFORM_CONTINUOUS_AUDIT_PROGRAM.md
required_reads:
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
programme_state_version: 1
updated_at: 2026-08-05T13:45:00Z
status: ready
current_cycle: 1
current_domain: none
active_task: none
branch: none
pull_request: none
exact_head: main-at-next-invocation
last_completed_domain: none
coverage_inventory: not_created
finding_ledger: not_created
open_material_findings: unknown
ready_remediation_issues: unknown
blocked_findings: unknown
proven:
  - The canonical audit prompt and Issue taxonomy are repository-backed.
derived:
  - The first cycle must establish a live module/surface inventory before selecting its first bounded audit package.
unknown:
  - Current whole-platform audit coverage and exact unaudited domains.
conflicts: []
blockers: []
next_action: Build the current module and observable-surface inventory from main, deduplicate existing audit work, then select the highest-risk unaudited domain for one bounded audit task.
```

## Programme rules

- Keep this file compact; detailed evidence belongs in bounded task records, Issues and evidence indexes.
- Update it after a completed audit package, a material queue change, a new blocker, or before rotation.
- Never store secrets, full logs or copied Issue bodies here.
- Exactly one `next_action` is required while the programme is not terminal.
- A completed audit package is not the end of the programme; refresh the queue and continue within the bounded invocation budget.
