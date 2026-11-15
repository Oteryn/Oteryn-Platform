# Audit LK: lock ordering and bounded pool usage

Task authority: owner request for `claude/happy-davinci-ijz3wy-fix-lk` based on
`d37a93d38e0175b3f12ba099e3cd0112a5457fac`. Single writer; no PR, deployment,
merged migration changes, game repository access, or feature enablement.

## L2-02

- Catalog activation now authorizes (and locks the actor account) before locking
  the active pointer, matching purchase's account/referral preparation order.
  The configured CLI adapter (`bin/oteryn-portal/src/catalog_cli.rs:22`) already
  locks the actor account; purchase's account preparation and pointer lock
  (`store/src/purchase_repository.rs:173,183`) retain their account-first order.
- Ticket changes lock owner and author in UUID order, before the ticket and RBAC
  fence. This includes account locks otherwise introduced by author foreign keys
  and configured authorization. The staff queue prepares these same participants
  before its outer authorization fence, so the two entry points agree.
- Recovery's base implementation had already avoided the handle/account cycle by
  taking the account first. That contradicted the gateway contract's §19.4 order.
  Recovery now acquires an account-table EXCLUSIVE fence after attempt/epoch and
  before resource rows, then locks handle before Identity. PostgreSQL's existing
  table locks on account updates, row-locking reads and FK checks make this fence
  compatible with admission, device revocation, passkeys and both revocation
  triggers, without changing merged migrations or relying on every caller to
  remember a new application advisory lock.

The unchanged competing paths are account-first: `gameauth/src/admission/tickets.rs:100`,
`gameauth/src/devices.rs:486,504`, and `identity/src/passkeys.rs:306`.
Their account locks precede `admission/protocol3.rs:120`'s handle revocation,
`passkeys.rs`'s explicit handle revocation, and the device/security trigger bodies
in `20261112100007_s19_protocol3_recovery.sql:71` and
`20261114100001_native_recovery_hardening.sql:34`. The recovery table fence
coordinates with those existing PostgreSQL locks; these callers and merged
migrations need no changes.

The recovery fence is intentionally conservative: recovery transactions serialize
with account mutations across accounts. Plain account readers remain concurrent.
Existing transaction/lock timeouts and feature gates remain in effect.

Regression coverage: `lk_catalog_account_before_pointer`,
`lk_crossed_staff_replies_finish`, and the strengthened
`audit05_04_recovery_waits_for_account_without_holding_handle`.

## L2-03

Remaining nested acquisitions were confirmed in commerce submit/approve,
sanction authorization, referral notification locale lookup, account notification
preferences/unsubscribe, the shared security notification adapter, and control
request evidence. They now pass the existing connection through authorization, preview,
clock, session-proof and account reads. `BrowserSession::get_in` retains the live
session expiry query. Watch policy locale lookup also receives the transaction.
No freshness checks are replaced with cached authorization. The account lifecycle
composition reuses the shared identity notification adapter, which now implements
`active_verified_in`; its single-connection regression also verifies revocation.

The following listed paths already had the relevant fixes on the base and retain
those implementations:

| Paths | Base evidence |
| --- | --- |
| admin RBAC and role writes | `rbac/mod.rs:158` exposes `check_in`; `rbac/storage.rs:65,153` uses transaction-aware authorization/account state |
| admin player mutations | `players/mod.rs:195,287,320,344,412,448,489,522` uses `check_in` |
| admin web/bootstrap | `web.rs:84,548,568` uses transaction-aware state and permission reads |
| content events and announcements | `events/repository.rs:81,145`, `announcements.rs:125` uses `check_in` |
| world administration | `world/src/admin.rs:330` rechecks permissions on its transaction |
| character creation | `create_web/service.rs:206,215,222,229,235` uses eligibility/replay/world/name/enqueue transaction helpers |
| notifications | `notify/src/service.rs:265,289,304,356` uses `account_in`; configured credentials/support adapters override `active_verified_in` |
| login observation | `identity/src/sessions.rs:176` prepares notification account state from the current login transaction |
| password notifications | `identity/src/credentials_http.rs:46` implements the configured transaction-aware password notification account adapter |
| weekly reports | `playerdata/src/reports.rs:358,380` prepares before reserving a transaction and revalidates the account under lock |
| native admissions/recovery signing | `admission/store.rs:206`, `tickets.rs:182`, `recovery.rs` calls `prepare_on`; `grant.rs:190` refreshes via `keys/trust.rs:223` `snapshot_in` |

