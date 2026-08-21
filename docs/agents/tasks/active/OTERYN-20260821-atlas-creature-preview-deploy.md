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

Qualify current merged `Oteryn/Oteryn-Atlas@71e2aca1ac49692751f5d6c59a7126835fe1896a` on the LAN-only Synology FullWorld preview at `192.168.1.2:8097`, publish the missing exact Game-derived static-creature runtime augmentation into the direct live code mount without downgrading or restarting the Atlas container, prove desktop/mobile Chromium behavior through the current global search UI, then restore temporary execution scaffolding and close Platform #1191 plus Atlas #30.

## Acceptance criteria

- [x] Live preview serves Atlas `71e2aca1ac49692751f5d6c59a7126835fe1896a` and exact current HTML/creature/search module bytes.
- [x] Exact Game producer evidence proves 1,068 NPC + 87,565 monster/spawn = 88,633 placements, unresolved 461, ambiguous 5 and semantic digest `sha256:01921968a6cb4f6ecea237820a053fc5052aaa1da556851f2c2a60d99890b5e1`.
- [x] Deterministic creature product contract is 5,746 shards and 1,945 search records.
- [x] Trusted-main diagnosis proves `/data/creatures/index.json` is missing from the current live revision before publication.
- [x] Authoritative live mount topology is split: code at `/srv/atlas/repo`, verified shared full-world data at `/srv/atlas/data`, and no `/srv/atlas` mount exists.
- [x] Trusted-main run 32514817675 proved atomic publication plus exact live bytes, Range 206 and all seven root contracts before its browser-launch step failed.
- [ ] Exact generated creature product is durably published under the direct code mount's `data/creatures` and served byte-for-byte after browser PASS.
- [ ] Publication, semantic, pixel, overview, runtime-index, pixel-bucket and minimap roots remain unchanged and the separate shared data mount is not mutated.
- [ ] HTTP 200/revision and HTTP Range 206 exact Content-Range/byte-count checks pass live after terminal publication.
- [ ] Real Chromium desktop and mobile E2E pass through `#search-input` / `#mobile-search-input` and `.semantic-search-result` without Atlas downgrade or container cutover.
- [ ] Browser evidence is retained for 30 days.
- [x] Failed-run task-owned resources are removed by ownership-scoped cleanup.
- [ ] Temporary deployment/E2E scaffolding is restored after successful live acceptance.
- [ ] This task and predecessor are archived and Platform #1191 plus Atlas #30 are closed only after terminal verification.

## Ownership

