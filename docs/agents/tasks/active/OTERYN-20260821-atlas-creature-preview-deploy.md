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

Deploy exact merged `Oteryn/Oteryn-Atlas@ffb09ad6e78487fe6be5fa2f0c3a18a9a3cefc92` to the LAN-only Synology FullWorld preview at `192.168.1.2:8097`, prove live desktop/mobile Chromium behavior, then restore temporary execution scaffolding and close Platform #1191 plus Atlas #30.

## Acceptance criteria

- [ ] `oteryn-atlas-fullworld-preview` serves exact Atlas `ffb09ad6e78487fe6be5fa2f0c3a18a9a3cefc92`.
- [x] Exact Game `1ce7c60714dbd5d87da16d2eb0b8eac0c30c2282` produces 1,068 NPC + 87,565 monster/spawn = 88,633 placements with semantic digest `sha256:01921968a6cb4f6ecea237820a053fc5052aaa1da556851f2c2a60d99890b5e1`.
- [x] Atlas deterministic index contract is 5,746 shards and 1,945 search records.
- [ ] Publication, semantic, pixel, overview, runtime-index, pixel-bucket and minimap roots remain unchanged after cutover.
- [ ] HTTP 200/revision header, HTTP 206/Content-Range and served-byte equality pass live.
- [ ] Real Chromium desktop and mobile E2E pass against `192.168.1.2:8097`.
- [ ] Exact pre-cutover rollback remains available until acceptance succeeds.
- [ ] Browser evidence artifact is retained for 30 days.
- [ ] Task-owned ephemeral resources are removed with ownership-scoped cleanup.
- [ ] Temporary workflow/E2E scaffolding is restored/removed after successful live acceptance.
- [ ] Task records are archived and Platform #1191 plus Atlas #30 are closed only after verified live PASS.

## Ownership

```yaml
owned_paths:
  - .github/workflows/repair-synology-autostart.yml
  - scripts/acceptance/atlas-creature-preview-e2e.cjs
  - deploy/ci/playwright-chromium.Dockerfile
  - deploy/synology/compose.yml
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
checkpoint_version: 2
phase: validate
session_id: atlas-creature-preview-20260821-repair-003
session_role: integrator
execution_mode: github-plus-synology-runner
updated_at: 2026-08-21T15:35:00Z
branch: fix/atlas-30-prebuilt-playwright-runtime
pr: 1203
status: validating
task_kind: e2e
validation_level: full
last_completed_step: replaced runtime browser installation with an official prebuilt Playwright 1.62.1 Chromium image pinned to the linux-amd64 manifest and removed active PR 1200 path overlap
proven:
  - Atlas target revision ffb09ad6e78487fe6be5fa2f0c3a18a9a3cefc92 is already merged upstream
  - Platform PR 1193 merged as f664d8d82b5421a47856b54dc75a5a68d27fd9fc
  - trusted-main run 32488861824 completed Game export/index, then was cancelled in browser-runtime preparation; stage/cutover/live E2E were skipped and cleanup passed
  - live preview remained healthy on revision 95c02ded6b8793b68effacb995f206fa462b42f9 after the cancelled run
  - cancelled runtime failure was inside playwright install --with-deps chromium / apt-dpkg on Synology
  - Microsoft Playwright v1.62.1-noble publishes linux-amd64 manifest sha256:c091b21d9fae78c76e85cd4356431e9b018402f172a214fc7d7a5e9a7e29d8ac
  - PR 1203 removes playwright browser/dependency installation from the Synology Docker build and uses matching @playwright/test 1.62.1 without browser download
  - earlier bounded prebuilt-image smoke with Playwright 1.62.0 launched Chromium 151.0.7922.34 successfully; exact temporary custom image/context were removed
conflicts: []
validation:
  - command: trusted-main run 32488861824 Game export/index
    result: PASS
  - command: trusted-main run 32488861824 deploy/E2E
    result: FAIL
    evidence: cancelled before stage/cutover; cleanup PASS
  - command: PR 1203 full diff self-review
    result: PASS
    evidence: only prebuilt browser runtime plus comment-only trusted-main trigger; active PR 1200 overlap removed from net diff
  - command: Synology prebuilt Playwright bounded smoke
    result: PASS
    evidence: Chromium 151.0.7922.34 launched headless; task-owned smoke image/context removed
next_action: complete exact Playwright 1.62.1 smoke and protected CI on PR 1203, then squash merge and verify the resulting trusted-main cutover plus desktop/mobile E2E before terminal restoration/issue closeout
```

## Self-review

```yaml
self_review:
  result: PASS
  exact_head_runtime_change: 1597c978c7800c25f401752c39dafe977a1b09ef
  acceptance_checked: true
  full_diff_checked: true
  negative_paths_checked: true
  rollback_checked: true
  compatibility_checked: true
  related_prs_checked: true
  findings: []
  evidence:
    - PR 1203 net diff contains deploy/ci/playwright-chromium.Dockerfile and comment-only deploy/synology/compose.yml trigger
    - PR 1200 overlap on deploy/synology/runner/compose.yml was reverted before readiness
```

## Source branch closeout

```yaml
source_branch_disposition: auto_delete_after_merge
source_branch_reason: bounded terminal deployment repair branch
source_branch_evidence: pending
```

## Notes

Platform Issue #1191 remains lifecycle authority. The retained operational deliverable is the LAN preview container plus the exact immutable Atlas revision directory. Raw legacy inputs never become Atlas runtime authority.
