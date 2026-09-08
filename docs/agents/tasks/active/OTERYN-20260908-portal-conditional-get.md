---
task_id: OTERYN-20260908-portal-conditional-get
governing_issue: 1351
required_reads:
  - docs/agents/BUILD_TEST_MATRIX.md
search_first:
  - Issue 1351 live state
  - current public SEO controller and feature-test contracts
optional_reads: []
---

# OTERYN-20260908-portal-conditional-get

## Goal

Add deterministic conditional GET support to the public `/robots.txt` and `/sitemap.xml` responses so unchanged successful representations can return HTTP 304 without changing content, routing, authorization, schema, Gateway or deployment contracts.

## Acceptance criteria

- [ ] successful robots and sitemap responses expose deterministic content ETags;
- [ ] matching `If-None-Match` returns 304 for unchanged content;
- [ ] existing content types and `public, no-cache, must-revalidate` semantics remain intact;
- [ ] sitemap dependency failure remains 503 + `no-store` and does not expose a reusable ETag;
- [ ] focused feature tests pass;
- [ ] exact-head CI and `platform-gate` pass;
- [ ] integration uses normal Merge Queue;
- [ ] resulting protected-main staging deployment is healthy and provides a genuine `platform-reconcile` timing proof for Issue 1345 if the live classifier selects that profile.

## Ownership

```yaml
owned_paths:
  - app/Http/Controllers/PublicPortal/PublicRobotsController.php
  - app/Http/Controllers/PublicPortal/PublicSitemapController.php
  - tests/Feature/PublicPortal/HomepageNavigationSeoTest.php
  - docs/agents/tasks/active/OTERYN-20260908-portal-conditional-get.md
modules:
  - PublicPortal SEO responses
dependencies:
  - Issue 1351
  - Issue 1345 platform-reconcile implementation already integrated
blockers:
  - do not integrate until resulting-main Synology deployment for PR 1353 is terminal and healthy
cross_repository_tasks: []
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-09-08T13:12:00Z
head: 2c3c88c30b6da35124ade7ed82dbd20cf8c0f557
branch: perf/1351-portal-conditional-get
pr: none
status: implementing
context_routes:
  - testing
owned_paths:
  - app/Http/Controllers/PublicPortal/PublicRobotsController.php
  - app/Http/Controllers/PublicPortal/PublicSitemapController.php
  - tests/Feature/PublicPortal/HomepageNavigationSeoTest.php
  - docs/agents/tasks/active/OTERYN-20260908-portal-conditional-get.md
proven:
  - protected main at task start is 2c3c88c30b6da35124ade7ed82dbd20cf8c0f557
  - current successful robots and sitemap responses use public no-cache must-revalidate but do not set ETag validators
  - repository already uses setEtag plus isNotModified for public editorial media
  - task changes no migration, schema, Gateway, Canary, auth, payment or deployment-control path
unknown:
  - exact-head validation result
  - resulting protected-main deployment profile and wall-time
conflicts: []
first_failure:
  marker: missing-conditional-validator
  evidence: current PublicRobotsController and PublicSitemapController return revalidating successful responses without an ETag
rejected_hypotheses:
  - weakening cache revalidation would trade correctness for performance
  - using timestamps rather than content-derived validators would make unchanged bytes unnecessarily miss 304
changed_paths:
  - docs/agents/tasks/active/OTERYN-20260908-portal-conditional-get.md
validation: []
blockers:
  - resulting-main Synology deployment for PR 1353 is still in progress
next_action: implement content-derived ETags and focused conditional-request tests; keep integration held until PR 1353 staging is healthy
```

## Source branch closeout

```yaml
source_branch_disposition: pending
source_branch_reason: implementation and exact-head validation are active
source_branch_evidence: Issue #1351; branch perf/1351-portal-conditional-get
```

## Notes

No production deployment, direct push to main, force-push, protected-environment bypass, secret mutation or gate weakening is authorized.
