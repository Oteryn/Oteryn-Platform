---
programme_id: OTERYN_PLATFORM_CONTINUOUS_AUDIT
programme_version: 3
canonical_prompt: docs/agents/prompts/OTERYN_PLATFORM_CONTINUOUS_AUDIT_PROGRAM.md
required_reads:
  - docs/agents/OTERYN_PLATFORM_PROGRAM_SCOPE.md
  - docs/agents/LIFECYCLE_CLOSEOUT_BATCHING.md
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
programme_state_version: 3
updated_at: 2026-08-07T08:44:00Z
status: ready
current_cycle: 1
programme_execution_snapshot:
  mode: live_query_required
  exhaustive: false
  current_domain: unknown
  active_task: unknown
  branch: unknown
  pull_request: unknown
  reason: Mutable execution ownership must be resolved from live tasks, branches, Issues and PRs at invocation time; unknown must never be interpreted as none.
last_merged_audit_head: f72fafd461f6bd2f41c5a58b975a5532f8e426ef
last_completed_domain: branch-lifecycle-deletion-safety
coverage_inventory:
  baseline: docs/agents/evidence/OTERYN-20260803-portal-exhaustive-current-main-audit/
  baseline_merge: cbbd7613cee13cf01931a0ba0f7ac089122132e0
  latest_audited_main: 17f4d5a0de3f029c036df61d326e369cc53bb0ef
  selected_delta_domain: main-push-ci-routing
finding_ledger_semantics: historical_identity_map_not_live_queue
finding_ledger:
  baseline_owners: [486, 487, 488, 489, 490, 491]
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
    - OPA-GOV-0016: 582
    - OPA-GOV-0017: 583
    - OPA-GOV-0018: 584
    - OPA-GOV-0019: 780
    - OPA-GOV-0020: 783
live_queue:
  mode: live_query_required
  exhaustive: false
  open_material_findings: unknown
  ready_remediation_issues: unknown
  blocked_findings: unknown
  ready_query: 'repo:blakinio/Oteryn-Platform is:issue is:open label:programme:platform label:programme:audit-repair label:agent:ready'
  blocked_query: 'repo:blakinio/Oteryn-Platform is:issue is:open label:programme:platform label:programme:audit-repair label:state:blocked'
  active_task_path: docs/agents/tasks/active/
  reason: The historical finding ledger preserves stable identities only. Open, ready, blocked and owned state must be recomputed from live repository evidence and cannot be inferred from this file.
proven:
  - PR #483 and its merged evidence are the authoritative existing module and observable-surface inventory.
  - Findings OPA-SEC-0001 through OPA-GOV-0010 are proven and their audit tasks are archived.
  - Findings OPA-GOV-0011 through OPA-GOV-0015 are proven; PR #580 merged as 42a3725f3ad6d4c6863aa15049aa2a8264ab24f9 and its audit task is archived.
  - OPA-GOV-0016 is proven in Issue #582: completed Game Catalog programme-registration audit PR #331 remains falsely active while programme #330 correctly continues.
  - OPA-GOV-0017 is proven in Issue #583: completed schema 1.3 architecture proposal PR #332 remains falsely active while downstream PR #338 consumes the contract.
  - OPA-GOV-0018 is proven in Issue #584: completed Cloudflare audit implementation/evidence PRs #409 and #415 retain workflow and tooling ownership while denied live reads remain a legitimate blocker.
  - OPA-GOV-0019 is proven in Issue #780: destructive branch cleanup validates one snapshot and later deletes refs by name without per-entry live SHA/PR/claim/protection revalidation.
  - OPA-GOV-0020 is proven in Issue #783: docs-only main pushes force all CI gates and full Acceptance, and a newer docs-only main generation cancelled the prior in-progress Acceptance run.
  - Audit PR #781 passed exact-head CI and Agent Governance on a0ba255e721c040c7fcfaaaae8e3593f3fd7557a and merged as f72fafd461f6bd2f41c5a58b975a5532f8e426ef.
  - PR #589 passed all six exact-head workflows on d157341c9ca8fd29c8f2a5e2bbf202fc813ebc1a and merged as 5bb9bf8588dbbb76bba83a8d35a32dea0ffef40b.
  - The branch-lifecycle-deletion-safety audit is archived and owns no paths or leases after its closeout merges.
  - PR #591 was an accidental post-merge duplicate and is closed obsolete with no unique changes.
  - Issue #547 is closed completed after repair PR #595 and independent audit #597; it is not a live remediation blocker.
  - Issues #555, #561 and #562 are terminal completed through merged lifecycle closeout PRs #598, #670 and #601 respectively.
  - Independent PASS-only validation is governed by docs/agents/LIFECYCLE_CLOSEOUT_BATCHING.md and is recorded on the existing target PR rather than a new audit PR.
