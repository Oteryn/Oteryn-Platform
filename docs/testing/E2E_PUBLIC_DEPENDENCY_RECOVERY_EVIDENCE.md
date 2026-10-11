# E2E Public Dependency Recovery Evidence

## Purpose

This record captures the bounded P1 browser/system resilience slice added through PR #106.

The slice complements existing failure-only evidence by proving the complete controlled lifecycle for representative public dependencies:

`known-good -> deterministic dependency denial -> fail-closed user-visible behavior -> dependency restoration -> successful browser recovery`.

The tests use only acceptance-scoped MariaDB and Redis principals and run on one Chromium project with zero retries.

## Legacy engine read dependency recovery

The `/online` resilience scenario:

1. requests `/online` with normal acceptance grants and proves HTTP `200` plus the seeded `Acceptance Hero` row;
2. revokes `SELECT` on `legacy_acceptance.cluster_sessions` from the acceptance read-only principal;
3. requests `/online` again and requires HTTP `503`;
4. verifies the error response does not expose `SQLSTATE` or the acceptance MariaDB root password;
5. restores the `SELECT` grant in `finally` cleanup;
6. requests `/online` again and requires HTTP `200` plus the seeded online character.

This adds direct browser recovery evidence beyond the pre-existing dependency-denial assertion.

## Redis runtime dependency recovery

The `/servers` resilience scenario:

1. requests `/servers` with the normal acceptance Redis ACL and proves `Runtime: ONLINE` plus one online player;
2. removes only the `HMGET` command permission from the dedicated acceptance runtime Redis user;
3. requests `/servers` and requires the bounded runtime-unavailable UI while configured server metadata remains renderable;
4. restores `HMGET` permission in `finally` cleanup;
5. requests `/servers` again and requires the live `ONLINE` runtime state and online-player count to return.

The test does not stop the shared Redis service and does not alter production configuration. It mutates only the disposable acceptance ACL user and restores the removed command permission before completion.

## Exact-SHA evidence

Implementation head:

`7f21ac65bad1da9514d0e1d6ade48a2da9ee8918`

Required workflow results on that head:

- `Acceptance E2E and Visual UX` run `29847628355` — PASS;
- `CI` run `29847629469` — PASS;
- `Agent Governance` run `29847629232` — PASS;
- `Phase 7 Production-Like Validation` run `29847629405` — PASS;
- `Platform DB Outage Validation` run `29847628752` — PASS.

Acceptance artifact:

- `acceptance-e2e-critical-29847628355-1`;
- artifact id `8502051195`;
- digest `sha256:87fa7d58515961c9fbd9c69632d8a114684727d532f0c82442065a940511a46e`.

Measured critical-profile evidence:

- primary Chromium smoke: PASS, `9 s` wall-clock;
- bounded Chromium/Firefox/WebKit portability: PASS, `23 s` wall-clock;
- bounded desktop/tablet/mobile responsive: PASS, `10 s` wall-clock;
- bounded `resilience-chromium`: PASS, `3 s` wall-clock;
- resilience retries: `0`;
- aggregate required critical result: `AUTOMATED_E2E_CRITICAL_PASS`.

The evidence packet records `resilience_result: success` and the resilience profile/project identity without storing credentials or dependency secrets.

## Classification boundary

The two dependency recovery paths are `STAGING_PROVEN` for the controlled production-like acceptance environment.

They do not prove:

- production database or Redis HA/failover;
- production recovery time or RTO/RPO;
- production network partition behavior;
- production Redis ACL provisioning or credential rotation;
- production legacy engine DB grants/endpoints;
- cross-region recovery or provider restart behavior.

Those facts remain environment-specific and, where applicable, require direct production evidence under issue #91.