Regression coverage: `lk_commerce_single_connection_and_sixteen_mutations`
(includes 16 synchronized login observers, with pools of both one and 16),
`lk_control_evidence_with_one_available_connection`,
`lk_sanction_authorization_with_one_available_connection`,
`lk_referral_notification_uses_transaction_connection`,
`lk_configured_notification_accounts_use_one_connection`, and
`lk_signing_cold_and_expired_cache_with_sixteen_held_transactions`.
Existing real-admission, notification, actor-to-result and authorization-race
coverage remains enabled.

## Negative controls

The new regressions were executed with the original implementations restored
from the exact base, then the candidate implementations were restored byte for
byte. All three controls failed as required:

| Test | Original result | Duration |
| --- | --- | --- |
| `lk_catalog_account_before_pointer` | PostgreSQL `40P01` deadlock | 1.11 s |
| `lk_crossed_staff_replies_finish` | PostgreSQL `40P01` deadlock | 1.14 s |
| `lk_commerce_single_connection_and_sixteen_mutations` | one-slot pool timeout | 10.66 s |

The control-evidence saturation test also timed out while the session-proof read
still used the pool; it passed after `BrowserSession::get_in` was wired in.

Lock-wait test polling explicitly calls `pg_stat_clear_snapshot()` while holding
the writer transaction. Otherwise `pg_stat_activity` may keep reporting the
snapshot taken before the competing connection starts waiting. All original
lock-order, revocation, refusal and timeout assertions remain in place.

The staff-support composition fixture supplies its host, MFA and feature settings
in an isolated child process. It exercises the production configured adapters
with one pool connection without depending on ambient deployment configuration.

The three browser tests that are ignored by default are run explicitly with the
prepared native Node, cached Chromium, and prepared Playwright. Missing Axe test
libraries (both pinned to 4.10.2) are unpacked into `/tmp/lk-browser-modules` after
verification against the registry's SHA-512 integrity values. No prepared tool,
repository dependency, lockfile or browser installation is replaced.

## Final validation

Rust 1.97.0; `cargo fmt --all --check` and locked Clippy with all targets and
`-D warnings` passed for admin, community, gameauth, identity, portal binary,
stats, store, support and web. Locked Cargo tests, including the explicit browser runs, passed for all nine
targets: **1,056 total passed, zero failures**. All three default-ignored browser
tests were run explicitly and passed (checkout 3.90 s, shop 3.88 s, search 8.35 s).
Tests used the real local PostgreSQL via `PORTAL_TEST_DATABASE_URL`. Full suites
ran serially to bound simultaneous schema-cleanup locks; concurrency barriers
inside the regression tests remained active.

| PostgreSQL regression | Duration |
| --- | --- |
| `audit05_04_recovery_waits_for_account_without_holding_handle` | 1.194 s |
| `lk_catalog_account_before_pointer` | 0.111 s |
| `lk_commerce_single_connection_and_sixteen_mutations` | 3.030 s |
| `lk_configured_notification_accounts_use_one_connection` | 0.946 s |
| `lk_control_evidence_with_one_available_connection` | 1.533 s |
| `lk_crossed_staff_replies_finish` | 0.248 s |
| `lk_referral_notification_uses_transaction_connection` | 1.557 s |
| `lk_sanction_authorization_with_one_available_connection` | 0.309 s |
| `lk_signing_cold_and_expired_cache_with_sixteen_held_transactions` | 3.834 s |
