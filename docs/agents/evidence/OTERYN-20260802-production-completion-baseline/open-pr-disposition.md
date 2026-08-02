# Open pull-request disposition evidence

Observed: 2026-08-02  
Repository: `blakinio/Oteryn-Platform`

## Corrected inventory

The independent audit found that the original baseline omitted seven Dependabot PRs. Before PR #453 was created, the live queue contained **19** open PRs, not 11. Six reached intentional terminal states during this audit and 13 remain intentionally open.

## Terminal during this audit

| PR | Disposition | Reason |
|---:|---|---|
| #116 | `close_request_only` | Stale blocked task/index record; issue #114 remains the durable tracker. |
| #182 | `close_obsolete` | Historical Liquid20 retry request. |
| #189 | `close_obsolete` | Historical Liquid20 attempt/retry record. |
| #328 | `close_request_only` | Task/index-only rename request; issue #324 remains open for the actual ADR/contract. |
| #335 | `close_superseded` | Current `main` already enforces and verifies Synology restart policy. |
| #387 | `close_superseded` | Replaced by later production-gate and public-edge evidence. |

## Intentionally open dependency updates

All seven omitted PRs were created on 2026-07-27 and were non-mergeable against current `main` when audited. A Dependabot rebase was requested for each; no merge is permitted until exact-head validation completes.

| PR | Disposition | Exact next action |
|---:|---|---|
| #222 | `active_with_current_next_action` | Rebase; clean Composer install/audit and affected full CI for Laravel 13.22. |
| #223 | `active_with_current_next_action` | Rebase; PHPUnit compatibility and full CI. |
| #224 | `active_with_current_next_action` | Rebase; PHPStan/static-analysis and full CI. |
| #225 | `active_with_current_next_action` | Rebase; path-scoped Game Gateway CI for `actions/setup-go@v7`. |
| #226 | `active_with_current_next_action` | Rebase; prove Node 24/self-hosted-runner compatibility and affected Docker login workflows. |
| #227 | `active_with_current_next_action` | Rebase; review v8 artifact digest/decompression semantics and validate affected workflows. |
| #228 | `active_with_current_next_action` | Rebase; prove Node 24/self-hosted-runner compatibility and validate all 11 affected upload workflows. |
| #229 | `active_with_current_next_action` | Rebase; review removed inputs, prove runner compatibility and validate Synology image builds. |

## Intentionally open programme work

| PR | Disposition | Exact dependency/next action |
|---:|---|---|
| #338 | `blocked_required_with_exact_dependency` | Canary schema `1.3.0` producer compatibility and rollout order must be proven before merging the inactive consumer. |
| #381 | `active_with_current_next_action` | Complete the exact frozen Issue #365 mutable-checkout validation and reconcile audit evidence. |
| #391 | `active_with_current_next_action` | Continue the bounded official-client live-reference harness; official-service execution remains separately gated. |
| #405 | `blocked_required_with_exact_dependency` | Refresh private-production mail/queue/session/cache/backup/restore/observability and exact-deployment evidence. |
| #412 | `active_with_current_next_action` | Temporary Issue #365 Synology validator preflight; close without merge after terminal validation. |

## Rule

No PR is retained merely because it is old or as a reminder. Open state requires a current executable next action or an exact dependency. Every retained PR must be revisited at the next programme barrier.
