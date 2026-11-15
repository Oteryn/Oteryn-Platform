# Audit 03 remediation

Base: `b4ebbbf15b1c361099dfc3877d18ba392a379b64`.
Branch: `claude/happy-davinci-ijz3wy-fix-03`.

The changes preserve the existing ingest/read flags and contract wire formats.
No merged migration was edited. The additive migration retains the legacy
reminder table and copies its pending IDs into a scope-aware table, synchronizing
existing delivery states between the two tables. Bank continuation is backfilled
only from the latest record; a legacy stream whose latest payload was already
lost fails closed until independently validated rebootstrap.

| Finding | Result | Implementation and regression |
| --- | --- | --- |
| 03-001 | Fixed | `src/retention.rs:13`; additive worker registration at `../../bin/oteryn-portal/src/worker.rs:191`. `postgres_production_retention_dispatch_erases_payload_preserves_replay`, `pg_production_retention_erases_item_snapshots_and_event_payloads`, `postgres_production_retention_erases_death_payloads_preserves_fences`. Database time, at most 512 payloads per table per dispatch, replay evidence preserved. |
| 03-002 | Fixed | `src/authority/reads.rs:172`, `src/store.rs:336`; 128-row keyset promotion batches stay uncommitted until atomic baseline publication. Ordinary operations share the authority fence and serialize per account; record-ID locks preserve global replay binding. Analogous core, progress, goals, drops and places reads refuse overflow beyond 4096 candidates rather than returning an incomplete result. `postgres_login_baseline_batches_are_complete_atomic_and_allow_other_accounts`, `postgres_goal_limit_refuses_overflow_instead_of_truncating`, `postgres_shared_authority_preserves_cross_account_record_binding`. |
| 03-003 | Not fixed | Needs `portal-rs/crates/characters/src/commands/storage.rs:228` and `portal-rs/crates/identity/src/termination.rs:312`, outside the allowed files. These are the real transfer-receipt and account-finalization commits where durable erasure work must be created atomically. Existing erasure methods remain unwired. |
| 03-004 | Fixed | `src/authority.rs:126`, `src/authority.rs:141`, `src/authority.rs:253`. Lock the LCFA feed before snapshots, require its completed current epoch and fresh database-clock evidence, and match the snapshot source to the authenticated feed source. This also protects the ownership proof used by community reads. `postgres_global_feed_invalidation_refuses_admission_and_ownership`. |
| 03-005 | Fixed | `src/logins.rs:283`, `src/logins.rs:311`; authorized generation changes replace retained occurrences atomically with baseline publication, revision comparisons apply only within a generation, and completed baseline reads exclude older generations. `postgres_authorized_epoch_rebuild_reuses_retained_occurrence`. |
| 03-006 | Fixed | `src/logins.rs:311`, `src/deaths/storage.rs:276`; login candidates use bounded keyset pages until the visible page is full; deaths select the latest entity first and then exclude erased/expired payloads before the presentation limit. `postgres_visible_login_behind_sold_character_candidates`, `postgres_expired_death_entities_do_not_hide_recent_history`. |
| 03-007 | Fixed | `src/progress/store.rs:155`, `src/progress/presentation.rs:166`, `src/store.rs:668`; observations carry freshness, stale non-timer sections are labelled, and stale timer evidence is unavailable. Payload-only legacy consumers refuse stale observations. `postgres_stale_boss_timer_is_unavailable_and_progress_is_labelled`. |
| 03-008 | Not fixed | Needs the shared account export entry point at `portal-rs/crates/identity/src/export.rs:26` and its registration at `:132`, outside the allowed files. Reports still have the verified GET attachment bypass at `src/reports/http.rs:116`; no claim is made that it has been mitigated. The combined export, fresh authentication, CSRF request, cross-family rolling budget and audit must be implemented as one flow. |
| 03-009 | Fixed | `src/items/store.rs:240`; retained replay bindings resolve after current authority/floor checks and before first-delivery expiry checks. `pg_expired_exact_replay_preserves_outcome_without_payload`. |
| 03-010 | Fixed | `src/store.rs:833` and the additive migration; minimal bank continuation survives presentation and replay pruning. Account erasure deletes it. `postgres_bank_continues_after_payload_and_metadata_expiry`. |
| 03-011 | Fixed | `src/progress/store.rs:117`; baseline flag is read under the stream lock, and true-to-false watermarks conflict without state change. `postgres_complete_baseline_cannot_be_revoked`. |
| 03-012 | Not fixed | Needs a production operator alarm reader/consumer, including the operator surface at `portal-rs/crates/admin/src/console/mod.rs:680`, outside the allowed files. `src/store.rs:430` still writes only the existing conflict reference. Adding another unread table or unhandled outbox kind would not satisfy the alarm requirement. |
| 03-013 | Fixed | `src/reminders/mod.rs:140` and the additive migration; uniqueness includes character and ownership cut, with NULLS NOT DISTINCT for deterministic account scope. The regression also checks legacy-table interoperability. `postgres_reminder_identity_includes_character_and_cut`. |
| 03-014 | Fixed | `src/reports.rs:357`; prepare notification account state before reserving a transaction connection, then lock/revalidate identity, recipient and locale and enqueue with immutable preparation on the supplied transaction. `postgres_email_week_real_account_adapter_one_connection`. |

