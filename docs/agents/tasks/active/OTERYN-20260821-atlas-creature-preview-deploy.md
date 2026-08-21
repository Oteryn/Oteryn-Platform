---
task_id: OTERYN-20260821-atlas-creature-preview-deploy
required_reads:
  - AGENTS.md
  - docs/agents/PLATFORM_AGENT_BOOTSTRAP.md
  - docs/agents/CONTEXT_HANDOFF.md
  - docs/agents/GITHUB_ONLY_EXECUTION.md
  - docs/agents/EXECUTION_RESOURCE_HYGIENE.md
  - docs/agents/CI_WORKFLOW_LIFECYCLE.md
  - docs/agents/DELIVERY_COMPLETENESS_AND_CLOSEOUT.md
  - docs/agents/SESSION_RECOVERY_AND_ORPHANED_EXECUTION.md
search_first:
  - synology atlas preview
  - repair-synology-autostart
optional_reads: []
---

# OTERYN-20260821-atlas-creature-preview-deploy

## Goal

Deploy exact terminal merged `Oteryn/Oteryn-Atlas@ffb09ad6e78487fe6be5fa2f0c3a18a9a3cefc92` for `Oteryn/Oteryn-Atlas#30` to the existing LAN-only Synology FullWorld preview at `192.168.1.2:8097`, prove desktop/mobile Chromium behavior against the live endpoint, and restore the temporary trusted-main execution scaffold after success.

## Acceptance criteria

- [ ] Exact Atlas `ffb09ad6e78487fe6be5fa2f0c3a18a9a3cefc92` is served by `oteryn-atlas-fullworld-preview`.
- [ ] Exact merged Game producer `1ce7c60714dbd5d87da16d2eb0b8eac0c30c2282` generates the accepted `static-creatures-v1` semantic digest from pinned `blakinio/Otheryn@e417c5e7c22986bf4acef0495eb47f7b72c97cce` evidence.
- [ ] Generated census is 1,068 NPC placements, 87,565 monster/spawn placements, 88,633 total; Atlas index is 5,746 shards and 1,945 search records.
- [ ] Publication, semantic, pixel, overview, runtime-index, pixel-bucket and verified minimap roots remain unchanged.
- [ ] HTTP 200, exact revision header, HTTP 206 and exact Content-Range pass after cutover.
- [ ] Served FullWorld creature runtime bytes and generated creature products match the exact merged Atlas/Game products.
- [ ] Real Chromium desktop and mobile on the Synology runner prove independent NPC/Monster toggles, bounded authenticated creature loading/cache, search/deep-link, factual inspector and non-empty static marker rendering.
- [ ] Cutover is fail-closed with rollback to the exact observed pre-cutover revision.
- [ ] Task-owned runner checkouts, generated temporary products, browser images and superseded candidate containers are removed only when ownership is proven.
- [ ] Bounded E2E screenshots/result/log are retained as a 30-day workflow artifact.
- [ ] The temporary Platform workflow extension and task-specific E2E script are removed after successful deployment; stale predecessor Issue #1188/PR #1190 is reconciled.
- [ ] Task records are archived, branches disposed intentionally, and Platform Issue #1191 plus Atlas Issue #30 close only after post-merge verification.

## Ownership

```yaml
owned_paths:
  - .github/workflows/repair-synology-autostart.yml
  - scripts/acceptance/atlas-creature-preview-e2e.cjs
  - docs/agents/tasks/active/OTERYN-20260821-atlas-creature-preview-deploy.md
  - docs/agents/tasks/active/OTERYN-20260820-atlas-first-paint-preview-deploy.md
modules:
  - synology-staging-runner
  - atlas-fullworld-preview
dependencies:
  - Oteryn/Oteryn-Game#32 merged as 1ce7c60714dbd5d87da16d2eb0b8eac0c30c2282
  - Oteryn/Oteryn-Atlas#33 merged as ffb09ad6e78487fe6be5fa2f0c3a18a9a3cefc92
  - existing oteryn-staging self-hosted runner
blockers: []
cross_repository_tasks:
  - Oteryn/Oteryn-Atlas#30
  - Oteryn/Oteryn-Game#29
```

## Context checkpoint

