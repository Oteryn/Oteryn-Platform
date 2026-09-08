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

Governing GitHub Issue: #1351 records the completed public SEO conditional-GET product slice.

Add deterministic content-derived HTTP validators to `/robots.txt` and `/sitemap.xml` so unchanged successful representations can return HTTP 304 without changing routing, authorization, response content, cache revalidation policy, schema, Gateway or deployment contracts.

## Acceptance criteria

- [x] successful robots and sitemap responses expose deterministic content ETags;
- [x] matching `If-None-Match` returns 304 for unchanged content with an empty response body;
- [x] existing content types and `public, no-cache, must-revalidate` semantics remain intact;
- [x] sitemap dependency failure remains 503 + `no-store` and does not expose a reusable ETag;
- [x] focused feature tests and exact-head runtime/static checks pass;
- [x] integration used protected Merge Queue with no direct-main bypass;
- [x] the accumulated product release was later deployed to staging by the normal protected-main pipeline and staging ended healthy with rollback skipped.

## Ownership

```yaml
owned_paths:
  - app/Http/Controllers/PublicPortal/PublicRobotsController.php
  - app/Http/Controllers/PublicPortal/PublicSitemapController.php
  - tests/Feature/PublicPortal/PublicSeoConditionalRequestTest.php
  - docs/agents/tasks/archive/OTERYN-20260908-portal-conditional-get.md
modules:
  - PublicPortal SEO responses
dependencies:
  - Issue 1345 remains separate issue-level ownership for a successful genuine live platform-reconcile timing proof
blockers:
  - none
cross_repository_tasks:
  - none
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-09-08T21:28:16Z
head: 7c8792d64e6baff599ab6a35595a521f212ee1e6
branch: perf/1351-portal-conditional-get
pr: 1354
status: completed
terminal_pr_policy: archive_pending
context_routes:
  - testing
owned_paths:
  - app/Http/Controllers/PublicPortal/PublicRobotsController.php
  - app/Http/Controllers/PublicPortal/PublicSitemapController.php
  - tests/Feature/PublicPortal/PublicSeoConditionalRequestTest.php
  - docs/agents/tasks/archive/OTERYN-20260908-portal-conditional-get.md
proven:
  - Before this task, successful `/robots.txt` and `/sitemap.xml` responses used `public, no-cache, must-revalidate` without an ETag validator.
  - PR 1354 changed only the two PublicPortal SEO controllers, focused feature coverage and its task record; it introduced no migration/schema, Gateway/Canary, auth/payment, secret or deployment-control change.
  - Successful robots and sitemap responses derive ETag from the exact response bytes with SHA-256 and call Symfony/Laravel `isNotModified()` on the incoming Request.
  - The sitemap dependency-failure path returns 503 + `no-store` before any ETag assignment.
  - Focused feature coverage proves initial ETag generation, matching `If-None-Match` 304 behavior, empty 304 body, cache directives and the 503/no-ETag failure boundary.
  - PR 1354 final exact head was 051e572df5f5d4c1256915d4066712598a383c8a.
  - Exact-head CI run 34233292528 completed success; runtime-tests job 102084613922 and required platform-gate job 102085390001 succeeded.
  - Exact-head Acceptance E2E run 34233292569 initially exposed a single WebKit internal-resource failure unrelated to the docs-only final checkpoint delta; one bounded rerun on the same exact SHA completed success with portability, responsive, dependency-resilience and keyboard-accessibility profiles green.
  - Exact-head supporting workflows also completed success: Portal Acceptance 34233292658, Phase 7 34233292647, Build Synology 34233292593, Platform DB Outage 34233292696, Edge Security 34233292535, Game Auth Ticket Concurrency 34233292642 and Agent Governance 34233292666.
  - Merge Queue candidate 7c8792d64e6baff599ab6a35595a521f212ee1e6 passed CI run 34240817420 with platform-gate job 102111237020.
  - PR 1354 merged only through Merge Queue and protected main became exactly 7c8792d64e6baff599ab6a35595a521f212ee1e6.
  - The source branch perf/1351-portal-conditional-get was automatically deleted after merge; branch search returned no retained source ref.
  - Resulting-main Build Synology run 34241173380 succeeded and produced the immutable Platform image for source 7c8792d64e6baff599ab6a35595a521f212ee1e6.
  - The first resulting-main Deploy Synology run 34241277797 genuinely selected `platform-reconcile`, but failed before health completion because the containerized Actions runner probed Synology host loopback from the wrong network namespace; fail-closed rollback restored the previously proven Platform runtime.
  - The failed deployment was diagnosed and repaired separately under Issue 1358 / PR 1359; that infrastructure defect did not require changing the conditional-GET product implementation.
  - PR 1359 merged through Merge Queue as protected main 3594f5ec9b3262326ba4875a68718ada3f19e666 and repaired fast/reconcile Platform probes to execute inside the exact Platform container namespace.
  - Resulting-main Build Synology run 34247355921 safely reused the already-built Platform provenance from the accumulated product release for the control-plane repair.
  - Resulting-main Deploy Synology run 34247444487 completed success. Its accumulated release classification explicitly included `app/Http/Controllers/PublicPortal/PublicRobotsController.php`, selected conservative `full` because deployment-control repair paths were also in the range, emitted `Oteryn Synology staging deployment is healthy`, and skipped rollback.
  - Current protected main 3594f5ec9b3262326ba4875a68718ada3f19e666 still contains the conditional-GET implementation in both SEO controllers.
derived:
  - The product goal of Issue 1351 is complete and the product code reached a healthy staging release after the independent Synology namespace defect was repaired.
  - The failed first `platform-reconcile` attempt is evidence of correct fail-closed rollback plus an infrastructure defect, not evidence that the ETag product behavior regressed.
  - The later healthy full deployment proves delivery of the accumulated product release but must not be represented as a successful `platform-reconcile` timing measurement.
unknown:
  - A successful genuine live `platform-reconcile` wall-time/savings measurement remains unknown and is owned by Issue 1345, not by this completed product task.
conflicts: []
first_failure:
  marker: missing-conditional-validator
  evidence: pre-task PublicRobotsController and PublicSitemapController returned revalidating successful responses without an ETag
rejected_hypotheses:
  - weakening cache revalidation would trade correctness for performance
  - using timestamps rather than content-derived validators would make unchanged bytes unnecessarily miss 304
  - treating the WebKit internal-resource flake as a product regression was rejected after the same exact head passed the bounded rerun
  - claiming Deploy run 34247444487 as the successful #1345 platform-reconcile benchmark would be false because that run selected `full`
changed_paths:
  - app/Http/Controllers/PublicPortal/PublicRobotsController.php
  - app/Http/Controllers/PublicPortal/PublicSitemapController.php
  - tests/Feature/PublicPortal/PublicSeoConditionalRequestTest.php
  - docs/agents/tasks/archive/OTERYN-20260908-portal-conditional-get.md
validation:
  - command: PR 1354 final exact-head CI 34233292528
    result: PASS
    evidence: runtime-tests and required platform-gate job 102085390001 succeeded on 051e572df5f5d4c1256915d4066712598a383c8a
  - command: PR 1354 exact-head Acceptance E2E 34233292569 attempt 2
    result: PASS
    evidence: the bounded same-SHA rerun completed successfully after the transient WebKit internal-resource failure
  - command: Merge Queue CI 34240817420
    result: PASS
    evidence: candidate 7c8792d64e6baff599ab6a35595a521f212ee1e6 passed required platform-gate job 102111237020
  - command: resulting accumulated protected-main staging deployment
    result: PASS
    evidence: Deploy Synology Staging run 34247444487 on main 3594f5ec9b3262326ba4875a68718ada3f19e666 ended healthy with rollback skipped after including the conditional-GET controller change in the accumulated release range
blockers: []
next_action: none
```

## Source branch closeout

```yaml
source_branch_disposition: auto_delete_after_merge
source_branch_reason: implementation PR 1354 merged through protected Merge Queue and the dedicated source branch has no retention purpose
source_branch_evidence: branch search after merge returned no perf/1351-portal-conditional-get ref; protected integration candidate was 7c8792d64e6baff599ab6a35595a521f212ee1e6
```

## Deployment-proof relationship

Issue #1345 intentionally remains open. The first genuine qualifying product release selected `platform-reconcile` but exposed the runner-network-namespace defect repaired by #1358/#1359 and rolled back safely. A later genuine Platform-only product release must provide the first successful repaired `platform-reconcile` timing proof; no no-op or synthetic product commit is authorized for that purpose.

## Notes

No production deployment, protected-main bypass, force-push, secret mutation, gate weakening or false successful timing claim occurred.