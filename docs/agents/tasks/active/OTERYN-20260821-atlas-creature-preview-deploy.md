---
task_id: OTERYN-20260821-atlas-creature-preview-deploy
required_reads:
  - AGENTS.md
  - docs/agents/PLATFORM_AGENT_BOOTSTRAP.md
  - docs/agents/EXECUTION_RESOURCE_HYGIENE.md
  - docs/agents/DELIVERY_COMPLETENESS_AND_CLOSEOUT.md
search_first:
  - synology atlas preview
  - repair-synology-autostart
optional_reads: []
---

# OTERYN-20260821-atlas-creature-preview-deploy

## Goal

Qualify current merged `Oteryn/Oteryn-Atlas@71e2aca1ac49692751f5d6c59a7126835fe1896a` on the LAN-only Synology FullWorld preview at `192.168.1.2:8097`, publish the missing exact Game-derived static-creature product without downgrading or restarting the Atlas container, prove desktop/mobile Chromium behavior, then restore temporary execution scaffolding and close Platform #1191 plus Atlas #30.

## Acceptance criteria

- [x] Live preview serves Atlas `71e2aca1ac49692751f5d6c59a7126835fe1896a` and exact current HTML/creature/search module bytes.
- [x] Historical exact Game producer evidence proves 1,068 NPC + 87,565 monster/spawn = 88,633 placements, unresolved 461, ambiguous 5 and semantic digest `sha256:01921968a6cb4f6ecea237820a053fc5052aaa1da556851f2c2a60d99890b5e1`.
- [x] Deterministic creature product contract is 5,746 shards and 1,945 search records.
- [x] Trusted-main diagnosis proves `/data/creatures/index.json` is missing from the current live revision.
- [ ] Exact generated creature product is atomically published into the already-live revision and served byte-for-byte.
- [ ] Publication, semantic, pixel, overview, runtime-index, pixel-bucket and minimap roots remain unchanged.
- [ ] HTTP 200/revision and HTTP Range 206 exact Content-Range/byte-count checks pass live.
- [ ] Real Chromium desktop and mobile E2E pass without Atlas downgrade or container cutover.
- [ ] Browser evidence is retained for 30 days.
- [x] Failed-run task-owned resources are removed by ownership-scoped cleanup.
- [ ] Temporary deployment/E2E scaffolding is restored after successful live acceptance.
- [ ] This task and predecessor are archived and Platform #1191 plus Atlas #30 are closed only after terminal verification.

## Ownership

