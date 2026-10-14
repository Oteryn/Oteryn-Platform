# CI acceptance coverage map

Read-only inventory of which workflow runs on which event, generated from `.github/workflows/*.yml` on `main` (2026-10-14, 53 workflows). It records current behavior and changes none. Its purpose is to make the PR-critical scope and the post-merge `main` scope explicit before anyone proposes to narrow the full run on `main` or to move specialist checks into the Merge Queue. The per-workflow table at the end is authoritative; the summaries are derived from it.

## Events

| Event | Workflows |
|---|---|
| `pull_request` to `main` | 43 of 53: `ci.yml` (every PR) and the specialist workflows whose `paths` or `paths-ignore` match the diff. |
| `merge_group` | 1 of 53: `ci`. No specialist workflow has this trigger. |
| `push` to `main` | 35 of 53: `ci.yml` (every push) and the workflows whose `paths` match the merged diff. |
| `pull_request_target` | 5: `cloudflare-oteryn-edge-audit`, `cloudflare-oteryn-endpoints`, `github-actions-storage-hygiene`, `historical-branch-audit`, `terminal-branch-lifecycle`. |
| `issue_comment` | 1: `cloudflare-oteryn-endpoints`. |
| `workflow_call` | 3: `acceptance-validation`, `terminal-branch-lifecycle-read-reusable`, `terminal-branch-lifecycle-reusable`. |
| `schedule` | 5: `acceptance-soak`, `acceptance-stability`, `codeql`, `github-actions-storage-hygiene`, `historical-branch-audit`. 2 run only on a schedule or manually: `acceptance-soak`, `acceptance-stability`. |
| `workflow_dispatch` | 41 of 53 (manual runs; see the table). |

## Required aggregate

`platform-gate` (`ci.yml`) is the stable aggregate. It requires `classify-changes` and `test` to succeed, and `platform-postgres` and `php-coverage-report` to succeed or be skipped. `test` evaluates `runtime-tests` through `scripts/ci/required_test_gate.py`, which also accepts a classified reuse of a heavy run (`reuse_heavy`). The specialist workflows are separate checks, not inputs of `platform-gate`.

## Scope by workflow group

Every workflow is in exactly one group. The counts are how many workflows of the group have each trigger; the per-workflow table gives the filters.

| Group | Workflows | Names | `pull_request` | `merge_group` | `push` | `schedule` |
|---|---|---|---|---|---|---|
| Core CI | 1 | `ci` | 1 | 1 | 1 | 0 |
| Portal and content acceptance | 13 | `acceptance-validation`, `announcements-acceptance`, `community-data-acceptance`, `content-scale-acceptance`, `downloads-acceptance`, `editorial-media-acceptance`, `error-state-acceptance`, `events-acceptance`, `portal-acceptance-contract`, `portal-e2e-audit`, `support-legal-acceptance`, `support-moderation-acceptance`, `wiki-reconciliation-acceptance` | 13 | 0 | 12 | 0 |
| Runtime, contract and repository governance | 13 | `agent-governance`, `branch-lifecycle`, `codeql`, `game-catalog-contract`, `game-gateway-ci`, `github-actions-storage-hygiene`, `historical-branch-audit`, `native-protocol-contract-audits`, `native-protocol-contract`, `parallel-coordinator-prompt-eval`, `playwright-runtime-validation`, `terminal-branch-lifecycle`, `tibia-linux-live-reference` | 13 | 0 | 11 | 3 |
| Whole-tree validation | 4 | `edge-security-emulation`, `game-auth-ticket-concurrency`, `phase7-production-like-validation`, `platform-db-outage-validation` | 4 | 0 | 0 | 0 |
| Deploy and edge | 18 | `build-synology-staging-images`, `character-bazaar-staging-control`, `character-bazaar-staging-validation`, `cloudflare-oteryn-edge-audit`, `cloudflare-oteryn-endpoint-main-operation`, `cloudflare-oteryn-endpoints`, `cloudflare-oteryn-hsts-stage1`, `cloudflare-oteryn-public-edge-repair`, `cloudflare-zone-edge-audit`, `deploy-synology-staging`, `oteryn-public-edge-validation`, `recover-synology-staging-schema`, `repair-synology-autostart`, `repair-synology-compose-orphans`, `synology-container-hygiene`, `synology-diagnostics`, `synology-production-target-preflight`, `synology-rollback-contract` | 12 | 0 | 11 | 0 |
| Scheduled soak | 2 | `acceptance-soak`, `acceptance-stability` | 0 | 0 | 0 | 2 |
| Reusable (`workflow_call` only) | 2 | `terminal-branch-lifecycle-read-reusable`, `terminal-branch-lifecycle-reusable` | 0 | 0 | 0 | 0 |

