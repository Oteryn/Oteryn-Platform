# Open pull-request disposition evidence

Observed: 2026-08-02  
Repository: `blakinio/Oteryn-Platform`

## Terminal during this audit

| PR | Terminal state | Reason |
|---:|---|---|
| #182 | closed, not merged | Obsolete historical Liquid20 retry request. |
| #189 | closed, not merged | Obsolete historical Liquid20 attempt/retry record. |
| #335 | closed, not merged | Superseded by current `main`: persistent services already use `restart: always`; `.github/workflows/repair-synology-autostart.yml` enforces, starts and verifies the exact runner/service set. |

## Candidate terminal cleanup

| PR | Proposed disposition | Evidence required/completed |
|---:|---|---|
| #116 | `close_request_only` | Contains only a stale blocked task/index record for issue #114. The scheduled dates have passed; run history must be checked through a fresh task rather than preserving an unmergeable stale task PR. Closing preserves the historical checkpoint and leaves issue #114 open. |
| #328 | `close_request_only` | Contains only task/index/project-state changes and no required rename ADR/contract. Issue #324 remains the authoritative unfinished deliverable. A fresh discovery branch should start from current `main` when execution resumes. |
| #387 | `close_superseded` | Earlier public-domain validation is incorporated and superseded by later production-gate evidence in PR #405 plus merged public-domain repair/edge-audit work on `main`. |

## Intentionally open

| PR | Disposition | Exact dependency/next action |
|---:|---|---|
| #225 | `merge_ready_after_gate` | Dependabot rebase/recreation, then path-filtered Game Gateway CI on the new exact head. |
| #338 | `blocked_required_with_exact_dependency` | Canary schema `1.3.0` producer compatibility and rollout order must be proven before merging the inactive Platform consumer. |
| #381 | `active_with_current_next_action` | Complete the exact frozen Issue #365 mutable-checkout validation and reconcile the current audit evidence. |
| #391 | `active_with_current_next_action` | Continue the bounded official-client live-reference harness; no official-service execution until its explicit safety gate. |
| #405 | `blocked_required_with_exact_dependency` | Refresh the production/private-runtime evidence after current programme decisions; production mail/queue/session/cache/backup/restore/observability and exact deployment proof remain unresolved. |
| #412 | `active_with_current_next_action` | Temporary Issue #365 Synology validator preflight; must be closed without merge after the validator task reaches a terminal result. |

## Rule

No PR is kept open merely as a reminder. Open state is justified only by a current executable next action or an exact required dependency. Historical request-only task PRs are closed and their issues remain the durable trackers.
