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

- [x] First-party Portal CSS/JS and critical home artwork use deterministic versioned URLs that change with asset content.
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
updated_at: 2026-09-08T11:07:00Z
head: 186f5dfaf13258f0adb512040cfba43fa517d03e
branch: fix/1344-owner-visible-portal-polish
pr: 1348
status: validating
context_routes:
  - web-cms
  - portal-completion
owned_paths:
  - resources/views/game/layout.blade.php
  - resources/views/home.blade.php
  - public/css/portal-owner-visible.css
  - tests/Feature/PortalVisualPolishTest.php
  - tests/Feature/Operations/SecurityHeadersTest.php
  - scripts/acceptance/tests/portal-polish-quality.spec.mjs
  - scripts/acceptance/tests/portal-visual-review.spec.mjs
proven:
  - PR #1334 merged through Merge Queue and its source branch was deleted.
  - Synology staging deploy 34212996824 completed successfully for protected main 01a41276d3f345da94b35ad00f093a9ca66482a3 and ended healthy.
  - Owner-visible acceptance failed after that deploy because the page still appeared effectively unchanged.
  - Public layouts previously used stable asset URLs without a content/release cache key.
  - The implementation computes deterministic 12-hex SHA-256 content versions for shared Portal CSS/JS, home-production CSS, the citadel hero and home discovery artwork.
  - A final override stylesheet changes the home hero to an explicit two-column realm stage on desktop and one-column mobile composition, with a framed native-size citadel and 3/2/1-column discovery cards.
  - Canonical draft PR #1348 owns this branch and exact task scope.
  - CI run 34217551280 is fully green on 3824b35eb5dec7c672b4a2109ccf81debdd76cce, including runtime-tests and platform-gate; Portal Acceptance Contract run 34217551335 is fully green including strict coverage and complete account lifecycle; Governance, Edge Security, CodeQL, DB Outage, Game Auth, Phase 7, Content Scale, Community Data, Wiki Reconciliation, Events and staging-image build are also green.
  - Acceptance run 34217551274 reached real exact-SHA Laravel HTTP and failed Chromium smoke only because portal-visual-review.spec.mjs used href$=/css/portal-art-direction.css, which cannot match the intentional ?v=<12-hex> versioned URL. Direct artifact 10052522971 (sha256:99fcd4afd46f381564563e95ad514b4ed6fbb4da970d1331708cec2a3d14f902) carries the exact repeated failure at portal-visual-review.spec.mjs:53.
  - The visual review contract now requires exactly one portal-art-direction stylesheet whose URL is same-origin, whose pathname is /css/portal-art-direction.css and whose v query is exactly 12 lowercase hexadecimal characters.
derived:
  - Content-versioned asset URLs remove stale-cache ambiguity without taking ownership of Synology/edge configuration.
  - The new composition is intentionally more visually distinct than the micro-spacing changes in PR #1334 while preserving the same semantic DOM and data contracts.
  - Security and visual acceptance contracts must require same-origin versioned stylesheet URLs rather than reverting cache-busting or introducing dummy queryless links.
unknown:
  - Fresh exact-head hosted validation result after the portal-visual-review versioned-art-direction contract repair.
  - Protected-main SHA and staging release after integration.
conflicts: []
first_failure:
  marker: portal-visual-review-versioned-art-direction-contract
  evidence: Acceptance E2E run 34217551274 / job 102032785013 / artifact 10052522971 failed because link[href$=/css/portal-art-direction.css] returned 0 after the stylesheet correctly gained ?v=<12-hex>
rejected_hypotheses:
  - The original lack of visible change was solely because PR #1334 had not deployed; deploy 34212996824 later completed successfully and the owner-visible gap remained the controlling acceptance failure.
  - Cache-busting should be removed to satisfy old PHP or Playwright selectors; that would reintroduce the deployed stale-asset ambiguity this task exists to remove.
  - A dummy queryless link should be added only to satisfy the old selector; that would test-game the acceptance contract instead of verifying the real stylesheet.
changed_paths:
  - docs/agents/tasks/active/OTERYN-20260908-owner-visible-portal-polish.md
  - resources/views/game/layout.blade.php
  - resources/views/home.blade.php
  - public/css/portal-owner-visible.css
  - tests/Feature/PortalVisualPolishTest.php
  - tests/Feature/Operations/SecurityHeadersTest.php
  - scripts/acceptance/tests/portal-polish-quality.spec.mjs
  - scripts/acceptance/tests/portal-visual-review.spec.mjs
validation:
  - command: staging deploy 34212996824
    result: PASS
    evidence: deployed baseline is healthy and proves the owner-visible problem is post-deploy rather than a missing deployment
  - command: CI 34217551280 / runtime-tests + platform-gate
    result: PASS
    evidence: full PHP runtime tests, Pint, PHPStan and aggregate platform-gate are green on 3824b35eb5dec7c672b4a2109ccf81debdd76cce
  - command: Portal Acceptance Contract 34217551335
    result: PASS
    evidence: strict portal coverage closure and complete account lifecycle are green on 3824b35eb5dec7c672b4a2109ccf81debdd76cce
  - command: Acceptance E2E 34217551274 / job 102032785013
    result: FAIL
    evidence: real Laravel HTTP smoke failed only on the stale href$ selector for content-versioned portal-art-direction.css; artifact 10052522971 carries exact failure evidence
blockers:
  - none
next_action: validate the fresh exact head after the portal-visual-review cache-version contract repair, then repair any remaining task-owned failure before protected integration
```

## Source branch closeout

```yaml
source_branch_disposition: pending
source_branch_reason: task is active and awaiting exact-head validation plus protected integration/deployed proof
source_branch_evidence: Issue #1344; PR #1348; branch fix/1344-owner-visible-portal-polish
```

## Notes

This task does not modify staging automation, production, DNS/Cloudflare, payment/auth data or external repositories. Deployed staging evidence must come from the normal protected-main pipeline after Merge Queue integration.
