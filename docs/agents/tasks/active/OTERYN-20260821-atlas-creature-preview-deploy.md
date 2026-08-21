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

Qualify current merged `Oteryn/Oteryn-Atlas@71e2aca1ac49692751f5d6c59a7126835fe1896a` on the LAN-only Synology FullWorld preview at `192.168.1.2:8097`, publish the missing exact Game-derived static-creature runtime augmentation into the direct live code mount without downgrading or restarting the Atlas container, prove desktop/mobile Chromium behavior, then restore temporary execution scaffolding and close Platform #1191 plus Atlas #30.

## Acceptance criteria

- [x] Live preview serves Atlas `71e2aca1ac49692751f5d6c59a7126835fe1896a` and exact current HTML/creature/search module bytes.
- [x] Exact Game producer evidence proves 1,068 NPC + 87,565 monster/spawn = 88,633 placements, unresolved 461, ambiguous 5 and semantic digest `sha256:01921968a6cb4f6ecea237820a053fc5052aaa1da556851f2c2a60d99890b5e1`.
- [x] Deterministic creature product contract is 5,746 shards and 1,945 search records.
- [x] Trusted-main diagnosis proves `/data/creatures/index.json` is missing from the current live revision.
- [x] Authoritative live mount topology is split: code at `/srv/atlas/repo`, verified shared full-world data at `/srv/atlas/data`, and no `/srv/atlas` mount exists.
- [ ] Exact generated creature product is atomically published under the direct code mount's `data/creatures` and served byte-for-byte.
- [ ] Publication, semantic, pixel, overview, runtime-index, pixel-bucket and minimap roots remain unchanged and the separate shared data mount is not mutated.
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
updated_at: 2026-08-21T17:52:00Z
head: 22af5f17dd1a4249ea6d6811553affb8dc680712
branch: fix/atlas-current-repo-mount
pr: 1208
status: validating
context_routes:
  - synology-staging
  - atlas-fullworld-preview
owned_paths:
  - .github/workflows/repair-synology-autostart.yml
  - docs/agents/tasks/active/OTERYN-20260821-atlas-creature-preview-deploy.md
proven:
  - Platform PR 1207 exact head cddadd01a96bce473cb397637a44bf3c6db283d3 passed all protected checks and squash-merged as bdf1e8cb941259a4bd7fb6a1c09bafba857ca2eb
  - trusted-main run 32508271569 executed on oteryn-synology-staging and exact Game export plus deterministic 5746-chunk 1945-search-record product generation passed
  - run 32508271569 logged live-running true live revision 71e2aca1ac49692751f5d6c59a7126835fe1896a and exact port 192.168.1.2:8097
  - run 32508271569 logged direct code mount source to /srv/atlas/repo and distinct verified full-world data source to /srv/atlas/data with no /srv/atlas mount
  - run 32508271569 failed before publication because the previous resolver searched for a nonexistent /srv/atlas mount; published/root/helper outputs remained empty, E2E was skipped, cleanup succeeded
  - previous trusted-main diagnostics proved exact current Atlas HTML and both relevant module bytes plus creature endpoints HTTP 404
derived:
  - the current failure is mount-resolution only; the generated creature product, current Atlas revision, runner, port and existing split mount topology are healthy
  - the intended runtime augmentation must be written under the direct /srv/atlas/repo source path so URL /data/creatures resolves through nginx root /srv/atlas/repo
  - the separate /srv/atlas/data source is shared verified full-world data and must remain unmodified
unknown:
  - PR 1208 exact-head protected CI result
  - trusted-main atomic publication and root-preservation result after PR 1208 merge
  - final desktop and mobile Chromium E2E result and 30-day artifact
conflicts: []
first_failure:
  marker: trusted-main run 32508271569 live-root empty while split mount topology is present
  evidence: job 96856966701 logged live-root empty and live-mounts showing direct /srv/atlas/repo plus /srv/atlas/data, then exited before root-shape/HTTP/publication markers; E2E skipped and cleanup PASS
rejected_hypotheses:
  - exact Game-derived creature product generation is failing; run 32508271569 generated expected 88633 records 5746 chunks 1945 search records and exact semantic digest
  - current Atlas container revision or port is wrong; run 32508271569 logged exact revision 71e2aca1ac49692751f5d6c59a7126835fe1896a running on 192.168.1.2:8097
  - the shared full-world data mount should receive creature files; authoritative topology shows it is a separate /srv/atlas/data mount while browser /data/creatures resolves under nginx root /srv/atlas/repo
changed_paths:
  - .github/workflows/repair-synology-autostart.yml
  - docs/agents/tasks/active/OTERYN-20260821-atlas-creature-preview-deploy.md
validation:
  - command: PR 1207 exact-head protected workflows at cddadd01a96bce473cb397637a44bf3c6db283d3
    result: PASS
    evidence: Agent Governance 32508023951 CI 32508023840 CodeQL 32508023654 and all other protected workflows succeeded before expected-head squash merge
  - command: trusted-main current-live run 32508271569 job 96856966701
    result: FAIL
    evidence: exact Game export/index PASS and live revision/port/mount topology proven; nonexistent /srv/atlas mount resolver produced empty live-root; no publication occurred; E2E skipped; cleanup PASS
  - command: PR 1208 split repo/shared-data mount resolver candidate
    result: NOT_RUN
    evidence: pending fresh exact-head protected checks and trusted-main execution
blockers: []
next_action: pass exact-head protected checks for PR 1208, squash merge only after all gates pass, then verify trusted-main publication under the direct repo mount, untouched shared data mount, exact live/root/range checks and desktop/mobile Chromium E2E; repair only the first reproducible failure if one remains
```

## Self-review

```yaml
self_review:
  result: PASS
  exact_head_runtime_change: 22af5f17dd1a4249ea6d6811553affb8dc680712
  acceptance_checked: true
  full_diff_checked: true
  negative_paths_checked: true
  rollback_checked: true
  compatibility_checked: true
  related_prs_checked: true
  findings: []
  evidence:
    - direct /srv/atlas/repo and /srv/atlas/data sources are resolved and asserted distinct before any mutation
    - shared data source is mounted read-only in helper validation and is never a publication target
    - parent/product publication remains conditional on exact current source bytes plus both creature endpoints returning 404
    - product publication uses same-filesystem atomic rename and rollback removes only task-owned augmentation
    - no container restart Atlas revision change broad Docker prune shared full-world root replacement or Game repository mutation is introduced
```

## Source branch closeout

```yaml
source_branch_disposition: auto_delete_after_merge
source_branch_reason: bounded split-mount creature runtime publication and acceptance branch
source_branch_evidence: pending PR 1208 merge and source-ref verification
```

## Notes

The current Atlas revision and existing split mount topology remain authoritative. Creature data becomes a durable runtime augmentation only after full live contract and desktop/mobile E2E PASS.