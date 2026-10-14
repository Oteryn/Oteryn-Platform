# CI acceptance coverage map

Read-only inventory of which workflow runs on which event, taken from `.github/workflows/*.yml` on `main` (2026-10-14). It records current behavior and changes none. Its purpose is to make the PR-critical scope and the post-merge `main` scope explicit before anyone proposes to narrow the full run on `main` or to move specialist checks into the Merge Queue.

## Events

| Event | What runs |
|---|---|
| `pull_request` to `main` | `ci.yml` (all PRs) plus every specialist workflow whose `paths` (or `paths-ignore`) match the diff. |
| `merge_group` | `ci.yml` only. No specialist workflow has a `merge_group` trigger. |
| `push` to `main` | `ci.yml` (all pushes) plus every specialist workflow whose `paths` match the merged diff. |
| `schedule` / `workflow_dispatch` | `acceptance-soak.yml` and `acceptance-stability.yml` (weekly, schedule only), `codeql.yml`, `historical-branch-audit.yml` and `github-actions-storage-hygiene.yml` (scheduled and also path-filtered on PRs), and manual runs. |

## Required aggregate

`platform-gate` (`ci.yml`) is the stable aggregate. It requires `classify-changes` and `test` to succeed, and `platform-postgres` and `php-coverage-report` to succeed or be skipped. `test` evaluates `runtime-tests` through `scripts/ci/required_test_gate.py`, which also accepts a classified reuse of a heavy run (`reuse_heavy`). The specialist workflows below are separate checks, not inputs of `platform-gate`.

## Scope by workflow group

| Group | Workflows | `pull_request` | `merge_group` | `push` to `main` |
|---|---|---|---|---|
| Core CI | `ci.yml` | all PRs | yes | all pushes |
| Portal and content acceptance | `acceptance-validation`, `portal-acceptance-contract`, `announcements-`, `community-data-`, `content-scale-`, `downloads-`, `editorial-media-`, `error-state-`, `events-`, `support-legal-`, `support-moderation-`, `wiki-reconciliation-acceptance` | path-filtered | no | same path filter |
| Runtime and contract | `playwright-runtime-validation`, `game-catalog-contract`, `game-gateway-ci`, `native-protocol-contract`, `native-protocol-contract-audits`, `codeql`, `agent-governance`, `branch-lifecycle`, `terminal-branch-lifecycle`, `parallel-coordinator-prompt-eval` | path-filtered | no | same path filter |
| Whole-tree validation | `phase7-production-like-validation`, `platform-db-outage-validation`, `game-auth-ticket-concurrency`, `edge-security-emulation` | every PR except `paths-ignore` (agent docs and templates) | no | not run |
| Deploy and edge | `synology-rollback-contract`, `character-bazaar-staging-validation`, `build-synology-staging-images`, `cloudflare-*`, `oteryn-public-edge-validation`, `synology-*` | path-filtered | no | mostly path-filtered, several not run |
| Scheduled soak | `acceptance-soak`, `acceptance-stability` | no | no | no (weekly cron) |

## Reading the map

- PR-critical scope: `ci.yml` plus the path-matched specialist workflows. They run on the PR head and are the only evidence for the specialist areas before merge.
- Main scope: the same path filters evaluated on the squash commit, plus all of `ci.yml`. The squash SHA differs from the `merge_group` candidate, so the `main` run is a fresh run, not a copy of the queue run.
- The Whole-tree validation group runs on PRs only. It has no `push` or `merge_group` trigger, so after a merge nothing re-runs it until the next PR.
- Specialist workflows have no `merge_group` trigger, so a Merge Queue candidate is not re-checked by them. Adding one requires the `merge_group` invocation and its required aggregation together; removing only the PR or `push` trigger leaves a gap.
- A specialist workflow that is path-filtered and not required does not block `platform-gate`; a skipped one is invisible to the aggregate.

## Not decided here

This map does not propose to limit the full run on `main`, to reuse PostgreSQL 16 or 17 evidence, to separate UI-only routing, or to change any required check. Each needs its own evidence (the review on #1610 lists what). No deploy, Synology or cutover file is part of this change.
