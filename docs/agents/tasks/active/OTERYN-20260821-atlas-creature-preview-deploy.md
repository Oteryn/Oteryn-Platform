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
checkpoint_version: 1
updated_at: 2026-08-21T15:55:00Z
head: 590d412d3336ed4b15132a40888c91f56b1879f2
branch: fix/atlas-30-prebuilt-playwright-runtime
pr: 1203
status: validating
context_routes:
  - synology-staging
  - atlas-fullworld-preview
owned_paths:
  - deploy/ci/playwright-chromium.Dockerfile
  - deploy/synology/compose.yml
  - docs/agents/tasks/active/OTERYN-20260821-atlas-creature-preview-deploy.md
proven:
  - Atlas target revision ffb09ad6e78487fe6be5fa2f0c3a18a9a3cefc92 is merged upstream
  - trusted-main run 32488861824 completed Game export/index and was cancelled before stage/cutover/live E2E
  - live preview remained on revision 95c02ded6b8793b68effacb995f206fa462b42f9 after the cancelled run
  - Playwright 1.62.1 prebuilt runtime smoke launched Chromium 151.0.7922.34 headless on Synology and the exact smoke image/context were removed
  - PR 1203 removes runtime playwright install --with-deps chromium and uses Microsoft Playwright v1.62.1 noble linux-amd64 manifest sha256:c091b21d9fae78c76e85cd4356431e9b018402f172a214fc7d7a5e9a7e29d8ac
derived:
  - a corrected checkpoint-only commit should regenerate protected checks without invalidating the already-proven bounded browser runtime smoke
unknown:
  - protected exact-head CI result after checkpoint repair
  - trusted-main deployment result after eventual merge
  - final live desktop and mobile E2E result
conflicts: []
first_failure:
  marker: checkpoint-validation failed on exact head 590d412d3336ed4b15132a40888c91f56b1879f2
  evidence: Agent Governance run 32499399999 job 96825346447 reported missing required checkpoint fields, checkpoint_version mismatch, and validation item 1 missing evidence
rejected_hypotheses:
  - browser runtime repair itself caused the current protected CI failure; current first failure is checkpoint schema validation
changed_paths:
  - deploy/ci/playwright-chromium.Dockerfile
  - deploy/synology/compose.yml
  - docs/agents/tasks/active/OTERYN-20260821-atlas-creature-preview-deploy.md
validation:
  - command: Synology prebuilt Playwright 1.62.1 bounded smoke
    result: PASS
    evidence: Chromium 151.0.7922.34 launched headless and exact task smoke image/context were removed
  - command: Agent Governance run 32499399999 checkpoint-validation job 96825346447
    result: FAIL
    evidence: validator reported checkpoint_version must be 1 plus missing blockers changed_paths context_routes derived first_failure head owned_paths rejected_hypotheses unknown and validation evidence
blockers: []
next_action: verify fresh protected CI on the new exact head, inspect the first relevant failed job if any, and continue toward squash merge only after all exact-head gates pass
```

## Self-review

```yaml
self_review:
  result: PASS
  exact_head_runtime_change: 590d412d3336ed4b15132a40888c91f56b1879f2
  acceptance_checked: true
  full_diff_checked: true
  negative_paths_checked: true
  rollback_checked: true
  compatibility_checked: true
  related_prs_checked: true
  findings: []
  evidence:
    - PR 1203 net diff contains the prebuilt browser runtime, comment-only trusted-main trigger, and active task checkpoint
    - PR 1200 overlap on deploy/synology/runner/compose.yml is absent from the net diff
```

## Source branch closeout

```yaml
source_branch_disposition: auto_delete_after_merge
source_branch_reason: bounded terminal deployment repair branch
source_branch_evidence: pending
```

## Notes

Platform Issue #1191 remains lifecycle authority. The retained operational deliverable is the LAN preview container plus the exact immutable Atlas revision directory. Raw legacy inputs never become Atlas runtime authority.