derived:
  - Any remaining payment activation gate must be resolved from current architecture, security and live Issue state; Issue #547 itself is no longer a blocker.
  - The documented PR and exact-head validation process remains advisory until Issue #552 is resolved.
  - Stale task records remain schema-valid until Issue #558 adds live-state governance.
  - Completed programme setup and contract-proposal tasks must release ownership without terminating their active programme or downstream consumers.
  - Completed audit tooling must release code/workflow ownership while preserving permission-dependent UNKNOWN evidence in a narrow blocked verification record.
  - Compatible lifecycle-only findings should be handed to one bounded reconciliation wave instead of generating one closeout PR and one audit Issue per task.
  - Destructive ref cleanup requires a live guard at deletion time; a reviewed snapshot cannot safely authorize later name-only deletion after mutable branch state changes.
  - Docs-only heavy-workflow economy currently holds at pull-request time but not after merge to main; main-push routing must preserve the same risk classification without suppressing required product-path validation.
  - A refreshed timestamp or programme version must never preserve a stale exhaustive queue; mutable queue fields remain explicitly live-query-derived.
unknown:
  - The owner-approved main ruleset, emergency bypass and stable required-check list.
  - Current open, ready, blocked and actively owned findings until live repository queries are executed.
conflicts:
  - ADR 0021 protects payment amount/currency integrity while the verified-event contract cannot carry or validate those facts.
  - Repository governance requires exact-head CI, audit, E2E and PR closeout while GitHub applies no main-branch enforcement.
  - Agent Governance proves local text validity but not live PR, branch, archive or ownership truth.
  - ADR 0024 requires active/open/ambiguous branches to fail closed, while the current destructive apply loop has a time-of-check/time-of-use gap before ref deletion.
  - Completed baseline Issue #452 requires docs/task/metadata-only changes not to run unrelated heavy browser/container/application gates, while current main-push CI and Acceptance routing does so.
  - Game Catalog and Cloudflare setup tasks remain active despite terminal setup/evidence PRs and newer programme, consumer or blocker ownership.
blockers:
  mode: live_query_required
  items: unknown
next_action: Refresh live ownership, open and blocked Issues, active tasks, PRs and recent main deltas; reconcile terminal findings; then select the highest-risk non-overlapping audit domain and route compatible lifecycle-only findings into a bounded batch handoff.
```

## Programme rules

- Keep this file compact; detailed evidence belongs in bounded task records, Issues and evidence indexes.
- Treat `finding_ledger` as a historical identity map only; never use it as proof that an Issue is currently open, ready, blocked or unclaimed.
- Resolve mutable queue and ownership state from live Issues, tasks, branches and PRs before dispatch or collision decisions.
- A PASS-only independent audit submits a review/comment on the exact existing target PR and updates its linked audit record; it does not create an audit PR.
- Several compatible lifecycle-only findings may use one batch audit Issue and one exact-head review with per-item verdicts under `docs/agents/LIFECYCLE_CLOSEOUT_BATCHING.md`.
- Material product findings remain independently actionable and must not be grouped merely to reduce PR count.
- Update this file after a durable programme-policy or finding-identity change; do not persist a mutable queue snapshot as authoritative live truth.
- Never store secrets, full logs or copied Issue bodies here.
- Exactly one `next_action` is required while the programme is not terminal.
- A completed package is not the end of the programme; refresh the queue and continue within the bounded invocation budget.
