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

- [ ] First-party Portal CSS/JS and critical home artwork use deterministic versioned URLs that change with asset content.
- [ ] Home/public presentation has a materially visible composition delta on desktop and mobile while preserving truthful content and existing routes.
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
  - public/css/home-production.css
  - tests/Feature/PortalVisualPolishTest.php
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
updated_at: 2026-09-08T10:17:00Z
head: 01a41276d3f345da94b35ad00f093a9ca66482a3
branch: fix/1344-owner-visible-portal-polish
pr: none
status: implementing
context_routes:
  - web-cms
  - portal-completion
owned_paths:
  - resources/views/game/layout.blade.php
  - resources/views/home.blade.php
  - public/css/home-production.css
  - tests/Feature/PortalVisualPolishTest.php
  - scripts/acceptance/tests/portal-polish-quality.spec.mjs
proven:
  - PR #1334 merged through Merge Queue and its source branch was deleted.
  - Synology staging deploy 34212996824 completed successfully for protected main 01a41276d3f345da94b35ad00f093a9ca66482a3 and ended healthy.
  - Owner-visible acceptance failed after that deploy because the page still appeared effectively unchanged.
  - Public layouts currently use stable asset URLs without a content/release cache key.
derived:
  - A content-versioned asset URL is the smallest repository-local cache-safe fix without taking ownership of Synology/edge configuration.
  - Existing visual-polish deltas should be made more obvious in the home composition rather than relying on micro-spacing changes.
unknown:
  - Whether stale first-party asset caching is the only reason the owner perceived no change.
conflicts: []
first_failure:
  marker: owner-visible-staging-acceptance
  evidence: healthy deploy 34212996824 followed by owner report that the site looked unchanged
rejected_hypotheses:
  - The original lack of visible change was solely because PR #1334 had not deployed; deploy 34212996824 later completed successfully and the owner-visible gap remained the controlling acceptance failure.
changed_paths:
  - docs/agents/tasks/active/OTERYN-20260908-owner-visible-portal-polish.md
validation:
  - command: staging deploy 34212996824
    result: PASS
    evidence: existing deployed baseline is healthy; new task changes are not yet implemented
blockers:
  - none
next_action: implement content-versioned Portal assets and a visibly stronger home composition, then run focused tests
```

## Source branch closeout

```yaml
source_branch_disposition: pending
source_branch_reason: task is active
source_branch_evidence: pending
```

## Notes

This task does not modify staging automation, production, DNS/Cloudflare, payment/auth data or external repositories. Deployed staging evidence must come from the normal protected-main pipeline after Merge Queue integration.
