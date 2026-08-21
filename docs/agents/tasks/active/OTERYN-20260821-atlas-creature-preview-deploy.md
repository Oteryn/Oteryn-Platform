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

Qualify the current newer merged Atlas `71e2aca1ac49692751f5d6c59a7126835fe1896a` already served by the LAN-only Synology FullWorld preview at `192.168.1.2:8097`, prove the static-creature contract with real desktop/mobile Chromium, then restore temporary execution scaffolding and close Platform #1191 plus Atlas #30. Do not downgrade to historical target `ffb09ad6e78487fe6be5fa2f0c3a18a9a3cefc92`.

## Acceptance criteria

- [x] Live preview was observed serving newer merged Atlas `71e2aca1ac49692751f5d6c59a7126835fe1896a`.
- [x] Exact historical Game creature producer evidence proves 1,068 NPC + 87,565 monster/spawn = 88,633 placements with semantic digest `sha256:01921968a6cb4f6ecea237820a053fc5052aaa1da556851f2c2a60d99890b5e1`.
- [x] Deterministic creature product contract is 5,746 shards and 1,945 search records.
- [ ] Current live creature product matches those counts/digest and exact revision header.
- [ ] Real Chromium desktop and mobile E2E pass against current live `192.168.1.2:8097` without a cutover/downgrade.
- [ ] Browser evidence artifact is retained for 30 days.
- [ ] Task-owned ephemeral resources are removed with ownership-scoped cleanup.
- [ ] Temporary deployment/E2E scaffolding is restored/removed after successful live acceptance.
- [ ] Task records are archived and Platform #1191 plus Atlas #30 are closed only after verified live PASS.

## Ownership

```yaml
owned_paths:
  - .github/workflows/atlas-current-live-acceptance.yml
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
updated_at: 2026-08-21T16:18:00Z
head: d112a90efbde22a900555f631bdd23f71764b0cc
branch: ops/atlas-30-live-revision-conflict
pr: 1204
status: validating
context_routes:
  - synology-staging
  - atlas-fullworld-preview
owned_paths:
  - .github/workflows/atlas-current-live-acceptance.yml
  - docs/agents/tasks/active/OTERYN-20260821-atlas-creature-preview-deploy.md
proven:
  - Platform PR 1203 merged as 0a2efbd1ddcac1fe8893c604d64de715b2257323 after protected exact-head checks passed
  - trusted-main run 32500594332 proved Game creature export and deterministic index plus Playwright 1.62.1 Chromium 151.0.7922.34 runtime
  - run 32500594332 observed live Atlas 71e2aca1ac49692751f5d6c59a7126835fe1896a and made no cutover because the then-current task still targeted older ffb09ad6e78487fe6be5fa2f0c3a18a9a3cefc92
  - Atlas 71e2aca1ac49692751f5d6c59a7126835fe1896a is merged PR 37 and is two commits ahead of ffb09ad6e78487fe6be5fa2f0c3a18a9a3cefc92
  - owner explicitly instructed continuation on the newer Atlas rather than downgrade
derived:
  - because the desired Atlas revision is already live, the next safe operation is read-only live qualification rather than another cutover
unknown:
  - live desktop and mobile creature E2E result on Atlas 71e2aca1ac49692751f5d6c59a7126835fe1896a
conflicts: []
first_failure:
  marker: none on the current-revision acceptance lifecycle yet
  evidence: current live-acceptance workflow is pending protected merge/trigger
rejected_hypotheses:
  - a downgrade to ffb09ad6e78487fe6be5fa2f0c3a18a9a3cefc92 is required; owner explicitly selected continuation on newer Atlas
  - Playwright runtime preparation remains blocked; run 32500594332 launched Chromium 151.0.7922.34 successfully
changed_paths:
  - .github/workflows/atlas-current-live-acceptance.yml
  - docs/agents/tasks/active/OTERYN-20260821-atlas-creature-preview-deploy.md
validation:
  - command: Atlas PR 37 exact candidate 3ed63890b42a1f7f530a474e8e7d4b6f0a5e1ca8 protected workflows
    result: PASS
    evidence: CI 32495168029 Creature overlays 32495168086 Extraction Provenance 32495168098 CodeQL 32495168023 Semantic search 32495168026 all succeeded before merge 71e2aca1ac49692751f5d6c59a7126835fe1896a
  - command: trusted-main run 32500594332
    result: PASS_PARTIAL
    evidence: exact creature export/index and prebuilt Chromium runtime passed; no live mutation occurred and task-owned cleanup passed
blockers: []
next_action: pass exact-head Platform checks for PR 1204, merge normally, then inspect the push-triggered current-live acceptance run and fix only reproducible current-Atlas E2E failures
```

## Self-review

```yaml
self_review:
  result: PASS
  exact_head_runtime_change: d112a90efbde22a900555f631bdd23f71764b0cc
  acceptance_checked: true
  full_diff_checked: true
  negative_paths_checked: true
  rollback_checked: true
  compatibility_checked: true
  related_prs_checked: true
  findings: []
  evidence:
    - new workflow is read-only toward the live preview and performs no cutover
    - only its own bounded browser image/context is removed
```

## Source branch closeout

```yaml
source_branch_disposition: auto_delete_after_merge
source_branch_reason: bounded current-live acceptance trigger
source_branch_evidence: pending
```

## Notes

The current live Atlas is authoritative for this closeout. Historical `ffb09ad6...` remains provenance for the original creature delivery but is no longer a deployment target.
