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
- Check browser flows in Polish and English at both configured widths.

ADM-01 ratifies transport framing. The allowlisted `analytics.catalog` entry has no accepted result-body projection in the Platform repository. The accepted analytics feed specifies aggregate families. Bazaar changes concern the existing Platform moderation records and result presentation. Jira synchronization is unknown.

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-11-22T16:56:26Z
head: 49b620fd3a934bdfdb4285cbba022737f52642be
branch: codex/feat-gm-world
pr: none
status: ready
context_routes: []
owned_paths:
  - portal-rs/crates/content/src/events/
  - portal-rs/crates/admin/src/bazaar.rs
  - portal-rs/crates/admin/src/control/canonical.rs
  - portal-rs/crates/admin/src/control/http.rs
  - portal-rs/crates/admin/src/control/http_tests.rs
  - portal-rs/crates/admin/src/control/tests.rs
  - portal-rs/crates/bazaar/src/moderation.rs
  - portal-rs/crates/internal-api/src/guard.rs
  - portal-rs/e2e/specs/gm-world.spec.mjs
  - portal-rs/e2e/specs/gm-bazaar.spec.mjs
  - portal-rs/e2e/support/admin-control-fixture.mjs
  - portal-rs/e2e/support/admin-control-results.json
  - portal-rs/e2e/staff-rbac-helpers.mjs
proven:
  - Live governing issue 2002 is open and owner correction defines the feature scope.
  - ADM-01 transport framing is ratified and existing ops reads use the control store.
  - The current portal binary built with the lockfile. The mTLS harness reached ready and seeded a disposable world on PostgreSQL 18.6.
  - A disposable agent claimed Platform-signed read requests through ADM-01 over mTLS. Signed responses reached the browser through the stored result path in all four projects.
  - The Bazaar browser cases preserved the public intake refusal. An eligible account's seeded pending report reached a separate moderator through the staff form and produced the expected audit.
derived: []
unknown:
  - Acceptance evidence for an analytics.catalog result-body schema is unavailable.
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
  - command: cargo test --locked -p oteryn-admin -p oteryn-content -p oteryn-bazaar --lib --no-fail-fast -- control::control_http::tests control::canonical::body_tests console::stored::tests gm_browser_result_fixtures_match_pinned_operation_schemas bazaar::tests events::tests moderation::tests
    result: PASS
    evidence: 33 admin, 15 Bazaar and 7 CMS event cases passed on PostgreSQL 18.6. Coverage includes the signed fractional result, malformed input and tamper refusals, the pending queue, the approval replay and the event translation audit.
  - command: cargo test --locked -p oteryn-internal-api --lib
    result: PASS
    evidence: All 5 internal API library cases passed.
  - command: node e2e/harness.mjs --seed-plan /tmp/gm-harness-seed.json --exit-after-ready
    result: PASS
    evidence: Native admission, public HTTP and the internal mTLS listener became ready. The seed CLI created the disposable world fixture.
  - command: Playwright gm-world and gm-bazaar specs in pl-1440, pl-390, en-1440 and en-390
    result: PASS
    evidence: 12 passed. The connected and unavailable ops paths, CMS lifecycle, staff report decision, independent approval and reconciled cancellation were exercised.
  - command: Playwright bilingual editorial event case in all four configured projects
    result: PASS
    evidence: 4 passed using staff sessions created by the existing seed helper.
  - command: cargo fmt --all --check
    result: PASS
    evidence: workspace formatting check passed.
blockers: []
next_action: Retain the qualified fast-forward follow-up branch for coordinator review under issue 2002.
```

## Source branch closeout

```yaml
source_branch_disposition: retain
source_branch_reason: Coordinator review of the five findings and the fractional telemetry correction.
source_branch_evidence: The review began at remote SHA 7e55874f70346005499d933dac1631288f03288a. Follow-up candidate 49b620fd3a934bdfdb4285cbba022737f52642be retains original base 33145d6ec7662cd01f91e304ba09fa904edeaf37.
```
