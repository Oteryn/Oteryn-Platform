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

Qualify current merged `Oteryn/Oteryn-Atlas@71e2aca1ac49692751f5d6c59a7126835fe1896a` on the LAN-only Synology FullWorld preview at `192.168.1.2:8097`, publish the missing exact Game-derived static-creature runtime augmentation without downgrading or restarting the Atlas container, prove desktop/mobile Chromium behavior, then restore temporary execution scaffolding and close Platform #1191 plus Atlas #30.

## Acceptance criteria

- [x] Live preview serves Atlas `71e2aca1ac49692751f5d6c59a7126835fe1896a` and exact current HTML/creature/search module bytes.
- [x] Exact Game producer evidence proves 1,068 NPC + 87,565 monster/spawn = 88,633 placements, unresolved 461, ambiguous 5 and semantic digest `sha256:01921968a6cb4f6ecea237820a053fc5052aaa1da556851f2c2a60d99890b5e1`.
- [x] Deterministic creature product contract is 5,746 shards and 1,945 search records.
- [x] Trusted-main diagnosis proves `/data/creatures/index.json` is missing from the current live revision.
- [x] Current Atlas source revision has no repository-owned top-level `data/` directory; creature data is runtime augmentation.
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
updated_at: 2026-08-21T17:25:00Z
head: 4da085899b5a1e4659cc166600b44fa79523a3a8
branch: fix/atlas-current-creature-parent
pr: 1207
status: validating
context_routes:
  - synology-staging
  - atlas-fullworld-preview
owned_paths:
  - .github/workflows/repair-synology-autostart.yml
  - docs/agents/tasks/active/OTERYN-20260821-atlas-creature-preview-deploy.md
proven:
  - Platform PR 1206 exact head f544bf99d89a0b7ae84c01ce0ee67e3bfa25d901 passed all protected checks and squash-merged as f0a254a3dc31d14a67ddce0f4deb23db8dccd77e
  - trusted-main run 32507240444 executed on oteryn-synology-staging and exact Game export plus deterministic 5746-chunk 1945-search-record product generation passed
  - run 32507240444 product step failed before any named HTTP probe or publication marker and reported no published/root/helper outputs; E2E was skipped and cleanup succeeded
  - current Atlas source at 71e2aca1ac49692751f5d6c59a7126835fe1896a has no top-level data directory, so repo/data is expected runtime augmentation rather than repository content
  - previous run 32505937090 proved exact live HTML fullworld-creatures.mjs and fullworld-search.mjs byte equality and /data/creatures/index.json HTTP 404
derived:
  - PR 1206 failed on the preflight requirement that /srv/atlas source already contain repo/data, not on Game export, deterministic index generation, current Atlas revision, HTTP serving, or browser runtime
  - creating repo/data only when absent and publishing data/creatures by same-filesystem atomic rename is the bounded runtime augmentation required by the current Atlas module contract
  - parent ownership must be tracked independently so failed E2E removes the parent only when this run created it and it is empty
unknown:
  - PR 1207 exact-head protected CI result
  - trusted-main atomic parent/product publication and root-preservation result after PR 1207 merge
  - final desktop and mobile Chromium E2E result and 30-day artifact
conflicts: []
first_failure:
  marker: trusted-main run 32507240444 initial product preflight before HTTP markers
  evidence: job 96850193683 completed exact Game export/index PASS then product step exited 1 before fullworld-html-http or creature endpoint markers; source verification confirms the tested repo/data parent is absent by design; E2E skipped and cleanup PASS
rejected_hypotheses:
  - exact Game-derived creature product generation is failing; run 32507240444 generated expected 88633 records 5746 chunks 1945 search records and exact semantic digest
  - current Atlas source revision is wrong; previous trusted-main diagnostics matched exact HTML and both relevant module bytes to 71e2aca1ac49692751f5d6c59a7126835fe1896a
  - a downgrade is required; current task authority explicitly retains the newer Atlas and no repair changes container revision
changed_paths:
  - .github/workflows/repair-synology-autostart.yml
  - docs/agents/tasks/active/OTERYN-20260821-atlas-creature-preview-deploy.md
validation:
  - command: PR 1206 exact-head protected workflows at f544bf99d89a0b7ae84c01ce0ee67e3bfa25d901
    result: PASS
    evidence: Agent Governance 32506870522 CI 32506870559 CodeQL 32506870524 DB outage 32506870541 Game Auth 32506870536 Edge Security 32506870544 and Phase 7 32506870510 all succeeded
  - command: trusted-main current-live publication run 32507240444 job 96850193683
    result: FAIL
    evidence: exact Game export/index PASS; product preflight failed before publication outputs or HTTP markers; E2E skipped; cleanup PASS
  - command: current Atlas source data directory lookup at 71e2aca1ac49692751f5d6c59a7126835fe1896a
    result: PASS
    evidence: repository contents lookup for top-level data returns 404 while exact creature consumer code intentionally resolves ../data/creatures as runtime augmentation
  - command: PR 1207 bounded parent creation candidate
    result: NOT_RUN
    evidence: pending fresh exact-head protected checks and trusted-main execution
blockers: []
next_action: pass exact-head protected checks for PR 1207, squash merge only after all gates pass, then verify trusted-main bounded parent creation, atomic creature-product publication, root preservation, exact HTTP Range and desktop/mobile Chromium E2E; repair only the first reproducible failure if one remains
```

## Self-review

```yaml
self_review:
  result: PASS
  exact_head_runtime_change: 4da085899b5a1e4659cc166600b44fa79523a3a8
  acceptance_checked: true
  full_diff_checked: true
  negative_paths_checked: true
  rollback_checked: true
  compatibility_checked: true
  related_prs_checked: true
  findings: []
  evidence:
    - live mount topology and root shape are emitted before mutation
    - parent creation is allowed only after both creature endpoints prove 404 and exact current Atlas source bytes pass
    - product publication uses same-filesystem temporary-directory rename and tracks parent-created and published ownership separately
    - rollback removes only task-owned creature product and conditionally the task-created empty parent
    - no container restart Atlas revision change broad Docker prune shared full-world root replacement or Game repository mutation is introduced
```

## Source branch closeout

```yaml
source_branch_disposition: auto_delete_after_merge
source_branch_reason: bounded current-live creature runtime-parent publication and acceptance branch
source_branch_evidence: pending PR 1207 merge and source-ref verification
```

## Notes

The current Atlas revision remains authoritative. The creature runtime augmentation becomes durable only after full live contract and desktop/mobile E2E PASS; until then both the creature directory and any parent directory created by this run are ownership-scoped and rollback-safe.