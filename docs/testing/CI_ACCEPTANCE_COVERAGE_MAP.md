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
| Portal audit (PR only) | `portal-e2e-audit` | path-filtered (its own workflow file) | no | not run after merge |
| Runtime and contract | `playwright-runtime-validation`, `tibia-linux-live-reference` (PR only, no push), `game-catalog-contract`, `game-gateway-ci`, `native-protocol-contract`, `native-protocol-contract-audits`, `codeql`, `agent-governance`, `branch-lifecycle`, `terminal-branch-lifecycle`, `parallel-coordinator-prompt-eval`, `historical-branch-audit` (also weekly cron) | path-filtered | no | same path filter |
| Whole-tree validation | `phase7-production-like-validation`, `platform-db-outage-validation`, `game-auth-ticket-concurrency`, `edge-security-emulation` | every PR except `paths-ignore` (agent docs and templates) | no | not run |
| Deploy and edge | `synology-rollback-contract`, `character-bazaar-staging-validation`, `build-synology-staging-images`, `cloudflare-*`, `oteryn-public-edge-validation`, `synology-container-hygiene`, `synology-production-target-preflight` (PR only), `synology-diagnostics` (push and manual only, never a PR) | path-filtered | no | not run, same filter, or a narrower trigger-file filter (see below) |
| Scheduled soak | `acceptance-soak`, `acceptance-stability` | no | no | no (weekly cron) |

## Reading the map

- PR-critical scope: `ci.yml` plus the path-matched specialist workflows. They run on the PR head and are the only evidence for the specialist areas before merge.
- Main scope: `ci.yml` in full, plus the specialist workflows whose `push` filter matches the squash commit. The squash SHA differs from the `merge_group` candidate, so the `main` run is a fresh run, not a copy of the queue run. `push` filters equal the `pull_request` filters for most specialist workflows, but not for four: `cloudflare-oteryn-hsts-stage1` and `cloudflare-oteryn-public-edge-repair` (PR: workflow, implementation, tests and docs; push: only their `ops/triggers/*.md` file), `cloudflare-oteryn-endpoint-main-operation` (3 PR paths, 1 push path) and `oteryn-public-edge-validation` (6 PR paths, 1 push path). Others run on PRs only (see the table), and several deploy workflows have a `push` trigger without a `pull_request` one. Read each workflow's own `on:` block before relying on a `main` run.
- The Whole-tree validation group runs on PRs only. It has no `push` or `merge_group` trigger, so after a merge nothing re-runs it until the next PR.
- Specialist workflows have no `merge_group` trigger, so a Merge Queue candidate is not re-checked by them. Adding one requires the `merge_group` invocation and its required aggregation together; removing only the PR or `push` trigger leaves a gap.
- A specialist workflow that is path-filtered and not required does not block `platform-gate`; a skipped one is invisible to the aggregate.

## Complete per-workflow triggers

Generated from the `on:` block of every file in `.github/workflows/` on `main`. `paths(n)` is a path filter with n entries; `narrower` means the `push` filter is a strict subset of the `pull_request` filter. This table, not the group summary above, is authoritative when the two differ. The last column is `workflow_dispatch` (`no (reusable)` marks `workflow_call` files).

