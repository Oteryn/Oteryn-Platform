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

Qualify current merged `Oteryn/Oteryn-Atlas@71e2aca1ac49692751f5d6c59a7126835fe1896a` on the LAN-only Synology FullWorld preview at `192.168.1.2:8097`, prove static NPC/monster behavior with real desktop/mobile Chromium without downgrade, then restore temporary execution scaffolding and close Platform #1191 plus Atlas #30.

## Acceptance criteria

- [x] Live preview was observed serving Atlas `71e2aca1ac49692751f5d6c59a7126835fe1896a`.
- [x] Historical exact Game producer evidence proves 1,068 NPC + 87,565 monster/spawn = 88,633 placements with semantic digest `sha256:01921968a6cb4f6ecea237820a053fc5052aaa1da556851f2c2a60d99890b5e1`.
- [x] Deterministic creature product contract is 5,746 shards and 1,945 search records.
- [ ] Current live revision header and exact served Atlas HTML/modules match `71e2aca1ac49692751f5d6c59a7126835fe1896a`.
- [ ] Current live creature product matches expected counts/search digest/semantic digest.
- [ ] HTTP Range 206 and exact Content-Range/byte count pass.
- [ ] Real Chromium desktop and mobile E2E pass without downgrade.
- [ ] Browser evidence is retained for 30 days.
- [x] Failed-run task-owned resources are removed by ownership-scoped cleanup.
- [ ] Temporary deployment/E2E scaffolding is restored after successful live acceptance.
- [ ] Task records are archived and Platform #1191 plus Atlas #30 are closed only after verified live PASS.

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
updated_at: 2026-08-21T16:57:00Z
head: 60820c6ca4bc1310821d12f960eeab75d168158c
branch: fix/atlas-current-live-acceptance-404
pr: 1205
status: validating
context_routes:
  - synology-staging
  - atlas-fullworld-preview
owned_paths:
  - .github/workflows/repair-synology-autostart.yml
  - docs/agents/tasks/active/OTERYN-20260821-atlas-creature-preview-deploy.md
proven:
  - Platform PR 1204 exact head 34eceb53da317af21f9b185052cdae2a2e8283ee passed Agent Governance CI CodeQL DB outage Game Auth Edge Security and Phase 7 then squash-merged as b70f08ea71d70cd020f5dc8a2ae5a5e16d8d9f2a
  - trusted-main current-live acceptance run 32504909145 executed on oteryn-synology-staging and preserved the live preview without cutover
  - run 32504909145 cleanup succeeded
  - run 32504909145 failed during live resource verification before browser-runtime build; E2E was skipped
  - the failed verification emitted HTTP 404 from one required live resource, but the previous implementation did not identify which URL
  - historical trusted-main run 32500594332 already proved exact Game creature export/index and Playwright 1.62.1 Chromium 151.0.7922.34 runtime
  - PR 1205 is the bounded URL-labelled diagnostic repair for this first current-live 404
derived:
  - the current blocker is a live resource path/content availability issue, not Atlas revision selection, protected CI, Playwright installation, or runner availability
  - the next run must identify each required URL explicitly before any further repair is chosen
unknown:
  - which required live resource returned HTTP 404 in run 32504909145
  - whether the current live creature product is complete enough for Chromium E2E
conflicts: []
first_failure:
  marker: trusted-main run 32504909145 Verify current live revision product and exact served bytes
  evidence: job 96842985122 returned curl HTTP 404 before browser runtime build; E2E skipped; cleanup PASS
rejected_hypotheses:
  - current live acceptance requires a downgrade; owner selected newer Atlas and run 32504909145 performed no cutover
  - the workflow budget is still exceeded; PR 1204 removed the extra workflow and exact-head CI passed
  - checkpoint schema is still invalid; PR 1204 Agent Governance passed
changed_paths:
  - .github/workflows/repair-synology-autostart.yml
  - docs/agents/tasks/active/OTERYN-20260821-atlas-creature-preview-deploy.md
validation:
  - command: PR 1204 exact-head protected workflows at 34eceb53da317af21f9b185052cdae2a2e8283ee
    result: PASS
    evidence: Agent Governance 32504637058 CI 32504637282 CodeQL 32504637146 DB outage 32504637052 Game Auth 32504637059 Edge Security 32504637074 Phase 7 32504637051 all succeeded
  - command: trusted-main current-live acceptance run 32504909145
    result: FAIL
    evidence: job 96842985122 failed with HTTP 404 in resource verification before E2E; cleanup succeeded and no cutover occurred
  - command: PR 1205 exact-head protected checks
    result: NOT_RUN
    evidence: pending fresh checks after binding checkpoint to PR 1205
blockers: []
next_action: pass protected checks for PR 1205, merge, use the resulting trusted-main log to either complete E2E or repair only the specifically proven missing live product
```

## Self-review

```yaml
self_review:
  result: PASS
  exact_head_runtime_change: efc21204e3fc5fa67a8943f5e7721cc3766592df
  acceptance_checked: true
  full_diff_checked: true
  negative_paths_checked: true
  rollback_checked: true
  compatibility_checked: true
  related_prs_checked: true
  findings: []
  evidence:
    - correction remains read-only toward the live preview
    - every required HTTP resource now prints a named status before fail-closed evaluation
    - semantic Range target is restored to the previously proven exact chunk path and Content-Range
    - no new workflow is added and workflow budget remains unchanged
```

## Source branch closeout

```yaml
source_branch_disposition: auto_delete_after_merge
source_branch_reason: bounded current-live acceptance diagnosis/repair branch
source_branch_evidence: PR 1205
```

## Notes

No downgrade is authorized or implemented. The next trusted-main run remains read-only toward `oteryn-atlas-fullworld-preview`; any live mutation requires specific evidence that a required creature product is missing and must retain exact rollback semantics.
