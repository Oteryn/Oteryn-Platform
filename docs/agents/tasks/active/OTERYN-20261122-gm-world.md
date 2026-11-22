---
task_id: OTERYN-20261122-gm-world
governing_issue: 2002
required_reads: []
search_first: []
optional_reads: []
---

# Ops console and Platform moderation

Governing issue [#2002](https://github.com/Oteryn/Oteryn-Platform/issues/2002). The coordinator limited this packet to existing trusted ops reads, CMS events, and Platform Bazaar listing moderation. The branch is retained for coordinator review.

## Acceptance criteria

- Exercise the existing overview, world, node, database, migrations, ledger, policy, and connected-agent views using their current result paths.
- Require a reason for CMS event saves and state changes, independent Bazaar invalidation approval, and listing report decisions. Keep authorization and audit in the existing transactions.
- Keep pending Bazaar invalidations paginated independently of approved history, suppress audit replays, and record event translation changes with per-locale content fingerprints.
- Validate successful ADM-01 results against the pinned schema for the stored operation. Show superseded Bazaar requests in their own history, and acknowledge the recorded approver's retry after reconciliation without a second audit or outbox row.
- Preserve command receipts and acknowledge exact stored ADM retries after ownership checks. Bind world and node details to their requested targets, enforce pinned integer widths, and render readiness and migration outcomes in both locales.
- Check browser flows in Polish and English at both configured widths.

ADM-01 ratifies transport framing. The pinned read result union includes `analytics.catalog`. This packet covers the existing console projections and the Platform Bazaar moderation records. Jira synchronization is unknown.

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-11-22T19:20:33Z
head: 03cbe4a65a557fe5ddebd4eef9b2971fe9e4685d
branch: codex/feat-gm-world
pr: none
status: ready
context_routes: []
owned_paths:
  - portal-rs/crates/content/src/events/
  - portal-rs/crates/admin/src/bazaar.rs
  - portal-rs/crates/admin/src/bazaar_tests.rs
  - portal-rs/crates/admin/src/console/dashboard_tests.rs
  - portal-rs/crates/admin/src/console/en.ftl
  - portal-rs/crates/admin/src/console/mod.rs
  - portal-rs/crates/admin/src/console/pl.ftl
  - portal-rs/crates/admin/src/console/projection.rs
  - portal-rs/crates/admin/src/console/stored.rs
  - portal-rs/crates/admin/src/console/stored_tests.rs
  - portal-rs/crates/admin/src/control/README.md
  - portal-rs/crates/admin/src/control/canonical.rs
  - portal-rs/crates/admin/src/control/http.rs
  - portal-rs/crates/admin/src/control/http_tests.rs
  - portal-rs/crates/admin/src/control/mod.rs
  - portal-rs/crates/admin/src/control/result_schema.rs
  - portal-rs/crates/admin/src/control/tests.rs
  - portal-rs/crates/bazaar/src/moderation.rs
  - portal-rs/crates/bazaar/src/moderation_tests.rs
  - portal-rs/crates/bazaar/src/game/runtime/invalidation.rs
  - portal-rs/crates/internal-api/src/guard.rs
  - portal-rs/e2e/specs/gm-world.spec.mjs
  - portal-rs/e2e/specs/gm-bazaar.spec.mjs
  - portal-rs/e2e/specs/content.spec.mjs
  - portal-rs/e2e/support/admin-control-fixture.mjs
  - portal-rs/e2e/support/admin-control-results.json
  - portal-rs/e2e/staff-rbac-helpers.mjs
proven:
  - Live governing issue 2002 is open and owner correction defines the feature scope.
  - ADM-01 transport framing is ratified and existing ops reads use the control store.
  - The current portal binary built with the lockfile. The mTLS harness reached ready and seeded a disposable world on PostgreSQL 18.6.
  - A disposable agent claimed Platform-signed read requests through ADM-01 over mTLS. The required viewport pair displayed the successful localized results and the agent ID, stored observation time and declared operations.
  - Successful command receipts follow the existing transport path. Exact historical retries succeed for the original delivery owner and are refused after epoch supersession.
  - The overview accepts the uint32 maximum. The first larger value, a fractional player count, the wrong shape and a null result are refused. Fractional node telemetry retains its schema-checked path.
  - World and node details are bound to the requested target at ingestion and presentation. The legacy signed mismatch test shows an unavailable result and retains the option to request fresh data.
  - Superseded approvals preserve their original audit. The worker checks the exact selected request against the current hold, and the final effective approval remains visible after cancellation.
  - The Bazaar browser cases preserved the public intake refusal. An eligible account's seeded pending report reached a separate moderator through the staff form and produced the expected audit.
derived: []
unknown:
  - Jira synchronization for issue 2002.
conflicts: []
first_failure:
  marker: ADM-01 result submission returned HTTP 400 for node.list with fractional CPU telemetry.
  evidence: The connected browser run exposed the PHP integer-only body guard on the ADM result route. The ADM I-JSON validator now preserves bounded object checks and accepts the signed fractional telemetry.
rejected_hypotheses: []
changed_paths:
  - portal-rs/crates/content/
  - portal-rs/crates/admin/
  - portal-rs/crates/bazaar/
  - portal-rs/crates/internal-api/src/guard.rs
  - portal-rs/templates/events/
  - portal-rs/templates/admin/events/
  - portal-rs/e2e/
validation:
  - command: cargo test --locked -p oteryn-admin -p oteryn-bazaar -p oteryn-content --lib --no-fail-fast -- control::control_http::tests control::canonical::body_tests console::stored::tests console::tests gm_browser_result_fixtures_match_pinned_operation_schemas bazaar::tests events::tests moderation::tests
    result: PASS
    evidence: 40 admin, 15 Bazaar and 7 CMS cases passed on PostgreSQL 18.6. The existing concurrent-mutation case hit a database I/O timeout in the overlapping run and passed in isolation in a fresh database.
  - command: cargo test --locked -p oteryn-admin -p oteryn-bazaar -p oteryn-content --lib -- invalidation_tests lifecycle_active_invalidate
    result: PASS
    evidence: 20 Bazaar invalidation and lifecycle cases passed serially in a separate database.
  - command: cargo test --locked -p oteryn-internal-api --lib
    result: PASS
    evidence: All 5 internal API library cases passed.
  - command: node e2e/harness.mjs --seed-plan /tmp/gm-harness-seed.json --exit-after-ready
    result: PASS
    evidence: Native admission, public HTTP and the internal mTLS listener became ready. The seed CLI created the disposable world fixture.
  - command: Playwright gm-world, gm-bazaar and content specs in pl-1440 and en-390
    result: PASS
    evidence: 14 passed with the existing local launcher. Coverage includes both signed detail mismatches, localized readiness and migration outcomes, post-cancellation approval replay and all four editorial cases per project. The Polish editorial news case passed in isolation after an initial logout connection reset.
  - command: cargo fmt --all --check
    result: PASS
    evidence: workspace formatting check passed.
blockers: []
next_action: Retain the qualified fast-forward follow-up branch for coordinator review under issue 2002.
```

## Source branch closeout

```yaml
source_branch_disposition: retain
source_branch_reason: Coordinator review of the five findings and the Bazaar worker handoff correction.
source_branch_evidence: The follow-up began at remote SHA 97fee35f366b7175ab974a2ce3a26d8eabe6483b. Tested product and test sources at 03cbe4a65a557fe5ddebd4eef9b2971fe9e4685d retain original base 33145d6ec7662cd01f91e304ba09fa904edeaf37.
```
