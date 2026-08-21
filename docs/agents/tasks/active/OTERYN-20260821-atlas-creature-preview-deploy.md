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
  - deploy/ci/playwright-chromium.Dockerfile
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
updated_at: 2026-08-21T12:49:53Z
lease_expires_at: 2026-08-21T13:34:53Z
head: 8f5a0b634f16ca85bd8a4a3d8b7fefaff33f7301
branch: ops/atlas-30-creature-preview-cleanup
pr: 1193
status: validating
task_kind: e2e
context_pressure: medium
context_growth: stable
context_score: 8
estimate_confidence: high
decomposition_decision: phased
decomposition_reason: one cross-repository producer-consumer-deployment acceptance flow with shared immutable revisions and one live preview
validation_level: full
last_completed_step: built the Chromium-only image on Synology with DOCKER_BUILDKIT=0, launched Chromium 151.0.7922.34 successfully, and removed the exact smoke image/context
session_rotation_count: 0
heavy_validation_runs: 1
stale_takeover_count: 1
human_interruptions: 0
invocation_started_at: 2026-08-21T11:47:00Z
last_progress_at: 2026-08-21T12:49:53Z
ci_checks_for_current_head: 0
ci_check_generation: draft
terminal_ci_wait_started_at: null
terminal_ci_checks_for_current_generation: 0
unchanged_state_checks: 0
identical_failure_retries: 0
repair_cycles_for_current_gate: 1
context_reconstruction_attempts: 1
stall_warnings: 0
context_routes:
  - synology-staging
  - github-only-execution
  - atlas-fullworld-preview
owned_paths:
  - .github/workflows/repair-synology-autostart.yml
  - scripts/acceptance/atlas-creature-preview-e2e.cjs
  - deploy/ci/playwright-chromium.Dockerfile
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
  - predecessor active task released workflow ownership and now records merged PR 1189 with terminal_pr_policy archive_pending
  - Platform main 3f1a0eeb42a777106bef466dbcb4150d8a1bb818 contains the bounded deployment scaffold from merged PR 1192
  - trusted-main run 32454899481 generated the exact Game export/index successfully but was cancelled during the heavy browser-runtime step; deploy/E2E were skipped and workflow cleanup reported success
  - Synology inspection found the cancelled run still owned live docker build/docker-buildx processes for image oteryn-atlas-creature-e2e:32454899481; those exact orphaned PIDs were terminated without pruning or touching retained services
  - BuildKit on the Synology runner stalled even with a bounded approximately 8 KiB context; the same Chromium-only image build succeeds with DOCKER_BUILDKIT=0
  - Synology cold smoke built image sha256:311540338bb96e7164e579eaa29425d467352b543731f7ae4babd46bff878ac4, Playwright 1.62.1 and Chromium 151.0.7922.34; direct headless Chromium launch returned chromium-smoke=PASS and the exact smoke image/context were then removed
derived:
  - reusing repair-synology-autostart.yml on oteryn-staging is the narrowest compliant GitHub-only route and avoids a second live execution mechanism
  - the deployment retry must use a dedicated Node + Chromium-only runtime and explicitly disable BuildKit on this runner until the stuck builder is separately repaired
  - deployment build can reconstruct the normalized creature export from exact Game + pinned legacy evidence, delete raw evidence before Atlas publication, and prove deterministic output before cutover
unknown:
  - exact live pre-cutover Atlas revision; executor will accept only the explicit known revision set and record the observed value for rollback
conflicts:
  - PR 1193 head moved concurrently from 2eaa439b15fe6e3349529552c1002e903511c9c3 through premature cleanup commits while live deploy/E2E was still unproven; no force update is allowed, and PR comment 5369848636 records the do-not-merge correction
first_failure:
  marker: agent-governance run 32453591054
  evidence: predecessor terminal PR lacked archive-pending policy and this task lacked open PR identity; both task records were corrected on the same owned branch
rejected_hypotheses:
  - direct workstation execution is unnecessary and intentionally not used
changed_paths:
  - .github/workflows/repair-synology-autostart.yml
  - scripts/acceptance/atlas-creature-preview-e2e.cjs
  - deploy/ci/playwright-chromium.Dockerfile
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
    evidence: run 32453591054 exposed only the two live task-liveness record mismatches now repaired
  - command: Platform trusted-main Atlas creature preview run
    result: FAIL
    evidence: run 32454899481 cancelled while preparing the heavy PHP + Chromium + Firefox + WebKit browser runtime; Game export/index PASS, deploy/E2E skipped, cleanup PASS
  - command: local focused Platform CI contracts on Chromium-only repair
    result: PASS
    evidence: test_workflow_trigger_economy.py, test_classify_changes.py, test_push_change_routing.py, test_required_test_gate.py and git diff --check
  - command: Synology Chromium-only cold smoke
    result: PASS
    evidence: DOCKER_BUILDKIT=0 build image sha256:311540338bb96e7164e579eaa29425d467352b543731f7ae4babd46bff878ac4; Playwright 1.62.1; chromium-smoke=PASS version=151.0.7922.34; exact image/context cleanup PASS
blockers: []
next_action: Publish the reconciled repair as a non-force fast-forward on PR 1193, then require fresh exact-head protected CI before squash merge and trusted-main deployment.
```

## Recovery checkpoint

```yaml
recovery:
  policy_version: 1
  generation: 2
  session_id: atlas-creature-preview-20260821-repair-002
  session_started_at: 2026-08-21T11:47:00Z
  checkpointed_at: 2026-08-21T12:49:53Z
  last_progress_at: 2026-08-21T12:49:53Z
  phase: publish-repair
  exact_head: 8f5a0b634f16ca85bd8a4a3d8b7fefaff33f7301
  pull_request: 1193
  active_operation: publish reconciled Chromium-only repair to the stabilized PR head using a non-force fast-forward
  external_run_ids:
    - 32454899481
  operation_started_at: 2026-08-21T12:49:53Z
  wait_deadline_at: 2026-08-21T13:34:53Z
  check_generation: repair-publish
  checks_used: 0
  status: active
  safe_to_resume: true
  resume_condition: PR 1193 head still equals 8f5a0b634f16ca85bd8a4a3d8b7fefaff33f7301 before update_ref
  next_action: Publish one coherent repair commit by non-force fast-forward and verify the resulting exact PR head/diff before protected CI.
```

## Source branch closeout

```yaml
source_branch_disposition: auto_delete_after_merge
source_branch_reason: bounded trusted-main deployment executor; final cleanup is performed by a separate terminal restoration commit/PR after live acceptance
source_branch_evidence: pending
```

## Notes

Project lane: `oteryn-platform-core`. Platform Issue #1191 is lifecycle authority. The retained production deliverable is the existing LAN preview container plus its exact immutable final Atlas revision directory; raw legacy inputs are never Atlas runtime authority.
