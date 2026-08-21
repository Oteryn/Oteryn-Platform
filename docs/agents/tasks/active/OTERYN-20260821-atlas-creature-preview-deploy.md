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

Qualify the current merged `Oteryn/Oteryn-Atlas@71e2aca1ac49692751f5d6c59a7126835fe1896a` already served by the LAN-only Synology FullWorld preview at `192.168.1.2:8097`, prove static NPC/monster behavior with real desktop/mobile Chromium without any downgrade or cutover, then restore temporary execution scaffolding and close Platform #1191 plus Atlas #30.

## Acceptance criteria

- [x] Live preview was observed serving Atlas `71e2aca1ac49692751f5d6c59a7126835fe1896a`.
- [x] Historical exact Game producer evidence proves 1,068 NPC + 87,565 monster/spawn = 88,633 placements with semantic digest `sha256:01921968a6cb4f6ecea237820a053fc5052aaa1da556851f2c2a60d99890b5e1`.
- [x] Deterministic creature product contract is 5,746 shards and 1,945 search records.
- [ ] Current live revision header and exact served Atlas HTML/creature/search module bytes match `71e2aca1ac49692751f5d6c59a7126835fe1896a`.
- [ ] Current live creature product matches the expected counts, search digest and semantic digest.
- [ ] HTTP Range returns 206 with exact Content-Range and byte count.
- [ ] Real Chromium desktop and mobile E2E pass against `192.168.1.2:8097` with no cutover/downgrade.
- [ ] Browser evidence artifact is retained for 30 days.
- [ ] Task-owned ephemeral execution resources are removed with ownership-scoped cleanup.
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
updated_at: 2026-08-21T16:40:00Z
head: 1b798b95bd36a04b48219bd7bb97de9410c8b5cf
branch: ops/atlas-30-live-revision-conflict
pr: 1204
status: validating
context_routes:
  - synology-staging
  - atlas-fullworld-preview
owned_paths:
  - .github/workflows/repair-synology-autostart.yml
  - docs/agents/tasks/active/OTERYN-20260821-atlas-creature-preview-deploy.md
proven:
  - Platform PR 1203 merged as 0a2efbd1ddcac1fe8893c604d64de715b2257323 after protected exact-head checks passed
  - trusted-main run 32500594332 proved exact historical Game creature export counts and digest plus deterministic 5746-chunk 1945-search-record index
  - trusted-main run 32500594332 proved the prebuilt Playwright 1.62.1 Chromium 151.0.7922.34 runtime and ownership-scoped cleanup
  - run 32500594332 observed live Atlas 71e2aca1ac49692751f5d6c59a7126835fe1896a and failed closed before any downgrade to historical ffb09ad6e78487fe6be5fa2f0c3a18a9a3cefc92
  - Atlas 71e2aca1ac49692751f5d6c59a7126835fe1896a is merged PR 37 and is two commits ahead of historical ffb09ad6e78487fe6be5fa2f0c3a18a9a3cefc92
  - owner explicitly instructed continuation on the newer Atlas rather than downgrade
  - the repository workflow budget is 53 so current-live acceptance is integrated into existing repair-synology-autostart.yml rather than adding a workflow
derived:
  - because the selected Atlas revision is already live, the safe next operation is qualification only and the current workflow contains no preview cutover path
  - the existing creature E2E remains compatible because current fullworld-creatures.mjs still injects creature-search and creature-results alongside the newer global semantic search
unknown:
  - exact-head protected CI result for the reconciled PR 1204 candidate
  - trusted-main current-live desktop/mobile E2E result after merge
conflicts: []
first_failure:
  marker: prior PR 1204 candidate 85920e63026eab91be5d2751e4ef8345420c21f6 failed repository admission
  evidence: Agent Governance rejected unsupported checkpoint result PASS_PARTIAL and CI rejected a 54th unregistered workflow above the budget of 53; both causes are removed in the current candidate
rejected_hypotheses:
  - a downgrade to ffb09ad6e78487fe6be5fa2f0c3a18a9a3cefc92 is required; owner explicitly selected current newer Atlas
  - Playwright runtime preparation remains blocked; run 32500594332 launched Chromium 151.0.7922.34 successfully
  - a separate workflow is required for current-live acceptance; existing repair-synology-autostart.yml is already the trusted Synology execution channel and stays within the workflow budget
changed_paths:
  - .github/workflows/repair-synology-autostart.yml
  - docs/agents/tasks/active/OTERYN-20260821-atlas-creature-preview-deploy.md
validation:
  - command: Atlas PR 37 exact candidate 3ed63890b42a1f7f530a474e8e7d4b6f0a5e1ca8 protected workflows
    result: PASS
    evidence: CI 32495168029 Creature overlays 32495168086 Extraction Provenance 32495168098 CodeQL 32495168023 and Semantic search 32495168026 all succeeded before merge 71e2aca1ac49692751f5d6c59a7126835fe1896a
  - command: trusted-main historical deployment attempt 32500594332
    result: FAIL
    evidence: export index and browser runtime passed but stage intentionally failed closed on newer live Atlas 71e2aca1ac49692751f5d6c59a7126835fe1896a before mutation; cleanup passed
  - command: reconciled PR 1204 exact-head protected checks
    result: NOT_RUN
    evidence: pending fresh checks after workflow-budget and checkpoint-contract repair
blockers: []
next_action: verify fresh exact-head checks for PR 1204, merge only after all required gates pass, then inspect the trusted-main current-live acceptance run and continue to terminal scaffold restoration and issue closeout only after live PASS
```

## Self-review

```yaml
self_review:
  result: PASS
  exact_head_runtime_change: 1b798b95bd36a04b48219bd7bb97de9410c8b5cf
  acceptance_checked: true
  full_diff_checked: true
  negative_paths_checked: true
  rollback_checked: true
  compatibility_checked: true
  related_prs_checked: true
  findings: []
  evidence:
    - current candidate preserves the existing persistent Synology restart-policy repair job
    - Atlas acceptance is read-only toward the live preview and performs no docker rm/run cutover of oteryn-atlas-fullworld-preview
    - task-owned browser image and temporary checkout/context are removed in the always-run cleanup step
    - no new workflow file remains in the net diff
```

## Source branch closeout

```yaml
source_branch_disposition: auto_delete_after_merge
source_branch_reason: bounded terminal current-live qualification branch
source_branch_evidence: pending
```

## Notes

Platform Issue #1191 remains lifecycle authority. The current live Atlas `71e2aca1ac49692751f5d6c59a7126835fe1896a` is the selected acceptance target. Historical `ffb09ad6...` remains provenance for the original creature delivery and must not replace the newer live revision.
