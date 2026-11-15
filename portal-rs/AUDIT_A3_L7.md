# Audit wave 3: L7 validation

Base: `90234d8d2d0a51a654b8ff6947d38372a740acba` (`origin/claude/happy-davinci-ijz3wy-audit3-int`).
Branch: `claude/happy-davinci-ijz3wy-fix-a3-l7`.
Single agent; no subagents. Only the listed findings were addressed; this is not an exhaustive audit.

## Findings

- **L7-001: Already fixed on base** — `.github/workflows/portal-rs-ci.yml:94`. The check job installs Chromium and pinned browser libraries, explicitly runs all four ignored browser tests at lines 110–113, and requires one passing test for each selector at lines 104–108. No workflow or browser assertion changes.
- **L7-002: Fixed** — `crates/web/tests/ratelimit.rs:180`. Freeze statement_timestamp() in every listed rate-limit fixture, keep MFA/session wall clocks live, and advance explicitly for rollover. The boundary test fills 2100-01-01 00:00:59.999 UTC, proves denial, advances 1 ms, and proves acceptance; stored windows must match that controlled clock. Remove earlier real-clock alignment waits in login/sudo tests.
- **L7-003: Fixed** — `bin/oteryn-portal/tests/connection_budget.rs:29`. Each active module request must block on its expected relation before returning its expected status and identifying main content, followed by the shared connection budget assertion. Authenticated and deeper character/collection routes replace static-only evidence.
- **L7-004: Fixed** — `bin/oteryn-portal/src/account_closure_tls_tests.rs:76`. Real TLS 1.3/HTTP 1.1 listener, using production TLS configuration, purpose map, router, and serving loop: no certificate, foreign CA, unpinned trusted CA, both valid pins against every opposite-purpose route, and matching-purpose positive controls. Domain tables remain empty.
- **L7-005: Fixed** — `bin/oteryn-portal/src/worker_tests.rs:857`. Wait for successful extension of all sixteen running leases beyond the observed baseline before releasing saturated handlers. The exhausted renewal-pool test synchronizes on an observer called when renewal is polled, while its sole connection remains held.
- **L7-007: Fixed** — `crates/gameauth/src/admission/audit05_tests.rs:624`. Assert Fresh under a schema clock before admission. Observe the assignment-lock wait, then advance two seconds. The signer checkpoint advances the clock after trust refresh. Assert refusal before signer preparation/signing respectively, preventing later checks from hiding a missing earlier recheck. Clear SQLx cached statements after installing clock overrides.

The composed binary deliberately refuses guilds, houses and deaths before SQL while G-4 contracts remain CANDIDATE (`crates/community/src/guilds.rs:249`, `houses.rs:160`, `deaths.rs:273`). Their exact 503 refusal content is asserted separately; these routes receive no database-execution credit. No candidate contract or feature gate was enabled to manufacture coverage.

No merged migration, contract fixture/bytes, RBAC-owned file, production rate limiter, or production freshness check was changed. The production listener/worker edits only expose local, non-global test seams around existing behavior. The rate-clock helper is test-only and installed after migrations into disposable schemas with pg_catalog explicitly last in search_path.

## Final checks

Rust 1.97.0; real local PostgreSQL 18.6 through PORTAL_TEST_DATABASE_URL. No tool reinstallation. Local PostgreSQL uses the CI settings max_locks_per_transaction=1024 and max_connections=300. An initial concurrent migration run hit the setup service's lower lock limit; it was corrected and validation rerun. An old sudo wall-clock alignment loop was also replaced after the frozen clock exposed its unbounded wait.

The package set for both Cargo commands was `oteryn-web`, `oteryn-identity`, `oteryn-wiki`, `oteryn-gameauth`, `oteryn-portal`:

- `cargo fmt --all --check`: PASS.
- `cargo clippy --locked --all-targets -p <each package above> -- -D warnings`: PASS.
- `cargo test --locked -p <each package above> -- --nocapture --test-threads=2`: PASS.
- Final Cargo parent-suite totals (child-process fixture summaries excluded): **604 passed, 0 failed, 2 existing ignored tests**. Existing browser ignores are unchanged; CI explicitly selects the four browser tests for L7-001. Local browser assertions were not rerun.
- Targeted timed run: **57 passed, 0 failed**: 46 PostgreSQL tests and 11 unit tests. All 46 PostgreSQL durations exceed 0.05 s; none are counted from a skipped return. Nextest filter exclusions are unrelated tests, not disabled tests; all crates also ran through cargo test above.