```yaml
owned_paths:
  - .github/workflows/repair-synology-autostart.yml
  - deploy/synology/compose.yml
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
updated_at: 2026-08-21T19:15:43Z
head: aa7a144da03f0e4d84b31452c95f900cc2c09a26
branch: fix/atlas-current-global-search-e2e
pr: 1209
status: validating
context_routes:
  - synology-staging
  - atlas-fullworld-preview
owned_paths:
  - .github/workflows/repair-synology-autostart.yml
  - deploy/synology/compose.yml
  - scripts/acceptance/atlas-creature-preview-e2e.cjs
  - docs/agents/tasks/active/OTERYN-20260821-atlas-creature-preview-deploy.md
proven:
  - Platform PR 1207 exact head cddadd01a96bce473cb397637a44bf3c6db283d3 passed protected checks and squash-merged as bdf1e8cb941259a4bd7fb6a1c09bafba857ca2eb
  - trusted-main run 32508271569 proved exact Game export/product generation, live revision 71e2aca1ac49692751f5d6c59a7126835fe1896a, port 192.168.1.2:8097, and the direct /srv/atlas/repo plus separate /srv/atlas/data mount topology before failing on the obsolete /srv/atlas resolver
  - Platform PR 1208 exact head abefa57fbf8f0634b1b5a54283c6592925e7b025 passed all protected workflow checks with no review threads/comments and squash-merged as ecbf3a92bc3898ac561d725418c4ff98a0a4a8cb
  - PR 1208 source branch fix/atlas-current-repo-mount was absent after merge
  - trusted-main Repair Synology Autostart run 32514817675 on exact Platform main ecbf3a92bc3898ac561d725418c4ff98a0a4a8cb regenerated the expected Game export and 5746-shard/1945-search-record creature product, resolved the direct /srv/atlas/repo mount, verified the separate /srv/atlas/data mount, atomically published the creature product, verified exact Atlas HTML/module/product bytes, preserved all seven roots, and passed the exact Range 206 contract
  - run 32514817675 built Playwright 1.62.1 on the pinned Microsoft runtime and launched Chromium 151.0.7922.34 successfully
  - run 32514817675 first failed only when host Docker tried to bind runner-internal path /work/Oteryn-Platform/Oteryn-Platform/scripts/acceptance/atlas-creature-preview-e2e.cjs; rollback then removed only the just-published creature product/parent and cleanup removed the task browser image
  - live runner container oteryn-synology-staging-runner is the unique oteryn-deploy-runner/runner service and maps container /work from host source /volume1/@docker/volumes/oteryn-deploy-runner_runner_work/_data
  - exact Atlas 71e2aca1ac49692751f5d6c59a7126835fe1896a web/fullworld.html defines canonical global search controls #search-input and #mobile-search-input
  - exact Atlas 71e2aca1ac49692751f5d6c59a7126835fe1896a web/fullworld-search.mjs renders .semantic-search-result and supplements global semantic search with validated creature records carrying factual position and record_id
  - PR 1209 changes current global-search E2E assertions, translates runner-internal /work to the host-visible runner work-volume source before nested Docker bind mounts, and changes only the existing task-owned compose retrigger comment with no Compose semantics
  - prior PR 1209 head 1bc3e1307a85ce0f8823fea93bce30cf043cfef1 passed every protected workflow except Acceptance E2E; its portability matrix passed 56/57 cases and the sole failure was WebKit page.goto reporting WebKit encountered an internal error on unrelated /account navigation
  - a mistaken broad community-test repair created while diagnosing that unrelated WebKit engine error was immediately removed by resetting the task branch to the clean 1bc3e1307a85ce0f8823fea93bce30cf043cfef1 state before the current bounded commits
derived:
  - the trusted-main browser failure is not an Atlas data/runtime failure; nested Docker receives host paths while GitHub workspace paths are runner-container paths
  - mapping the unique deploy runner's /work source and appending the runner workspace-relative path produces the host-visible workspace path without changing runner infrastructure
  - even a browser PASS through the historical injected creature-only search would not satisfy the explicit current global-search acceptance requirement
unknown:
  - PR 1209 fresh exact-head protected CI result after the workflow host-path repair and checkpoint commit
  - trusted-main global-search desktop/mobile result after PR 1209 merge
  - final successful 30-day browser evidence artifact identity
conflicts: []
first_failure:
  marker: trusted-main run 32514817675 E2E nested-Docker bind mount cannot resolve runner-internal /work workspace path
  evidence: job 96874119778 logged live-creature-product-and-roots=PASS and chromium-runtime=151.0.7922.34, then Docker daemon returned bind mount failed because /work/Oteryn-Platform/Oteryn-Platform/scripts/acceptance/atlas-creature-preview-e2e.cjs does not exist on the Docker host; the runner container inspection proves /work is backed by host source /volume1/@docker/volumes/oteryn-deploy-runner_runner_work/_data
rejected_hypotheses:
  - exact Game-derived creature product generation is failing; run 32514817675 regenerated exact 88633 records, 5746 chunks, 1945 search records and semantic digest
  - publication into the direct repo mount is failing; run 32514817675 logged creature-product-publish=ATOMIC_PASS and exact live product/root checks before browser execution
  - current Atlas container revision or port is wrong; run 32514817675 logged exact revision 71e2aca1ac49692751f5d6c59a7126835fe1896a on 192.168.1.2:8097
  - the shared full-world data mount should receive creature files; authoritative topology proves it is a separate /srv/atlas/data mount while browser /data/creatures resolves under /srv/atlas/repo
  - the historical #creature-search path is the current canonical acceptance UX; exact current Atlas HTML/search module prove global desktop/mobile search is canonical
changed_paths:
  - .github/workflows/repair-synology-autostart.yml
  - deploy/synology/compose.yml
  - scripts/acceptance/atlas-creature-preview-e2e.cjs
  - docs/agents/tasks/active/OTERYN-20260821-atlas-creature-preview-deploy.md
validation:
  - command: PR 1208 exact-head protected workflows at abefa57fbf8f0634b1b5a54283c6592925e7b025
    result: PASS
    evidence: CI 32510310573, Agent Governance 32510310576, CodeQL 32510310310, Game Auth 32510310585, Phase 7 32510310590, Edge 32510310600 and DB Outage 32510310541 all succeeded
  - command: trusted-main run 32514817675 job 96874119778
    result: FAIL
    evidence: exact Game/product/publication/served-byte/Range/seven-root checks and pinned Chromium runtime passed; first browser E2E command failed only at nested-Docker bind-path resolution; publication rollback and cleanup passed
  - command: exact Atlas 71e2aca source inspection of web/fullworld.html and web/fullworld-search.mjs
    result: PASS
    evidence: #search-input, #mobile-search-input and dynamically rendered .semantic-search-result are the current global-search contract; creature supplement records expose factual record_id and position
  - command: node --check scripts/acceptance/atlas-creature-preview-e2e.cjs
    result: PASS
    evidence: exact task branch script parsed successfully on the authorized Synology host
  - command: PR 1209 exact-head protected workflows
    result: NOT_RUN
    evidence: fresh checks required after current checkpoint commit
blockers: []
next_action: pass fresh exact-head protected CI for PR 1209, inspect full diff/review state, squash merge at the exact expected head, then require a fresh trusted-main run to prove atomic creature publication or exact reuse, unchanged seven roots, Range 206, and current global-search desktop/mobile Chromium PASS before terminal cleanup
```

