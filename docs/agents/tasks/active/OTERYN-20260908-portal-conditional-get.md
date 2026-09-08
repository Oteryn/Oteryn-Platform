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
- [x] matching `If-None-Match` returns 304 for unchanged content;
- [x] existing content types and `public, no-cache, must-revalidate` semantics remain intact;
- [x] sitemap dependency failure remains 503 + `no-store` and does not expose a reusable ETag;
- [x] focused feature tests pass;
- [x] exact-head CI and `platform-gate` pass on the validated implementation head;
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
blockers: []
cross_repository_tasks: []
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-09-08T13:34:00Z
head: b2179e9b3bd517b816fd026e76dfc83b65510d7e
branch: perf/1351-portal-conditional-get
pr: 1354
status: ready
terminal_pr_policy: archive_pending
context_routes:
  - testing
owned_paths:
  - app/Http/Controllers/PublicPortal/PublicRobotsController.php
  - app/Http/Controllers/PublicPortal/PublicSitemapController.php
  - tests/Feature/PublicPortal/PublicSeoConditionalRequestTest.php
  - docs/agents/tasks/active/OTERYN-20260908-portal-conditional-get.md
proven:
  - protected main is 2acb8b548b413b9ce522ff5c507b4f10c9d0ff0e after docs-only PR 1355 removed the terminal PR 1353 active-task residue
  - prior successful robots and sitemap responses used public no-cache must-revalidate but did not set ETag validators
  - repository already uses setEtag plus isNotModified for public editorial media
  - robots and sitemap successful responses now derive ETag from exact response bytes and call isNotModified on the incoming Request
  - sitemap 503 failure path returns before any ETag assignment and keeps no-store
  - focused tests cover initial ETag, matching If-None-Match 304, empty 304 body, cache directives, and 503 without ETag
  - task changes no migration, schema, Gateway, Canary, auth, payment or deployment-control path
  - PR 1354 is the canonical validation/integration PR for this task and has zero discussion comments and zero inline review threads
  - implementation head b2179e9b3bd517b816fd026e76dfc83b65510d7e has CI 34232282372 success including runtime-tests and platform-gate job 102081951151 success
  - implementation head b2179e9b3bd517b816fd026e76dfc83b65510d7e has Acceptance E2E 34232282318 success, Portal Acceptance 34232282257 success, Phase 7 34232282174 success, Build Synology 34232282212 success, DB Outage 34232282200 success, Edge Security 34232282373 success and Game Auth 34232282258 success
  - the Agent Governance failure on that implementation head was caused only by the then-stale merged PR 1353 active packet; PR 1355 subsequently archived that packet through protected Merge Queue and main no longer contains it
derived:
  - content-derived validators preserve representation correctness while allowing unchanged successful responses to complete as conditional HTTP 304
  - sitemap failure responses remain non-reusable because ETag assignment occurs only after successful dependency resolution and rendering
  - with the PR 1353 lifecycle residue removed from current main, the final checkpoint head can be evaluated against clean live ownership without changing application behavior
unknown:
  - final checkpoint exact-head validation result
  - resulting protected-main deployment profile and wall-time
conflicts: []
first_failure:
  marker: missing-conditional-validator
  evidence: pre-task PublicRobotsController and PublicSitemapController returned revalidating successful responses without an ETag
rejected_hypotheses:
  - weakening cache revalidation would trade correctness for performance
  - using timestamps rather than content-derived validators would make unchanged bytes unnecessarily miss 304
  - treating the old Governance failure as a product-code defect would misdiagnose terminal PR 1353 lifecycle residue that is now removed from protected main
changed_paths:
  - app/Http/Controllers/PublicPortal/PublicRobotsController.php
  - app/Http/Controllers/PublicPortal/PublicSitemapController.php
  - tests/Feature/PublicPortal/PublicSeoConditionalRequestTest.php
  - docs/agents/tasks/active/OTERYN-20260908-portal-conditional-get.md
validation:
  - command: PR 1354 implementation-head CI 34232282372
    result: PASS
    evidence: formatting, PHPStan, full runtime tests, required test gate and platform-gate job 102081951151 all succeeded on b2179e9b3bd517b816fd026e76dfc83b65510d7e
  - command: PR 1354 implementation-head Acceptance E2E 34232282318
    result: PASS
    evidence: Chromium smoke, browser portability, responsive, public dependency resilience and keyboard accessibility passed; bounded non-applicable heavier profiles were skipped by workflow policy
  - command: PR 1354 implementation-head supporting contracts
    result: PASS
    evidence: Portal Acceptance 34232282257, Phase 7 34232282174, Build Synology 34232282212, DB Outage 34232282200, Edge Security 34232282373 and Game Auth 34232282258 succeeded
blockers: []
next_action: archive this task after protected integration and healthy resulting-main staging proof
```

## Source branch closeout

```yaml
source_branch_disposition: auto_delete_after_merge
source_branch_reason: delete source branch only after verified protected integration and deployed staging proof
source_branch_evidence: Issue #1351; PR #1354; branch perf/1351-portal-conditional-get
```

## Notes

No production deployment, direct push to main, force-push, protected-environment bypass, secret mutation or gate weakening is authorized.