## PostgreSQL test names and durations

| Test | Seconds |
|---|---:|
| `oteryn-gameauth admission::audit05_tests::wave2_03_report_expires_while_assignment_locked` | 1.165 |
| `oteryn-gameauth admission::audit05_tests::wave2_03_report_rechecked_after_trust_refresh` | 2.210 |
| `oteryn-portal::connection_budget composed_modules_share_one_postgres_connection_budget` | 2.340 |
| `oteryn-portal::bin/oteryn-portal account_closure::tls_tests::dedicated_closure_listener_refuses_untrusted_unpinned_and_cross_purpose_clients` | 1.533 |
| `oteryn-portal::bin/oteryn-portal code_routes::tests::db_codes_production_dispatch_issues_redeems_and_refuses_unfulfillable_items` | 1.883 |
| `oteryn-portal::bin/oteryn-portal audit_l9_tests::production_proxy_budgets` | 2.521 |
| `oteryn-portal::bin/oteryn-portal credentials_http::tests::audit02_sudo_succeeds_when_sixteen_requests_fill_pool` | 3.657 |
| `oteryn-portal::bin/oteryn-portal credentials_http::tests::audit02_sudo_uses_one_connection_and_preserves_retry_after` | 3.540 |
| `oteryn-portal::bin/oteryn-portal login_http::tests::audit02_paused_password_completion_refuses_reset_even_after_anonymous_get` | 2.625 |
| `oteryn-portal::bin/oteryn-portal login_http::tests::logout_delete_failure_keeps_deletion_cookie_through_error_renderer` | 1.276 |
| `oteryn-portal::bin/oteryn-portal credentials_http::tests::postgres_sudo_rotation_csrf_refusal_and_expiry` | 6.445 |
| `oteryn-portal::bin/oteryn-portal login_http::tests::postgres_browser_login_logout_csrf_and_limits` | 2.791 |
| `oteryn-portal::bin/oteryn-portal login_http::tests::postgres_source_limit_bad_input_and_missing_transport` | 1.099 |
| `oteryn-portal::bin/oteryn-portal login_http::tests::postgres_login_returns_to_stored_authorize_target_once` | 2.779 |
| `oteryn-portal::bin/oteryn-portal login_http::tests::production_proxy_policy_isolates_registration_verification_login_and_search` | 1.536 |
| `oteryn-portal::bin/oteryn-portal mfa_http::tests::postgres_mfa_limits_and_missing_transport` | 1.405 |
| `oteryn-portal::bin/oteryn-portal mfa_http::tests::postgres_mfa_browser_csrf_limiter_and_login_challenge` | 2.313 |
| `oteryn-portal::bin/oteryn-portal worker::tests::handler_finishes_while_renewal_waits_for_capacity` | 1.043 |
| `oteryn-portal::bin/oteryn-portal mfa_http::tests::postgres_password_browser_rotation_releases_account_lock` | 2.219 |
| `oteryn-web search::tests::postgres_http_success_refusal_bad_input_and_escaping` | 1.144 |
| `oteryn-portal::bin/oteryn-portal worker::tests::sixteen_locked_topups_finish_after_heartbeat_with_exhausted_handler_pool` | 2.011 |
| `oteryn-web search::tests::postgres_pagination_and_shared_search_abuse_boundary` | 1.441 |
| `oteryn-web search::tests::postgres_publication_translation_and_full_text` | 1.707 |
| `oteryn-web search::tests::postgres_reserved_page_destinations` | 1.071 |
| `oteryn-web search::tests::postgres_world_candidates_require_current_public_api_allow` | 1.523 |
| `oteryn-web search::tests::postgres_world_visible_pagination` | 1.709 |
| `oteryn-web verify::budget_tests::audit02_resend_locales_share_source_budget_and_cooldown` | 1.432 |
| `oteryn-web::ratelimit anonymous_source_budget_survives_rotating_identifiers` | 1.572 |
| `oteryn-web::ratelimit bad_key_sets_are_refused_without_charging` | 1.484 |
| `oteryn-web::ratelimit cleanup_removes_old_windows_and_preserves_active_limits` | 1.104 |
| `oteryn-web::ratelimit expired_window_rolls_over_and_keys_are_independent` | 1.405 |
| `oteryn-web::ratelimit fifty_concurrent_increments_across_instances_never_exceed_budget` | 1.959 |
| `oteryn-web::ratelimit limit_returns_429_retry_after_and_localized_html_or_api_json` | 1.463 |
| `oteryn-web::ratelimit multiple_keys_charge_independently_and_refusal_is_retained` | 1.355 |
| `oteryn-web::ratelimit normalized_emails_share_budget_and_raw_keys_never_enter_table` | 2.008 |
| `oteryn-web::ratelimit proxy_trust_controls_the_actual_database_key` | 1.895 |
| `oteryn-web::ratelimit unavailable_database_fails_closed_at_http_boundary` | 1.758 |
| `oteryn-wiki web::tests::database_browser_create_review_publish_and_csrf` | 1.483 |
| `oteryn-wiki web::tests::database_editorial_lifecycle_and_restore` | 1.646 |
| `oteryn-wiki web::tests::database_history_immutable_and_audit_atomic` | 1.720 |
| `oteryn-wiki web::tests::database_history_pages_bound_response_and_retain_older_revisions` | 1.522 |
| `oteryn-wiki web::tests::database_public_search_locale_pagination_and_throttling` | 1.540 |
| `oteryn-wiki web::tests::database_queued_create_edit_restore_recheck_revocation` | 1.333 |
| `oteryn-wiki web::tests::database_refusals_bad_input_and_concurrent_edits` | 1.277 |
| `oteryn-wiki web::tests::history_exact_ten_snapshots_has_no_older_control` | 1.533 |
| `oteryn-wiki web::tests::postgres_pagination_wiki_ceiling` | 1.627 |

