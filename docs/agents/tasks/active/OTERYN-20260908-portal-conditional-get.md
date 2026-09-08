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

- [x] successful robots and sitemap responses expose deterministic content ETags;
- [x] matching `If-None-Match` returns 304 for unchanged content by implementation contract;
- [x] existing content types and `public, no-cache, must-revalidate` semantics remain intact by implementation contract;
- [x] sitemap dependency failure remains 503 + `no-store` and does not expose a reusable ETag by implementation contract;
- [ ] focused feature tests pass;
- [ ] exact-head CI and `platform-gate` pass;
- [ ] integration uses normal Merge Queue;
- [ ] resulting protected-main staging deployment is healthy and provides a genuine `platform-reconcile` timing proof for Issue 1345 if the live classifier selects that profile.

## Ownership

```yaml
owned_paths:
  - app/Http/Controllers/PublicPortal/PublicRobotsController.php
  - app/Http/Controllers/PublicPortal/PublicSitemapController.php
  - tests/Feature/PublicPortal/PublicSeoConditionalRequestTest.php
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
updated_at: 2026-09-08T13:16:00Z
head: c6cf758704a2c52d5fa07f7adb5f58b320a26ddf
branch: perf/1351-portal-conditional-get
pr: 1354
status: validating
context_routes:
  - testing
owned_paths:
  - app/Http/Controllers/PublicPortal/PublicRobotsController.php
  - app/Http/Controllers/PublicPortal/PublicSitemapController.php
  - tests/Feature/PublicPortal/PublicSeoConditionalRequestTest.php
  - docs/agents/tasks/active/OTERYN-20260908-portal-conditional-get.md
proven:
  - protected main at task start is 2c3c88c30b6da35124ade7ed82dbd20cf8c0f557
  - prior successful robots and sitemap responses used public no-cache must-revalidate but did not set ETag validators
  - repository already uses setEtag plus isNotModified for public editorial media
  - robots and sitemap successful responses now derive ETag from exact response bytes and call isNotModified on the incoming Request
  - sitemap 503 failure path returns before any ETag assignment and keeps no-store
  - focused tests cover initial ETag, matching If-None-Match 304, empty 304 body, cache directives, and 503 without ETag
  - task changes no migration, schema, Gateway, Canary, auth, payment or deployment-control path
  - PR 1354 is the canonical validation/integration PR for this task
unknown:
  - exact-head validation result
  - resulting protected-main deployment profile and wall-time
conflicts: []
first_failure:
  marker: missing-conditional-validator
  evidence: pre-task PublicRobotsController and PublicSitemapController returned revalidating successful responses without an ETag
rejected_hypotheses:
  - weakening cache revalidation would trade correctness for performance
  - using timestamps rather than content-derived validators would make unchanged bytes unnecessarily miss 304
changed_paths:
  - app/Http/Controllers/PublicPortal/PublicRobotsController.php
  - app/Http/Controllers/PublicPortal/PublicSitemapController.php
  - tests/Feature/PublicPortal/PublicSeoConditionalRequestTest.php
  - docs/agents/tasks/active/OTERYN-20260908-portal-conditional-get.md
validation:
  - command: PR 1354 exact-head workflows
    result: PENDING
    evidence: waiting for GitHub Actions on exact head
blockers:
  - resulting-main Synology deployment for PR 1353 is still in progress
next_action: complete exact-head validation; after PR 1353 staging is healthy, transition to ready/archive-pending and enter normal Merge Queue
```

## Source branch closeout

```yaml
source_branch_disposition: auto_delete_after_merge
source_branch_reason: delete source branch only after verified protected integration and deployed staging proof
source_branch_evidence: Issue #1351; PR #1354; branch perf/1351-portal-conditional-get
```

## Notes

No production deployment, direct push to main, force-push, protected-environment bypass, secret mutation or gate weakening is authorized.
