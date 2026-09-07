---
task_id: OTERYN-20260906-premium-portal-redesign
governing_issue: 1297
required_reads:
  - docs/architecture/MODULE_CATALOG.md
  - docs/architecture/TEST_STRATEGY.md
  - docs/agents/BUILD_TEST_MATRIX.md
search_first:
  - resources/views
  - public/css
  - scripts/acceptance
optional_reads: []
---

# Reference-aligned Oteryn player portal integration

## Goal and boundary

Issue #1297, existing PR #1298. Complete the reference-aligned player-facing Laravel/Blade redesign and integrate through protected main after exact-candidate verification. The owner's current instruction “masz to wdrozyc na main” supersedes the earlier no-merge hold. No production deployment, live credentials/data/payment operation, protection bypass, backend contract change or cross-repository work.

## Acceptance criteria

- [x] Preserve the single owned redesign PR and prior functional page-family coverage.
- [x] Prepare the shared art direction, homepage/news composition, icons, EN/PL and scenery dependencies.
- [ ] Pass exact-candidate PHP/static and required/browser checks without suppressing failures.
- [ ] Inspect real EN/PL guest/account/edge-state renders across phone/tablet/desktop/wide.
- [ ] Integrate through the protected PR route, verify main, archive the terminal task and verify source-branch disposition.

## Ownership

One writer owns this task's player-facing presentation, focused test repairs and evidence. Administrator design, controllers, domain/security/payment behavior, routes, databases, workflows and unrelated tasks remain untouched. The former temporary art-direction execution prompt is retired; its history is preserved in Git.

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: '2026-09-07T14:39:29Z'
head: 341e5021499f3b437a4985409e4792436529a504
branch: feat/20260906-premium-portal-redesign
pr: 1298
status: validating
context_routes:
  - web-cms
  - testing
  - agent-governance
owned_paths:
  - resources/views/**
  - public/css/portal-art-direction.css
  - public/css/home-production.css
  - public/images/oteryn-vistas.webp
  - lang/en/portal_art.php
  - lang/pl/portal_art.php
  - scripts/acceptance/tests/portal-visual-review.spec.mjs
  - scripts/acceptance/tests/support-legal-acceptance.spec.mjs
  - tests/Feature/PortalArtDirectionTest.php
  - docs/testing/PORTAL_ART_DIRECTION_CORRECTION_2026-09-07.md
  - docs/agents/tasks/active/OTERYN-20260906-premium-portal-redesign.md
  - docs/agents/prompts/OTERYN-PORTAL-ART-DIRECTED-REDESIGN-ASTRA.md
proven:
  - Owner now explicitly authorizes main integration in the current conversation; PR comment records that update. Production and protection bypass remain excluded.
  - Fresh protected main is 721d560c18ef8a6eefbf4ccd685d8302985bbd3d; original PR head is 341e5021499f3b437a4985409e4792436529a504.
  - Prepared shared art layer, homepage/news composition, icons, EN/PL copy and decorative source crops form one coherent presentation package.
  - The exact current visual test blob d285807369ad40c73ca5796c451117615aecd2b7 was reconstructed before two fixture repairs; support test starting blob is f0af2046ab523243972da0594ea1cfcc280c83e6.
derived:
  - The registered acceptance identity provides the backing account that the former read-model-only fixture omitted.
unknown:
  - New exact-candidate hosted PHP/static/browser results and complete visual acceptance are pending.
conflicts: []
first_failure:
  marker: PRIOR_BROWSER_FIXTURE_FAILURE
  evidence: Historical acceptance 34124102942 / artifact 10019441659 fails character creation and worker fixture setup; old results are not new candidate proof.
rejected_hypotheses:
  - A main integration request authorizes production or protection bypass.
changed_paths:
  - resources/views/**
  - public/css/portal-art-direction.css
  - public/css/home-production.css
  - public/images/oteryn-vistas.webp
  - lang/en/portal_art.php
  - lang/pl/portal_art.php
  - scripts/acceptance/tests/portal-visual-review.spec.mjs
  - scripts/acceptance/tests/support-legal-acceptance.spec.mjs
  - tests/Feature/PortalArtDirectionTest.php
  - docs/testing/PORTAL_ART_DIRECTION_CORRECTION_2026-09-07.md
  - docs/agents/tasks/active/OTERYN-20260906-premium-portal-redesign.md
  - docs/agents/prompts/OTERYN-PORTAL-ART-DIRECTED-REDESIGN-ASTRA.md
validation:
  - command: Local CSS parser, JavaScript/PHP syntax, translation/asset dependency and checkpoint checks
    result: PASS
    evidence: Both CSS files parse; both repaired browser specs pass node --check; both translation files and the new PHP test pass php -l; shared-layout and locale dependencies checked. Local checks are not Laravel/browser acceptance.
  - command: Existing exact-SHA GitHub Actions PHP/static and real Laravel browser acceptance
    result: NOT_RUN
    evidence: Run on the published coherent candidate; no old-head pass is reused as current acceptance.
blockers:
  - No verified candidate results yet; retain draft until required evidence and visual review are complete.
next_action: Publish the coherent art-directed candidate, execute existing isolated acceptance and inspect matching screenshots before protected-main integration.
```

## Evidence and execution

`docs/testing/PORTAL_ART_DIRECTION_CORRECTION_2026-09-07.md` records implementation and scenery provenance. `docs/testing/PORTAL_REDESIGN_REVIEW_2026-09-06.md` preserves the full route ledger. Exact-head screenshots are evidence only when their manifest and the PR tested candidate agree. Concept renders and local syntax checks never substitute for actual application acceptance.

Use the existing GitHub-hosted Laravel runtime and isolated synthetic data. No workstation, Synology or staging/production access is authorized. No unsupported subagent or background continuation is claimed.

## Source branch closeout

```yaml
source_branch_disposition: pending
source_branch_reason: Owner authorizes protected-main integration; verify normal source-branch cleanup after actual merge
source_branch_evidence: Current conversation and live PR 1298
```