| Workflow | `pull_request` | `merge_group` | `push` to `main` | `schedule` | Manual |
|---|---|---|---|---|---|
| `acceptance-soak.yml` | no | no | no | weekly/cron | yes |
| `acceptance-stability.yml` | no | no | no | weekly/cron | yes |
| `acceptance-validation.yml` | paths(11) | no | paths(11) (same) | no | yes |
| `agent-governance.yml` | paths(7) | no | paths(7) (same) | no | yes |
| `announcements-acceptance.yml` | paths(13) | no | paths(13) (same) | no | yes |
| `branch-lifecycle.yml` | paths(6) | no | paths(6) (same) | no | yes |
| `build-synology-staging-images.yml` | paths(74) | no | paths(74) (same) | no | yes |
| `character-bazaar-staging-control.yml` | no | no | paths(5) | no | yes |
| `character-bazaar-staging-validation.yml` | paths(9) | no | paths(9) (same) | no | yes |
| `ci.yml` | all | all | all | no | no |
| `cloudflare-oteryn-edge-audit.yml` | paths(9) | no | no | no | no |
| `cloudflare-oteryn-endpoint-main-operation.yml` | paths(3) | no | paths(1) (narrower) | no | no |
| `cloudflare-oteryn-endpoints.yml` | paths(7) | no | no | no | yes |
| `cloudflare-oteryn-hsts-stage1.yml` | paths(5) | no | paths(1) (narrower) | no | no |
| `cloudflare-oteryn-public-edge-repair.yml` | paths(5) | no | paths(1) (narrower) | no | no |
| `cloudflare-zone-edge-audit.yml` | paths(7) | no | paths(7) (same) | no | yes |
| `codeql.yml` | paths(14) | no | paths(14) (same) | weekly/cron | yes |
| `community-data-acceptance.yml` | paths(31) | no | paths(31) (same) | no | yes |
| `content-scale-acceptance.yml` | paths(36) | no | paths(36) (same) | no | yes |
| `deploy-synology-staging.yml` | no | no | no | no | yes |
| `downloads-acceptance.yml` | paths(18) | no | paths(18) (same) | no | yes |
| `edge-security-emulation.yml` | all except 5 ignored | no | no | no | yes |
| `editorial-media-acceptance.yml` | paths(14) | no | paths(14) (same) | no | yes |
| `error-state-acceptance.yml` | paths(17) | no | paths(17) (same) | no | yes |
| `events-acceptance.yml` | paths(10) | no | paths(10) (same) | no | yes |
| `game-auth-ticket-concurrency.yml` | all except 5 ignored | no | no | no | no |
| `game-catalog-contract.yml` | paths(10) | no | paths(10) (same) | no | yes |
| `game-gateway-ci.yml` | paths(2) | no | paths(2) (same) | no | no |
| `github-actions-storage-hygiene.yml` | paths(3) | no | no | weekly/cron | yes |
| `historical-branch-audit.yml` | paths(11) | no | paths(11) (same) | weekly/cron | yes |
| `native-protocol-contract-audits.yml` | paths(8) | no | paths(8) (same) | no | no |
| `native-protocol-contract.yml` | paths(5) | no | paths(5) (same) | no | no |
| `oteryn-public-edge-validation.yml` | paths(6) | no | paths(1) (narrower) | no | no |
| `parallel-coordinator-prompt-eval.yml` | paths(3) | no | paths(3) (same) | no | yes |
| `phase7-production-like-validation.yml` | all except 5 ignored | no | no | no | yes |
| `platform-db-outage-validation.yml` | all except 5 ignored | no | no | no | yes |
| `playwright-runtime-validation.yml` | paths(8) | no | paths(8) (same) | no | yes |
| `portal-acceptance-contract.yml` | paths(17) | no | paths(17) (same) | no | yes |
| `portal-e2e-audit.yml` | paths(1) | no | no | no | yes |
| `recover-synology-staging-schema.yml` | no | no | no | no | yes |
| `repair-synology-autostart.yml` | no | no | paths(3) | no | yes |
| `repair-synology-compose-orphans.yml` | no | no | no | no | yes |
| `support-legal-acceptance.yml` | paths(23) | no | paths(23) (same) | no | yes |
| `support-moderation-acceptance.yml` | paths(24) | no | paths(24) (same) | no | yes |
| `synology-container-hygiene.yml` | paths(5) | no | no | no | yes |
| `synology-diagnostics.yml` | no | no | paths(1) | no | yes |
| `synology-production-target-preflight.yml` | paths(8) | no | no | no | yes |
| `synology-rollback-contract.yml` | paths(11) | no | paths(11) (same) | no | yes |
| `terminal-branch-lifecycle-read-reusable.yml` | no | no | no | no | no (reusable) |
| `terminal-branch-lifecycle-reusable.yml` | no | no | no | no | no (reusable) |
| `terminal-branch-lifecycle.yml` | paths(15) | no | paths(15) (same) | no | yes |
| `tibia-linux-live-reference.yml` | paths(2) | no | no | no | yes |
| `wiki-reconciliation-acceptance.yml` | paths(23) | no | paths(23) (same) | no | yes |

## Not decided here

This map does not propose to limit the full run on `main`, to reuse PostgreSQL 16 or 17 evidence, to separate UI-only routing, or to change any required check. Each needs its own evidence (the review on #1610 lists what). No deploy, Synology or cutover file is part of this change.
