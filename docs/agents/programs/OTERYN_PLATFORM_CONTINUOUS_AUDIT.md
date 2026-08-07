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
updated_at: 2026-08-07T10:07:00Z
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
last_merged_audit_head: bf16812e4720fdd90a2483a048c2706592f662d8
last_completed_domain: payment-partial-refund-integrity
coverage_inventory:
  baseline: docs/agents/evidence/OTERYN-20260803-portal-exhaustive-current-main-audit/
  baseline_merge: cbbd7613cee13cf01931a0ba0f7ac089122132e0
  latest_audited_main: befb5ca3d148ffdb0c582c26d06a6f644367e5be
  selected_delta_domain: payment-partial-refund-integrity
finding_ledger_semantics: historical_identity_map_not_live_queue
finding_ledger:
  baseline_owners: [486, 487, 488, 489, 490, 491]
  current_cycle_findings:
    - OPA-SEC-0001: 547
    - OPA-SEC-0002: 797
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
    - OPA-GOV-0021: 788
    - OPA-GOV-0022: 793
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
  - OPA-SEC-0001 is historically proven in Issue #547 and repaired through merged PR #595; current verified provider events carry authenticated currency/amount and enforce per-event settlement matching.
  - OPA-SEC-0002 is proven in Issue #797: after the first partial refund, a distinct later partial-refund event can become duplicate_state NOOP while its authenticated refund amount is not accumulated or durably represented.
  - Findings OPA-GOV-0001 through OPA-GOV-0015 are proven and retain their durable identities in the finding ledger.
  - OPA-GOV-0016 is proven in Issue #582: completed Game Catalog programme-registration audit PR #331 remained falsely active while programme #330 correctly continued.
  - OPA-GOV-0017 is proven in Issue #583: completed schema 1.3 architecture proposal PR #332 remained falsely active while downstream PR #338 consumed the contract.
  - OPA-GOV-0018 is proven in Issue #584: completed Cloudflare audit implementation/evidence PRs #409 and #415 retained workflow and tooling ownership while denied live reads remained a legitimate blocker.
  - OPA-GOV-0019 is proven in Issue #780 and repaired through PR #789; the stale-snapshot deletion gap is historical identity evidence.
  - OPA-GOV-0020 is proven in Issue #783: docs-only main pushes force all CI gates and full Acceptance, and a newer docs-only main generation cancelled the prior in-progress Acceptance run.
  - OPA-GOV-0021 is proven in Issue #788: tasks with pr none and an existing branch bypass live PR discovery and are treated as active BRANCH_ONLY ownership even when matching PR state exists.
  - OPA-GOV-0022 is historically proven in Issue #793 and repaired through PR #796 using a remote force-with-lease deletion boundary; Issue #793 is closed completed and its repair task is archived through PR #798.
  - Audit PR #781 passed exact-head CI and Agent Governance and merged as f72fafd461f6bd2f41c5a58b975a5532f8e426ef; its audit task is archived.
  - Audit PR #784 passed exact-head CI run 31164308992 and Agent Governance run 31164310591 and merged as 8478b627609f9d82799bc5866c8ba504d5751f19; its audit task is archived.
  - Audit PR #790 passed exact-head CI run 31165266121 and Agent Governance run 31165266632 and merged as 26a92a5d49b86fb121cebd2cbd57525c3a3140ad; its audit task is archived.
  - Audit PR #794 passed exact-head CI run 31167549465 and Agent Governance run 31167550571 and merged as 67cbe391967ee7fd2bf26e4eda412820b805f981; its audit task is archived through PR #795.
  - Audit PR #799 passed Agent Governance run 31168550882 on exact head 58e64dba046811d8b837ef61fc390fa7e306f73e; protected merge accepted that exact head as bf16812e4720fdd90a2483a048c2706592f662d8 after the repository-required merge contexts were satisfied.
  - Issue #558 is closed completed and current main contains the live active-task liveness enforcement introduced by PR #779; OPA-GOV-0021 records a bounded omitted-PR reconciliation gap in that implementation.
  - Issues #555, #561 and #562 are terminal completed through their recorded lifecycle closeouts.
  - Independent PASS-only validation is governed by docs/agents/LIFECYCLE_CLOSEOUT_BATCHING.md and is recorded on the existing target PR rather than a new audit PR.
derived:
  - Payment amount/currency contract support is no longer the current payment-integrity conflict; the remaining proven payment blocker is repeated/cumulative partial-refund accounting in OPA-SEC-0002.
  - Issue #321 must remain pre-production until OPA-SEC-0002 and its separate real-provider/customer-facing acceptance gates are terminally satisfied.
  - OPA-GOV-0022 is no longer a live branch-lifecycle blocker after PR #796; future branch-lifecycle auditing must use current main rather than the historical finding body.
  - Compatible lifecycle-only findings should be handed to one bounded reconciliation wave instead of generating one closeout PR and one audit Issue per task.
  - Docs-only heavy-workflow economy currently holds at pull-request time but not after merge to main; main-push routing must preserve the same risk classification without suppressing required product-path validation.
  - Live task liveness must discover PR truth from branch identity when the task omits a PR number, while preserving genuine pre-PR branches and exact branch-reuse semantics.
  - A refreshed timestamp or programme version must never preserve a stale exhaustive queue; mutable queue fields remain explicitly live-query-derived.
unknown:
  - The owner-approved emergency bypass and complete stable required-check set beyond the currently observable protected contexts.
  - Current open, ready, blocked and actively owned findings until live repository queries are executed.
conflicts:
  - ADR 0021 and parent Issue #321 require durable partial-refund financial truth and refund lifecycle evidence, while the current state-only model can consume a distinct second partial refund as duplicate_state without preserving its amount.
  - Completed baseline Issue #452 requires docs/task/metadata-only changes not to run unrelated heavy browser/container/application gates, while current main-push CI and Acceptance routing has the OPA-GOV-0020 gap until its live repair is terminally verified.
  - Issue #558 requires branch/PR/task identity reconciliation and terminal retained-branch classification, while the branch-only path can bypass PR reconciliation when pr is none.
blockers:
  mode: live_query_required
  items: unknown
next_action: Refresh live ownership, open and blocked Issues, active tasks, PRs and recent main deltas in the next invocation, then select the next highest-risk non-overlapping audit domain.
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