```yaml
owned_paths:
  - .github/workflows/repair-synology-autostart.yml
  - scripts/acceptance/atlas-creature-preview-e2e.cjs
  - deploy/ci/playwright-chromium.Dockerfile
  - docs/agents/tasks/active/OTERYN-20260821-atlas-creature-preview-deploy.md
  - docs/agents/tasks/active/OTERYN-20260820-atlas-first-paint-preview-deploy.md
modules:
  - synology-staging-runner
  - atlas-fullworld-preview
cross_repository_tasks:
  - Oteryn/Oteryn-Atlas#30
blockers: []
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-08-21T17:11:00Z
head: 22cdc67813491cc943c467362bbb610c4b2c3b4c
branch: fix/atlas-current-creature-product
pr: 1206
status: validating
context_routes:
  - synology-staging
  - atlas-fullworld-preview
owned_paths:
  - .github/workflows/repair-synology-autostart.yml
  - docs/agents/tasks/active/OTERYN-20260821-atlas-creature-preview-deploy.md
proven:
  - Platform PR 1205 exact head 7fa09904778837ad0457fde6e7a6014468f4be3b passed all protected checks and squash-merged as 0d50a29129173d0f92323f7393c9b934c4fd41bd
  - trusted-main run 32505937090 executed on oteryn-synology-staging without Atlas downgrade or cutover and cleanup succeeded
  - run 32505937090 proved fullworld HTML fullworld-creatures.mjs and fullworld-search.mjs all returned HTTP 200 and matched exact Atlas 71e2aca1ac49692751f5d6c59a7126835fe1896a bytes
  - run 32505937090 proved the first missing live resource is /data/creatures/index.json with HTTP 404 before browser runtime build
  - historical trusted-main run 32500594332 proved exact Game export counts semantic digest deterministic 5746-chunk 1945-search-record product and Playwright 1.62.1 Chromium 151.0.7922.34 runtime
derived:
  - current Atlas code is correct and the bounded missing deliverable is the generated Game-derived data/creatures product
  - publishing only an absent repo/data/creatures directory by same-filesystem atomic rename preserves the current Atlas revision and existing shared data roots
  - if E2E fails after task-owned publication the exact newly published directory can be removed to restore the observed pre-publication 404 state
unknown:
  - PR 1206 exact-head protected CI result
  - trusted-main atomic publication and root-preservation result after PR 1206 merge
  - final desktop and mobile Chromium E2E result and 30-day artifact
conflicts: []
first_failure:
  marker: trusted-main run 32505937090 creature-index-http=404
  evidence: job 96846526625 proved current Atlas HTML and both modules HTTP 200 with exact bytes, then /data/creatures/index.json returned 404 and E2E was skipped; cleanup PASS
rejected_hypotheses:
  - current Atlas source revision is wrong or incomplete; exact HTML and both relevant modules matched 71e2aca1ac49692751f5d6c59a7126835fe1896a
  - a downgrade to ffb09ad6e78487fe6be5fa2f0c3a18a9a3cefc92 is required; current task authority explicitly retains the newer Atlas
  - Synology runner or Playwright runtime is unavailable; current runner executed and historical run 32500594332 launched Chromium 151.0.7922.34
changed_paths:
  - .github/workflows/repair-synology-autostart.yml
  - docs/agents/tasks/active/OTERYN-20260821-atlas-creature-preview-deploy.md
validation:
  - command: PR 1205 exact-head protected workflows at 7fa09904778837ad0457fde6e7a6014468f4be3b
    result: PASS
    evidence: Agent Governance 32505567463 CI 32505567304 CodeQL 32505567365 DB outage 32505567239 Game Auth 32505567409 Edge Security 32505567404 and Phase 7 32505567440 all succeeded
  - command: trusted-main current-live diagnostic run 32505937090 job 96846526625
    result: FAIL
    evidence: current Atlas source probes passed but creature-index-http=404; browser E2E skipped; ownership-scoped cleanup succeeded
  - command: PR 1206 atomic creature-product publication candidate
    result: NOT_RUN
    evidence: pending fresh exact-head protected checks and trusted-main execution
blockers: []
next_action: pass exact-head protected checks for PR 1206, squash merge only after all gates pass, then verify trusted-main atomic creature-product publication, root preservation, exact HTTP Range and desktop/mobile Chromium E2E; repair only the first reproducible failure if one remains
```

## Self-review

```yaml
self_review:
  result: PASS
  exact_head_runtime_change: 22cdc67813491cc943c467362bbb610c4b2c3b4c
  acceptance_checked: true
  full_diff_checked: true
  negative_paths_checked: true
  rollback_checked: true
  compatibility_checked: true
  related_prs_checked: true
  findings: []
  evidence:
    - publication is restricted to an absent repo/data/creatures path under the exact live Atlas mount
    - same-filesystem temporary-directory rename provides atomic visibility and partial states fail closed
    - failure cleanup removes only the directory published by this run and verifies the original 404 state
    - no container restart Atlas revision change broad Docker prune shared root replacement or Game repository mutation is introduced
```

## Source branch closeout

```yaml
source_branch_disposition: auto_delete_after_merge
source_branch_reason: bounded current-live creature-product publication and acceptance branch
source_branch_evidence: pending PR 1206 merge and source-ref verification
```

## Notes

The current Atlas revision remains authoritative. The creature directory is an intended durable runtime product only after full live contract and desktop/mobile E2E PASS; until then it is task-owned and rollback-scoped.