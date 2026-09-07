---
task_id: OTERYN-20260906-premium-portal-redesign
governing_issue: 1297
required_reads:
  - docs/architecture/MODULE_CATALOG.md
  - docs/architecture/SECURITY_ARCHITECTURE.md
  - docs/architecture/TEST_STRATEGY.md
  - docs/agents/BUILD_TEST_MATRIX.md
search_first:
  - resources/views
  - public/css
  - scripts/acceptance
optional_reads: []
---

# Complete Oteryn Platform cinematic redesign

## Goal and authority

Issue #1297; existing PR #1298. Finish the owner's complete website redesign against the three supplied cinematic references, prove it in the real Laravel application and integrate through protected main. The latest explicit owner instruction supersedes the earlier no-merge/human-preview-only hold. No production deployment, protected environment, live identity/payment, gate bypass or cross-repository action is authorized.

## Acceptance criteria

- [x] Retain the existing functional public/player work and route ledger, rather than start a competing rewrite.
- [ ] Publish coherent cinematic presentation across public, identity, error and administrator page families.
- [ ] Pass current-head application/static and required CI checks.
- [ ] Prove real guest/account/admin navigation and applicable actor-to-result paths, EN/PL and responsive/edge states; inspect actual screenshots and repair findings.
- [ ] Review the complete exact candidate diff and integrate through repository protections under the owner's instruction.
- [ ] Record terminal evidence, archive the packet and verify the source-branch disposition.

## Ownership

This single writer owns presentation under resources/views, shared CSS/JS, decorative Oteryn artwork, presentation-only translations, focused PHP/browser tests and this task's evidence. The owner's whole-Platform request now includes administrator presentation; it does not change RBAC, MFA or privileged operation semantics. Controllers, routes, domain/authentication/payment behavior, schemas, workflows/protection and production are excluded. Independent audit #1294 and inactive catalog consumer #338 remain untouched.

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-09-07T14:58:20Z
head: 341e5021499f3b437a4985409e4792436529a504
branch: feat/20260906-premium-portal-redesign
pr: 1298
status: implementing
context_routes:
  - web-cms
  - auth-identity
  - admin-rbac
  - testing
owned_paths:
  - resources/views/**
  - public/css/**
  - public/js/portal-navigation.js
  - public/images/oteryn-*.webp
  - lang/en/portal_art.php
  - lang/pl/portal_art.php
  - scripts/acceptance/tests/portal-visual-review.spec.mjs
  - scripts/acceptance/tests/support-legal-acceptance.spec.mjs
  - tests/Feature/HomeTest.php
  - tests/Feature/HomePreviewTest.php
  - docs/testing/PORTAL_RENDER_*
  - docs/agents/tasks/active/OTERYN-20260906-premium-portal-redesign.md
proven:
  - Protected main is 721d560c18ef8a6eefbf4ccd685d8302985bbd3d and the open owned PR resumes at 341e5021499f3b437a4985409e4792436529a504.
  - The owner explicitly requested whole-platform completion and main integration; PR comment 5572165616 records the updated task boundary.
  - Existing visual evidence at 7b4d03ea is a functional baseline, not acceptance of the newly requested cinematic design.
  - Source inspection shows the original visual fixture created only a read-model binding, while RegistrationController invokes the genuine isolated provisioning path.
  - The reconstructed current visual test matches Git blob d285807369ad40c73ca5796c451117615aecd2b7 before the focused fixture repair.
  - Local JavaScript syntax and presentation translation PHP syntax checks passed.
derived: []
unknown:
  - The prepared candidate has not yet passed current-head Laravel or browser acceptance.
  - Final whole-platform visual acceptance and protected-main integration remain unverified.
conflicts: []
first_failure:
  marker: CANDIDATE_PUBLICATION_AND_ACCEPTANCE_PENDING
  evidence: Prior Acceptance run 34124102942 failed character creation and fixture setup; prior Support Legal run 34124102947 reported undeclared expected negative responses.
rejected_hypotheses:
  - Maintenance/deployment restrictions forbid independent presentation implementation.
changed_paths:
  - public/css/portal-art-direction.css
  - public/css/home-production.css
  - public/css/portal-admin.css
  - resources/views/home.blade.php
  - resources/views/admin/layout.blade.php
  - resources/views/admin/partials/navigation.blade.php
  - scripts/acceptance/tests/portal-visual-review.spec.mjs
  - scripts/acceptance/tests/support-legal-acceptance.spec.mjs
  - docs/testing/PORTAL_RENDER_ALIGNMENT_2026-09-07.md
validation:
  - command: node --check on the two prepared acceptance tests
    result: PASS
    evidence: Local syntax; no browser outcome implied.
  - command: php -l on the EN and PL presentation translation files
    result: PASS
    evidence: Local PHP 8.4 syntax; required PHP 8.5 application validation remains hosted.
  - command: exact-candidate Laravel and browser acceptance
    result: NOT_RUN
    evidence: Pending publication of the coherent candidate.
blockers:
  - Local DNS and Composer dependencies are unavailable; use the existing repository-hosted runtime.
  - Historical tool rejections are not treated as resolved until the focused authorized repair is accepted by the current write path.
next_action: Publish the coherent presentation and focused test candidate, then inspect exact-head hosted test results and actual family renders before protected integration.
```

## Evidence and recovery

`docs/testing/PORTAL_RENDER_ALIGNMENT_2026-09-07.md` records this implementation pass and provenance. The preceding route ledger and PR attestation preserve historical evidence. No background worker, actual deployment or final acceptance is claimed. Preserve all failing assertions and diagnostic guards.

## Source branch closeout

```yaml
source_branch_disposition: auto_delete_after_merge
source_branch_reason: owner now requests protected-main integration; verify deletion only after successful accepted merge and terminal closeout
source_branch_evidence: latest owner instruction; PR 1298 comment 5572165616
```
