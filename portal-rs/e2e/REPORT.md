# E2E-FIX2 qualification

The seven previously red browser flows are closed within the composition scope. Account lifecycles complete through the existing services; unavailable Game authority produces the documented refusal. This qualifies the executed cases below, not every broader product workflow.

## Changes

- Password changes use recovery’s transactional authorization revocation effects. OAuth/device families and browser sessions are revoked together with the native security-generation update. Five new PostgreSQL security regressions cover password success/refusal/input, real e-mail proof/confirm/revert, and transactional TOTP rollback/replay.
- E-mail HTTP and worker composition use the transactional proof/revocation adapter. The browser exercises real recovery proof, both mail links, confirmation, reversion and replay refusal.
- Account notification preferences and deletion request/cancel compose the existing services with session ownership, CSRF, named rate limits and PL/EN pages. Signed one-click unsubscribe is read-only on GET, bounded to its optional category, and idempotent on POST. The signing key uses the existing producer’s raw-byte contract.
- Character bazaar list, owner list and listing detail use explicit localized HTML 503/no-store refusal pages behind a flag that defaults off. The only CharacterEvidence implementation is a test fixture; the [T58 boundary](../crates/bazaar/src/game/README.md) documents unaccepted receipt, capability and signing dependencies. No listing, bid, transfer or Game evidence is invented.
- MFA-authenticated staff can open the requested queues and control page. Bootstrap staff can open console navigation; Game reads still need their catalog permissions. Non-staff and bootstrap Game-query attempts remain refused.
- Reserved/legal/help pages without publication show localized unpublished placeholders. Public cold sources use shared-shell HTML 503 pages. Missing selection filters show the existing form/refusal; journal navigation selects the latest closed UTC week. Error adaptation preserves security/protocol headers and cookies. The browser icon uses the existing brand mark.

## Rust acceptance

```text
cargo fmt --all --check: PASS
cargo clippy --locked --all-targets -- -D warnings: PASS
cargo test --locked: 1049 passed; 0 failed; 3 pre-existing ignored
cargo test --locked -- --ignored: 3 passed; 0 failed
Explicitly named PostgreSQL cases executed: 477
python3 tools/validation/legacy_token_ratchet.py: ratchet ok
```

PostgreSQL 18.6 and Rust 1.97.0 were used with PORTAL_TEST_DATABASE_URL set to the disposable test database and RUST_TEST_THREADS=1. No database test was skipped for missing configuration. The database count is auditable from terminal test-name components beginning postgres_, database_, db_, pg_ or composed_modules; additional unlabelled identity/notification fixtures also use PostgreSQL. It is a conservative named-case count. The two existing bazaar manifest-key notices are unchanged; strict Clippy passes. No new SQL query macros or migrations were added.

## Final browser acceptance — PASS

Base: `0e019da51ec3a014d3fd2419ec940d3ea04d4998`. Source delta SHA-256: `c339f143565b7b8ea0dc88b9f3ea4e0c30d48370b64d2a76961228c5143e7185` (all changed source/test/documentation files except this report and qualification.json). Started: `2026-11-06T05:16:48.703Z`. Native Node 24.22.1 and installed Chromium were used. Retries are zero.

| Project | Passed | Failed | Skipped |
| --- | ---: | ---: | ---: |
| pl-1440 | 50 | 0 | 0 |
| pl-390 | 50 | 0 | 0 |
| en-1440 | 50 | 0 | 0 |
| en-390 | 50 | 0 | 0 |
| **Total** | **200** | **0** | **0** |

All status, CSRF, ownership, keyboard, shell, overflow and serious/critical accessibility assertions remain. Exact documented 503 outcomes require no-store and appropriate HTML/no disclosure checks. Missing routes, arbitrary errors and unexpected console/script errors fail. Expected browser network notices for explicitly asserted 503 destinations are retained separately in the normalized findings. New account pages additionally run at 360 px.

The local mailbox supports implicit TLS on 465 and authenticated STARTTLS on 587 with certificate verification. E-mail-change mail waits for the existing one-minute worker schedule. Registration allows 90 seconds for real browser proof of work without reducing difficulty. Native form navigation has a 30-second budget; the complete public sweep has 240 seconds so every repaired HTML refusal receives its full checks. No retry or force-click is used.

## Executed case matrix

