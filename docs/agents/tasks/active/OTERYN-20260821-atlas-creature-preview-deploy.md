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

Deploy the terminal merged Atlas static-creature slice for `Oteryn/Oteryn-Atlas#30` to the existing LAN-only Synology FullWorld preview at `192.168.1.2:8097`, prove desktop/mobile Chromium behavior against the live endpoint, and restore the temporary trusted-main execution scaffold after success.

## Acceptance criteria

- [ ] Exact final merged Atlas revision for Issue #30 is served by `oteryn-atlas-fullworld-preview`.
- [ ] Exact merged Game producer `1ce7c60714dbd5d87da16d2eb0b8eac0c30c2282` generates the accepted `static-creatures-v1` semantic digest from pinned `blakinio/Otheryn@e417c5e7c22986bf4acef0495eb47f7b72c97cce` evidence.
- [ ] Generated census is 1,068 NPC placements, 87,565 monster/spawn placements, 88,633 total; Atlas index is 5,746 shards and 1,945 search records.
- [ ] Publication, semantic, pixel, overview, runtime-index and pixel-bucket roots remain unchanged.
- [ ] HTTP 200, exact revision header, HTTP 206 and exact Content-Range pass after cutover.
- [ ] Served FullWorld creature runtime bytes match the exact merged Atlas revision.
- [ ] Real Chromium desktop and mobile on the Synology runner prove independent NPC/Monster toggles, bounded authenticated creature loading, search/deep-link, inspector and non-empty static marker rendering.
- [ ] Cutover is fail-closed with rollback to the exact observed pre-cutover revision.
- [ ] Task-owned runner checkouts, generated temporary products, browser containers/images and superseded candidate containers are removed when ownership is proven.
- [ ] The temporary Platform workflow extension is restored after successful deployment; stale predecessor Issue #1188/PR #1190 is reconciled.
- [ ] Task records are archived, branches disposed intentionally, and Platform Issue #1191 plus Atlas Issue #30 close only after post-merge verification.

## Ownership

```yaml
owned_paths:
  - .github/workflows/repair-synology-autostart.yml
  - docs/agents/tasks/active/OTERYN-20260821-atlas-creature-preview-deploy.md
  - docs/agents/tasks/active/OTERYN-20260820-atlas-first-paint-preview-deploy.md
modules:
  - synology-staging-runner
  - atlas-fullworld-preview
dependencies:
  - Oteryn/Oteryn-Game#32 merged as 1ce7c60714dbd5d87da16d2eb0b8eac0c30c2282
  - Oteryn/Oteryn-Atlas#33 exact-head validation and merge
  - existing oteryn-staging self-hosted runner
blockers:
  - Atlas PR #33 must merge before the deployment executor can pin the final Atlas main SHA
cross_repository_tasks:
  - Oteryn/Oteryn-Atlas#30
  - Oteryn/Oteryn-Game#29
```

## Context checkpoint

```yaml
checkpoint_version: 1
policy_version: 2
phase: integrate
session_id: atlas-creature-preview-20260821-001
session_role: integrator
execution_mode: github-only
execution_reason: local workstation is intentionally unavailable; the registered Synology self-hosted runner provides the narrow trusted execution path
updated_at: 2026-08-21T06:02:00Z
lease_expires_at: 2026-08-21T06:47:00Z
head: a806d4f70e8cbddc5e7a6f0130ed669ae62651b4
branch: ops/atlas-30-creature-preview-deploy
pr: none
status: waiting
task_kind: e2e
context_pressure: medium
context_growth: stable
context_score: 8
estimate_confidence: high
decomposition_decision: phased
decomposition_reason: one cross-repository producer-consumer-deployment acceptance flow with shared immutable revisions and one live preview
validation_level: focused
last_completed_step: claimed Platform operational lifecycle and reconciled superseded cleanup PR 1190
session_rotation_count: 0
heavy_validation_runs: 0
stale_takeover_count: 1
human_interruptions: 0
context_routes:
  - synology-staging
  - github-only-execution
  - atlas-fullworld-preview
owned_paths:
  - .github/workflows/repair-synology-autostart.yml
  - docs/agents/tasks/active/OTERYN-20260821-atlas-creature-preview-deploy.md
  - docs/agents/tasks/active/OTERYN-20260820-atlas-first-paint-preview-deploy.md
proven:
  - Game producer PR 32 merged as 1ce7c60714dbd5d87da16d2eb0b8eac0c30c2282 after protected Merge Gate success
  - Atlas initial creature overlay PR 32 merged as cefbafe518e2d3bea150a88bf9d025bd8a1e474d; Issue 30 was reopened because terminal inspector/deployment acceptance remained
  - Atlas repair PR 33 is the current exact-head completion candidate for inspector/deep-link/bounded trust behavior
  - existing Platform trusted-main deployment scaffold successfully deployed prior Atlas revision f99605a69981d9a1d2bca523aec3dff67a31e175 in run 32394546737
  - predecessor cleanup PR 1190 was intentionally closed unmerged with Branch-Disposition delete because Issue 1191 must reuse the retained scaffold before final restoration
derived:
  - reusing repair-synology-autostart.yml on oteryn-staging is the narrowest compliant GitHub-only route and avoids a second live execution mechanism
unknown:
  - final squash-merge SHA of Atlas PR 33
conflicts: []
first_failure:
  marker: none
  evidence: none
rejected_hypotheses:
  - direct workstation execution is unnecessary and intentionally not used
changed_paths:
  - docs/agents/tasks/active/OTERYN-20260821-atlas-creature-preview-deploy.md
validation:
  - command: Atlas PR 33 creature-overlays workflow
    result: PASS
    evidence: run 32452804468 on repaired exact head
  - command: Atlas PR 33 standard exact-head suite
    result: NOT_RUN
    evidence: CI/CodeQL were still running at checkpoint creation
blockers:
  - Atlas PR 33 final merge SHA is required before Platform deployment workflow can be pinned safely
next_action: After Atlas PR 33 exact-head CI, Extraction Provenance, CodeQL and creature-overlay checks pass, squash-merge it and pin that exact merged SHA into the Platform deployment executor.
```

## Source branch closeout

```yaml
source_branch_disposition: auto_delete_after_merge
source_branch_reason: bounded trusted-main deployment executor; final cleanup is performed by a separate terminal restoration commit/PR after live acceptance
source_branch_evidence: pending
```

## Notes

Project lane: `oteryn-platform-core`. Platform Issue #1191 is lifecycle authority. The retained production deliverable is the existing LAN preview container plus its exact immutable revision directory; raw legacy inputs are never Atlas runtime authority.