## Reading the map

- PR-critical scope: `ci.yml` plus the path-matched specialist workflows. They are the only evidence for the specialist areas before merge. Checkout refs of the 43 workflows with a `pull_request` trigger, by the set of classes they use: only PR head: 15; only default/event merge ref: 9; PR head + default/event merge ref: 8; default/event merge ref + other ref: 4; PR head + other ref: 3; PR head + default/event merge ref + merge-group candidate: 1; PR head + base: 1; PR head + base + default/event merge ref: 1; no checkout: 1 (checkout classes by `ref:` expression, counted from the table below by `ref:` expression, not by which job runs on which event).
- Evidence about the exact PR head SHA applies only to a workflow whose checkout is `PR head`. A default or `github.sha` checkout on `pull_request` is GitHub's synthetic merge commit of head and base, which can differ from the head once the base moves. In `ci.yml` the `merge-group candidate / pushed commit` checkouts belong to jobs that run on `merge_group` and `push`, not on `pull_request`.
- Main scope: `ci.yml` in full, plus the workflows whose `push` filter matches the squash commit. The squash SHA differs from the `merge_group` candidate, so the `main` run is a fresh run, not a copy of the queue run.
- Filters that differ between `pull_request` and `push`: `cloudflare-oteryn-endpoint-main-operation`, `cloudflare-oteryn-hsts-stage1`, `cloudflare-oteryn-public-edge-repair`, `oteryn-public-edge-validation` (the `push` filter is narrower; for example `cloudflare-oteryn-hsts-stage1` runs on a PR for its workflow, implementation, tests and docs, but on push only for its `ops/triggers` file).
- Run on a PR but have no `push` trigger, so nothing re-runs them on `main` after merge (11): `cloudflare-oteryn-edge-audit`, `cloudflare-oteryn-endpoints`, `edge-security-emulation`, `game-auth-ticket-concurrency`, `github-actions-storage-hygiene`, `phase7-production-like-validation`, `platform-db-outage-validation`, `portal-e2e-audit`, `synology-container-hygiene`, `synology-production-target-preflight`, `tibia-linux-live-reference`. Of these, `github-actions-storage-hygiene` also runs on a schedule.
- Run on `push` to `main` but have no `pull_request` trigger (3): `character-bazaar-staging-control`, `repair-synology-autostart`, `synology-diagnostics`; none of them has a schedule.
- Specialist workflows have no `merge_group` trigger, so a Merge Queue candidate is not re-checked by them. Adding one requires the `merge_group` invocation and its required aggregation together; removing only the PR or `push` trigger leaves a gap.
- A path-filtered specialist workflow does not block `platform-gate`; a skipped one is invisible to the aggregate.

## Complete per-workflow triggers

Generated from the `on:` block and every `actions/checkout` step of each workflow. `paths(n)` is a path filter with n entries; `narrower` means the `push` filter is a strict subset of the `pull_request` filter. "Other events" lists remaining triggers. "Checkout ref" lists each distinct `ref:` expression with its class; an empty `ref:` is the default ref.

