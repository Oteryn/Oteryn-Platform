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
updated_at: 2026-09-08T10:49:22Z
head: 13ffc5e4e70d539caf7f952b825bb06b67601536
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
proven:
  - PR #1334 merged through Merge Queue and its source branch was deleted.
  - Synology staging deploy 34212996824 completed successfully for protected main 01a41276d3f345da94b35ad00f093a9ca66482a3 and ended healthy.
  - Owner-visible acceptance failed after that deploy because the page still appeared effectively unchanged.
  - Public layouts previously used stable asset URLs without a content/release cache key.
  - The implementation computes deterministic 12-hex SHA-256 content versions for shared Portal CSS/JS, home-production CSS, the citadel hero and home discovery artwork.
  - A final override stylesheet changes the home hero to an explicit two-column realm stage on desktop and one-column mobile composition, with a framed native-size citadel and 3/2/1-column discovery cards.
  - Canonical draft PR #1348 owns this branch and exact task scope.
  - Agent Governance is green after binding the active task packet to PR #1348.
  - CI run 34215456344 reached the full PHPUnit suite with Pint and PHPStan green; its single failure was the stale SecurityHeadersTest assertion requiring queryless stylesheet URLs after cache-busting was intentionally introduced.
derived:
  - Content-versioned asset URLs remove stale-cache ambiguity without taking ownership of Synology/edge configuration.
  - The new composition is intentionally more visually distinct than the micro-spacing changes in PR #1334 while preserving the same semantic DOM and data contracts.
  - The SecurityHeadersTest contract must require same-origin versioned stylesheet URLs rather than reverting cache-busting.
unknown:
  - Fresh exact-head hosted validation result after the SecurityHeadersTest contract repair.
  - Protected-main SHA and staging release after integration.
conflicts: []
first_failure:
  marker: security-headers-versioned-asset-contract
  evidence: runtime-tests job 102026171086 in CI run 34215456344 failed only because tests/Feature/Operations/SecurityHeadersTest.php expected href=asset(path) without the intentional ?v=<12-hex> content version
rejected_hypotheses:
  - The original lack of visible change was solely because PR #1334 had not deployed; deploy 34212996824 later completed successfully and the owner-visible gap remained the controlling acceptance failure.
  - Cache-busting should be removed to satisfy the old SecurityHeadersTest; that would reintroduce the deployed stale-asset ambiguity this task exists to remove.
changed_paths:
  - docs/agents/tasks/active/OTERYN-20260908-owner-visible-portal-polish.md
  - resources/views/game/layout.blade.php
  - resources/views/home.blade.php
  - public/css/portal-owner-visible.css
  - tests/Feature/PortalVisualPolishTest.php
  - tests/Feature/Operations/SecurityHeadersTest.php
  - scripts/acceptance/tests/portal-polish-quality.spec.mjs
validation:
  - command: staging deploy 34212996824
    result: PASS
    evidence: deployed baseline is healthy and proves the owner-visible problem is post-deploy rather than a missing deployment
  - command: Agent Governance 34215368352
    result: FAIL
    evidence: first candidate omitted live PR #1348 from the task packet; all checkpoint/Issue/source-branch validators passed before liveness rejected pr none
  - command: subsequent Agent Governance on head 13ffc5e4e70d539caf7f952b825bb06b67601536
    result: PASS
    evidence: live active-task ownership and Control Room validation passed after PR identity repair
  - command: CI 34215456344 / runtime-tests 102026171086
    result: FAIL
    evidence: Pint and PHPStan passed; PHPUnit had 1 failure and 77 passes, with SecurityHeadersTest line 23 still expecting queryless portal-system.css while the application intentionally emitted a same-origin content version
blockers:
  - none
next_action: validate the fresh exact head after the SecurityHeadersTest cache-version contract repair, then repair any remaining task-owned failure before protected integration
```

## Source branch closeout

```yaml
source_branch_disposition: pending
source_branch_reason: task is active and awaiting exact-head validation plus protected integration/deployed proof
source_branch_evidence: Issue #1344; PR #1348; branch fix/1344-owner-visible-portal-polish
```

## Notes

This task does not modify staging automation, production, DNS/Cloudflare, payment/auth data or external repositories. Deployed staging evidence must come from the normal protected-main pipeline after Merge Queue integration.
