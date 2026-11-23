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
- Validate cached signed policy and ledger results before rendering and allow fresh queries when the schema fails. Replay a Bazaar suspension only for the same actor and request payload. Preserve the previous and persisted event publisher in the audit, and localize each world revision label.
- Integrate current main while preserving event pagination, the two translation additions, and both local harness integrations.
- Check browser flows in Polish and English at both configured widths.

ADM-01 ratifies transport framing. The pinned read result union includes `analytics.catalog`. This packet covers the existing console projections and the Platform Bazaar moderation records. Jira synchronization is unknown.

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-11-22T20:52:39Z
head: ae8786651dbc41156aacd9f5c99486b5e3dad067
branch: codex/feat-gm-world
pr: 2036
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
  - portal-rs/e2e/specs/public-depth-editorial.spec.mjs
  - portal-rs/e2e/support/admin-control-fixture.mjs
  - portal-rs/e2e/support/admin-control-results.json
  - portal-rs/e2e/staff-rbac-helpers.mjs
  - portal-rs/migrations/20261122200000_bazaar_suspension_requests.sql
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
  - Invalid legacy signed ledger and policy results produce unavailable views and trigger replacement requests. The stored-console tests observe both replacement request IDs and the valid rendering after reconciliation.
  - An exact suspension retry keeps the hold revision, independent approval, and single suspension audit. Reusing the UUID from a second actor is refused.
  - The main merge retained the public event pagination case, both event translation keys, and both local launcher integrations. The separate outbox harness answered its health check.
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
  - portal-rs/migrations/20261122200000_bazaar_suspension_requests.sql
validation:
  - command: cargo test --locked -p oteryn-admin -p oteryn-bazaar -p oteryn-content --lib --no-fail-fast -- control::control_http::tests control::canonical::body_tests console::stored::tests console::tests gm_browser_result_fixtures_match_pinned_operation_schemas bazaar::tests events::tests moderation::tests invalidation_tests lifecycle_active_invalidate
    result: PASS
    evidence: All 40 admin, 36 Bazaar and eight CMS cases passed on PostgreSQL 18.6.
  - command: cargo test --locked -p oteryn-content --test transaction_authorization
    result: PASS
    evidence: All seven content authorization and transaction cases passed.
  - command: cargo test --locked -p oteryn-internal-api --lib
    result: PASS
    evidence: All 5 internal API library cases passed.
  - command: node e2e/harness.mjs --seed-plan /tmp/gm-harness-seed.json --exit-after-ready
    result: PASS
    evidence: Native admission, public HTTP and the internal mTLS listener became ready. The seed CLI created the disposable world fixture.
  - command: Playwright gm-world, gm-bazaar and content specs in pl-1440 and en-390
    result: PASS
    evidence: All 14 cases passed at the default settings. The cases cover signed detail mismatches, all four localized world revision labels, suspension and approval retries, publisher changes, and both Wiki cases.
  - command: Playwright public-depth-editorial event pagination in pl-1440 and en-390
    result: PASS
    evidence: Both cases preserved localized links, page bounds and archival removal on the integrated revision.
  - command: cargo test --locked -p oteryn-db --test migrations
    result: PASS
    evidence: All four checks passed, including unique embedded migration versions.
  - command: cargo fmt --all --check
    result: PASS
    evidence: workspace formatting check passed.
blockers: []
next_action: Retain the qualified fast-forward follow-up branch for coordinator review under issue 2002.
```

## Source branch closeout

```yaml
source_branch_disposition: retain
source_branch_reason: Coordinator review of the five findings and the integrated revision under issue 2002.
source_branch_evidence: The follow-up began at remote SHA 9038dc292163277df5998ac8c8d79cd6c53de975. Merge cc017130685509e60f5d5d9a93cea36066a05f74 integrated main at 6c2ba2ed76b4bef8e6877ed71ef18a6ee094a12a. The validated portal sources are at ae8786651dbc41156aacd9f5c99486b5e3dad067.
```

The previous qualification covered the Bazaar worker handoff at `03cbe4a65a557fe5ddebd4eef9b2971fe9e4685d` on original base `33145d6ec7662cd01f91e304ba09fa904edeaf37`. The concurrent mutation and Polish news cases passed on isolated reruns in that qualification.
