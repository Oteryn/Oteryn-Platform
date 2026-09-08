---
task_id: OTERYN-20260908-owner-visible-portal-polish
governing_issue: 1344
required_reads:
  - docs/agents/programs/OTERYN_PORTAL_COMPLETION.md
  - docs/agents/programs/OTERYN_PORTAL_COMPLETION_WORK_ALLOCATION.md
  - docs/agents/prompts/OTERYN-PORTAL-COMPLETION-EXECUTION-PROMPT.md
search_first:
  - current live Issue #1344, protected main, active Portal ownership and staging deploy evidence
optional_reads: []
---

# OTERYN-20260908-owner-visible-portal-polish

## Goal

Governing GitHub Issue: #1344 — make the deployed Portal visual change unmistakably owner-visible and make first-party presentation assets cache-safe after protected-main releases.

## Acceptance criteria

- [x] First-party shared Portal CSS/JS and critical home artwork use deterministic versioned URLs that change with asset content.
- [x] Home/public presentation has a materially visible composition delta on desktop and mobile while preserving truthful content and existing routes.
- [ ] Existing EN/PL, responsive, keyboard/focus, reduced-motion, security and Portal contracts remain green.
- [ ] Exact-head browser acceptance, CI/platform-gate and Portal Acceptance Contract pass.
- [ ] Protected integration uses Merge Queue only.
- [ ] Automatic Synology staging deploy completes successfully and deployed public staging health is proven on the resulting release.
- [ ] Owner-visible acceptance is not declared complete before deployed evidence exists.

## Ownership