## Mutation checks

Eight expected test failures were observed. Every temporary production mutation was restored before final checks.

| Mutation | Regression test | Detected |
|---|---|---|
| `l2-real-clock` | `expired_window_rolls_over_and_keys_are_independent` | yes (exit 101) |
| `l3-missing-route` | `composed_modules_share_one_postgres_connection_budget` | yes (exit 101) |
| `l4-unpinned` | `dedicated_closure_listener_refuses_` | yes (exit 101) |
| `l4-purpose` | `dedicated_closure_listener_refuses_` | yes (exit 101) |
| `l5-no-heartbeat` | `sixteen_locked_topups_finish_after_heartbeat_with_exhausted_handler_pool` | yes (exit 101) |
| `l5-no-heartbeat` | `handler_finishes_while_renewal_waits_for_capacity` | yes (exit 101) |
| `l7-no-post-wait` | `wave2_03_report_expires_while_assignment_locked` | yes (exit 101) |
| `l7-no-post-refresh` | `wave2_03_report_rechecked_after_trust_refresh` | yes (exit 101) |

The clock mutation bypassed the schema function with pg_catalog.statement_timestamp(); the missing-route mutation used a nonexistent news URL; the pin mutation admitted unpinned leaves; the purpose mutation mapped the portfolio pin to the closure role; the heartbeat mutation disabled execute's heartbeat branch; the freshness mutations independently removed the post-wait clamp and first post-refresh revalidation. Failures came from executed assertions/timeouts, not compilation failures.

## Additional paths outside the initial ownership list

- `portal-rs/crates/identity/src/credentials_http_tests.rs`
- `portal-rs/crates/identity/src/credentials_test_db.rs`
- `portal-rs/crates/identity/src/lib.rs`
- `portal-rs/crates/identity/src/login_http_tests.rs`
- `portal-rs/crates/identity/src/login_test_db.rs`
- `portal-rs/crates/identity/src/mfa_test_db.rs`
- `portal-rs/crates/identity/src/reset_test_db.rs`
- `portal-rs/crates/wiki/src/web/tests.rs`
- `portal-rs/test-support/rate_clock.rs`
- `portal-rs/AUDIT_A3_L7.md`
