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
- [x] Task-owned execution resources from failed run 32500594332 were cleaned by the always-run ownership-scoped cleanup step.
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
blockers:
  - live preview now serves newer Atlas 71e2aca1ac49692751f5d6c59a7126835fe1896a, so the task's fail-closed stage correctly refuses to overwrite it with older target ffb09ad6e78487fe6be5fa2f0c3a18a9a3cefc92 without explicit reconciliation authority
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-08-21T16:12:00Z
head: 0a2efbd1ddcac1fe8893c604d64de715b2257323
branch: ops/atlas-30-live-revision-conflict
pr: pending-blocker-checkpoint
status: blocked
context_routes:
  - synology-staging
  - atlas-fullworld-preview
owned_paths:
  - docs/agents/tasks/active/OTERYN-20260821-atlas-creature-preview-deploy.md
proven:
  - Platform PR 1203 passed all exact-head protected checks on ad83a0549d5cc8fa766a0728e8634f81f8ac2e39 and squash-merged as 0a2efbd1ddcac1fe8893c604d64de715b2257323
  - trusted-main run 32500594332 executed on runner oteryn-synology-staging
  - run 32500594332 rebuilt exact Game creature export with 1068 NPC 87565 monster-spawn 461 unresolved 5 ambiguous semantic digest sha256:01921968a6cb4f6ecea237820a053fc5052aaa1da556851f2c2a60d99890b5e1
  - run 32500594332 built deterministic Atlas index with 88633 records 5746 chunks and 1945 search records
  - run 32500594332 built the Playwright 1.62.1 prebuilt runtime and launched Chromium 151.0.7922.34 successfully without apt-dpkg browser installation
  - stage in run 32500594332 observed live Atlas revision 71e2aca1ac49692751f5d6c59a7126835fe1896a and failed before cutover because it is outside the authorized historical revision allowlist
  - Atlas 71e2aca1ac49692751f5d6c59a7126835fe1896a is the merged PR 37 global-search release and is two commits ahead of target ffb09ad6e78487fe6be5fa2f0c3a18a9a3cefc92
  - deploy and desktop-mobile E2E were skipped in run 32500594332 and cleanup succeeded
derived:
  - adding 71e2aca1ac49692751f5d6c59a7126835fe1896a to the rollback allowlist and deploying ffb09ad6e78487fe6be5fa2f0c3a18a9a3cefc92 would intentionally downgrade a newer merged Atlas runtime and remove later global-search functionality
unknown:
  - which separately authorized deployment lifecycle placed Atlas 71e2aca1ac49692751f5d6c59a7126835fe1896a on the shared 192.168.1.2:8097 preview
  - whether the owner intends the older exact ffb09ad6e78487fe6be5fa2f0c3a18a9a3cefc92 acceptance target to replace the newer live Atlas revision
conflicts:
  - task exact-delivery target is ffb09ad6e78487fe6be5fa2f0c3a18a9a3cefc92 while authoritative live shared preview is already a newer descendant 71e2aca1ac49692751f5d6c59a7126835fe1896a
first_failure:
  marker: trusted-main run 32500594332 stage rejected unexpected live Atlas revision 71e2aca1ac49692751f5d6c59a7126835fe1896a
  evidence: job 96829355119 step Inspect live preview and stage exact Atlas revision exited 1 before cutover; deployment and E2E were skipped; cleanup succeeded
rejected_hypotheses:
  - prebuilt Playwright runtime is still the blocker; run 32500594332 proved Playwright 1.62.1 and Chromium 151.0.7922.34 PASS
  - creature export or deterministic index generation is failing; both completed PASS with exact expected census digest and counts in run 32500594332
changed_paths:
  - docs/agents/tasks/active/OTERYN-20260821-atlas-creature-preview-deploy.md
validation:
  - command: exact-head protected CI for PR 1203 head ad83a0549d5cc8fa766a0728e8634f81f8ac2e39
    result: PASS
    evidence: Agent Governance CI and all other exact-head protected workflows completed success before expected-head squash merge
  - command: trusted-main deployment run 32500594332 job 96829355119
    result: FAIL
    evidence: export index and browser runtime PASS; stage failed closed on newer live Atlas 71e2aca1ac49692751f5d6c59a7126835fe1896a before any cutover; cleanup PASS
  - command: compare Atlas ffb09ad6e78487fe6be5fa2f0c3a18a9a3cefc92 to live 71e2aca1ac49692751f5d6c59a7126835fe1896a
    result: PASS
    evidence: GitHub comparison reports status ahead by 2 and behind by 0 with ffb09ad6e78487fe6be5fa2f0c3a18a9a3cefc92 as merge base
blockers:
  - unresolved shared-preview ownership conflict prevents safe downgrade from newer live Atlas 71e2aca1ac49692751f5d6c59a7126835fe1896a to exact historical target ffb09ad6e78487fe6be5fa2f0c3a18a9a3cefc92
next_action: reconcile whether the shared preview must preserve newer Atlas 71e2aca1ac49692751f5d6c59a7126835fe1896a or is explicitly authorized to downgrade to ffb09ad6e78487fe6be5fa2f0c3a18a9a3cefc92 before any further live mutation
```

## Self-review

```yaml
self_review:
  result: BLOCKED
  exact_head_runtime_change: 0a2efbd1ddcac1fe8893c604d64de715b2257323
  acceptance_checked: true
  full_diff_checked: true
  negative_paths_checked: true
  rollback_checked: true
  compatibility_checked: true
  related_prs_checked: true
  findings:
    - newer live Atlas descendant makes the historical exact-target cutover a functional downgrade rather than an ordinary retry
  evidence:
    - run 32500594332 failed closed before mutation on live revision 71e2aca1ac49692751f5d6c59a7126835fe1896a
    - GitHub comparison proves live revision is two commits ahead of the requested target
```

## Source branch closeout

```yaml
source_branch_disposition: retain_until_blocker_reconciled
source_branch_reason: durable blocker checkpoint for the shared live preview conflict
source_branch_evidence: pending
```

## Notes

Platform Issue #1191 remains lifecycle authority. No live downgrade, issue closure, task archival, or deployment-scaffold restoration is safe until the newer shared-preview revision conflict is explicitly reconciled.