```yaml
checkpoint_version: 1
policy_version: 2
phase: validate
session_id: atlas-creature-preview-20260821-001
session_role: integrator
execution_mode: github-only
execution_reason: the registered Synology self-hosted runner is the narrow trusted execution path for the LAN-only preview; no workstation dependency is required
updated_at: 2026-08-21T12:31:33Z
lease_expires_at: 2026-08-21T13:16:33Z
head: 3f1a0eeb42a777106bef466dbcb4150d8a1bb818
branch: ops/atlas-30-creature-preview-deploy
pr: 1192
status: validating
terminal_pr_policy: archive_pending
task_kind: e2e
context_pressure: medium
context_growth: stable
context_score: 8
estimate_confidence: high
decomposition_decision: phased
decomposition_reason: one cross-repository producer-consumer-deployment acceptance flow with shared immutable revisions and one live preview
validation_level: full
last_completed_step: merged PR 1192 reached trusted main; two bounded deployment attempts failed before stage/cutover while cleanup passed, isolating the current blocker to Chromium runtime preparation
session_rotation_count: 0
heavy_validation_runs: 3
stale_takeover_count: 1
human_interruptions: 0
context_routes:
  - synology-staging
  - github-only-execution
  - atlas-fullworld-preview
owned_paths:
  - .github/workflows/repair-synology-autostart.yml
  - scripts/acceptance/atlas-creature-preview-e2e.cjs
  - docs/agents/tasks/active/OTERYN-20260821-atlas-creature-preview-deploy.md
  - docs/agents/tasks/active/OTERYN-20260820-atlas-first-paint-preview-deploy.md
proven:
  - Game producer PR 32 merged as 1ce7c60714dbd5d87da16d2eb0b8eac0c30c2282 after protected Merge Gate success
  - Game real pinned corpus produced 1,068 NPC + 87,565 monster/spawn placements, 461 unresolved, 5 ambiguous, digest sha256:01921968a6cb4f6ecea237820a053fc5052aaa1da556851f2c2a60d99890b5e1
  - Atlas initial creature overlay PR 32 merged as cefbafe518e2d3bea150a88bf9d025bd8a1e474d
  - Atlas terminal repair PR 33 exact head 43512c25fbe01702e4d7c578e72b8610e9d1aa81 passed Creature overlays run 32452804468, Extraction Provenance 32452804485, CI/atlas-gate 32452804501 and CodeQL 32452804539
  - Atlas PR 33 squash-merged as ffb09ad6e78487fe6be5fa2f0c3a18a9a3cefc92
  - existing Platform trusted-main deployment scaffold successfully deployed prior Atlas revision f99605a69981d9a1d2bca523aec3dff67a31e175 in run 32394546737
  - predecessor cleanup PR 1190 was intentionally closed unmerged with Branch-Disposition delete because Issue 1191 reuses the retained scaffold before final restoration
  - predecessor active task released workflow ownership and records merged PR 1189 with terminal_pr_policy archive_pending
  - Platform PR 1192 squash-merged to main as 3f1a0eeb42a777106bef466dbcb4150d8a1bb818 and is now terminal
  - trusted-main deployment run 32454899481 attempt 1 was cancelled before inspect/stage/cutover and its cleanup passed
  - trusted-main deployment run 32454899481 attempt 2 rebuilt the exact Game export and Atlas index successfully, then failed in Chromium runtime preparation before inspect/stage/cutover; cleanup passed
  - attempt 2 runtime preparation tried to install Chromium Firefox and WebKit dependencies in the generic Playwright image, pulling 322 packages before the Docker build received SIGTERM exit 143
  - neither deployment attempt changed the live Atlas preview because all stage and cutover steps were skipped
  - the live deployment failure is isolated to test-runtime preparation and does not invalidate the already-proven Atlas/Game product artifacts
derived:
  - the next repair should validate and reuse the cached Chromium runtime or prepare a Chromium-specific runtime without rebuilding unused Firefox/WebKit dependencies
  - reusing repair-synology-autostart.yml on oteryn-staging remains the narrowest compliant GitHub-only route and avoids a second live deployment mechanism
unknown:
  - exact live pre-cutover Atlas revision; executor will accept only the explicit known revision set and record the observed value for rollback
conflicts: []
first_failure:
  marker: trusted-main deployment run 32454899481 attempt 2 Chromium runtime preparation
  evidence: Docker Playwright dependency installation reached 322 packages and exited 143 before stage/cutover; cleanup PASS
rejected_hypotheses:
  - direct workstation execution is unnecessary and intentionally not used
  - repeating the same full three-browser Playwright rebuild is not a valid repair after the identical runtime-preparation failure
changed_paths:
  - .github/workflows/repair-synology-autostart.yml
  - scripts/acceptance/atlas-creature-preview-e2e.cjs
  - docs/agents/tasks/active/OTERYN-20260821-atlas-creature-preview-deploy.md
  - docs/agents/tasks/active/OTERYN-20260820-atlas-first-paint-preview-deploy.md
validation:
  - command: Atlas PR 33 Creature overlays
    result: PASS
    evidence: run 32452804468
  - command: Atlas PR 33 Extraction Provenance
    result: PASS
    evidence: run 32452804485
  - command: Atlas PR 33 CI / atlas-gate / real Chrome WebGL proof
    result: PASS
    evidence: run 32452804501
  - command: Atlas PR 33 CodeQL
    result: PASS
    evidence: run 32452804539
  - command: Platform PR 1192 first Agent Governance exact-head attempt
    result: FAIL
    evidence: run 32453591054 exposed live task-liveness record mismatches that were repaired before the implementation delivery
  - command: trusted-main Atlas deployment attempt 1
    result: FAIL
    evidence: run 32454899481 attempt 1 cancelled before stage/cutover; cleanup PASS
  - command: trusted-main Atlas deployment attempt 2
    result: FAIL
    evidence: run 32454899481 attempt 2 failed only in Chromium runtime preparation with exit 143; stage/cutover skipped and cleanup PASS
blockers:
  - Chromium acceptance runtime preparation must be repaired before another trusted-main deployment attempt
next_action: Repair Chromium runtime preparation, complete the trusted-main deployment and live desktop/mobile E2E, then archive this task through terminal cleanup PR 1193.
```

## Source branch closeout

```yaml
source_branch_disposition: auto_delete_after_merge
source_branch_reason: bounded trusted-main deployment executor; final cleanup is performed by a separate terminal restoration commit/PR after live acceptance
source_branch_evidence: PR 1192 merged to main as 3f1a0eeb42a777106bef466dbcb4150d8a1bb818; final task archival remains pending live deployment acceptance and PR 1193
```

## Notes

Project lane: `oteryn-platform-core`. Platform Issue #1191 is lifecycle authority. The retained production deliverable is the existing LAN preview container plus its exact immutable final Atlas revision directory; raw legacy inputs are never Atlas runtime authority.