| Case | PL 1440 | PL 390 | EN 1440 | EN 390 |
| --- | --- | --- | --- | --- |
| overview: owner page and CSRF on every rendered mutation | PASS | PASS | PASS | PASS |
| privacy: owner page and CSRF on every rendered mutation | PASS | PASS | PASS | PASS |
| sessions: owner page and CSRF on every rendered mutation | PASS | PASS | PASS | PASS |
| passkeys: owner page and CSRF on every rendered mutation | PASS | PASS | PASS | PASS |
| email change: owner page and CSRF on every rendered mutation | PASS | PASS | PASS | PASS |
| password change: owner page and CSRF on every rendered mutation | PASS | PASS | PASS | PASS |
| sudo: owner page and CSRF on every rendered mutation | PASS | PASS | PASS | PASS |
| MFA settings: owner page and CSRF on every rendered mutation | PASS | PASS | PASS | PASS |
| character creation: owner page and CSRF on every rendered mutation | PASS | PASS | PASS | PASS |
| wallet balance: owner page and CSRF on every rendered mutation | PASS | PASS | PASS | PASS |
| premium: owner page and CSRF on every rendered mutation | PASS | PASS | PASS | PASS |
| payments: owner page and CSRF on every rendered mutation | PASS | PASS | PASS | PASS |
| store: owner page and CSRF on every rendered mutation | PASS | PASS | PASS | PASS |
| top up: owner page and CSRF on every rendered mutation | PASS | PASS | PASS | PASS |
| wallet history: owner page and CSRF on every rendered mutation | PASS | PASS | PASS | PASS |
| transfer: owner page and CSRF on every rendered mutation | PASS | PASS | PASS | PASS |
| bazaar: documented game-authority refusal | PASS | PASS | PASS | PASS |
| character listings: documented game-authority refusal | PASS | PASS | PASS | PASS |
| item listings: documented game-authority refusal | PASS | PASS | PASS | PASS |
| hunts: owner page and CSRF on every rendered mutation | PASS | PASS | PASS | PASS |
| journal reports: documented game-authority refusal | PASS | PASS | PASS | PASS |
| game login history: documented game-authority refusal | PASS | PASS | PASS | PASS |
| dashboard: documented game-authority refusal | PASS | PASS | PASS | PASS |
| hirelings: owner page and CSRF on every rendered mutation | PASS | PASS | PASS | PASS |
| tickets: owner page and CSRF on every rendered mutation | PASS | PASS | PASS | PASS |
| player reports: owner page and CSRF on every rendered mutation | PASS | PASS | PASS | PASS |
| party finder: owner page and CSRF on every rendered mutation | PASS | PASS | PASS | PASS |
| puzzles: owner page and CSRF on every rendered mutation | PASS | PASS | PASS | PASS |
| collection: owner page and CSRF on every rendered mutation | PASS | PASS | PASS | PASS |
| season pass: documented game-authority refusal | PASS | PASS | PASS | PASS |
| notification preferences: owner page and CSRF on every rendered mutation | PASS | PASS | PASS | PASS |
| data export: owner page and CSRF on every rendered mutation | PASS | PASS | PASS | PASS |
| account deletion: owner page and CSRF on every rendered mutation | PASS | PASS | PASS | PASS |
| support ticket create, reply and refusal to another account | PASS | PASS | PASS | PASS |
| character request reaches pending state without a game result | PASS | PASS | PASS | PASS |
| player report: create a private pending report | PASS | PASS | PASS | PASS |
| party post: unavailable ownership authority refuses the mutation | PASS | PASS | PASS | PASS |
| staff bootstrap, mandatory MFA, admin navigation and non-staff refusal | PASS | PASS | PASS | PASS |
| registration, mail verification, wrong password, login, logout, reset and single-use link | PASS | PASS | PASS | PASS |
| TOTP enrollment, challenge, recovery login and one-time recovery code | PASS | PASS | PASS | PASS |
| e-mail change: recovery proof, mail confirmation, revert and replay refusal | PASS | PASS | PASS | PASS |
| notification preferences persist; signed unsubscribe is one-click, bounded and idempotent | PASS | PASS | PASS | PASS |
| account deletion: password and confirmation refusal, request revokes sessions, then cancel | PASS | PASS | PASS | PASS |
| account lifecycle pages: shared shell, keyboard and accessibility at 360 pixels | PASS | PASS | PASS | PASS |
| all public header and footer links: shell, links, keyboard, accessibility and mobile | PASS | PASS | PASS | PASS |
| privacy changes persist and account export contains the owning account | PASS | PASS | PASS | PASS |
| sessions: revoke other browser sessions and refuse the revoked cookie | PASS | PASS | PASS | PASS |
| password change requires sudo, revokes sessions and accepts the new password | PASS | PASS | PASS | PASS |
| login rate limit refuses the sixth wrong password and supplies Retry-After | PASS | PASS | PASS | PASS |
| recovery key: one-time reset revokes sessions; regenerate and revoke | PASS | PASS | PASS | PASS |