```yaml
owned_paths:
  - docs/agents/tasks/active/OTERYN-20260908-owner-visible-portal-polish.md
  - docs/agents/tasks/archive/OTERYN-20260908-owner-visible-portal-polish.md
  - resources/views/game/layout.blade.php
  - resources/views/home.blade.php
  - resources/views/identity/layout.blade.php
  - resources/views/errors/layout.blade.php
  - resources/views/admin/layout.blade.php
  - public/css/portal-owner-visible.css
  - tests/Feature/PortalVisualPolishTest.php
  - tests/Feature/Operations/SecurityHeadersTest.php
  - scripts/acceptance/tests/portal-polish-quality.spec.mjs
  - scripts/acceptance/tests/portal-visual-review.spec.mjs
modules:
  - web-cms
dependencies:
  - PR #1334 merged and staging deploy 34212996824 proven healthy
blockers:
  - none
cross_repository_tasks:
  - none
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-09-08T11:18:00Z
head: 352dd9851c5c7bc0ef396181976f1bf4fd79bcaa
branch: fix/1344-owner-visible-portal-polish
pr: 1348
status: validating
context_routes:
  - web-cms
  - portal-completion
owned_paths:
  - resources/views/game/layout.blade.php
  - resources/views/home.blade.php
  - resources/views/identity/layout.blade.php
  - resources/views/errors/layout.blade.php
  - resources/views/admin/layout.blade.php
  - public/css/portal-owner-visible.css
  - tests/Feature/PortalVisualPolishTest.php
  - tests/Feature/Operations/SecurityHeadersTest.php
  - scripts/acceptance/tests/portal-polish-quality.spec.mjs
  - scripts/acceptance/tests/portal-visual-review.spec.mjs
proven:
  - PR #1334 merged through Merge Queue and its source branch was deleted.
  - Synology staging deploy 34212996824 completed successfully for protected main 01a41276d3f345da94b35ad00f093a9ca66482a3 and ended healthy.
  - Owner-visible acceptance failed after that deploy because the page still appeared effectively unchanged.
  - The implementation computes deterministic 12-hex SHA-256 content versions for shared Portal CSS/JS, home-production CSS, the citadel hero and home discovery artwork.
  - A final override stylesheet changes the home hero to an explicit two-column realm stage on desktop and one-column mobile composition, with a framed native-size citadel and 3/2/1-column discovery cards.
  - CI run 34217551280 is fully green on 3824b35eb5dec7c672b4a2109ccf81debdd76cce, including runtime-tests and platform-gate; Portal Acceptance Contract run 34217551335 is fully green including strict coverage and complete account lifecycle; Governance, Edge Security, CodeQL, DB Outage, Game Auth, Phase 7, Content Scale, Community Data, Wiki Reconciliation, Events and staging-image build are also green.
  - Acceptance run 34217551274 failed because the old visual-review selector required a queryless portal-art-direction URL.
  - Acceptance run 34219159831 on 7ffb861c44c3fce38e4a8080bdc61559785f1174 proved the public game layout correctly serves portal-art-direction.css?v=<12-hex>, but identity, error and admin layouts still used queryless shared Portal CSS. Artifact 10053148808 (sha256:04b8d9043f7be3f4e745f72e552034e593d28dc2a9d24b4248b92b55084ef3a6) isolated that gap: public captures completed through catalog pages, while login/account/error/MFA/admin captures failed the versioned shared-art-direction assertion.
  - Identity now versions portal-system.css, portal-pages.css and portal-art-direction.css; error now versions portal-system.css and portal-art-direction.css; admin now versions its shared portal-art-direction.css. No auth, permission or route semantics changed.
derived:
  - Content-versioned shared asset URLs remove stale-cache ambiguity across public, identity, error and admin Portal surfaces without taking ownership of Synology/edge configuration.
  - The visual review assertion is a real deployment contract and must not be satisfied by dummy queryless links.
unknown:
  - Fresh exact-head hosted validation after all shared Portal layouts use versioned art-direction assets.
  - Protected-main SHA and staging release after integration.
conflicts: []
first_failure:
  marker: shared-layout-cache-version-gap
  evidence: Acceptance E2E 34219159831 / artifact 10053148808 showed public pages versioned correctly, then login/account/error/MFA/admin surfaces failed because their layouts retained queryless shared CSS
rejected_hypotheses:
  - Cache-busting should be removed to satisfy old selectors; this would reintroduce the deployed stale-asset ambiguity.
  - A dummy queryless link should be added only to satisfy visual acceptance; this would test-game the contract.
  - The public game layout failed to version portal-art-direction.css; raw acceptance HTML proves it served portal-art-direction.css?v=45fa4d8d905d on the failed run.
changed_paths:
  - docs/agents/tasks/active/OTERYN-20260908-owner-visible-portal-polish.md
  - resources/views/game/layout.blade.php
  - resources/views/home.blade.php
  - resources/views/identity/layout.blade.php
  - resources/views/errors/layout.blade.php
  - resources/views/admin/layout.blade.php
  - public/css/portal-owner-visible.css
  - tests/Feature/PortalVisualPolishTest.php
  - tests/Feature/Operations/SecurityHeadersTest.php
  - scripts/acceptance/tests/portal-polish-quality.spec.mjs
  - scripts/acceptance/tests/portal-visual-review.spec.mjs
validation:
  - command: CI 34217551280 / runtime-tests + platform-gate
    result: PASS
    evidence: full PHP runtime tests, Pint, PHPStan and aggregate platform-gate green before the shared-layout completion
  - command: Portal Acceptance Contract 34217551335
    result: PASS
    evidence: strict portal coverage closure and complete account lifecycle green before the shared-layout completion
  - command: Acceptance E2E 34219159831 / artifact 10053148808
    result: FAIL
    evidence: exact artifact isolates remaining queryless shared CSS to identity/error/admin layouts; public versioned asset HTML is present and valid
blockers:
  - none
next_action: run fresh exact-head validation, reconcile latest protected main if needed, then proceed to protected integration only when all required gates are green
```

## Source branch closeout

```yaml
source_branch_disposition: pending
source_branch_reason: task is active and awaiting exact-head validation plus protected integration/deployed proof
source_branch_evidence: Issue #1344; PR #1348; branch fix/1344-owner-visible-portal-polish
```

## Notes

This task does not modify staging automation, production, DNS/Cloudflare, payment/auth data or external repositories. Deployed staging evidence must come from the normal protected-main pipeline after Merge Queue integration.