## Self-review

```yaml
self_review:
  result: PASS
  exact_head_runtime_change: aa7a144da03f0e4d84b31452c95f900cc2c09a26
  acceptance_checked: true
  full_diff_checked: true
  negative_paths_checked: true
  rollback_checked: true
  compatibility_checked: true
  related_prs_checked: true
  findings: []
  evidence:
    - workflow changes only the live browser E2E path translation from runner-internal /work to the unique deploy runner mount source and validates required host-visible files before execution
    - no deploy/synology/runner/compose.yml mutation is present
    - desktop uses #search-input and #semantic-search-results-desktop .semantic-search-result
    - mobile uses #mobile-search-input and #semantic-search-results-mobile .semantic-search-result
    - result selection checks factual label kind X/Y and deep-link X/Y/floor selected/q/creature state
    - factual inspector checks name kind record exact X/Y/floor semantic digest and static marker fallback
    - visible creature overlay surface, bounded cache/draw diagnostics, semantic-search readiness and relevant page/console errors are asserted
    - compose change is comment-only and intentionally retriggers trusted-main acceptance after merge
    - no Atlas revision, live container, shared-data root, runner infrastructure or Game repository behavior changes
```

## Source branch closeout

```yaml
source_branch_disposition: auto_delete_after_merge
source_branch_reason: bounded current-global-search browser acceptance plus nested-Docker host-path repair
source_branch_evidence: pending PR 1209 merge and post-merge source-ref verification
```

## Notes

The current Atlas revision and split mount topology remain authoritative. Creature data becomes durable only after exact live product/root/range checks and the current global-search desktop/mobile E2E both pass.