| Workflow | `pull_request` | `merge_group` | `push` to `main` | `schedule` | Manual | Other events | Checkout ref |
|---|---|---|---|---|---|---|---|
| `acceptance-soak.yml` | no | no | no | cron | yes | - | no checkout |
| `acceptance-stability.yml` | no | no | no | cron | yes | - | no checkout |
| `acceptance-validation.yml` | paths(11) | no | paths(11) (same) | no | yes | `workflow_call` | PR head: `${{ github.event.pull_request.head.sha \|\| github.sha }}` |
| `agent-governance.yml` | paths(7) | no | paths(7) (same) | no | yes | - | default ref (merge ref on `pull_request`, pushed commit on `push`); other: `1bfb5ff98c8aa156e73669a14e083a1d464c29fb` |
| `announcements-acceptance.yml` | paths(13) | no | paths(13) (same) | no | yes | - | PR head: `${{ github.event.pull_request.head.sha \|\| github.sha }}` |
| `branch-lifecycle.yml` | paths(6) | no | paths(6) (same) | no | yes | - | default ref (merge ref on `pull_request`, pushed commit on `push`); other: `main` |
| `build-synology-staging-images.yml` | paths(74) | no | paths(74) (same) | no | yes | - | PR head: `${{ github.event.pull_request.head.sha \|\| github.sha }}`; event SHA (merge ref on `pull_request`, pushed commit on `push`): `${{ github.sha }}` |
| `character-bazaar-staging-control.yml` | no | no | paths(5) | no | yes | - | event SHA (merge ref on `pull_request`, pushed commit on `push`): `${{ github.sha }}` |
| `character-bazaar-staging-validation.yml` | paths(9) | no | paths(9) (same) | no | yes | - | default ref (merge ref on `pull_request`, pushed commit on `push`) |
| `ci.yml` | all | all | all | no | no | - | PR head: `${{ github.event.pull_request.head.sha \|\| github.sha }}`; default ref (merge ref on `pull_request`, pushed commit on `push`); merge-group candidate / pushed commit: `${{ github.event.merge_group.head_sha \|\| github.event.pull_request.head.sha \|\| github.sha }}`; merge-group candidate / pushed commit: `${{ github.event.merge_group.head_sha \|\| github.sha }}` |
| `cloudflare-oteryn-edge-audit.yml` | paths(9) | no | no | no | no | `pull_request_target` | PR head: `${{ github.event.pull_request.head.sha }}`; base: `${{ github.event.pull_request.base.sha }}` |
| `cloudflare-oteryn-endpoint-main-operation.yml` | paths(3) | no | paths(1) (narrower) | no | no | - | PR head: `${{ github.event.pull_request.head.sha }}`; event SHA (merge ref on `pull_request`, pushed commit on `push`): `${{ github.sha }}` |
| `cloudflare-oteryn-endpoints.yml` | paths(7) | no | no | no | yes | `issue_comment`, `pull_request_target` | PR head: `${{ github.event.pull_request.head.sha \|\| github.sha }}`; base: `${{ github.event.pull_request.base.sha }}`; event SHA (merge ref on `pull_request`, pushed commit on `push`): `${{ github.sha }}` |
| `cloudflare-oteryn-hsts-stage1.yml` | paths(5) | no | paths(1) (narrower) | no | no | - | PR head: `${{ github.event.pull_request.head.sha }}`; event SHA (merge ref on `pull_request`, pushed commit on `push`): `${{ github.sha }}` |
| `cloudflare-oteryn-public-edge-repair.yml` | paths(5) | no | paths(1) (narrower) | no | no | - | PR head: `${{ github.event.pull_request.head.sha }}`; event SHA (merge ref on `pull_request`, pushed commit on `push`): `${{ github.sha }}` |
| `cloudflare-zone-edge-audit.yml` | paths(7) | no | paths(7) (same) | no | yes | - | PR head: `${{ github.event.pull_request.head.sha \|\| github.sha }}`; event SHA (merge ref on `pull_request`, pushed commit on `push`): `${{ github.sha }}` |
| `codeql.yml` | paths(14) | no | paths(14) (same) | cron | yes | - | default ref (merge ref on `pull_request`, pushed commit on `push`) |
| `community-data-acceptance.yml` | paths(31) | no | paths(31) (same) | no | yes | - | PR head: `${{ github.event.pull_request.head.sha \|\| github.sha }}` |
| `content-scale-acceptance.yml` | paths(36) | no | paths(36) (same) | no | yes | - | PR head: `${{ github.event.pull_request.head.sha \|\| github.sha }}` |
| `deploy-synology-staging.yml` | no | no | no | no | yes | - | other: `${{ inputs.action == 'deploy' && inputs.release_sha \|\| inputs.action == 'cutover' && github.sha \|\| 'main' }}` |
| `downloads-acceptance.yml` | paths(18) | no | paths(18) (same) | no | yes | - | PR head: `${{ github.event.pull_request.head.sha \|\| github.sha }}` |
| `edge-security-emulation.yml` | all except 5 ignored | no | no | no | yes | - | PR head: `${{ github.event.pull_request.head.sha \|\| github.sha }}` |
| `editorial-media-acceptance.yml` | paths(14) | no | paths(14) (same) | no | yes | - | PR head: `${{ github.event.pull_request.head.sha \|\| github.sha }}` |
| `error-state-acceptance.yml` | paths(17) | no | paths(17) (same) | no | yes | - | PR head: `${{ github.event.pull_request.head.sha \|\| github.sha }}` |
| `events-acceptance.yml` | paths(10) | no | paths(10) (same) | no | yes | - | PR head: `${{ github.event.pull_request.head.sha \|\| github.sha }}` |
| `game-auth-ticket-concurrency.yml` | all except 5 ignored | no | no | no | no | - | PR head: `${{ github.event.pull_request.head.sha \|\| github.sha }}`; default ref (merge ref on `pull_request`, pushed commit on `push`) |
| `game-catalog-contract.yml` | paths(10) | no | paths(10) (same) | no | yes | - | default ref (merge ref on `pull_request`, pushed commit on `push`) |
| `game-gateway-ci.yml` | paths(2) | no | paths(2) (same) | no | no | - | default ref (merge ref on `pull_request`, pushed commit on `push`) |
| `github-actions-storage-hygiene.yml` | paths(3) | no | no | cron | yes | `pull_request_target` | PR head: `${{ github.event.pull_request.head.sha \|\| github.sha }}`; other: `refs/heads/main` |
| `historical-branch-audit.yml` | paths(11) | no | paths(11) (same) | cron | yes | `pull_request_target` | default ref (merge ref on `pull_request`, pushed commit on `push`); event SHA (merge ref on `pull_request`, pushed commit on `push`): `${{ github.sha }}`; other: `main` |
| `native-protocol-contract-audits.yml` | paths(8) | no | paths(8) (same) | no | no | - | default ref (merge ref on `pull_request`, pushed commit on `push`) |
| `native-protocol-contract.yml` | paths(5) | no | paths(5) (same) | no | no | - | default ref (merge ref on `pull_request`, pushed commit on `push`) |
| `oteryn-public-edge-validation.yml` | paths(6) | no | paths(1) (narrower) | no | no | - | PR head: `${{ github.event.pull_request.head.sha }}`; event SHA (merge ref on `pull_request`, pushed commit on `push`): `${{ github.sha }}` |
| `parallel-coordinator-prompt-eval.yml` | paths(3) | no | paths(3) (same) | no | yes | - | default ref (merge ref on `pull_request`, pushed commit on `push`) |
| `phase7-production-like-validation.yml` | all except 5 ignored | no | no | no | yes | - | PR head: `${{ github.event.pull_request.head.sha \|\| github.sha }}` |
| `platform-db-outage-validation.yml` | all except 5 ignored | no | no | no | yes | - | PR head: `${{ github.event.pull_request.head.sha \|\| github.sha }}` |
| `playwright-runtime-validation.yml` | paths(8) | no | paths(8) (same) | no | yes | - | PR head: `${{ github.event.pull_request.head.sha \|\| github.sha }}`; event SHA (merge ref on `pull_request`, pushed commit on `push`): `${{ github.sha }}` |
| `portal-acceptance-contract.yml` | paths(17) | no | paths(17) (same) | no | yes | - | PR head: `${{ github.event.pull_request.head.sha \|\| github.sha }}` |
| `portal-e2e-audit.yml` | paths(1) | no | no | no | yes | - | no checkout |
| `recover-synology-staging-schema.yml` | no | no | no | no | yes | - | event SHA (merge ref on `pull_request`, pushed commit on `push`): `${{ github.sha }}` |
| `repair-synology-autostart.yml` | no | no | paths(3) | no | yes | - | no checkout |
| `repair-synology-compose-orphans.yml` | no | no | no | no | yes | - | other: `main` |
| `support-legal-acceptance.yml` | paths(23) | no | paths(23) (same) | no | yes | - | PR head: `${{ github.event.pull_request.head.sha \|\| github.sha }}` |
| `support-moderation-acceptance.yml` | paths(24) | no | paths(24) (same) | no | yes | - | PR head: `${{ github.event.pull_request.head.sha \|\| github.sha }}` |
| `synology-container-hygiene.yml` | paths(5) | no | no | no | yes | - | PR head: `${{ github.event.pull_request.head.sha }}`; other: `main` |
| `synology-diagnostics.yml` | no | no | paths(1) | no | yes | - | no checkout |
| `synology-production-target-preflight.yml` | paths(8) | no | no | no | yes | - | PR head: `${{ github.event.pull_request.head.sha \|\| github.sha }}`; other: `main` |
| `synology-rollback-contract.yml` | paths(11) | no | paths(11) (same) | no | yes | - | default ref (merge ref on `pull_request`, pushed commit on `push`) |
| `terminal-branch-lifecycle-read-reusable.yml` | no | no | no | no | no | `workflow_call` | default ref (merge ref on `pull_request`, pushed commit on `push`); other: `${{ inputs.platform_ref }}` |
| `terminal-branch-lifecycle-reusable.yml` | no | no | no | no | no | `workflow_call` | other: `${{ inputs.platform_ref }}`; other: `main` |
| `terminal-branch-lifecycle.yml` | paths(15) | no | paths(15) (same) | no | yes | `pull_request_target` | default ref (merge ref on `pull_request`, pushed commit on `push`); other: `main` |
| `tibia-linux-live-reference.yml` | paths(2) | no | no | no | yes | - | default ref (merge ref on `pull_request`, pushed commit on `push`) |
| `wiki-reconciliation-acceptance.yml` | paths(23) | no | paths(23) (same) | no | yes | - | PR head: `${{ github.event.pull_request.head.sha \|\| github.sha }}` |

## Not decided here

This map does not propose to limit the full run on `main`, to reuse PostgreSQL 16 or 17 evidence, to separate UI-only routing, or to change any required check. Each needs its own evidence (the review on #1610 lists what). No deploy, Synology or cutover file is part of this change.
