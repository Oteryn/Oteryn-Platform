---
task_id: OTERYN-20260814-public-today-architecture
mode: architecture
issue: 1049
status: completed
programme: OTERYN_PORTAL_COMPLETION
project_lane: oteryn-platform-core
phase: closeout
execution_mode: github_connector
---

# OTERYN-20260814-public-today-architecture

## Goal

Define one focused canonical `PublicPortal Today` architecture that turns accepted ADR 0032 composition/privacy rules into an implementation-ready public guest slice without creating a new data authority or leaking owner-private state through shared caching.

## Final result

- Focused canonical architecture: `docs/architecture/PUBLIC_PORTAL_TODAY_ARCHITECTURE.md`.
- Architecture routing updated in `docs/architecture/ARCHITECTURE_AUTHORITY.md`.
- ADR 0032 remains the durable authority; no duplicate ADR was allocated.
- First implementation slice remains `PUBLIC_GUEST`; owner-private composition remains behind the full ADR 0032 cache/security gate.
- LiveOps-free implementation acceptance was repaired to require truthful unavailable/not-yet-provided behavior instead of fabricated stale/recovery evidence.
- Runtime/browser E2E for this architecture-only task: `NOT_APPLICABLE`.

## Exact implementation evidence

```yaml
implementation_pr: 1055
implementation_head: 831735add0138459894c3b5802fbacd61faf1491
merge_commit: 83507a34a0e72be98a6147dfb8f8cf6f62f21982
issue_1049: CLOSED_COMPLETED
exact_head_ci:
  ci_run: 31812988959
  agent_governance_run: 31812989034
  native_protocol_contract_run: 31812988913
  native_protocol_contract_audits_run: 31812988907
  edge_security_emulation_run: 31812988831
  game_auth_ticket_concurrency_run: 31812988848
  platform_db_outage_validation_run: 31812988843
  phase7_production_like_validation_run: 31812988887
  result: PASS
full_diff_self_review:
  review_id: 4938575832
  result: PASS
review_threads: RESOLVED
runtime_browser_e2e: NOT_APPLICABLE
```

## Source branch closeout

```yaml
source_branch_disposition: auto_delete_after_merge
source_branch_reason: the dedicated PublicPortal Today architecture branch has no continuing ownership or recovery purpose after PR #1055 merged and the task was archived
source_branch_evidence: live branch search confirms the PublicPortal Today source branch is absent, while repository metadata confirms delete_branch_on_merge=true; immutable PR #1055 and this archive retain the delivery provenance
```

## Closeout

The implementation PR was squash-merged only after synchronization with the then-current protected `main` and a fresh exact-head validation generation. Issue #1049 closed with reason `completed`. This archive move releases active task ownership; no runtime, deployment, protected-environment, external-repository or production activation work is performed by this closeout.