The staged-promotion concurrency regression was also run against the previous
exclusive authority locking and unbounded query: it failed because the other
account timed out. The new variant checks all 257 projected occurrences and
invisibility of uncommitted projection state. No partial baseline is published.

## Validation

Installed repository-pinned Rust 1.97.0; `CARGO_BUILD_JOBS=2`,
`RUST_TEST_THREADS=1`. PostgreSQL tests used `PORTAL_TEST_DATABASE_URL` from
the prepared local test service. All sixteen new PostgreSQL regressions executed
and took more than 0.05 seconds. Earlier overlapping/default-parallel runs
exhausted PostgreSQL's shared lock table; the complete final suites ran serially.

- `cargo fmt --all --check`: passed.
- `cargo clippy --locked --all-targets -p oteryn-playerdata -p oteryn-portal -- -D warnings`: passed.
- `cargo test --locked -p oteryn-playerdata -- --nocapture`: 143 passed, 0 failed, 0 ignored.
- `cargo test --locked -p oteryn-portal -- --nocapture`: 129 unit tests and 3 integration tests passed, 0 failed. One existing Chromium test remains ignored; no test was newly ignored or weakened.
- Aggregate: **275 passed, 0 failed, 1 pre-existing ignored**.

The promotion regression measures actual PostgreSQL result batch cardinality
and asserts its maximum is 128 while publishing all 257 occurrences atomically.
The retention regression also proves a 31-day-old death payload survives the
full 90-day death/PvP retention window.

| New PostgreSQL regression | Duration (seconds) |
| --- | ---: |
| `postgres_global_feed_invalidation_refuses_admission_and_ownership` | 0.962 |
| `postgres_login_baseline_batches_are_complete_atomic_and_allow_other_accounts` | 3.597 |
| `postgres_expired_death_entities_do_not_hide_recent_history` | 0.466 |
| `postgres_production_retention_erases_death_payloads_preserves_fences` | 1.348 |
| `pg_expired_exact_replay_preserves_outcome_without_payload` | 0.142 |
| `pg_production_retention_erases_item_snapshots_and_event_payloads` | 0.974 |
| `postgres_authorized_epoch_rebuild_reuses_retained_occurrence` | 1.078 |
| `postgres_visible_login_behind_sold_character_candidates` | 0.914 |
| `postgres_complete_baseline_cannot_be_revoked` | 0.168 |
| `postgres_goal_limit_refuses_overflow_instead_of_truncating` | 0.074 |
| `postgres_stale_boss_timer_is_unavailable_and_progress_is_labelled` | 0.220 |
| `postgres_reminder_identity_includes_character_and_cut` | 1.075 |
| `postgres_email_week_real_account_adapter_one_connection` | 1.026 |
| `postgres_bank_continues_after_payload_and_metadata_expiry` | 1.088 |
| `postgres_production_retention_dispatch_erases_payload_preserves_replay` | 2.518 |
| `postgres_shared_authority_preserves_cross_account_record_binding` | 1.196 |
