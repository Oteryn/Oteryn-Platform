---
task_id: OTERYN-20260816-public-today-runtime
repository: blakinio/Oteryn-Platform
mode: implementation
issue: 1113
programme: OTERYN_PORTAL_COMPLETION
status: completed
closed_at: 2026-08-16T21:02:19+02:00
implementation_pr: 1114
implementation_merge_sha: 5cc42a9bc2a4732265cf83533fe7681ab1ab5865
validated_head: 10807dc51ba0d16a6b0ed745ae3657540d70b392
---

# OTERYN-20260816-public-today-runtime — COMPLETED

## Terminal result

Issue #1113 and PR #1114 are terminal. The bounded `PUBLIC_GUEST` Today command-centre slice was squash-merged into protected `main` as `5cc42a9bc2a4732265cf83533fe7681ab1ab5865` after exact-head validation on `10807dc51ba0d16a6b0ed745ae3657540d70b392`.

The source branch `feat/issue-1113-public-today` is absent from the live repository branch index after merge. This closeout changes lifecycle documentation only and releases the task's implementation ownership.

## Delivered boundary

- Public `/en/today` and `/pl/today` compose source-owned Announcements, Events and CMS/news providers.
- LiveOps remains explicitly unavailable/not-yet-provided because no authoritative Platform LiveOps runtime provider exists in this slice; no current, stale or recovered runtime fact is fabricated.
- Present, healthy-empty and unavailable provider states remain distinct; partial composition is explicit.
- Card priority/order is deterministic and typed.
- Today responses are `no-store`/no-cache and introduce no derived Today cache or index.
- Public navigation, localized route registration, sitemap/canonical metadata and EN/PL copy are integrated.
- Browser acceptance proves Today success/empty/partial/absent-LiveOps semantics, Firefox/WebKit portability, desktop/tablet/mobile responsive behavior, dependency resilience and keyboard accessibility at retries zero.
- Existing shared homepage provider truth is preserved; acceptance fixtures were aligned without weakening shared tests or changing source-provider priority.

## Final exact-head CI evidence

Validated head: `10807dc51ba0d16a6b0ed745ae3657540d70b392`

```yaml
agent_governance:
  run: 31965948284
  result: PASS
ci:
  run: 31965948292
  result: PASS
  evidence: Composer validation/audit, Pint, PHPStan/Larastan, full PHPUnit, required test gate
phase7_production_like:
  run: 31965948373
  result: PASS
portal_acceptance_contract:
  run: 31965948320
  result: PASS
acceptance_e2e_visual_ux:
  run: 31965948266
  result: PASS
  evidence: Chromium smoke, Firefox/WebKit portability, desktop/tablet/mobile responsive profile, public dependency resilience, keyboard accessibility, retries zero
content_scale_acceptance:
  run: 31965948365
  result: PASS
edge_security_emulation:
  run: 31965948345
  result: PASS
platform_db_outage:
  run: 31965948521
  result: PASS
game_auth_ticket_concurrency:
  run: 31965948273
  result: PASS
build_synology_staging_images:
  run: 31965948317
  result: PASS
community_data_acceptance:
  run: 31965948315
  result: PASS
```

All 11 workflow runs associated with the validated implementation head were terminal `success` before merge.

## Review and merge evidence

```yaml
whole_diff_self_review: PASS
review_findings_repaired:
  - preserved current sitemap behavior and added Today minimally
  - removed invalid controller inheritance
  - updated strict surface count only after validator proved 31/31 surfaces with zero gaps
  - removed PHPStan-rejected redundant list normalization
  - made Today event/news links locale-explicit
  - aligned homepage shared-upcoming fixture truth
  - removed responsive heading fixture collision inside owned Today acceptance fixtures
submitted_reviews_at_final_head: 0
review_threads_at_final_head: 0
pr_ready_before_merge: true
mergeable_before_merge: true
protected_base_before_merge: 286efb1625d510c9d2cc344cb51a2438b31ebe48
expected_head_merge_guard: 10807dc51ba0d16a6b0ed745ae3657540d70b392
merge_method: squash
merge_sha: 5cc42a9bc2a4732265cf83533fe7681ab1ab5865
issue_1113_state: closed
issue_1113_state_reason: completed
source_branch: feat/issue-1113-public-today
source_branch_disposition: auto_delete_after_merge
```

## Authorization / nonclaims

No Oteryn-v2/Canary repository access, external/server-repository operation, production/staging/protected-environment mutation, deployment, secret mutation or owner-funded Codex/OpenAI/API operation was performed or claimed.

The absence of an authoritative LiveOps runtime provider remains an explicit product/data availability fact, not a fabricated empty state and not a blocker to the completed public Today slice.

## Lifecycle closeout

This archived record is the lifecycle-only closeout artifact. The implementation task is no longer active and owns no paths after this archive PR merges.