## Navigation findings

| Project | Public links | Public findings | Unexpected console errors | Staff destinations | Staff findings |
| --- | ---: | ---: | ---: | ---: | ---: |
| pl-1440 | 45 | 0 | 0 | 20 | 0 |
| pl-390 | 45 | 0 | 0 | 20 | 0 |
| en-1440 | 45 | 0 | 0 | 20 | 0 |
| en-390 | 45 | 0 | 0 | 20 | 0 |

Normalized case statuses, timings and destination findings are in [qualification.json](qualification.json). Cookies, private keys, raw mail and traces are excluded. Diagnostic runs are not qualification evidence. An initial Rust run hit a private-listener port conflict with the retained browser server; after stopping its serve/worker children, the connection-budget regression passed and the entire Rust acceptance was rerun.

## Boundaries left open

- Accepted G-7 custody/signing/receipt authority and successful Game-backed bazaar completion remain unavailable; the qualified outcome is the documented refusal.
- Account deletion finalization and accounts with characters/trades/reserves or pending e-mail changes remain closed at the existing lifecycle boundary. This task qualifies request/cancel, not Game finalization.
- Staff mutations, four-eyes/Game execution and the broader editorial/player-data/product workflows outside the seven red flows are not qualified by navigation alone. Existing permissions and unavailable seams remain enforced.
- No PR, merge, deployment, live payment or Game dispatch was performed. Package integration remains with the coordinator.

## Changed files

- `portal-rs/bin/oteryn-portal/src/main.rs`
- `portal-rs/crates/admin/src/console/mod.rs`
- `portal-rs/crates/admin/src/console/players.rs`
- `portal-rs/crates/admin/src/control/controls_http.rs`
- `portal-rs/crates/admin/src/support_web.rs`
- `portal-rs/crates/bazaar/src/characters/README.md`
- `portal-rs/crates/bazaar/src/characters/http.rs`
- `portal-rs/crates/content/src/pages.rs`
- `portal-rs/crates/content/src/pages/tests.rs`
- `portal-rs/crates/identity/src/account_lifecycle_en.ftl`
- `portal-rs/crates/identity/src/account_lifecycle_http.rs`
- `portal-rs/crates/identity/src/account_lifecycle_pl.ftl`
- `portal-rs/crates/identity/src/credentials.md`
- `portal-rs/crates/identity/src/credentials.rs`
- `portal-rs/crates/identity/src/credentials_http.rs`
- `portal-rs/crates/identity/src/credentials_tests.rs`
- `portal-rs/crates/identity/src/email_change.rs`
- `portal-rs/crates/identity/src/email_change_http.rs`
- `portal-rs/crates/identity/src/email_change_mail.rs`
- `portal-rs/crates/identity/src/email_change_tests.rs`
- `portal-rs/crates/identity/src/mfa.rs`
- `portal-rs/crates/identity/src/mfa_tests.rs`
- `portal-rs/crates/identity/src/recovery_key.rs`
- `portal-rs/crates/playerdata/src/reports/http.rs`
- `portal-rs/crates/stats/src/character/http.rs`
- `portal-rs/crates/stats/src/world/http.rs`
- `portal-rs/crates/web/src/chrome.rs`
- `portal-rs/crates/web/src/favicon.rs`
- `portal-rs/crates/web/src/lib.rs`
- `portal-rs/e2e/README.md`
- `portal-rs/e2e/REPORT.md`
- `portal-rs/e2e/helpers.mjs`
- `portal-rs/e2e/local.mjs`
- `portal-rs/e2e/playwright.config.mjs`
- `portal-rs/e2e/qualification.json`
- `portal-rs/e2e/specs/account.spec.mjs`
- `portal-rs/e2e/specs/admin.spec.mjs`
- `portal-rs/e2e/specs/identity.spec.mjs`
- `portal-rs/e2e/specs/lifecycle.spec.mjs`
- `portal-rs/e2e/specs/public.spec.mjs`
- `portal-rs/templates/account/lifecycle.html`
- `portal-rs/templates/account/lifecycle/askama.toml`
- `portal-rs/templates/bazaar/en.ftl`
- `portal-rs/templates/bazaar/pl.ftl`
- `portal-rs/templates/pages/en.ftl`
- `portal-rs/templates/pages/pl.ftl`
