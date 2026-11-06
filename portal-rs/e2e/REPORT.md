# Portal qualification — 5–6 November 2026

Qualification is incomplete and must not be treated as release readiness. This report separates successful browser paths from missing flows and documented integration boundaries. No PR, deployment, game-side mutation, real payment or real credential was used.

Baseline: fetched `main` at `bd9898c4`; then normally merged current `main` at `5c4e5eb2` (merge `743748a1`) before final qualification. The upstream merge includes shared-shell UI, staff action catalogs, recovery-key domain and import command changes. Work branch: `codex/portal-rs-e2e-night`. Runtime: PostgreSQL 18.6, Rust 1.97.0, native Node 22.16.0 and the preinstalled `/usr/bin/chromium`. Browser packages and exact run instructions are in [README.md](README.md); source-derived variables and enabled flags are in [ENVIRONMENT.md](ENVIRONMENT.md).

## Delivered fixes

1. `1eea2477` — staff routing used `PendingAuthentication` for both browser identity and authoritative account state, so real staff could never reach the panel. Startup now composes validated browser sessions, active verified account state and current account-bound MFA grants. Host checks, permission checks, CSRF and MFA remain mandatory. The admin crate regression exercises absent/foreign/revoked grants, suspension, missing verification and a real enrolled MFA grant; existing RBAC/CSRF assertions remain.
2. `665cdaf5` — startup constructed a separate PostgreSQL pool for almost every module, including presentation/session middleware. A broad all-module sweep exhausted the local database's client budget. Every module now receives a clone of one shared pool; the existing pool limit is 16 connections. The binary regression visits sixteen surfaces and counts its connections through `pg_stat_activity`: the previous code fails with 14 retained independent connections; shared composition passes the serial-sweep budget of 10. No database-wide connection limit was increased.

3. `ddfd0b60` — entity-wiki refusals returned plain text, so a browser received no document title or language. They now reuse the existing localized wiki document renderer. The owning regression checks HTML structure, both locales and unchanged 404/422/503 statuses.
4. `a08f7e43` — news combined theme tokens with a legacy body background. Under a light browser preference the link color and body background came from different themes, causing a serious contrast violation. The news document now applies its selected theme to the body and has explicit head/body structure. The owning regression checks both languages and title escaping.

5. `c11a4d34` — the upstream theme added a final input rule using a permanently dark decorative tile background, overriding the semantic surface while light-mode text stayed dark. Inputs now retain their theme-aware surface background. The existing owning search browser regression was first confirmed red, then expanded to light/dark preferences at both widths in PL/EN; all axe assertions remain intact.

6. `fffe7017` — player-report sessions never populated their origin allowlist from `OTERYN_PUBLIC_ORIGIN`. A valid browser submission therefore returned 403 despite a correct synchronizer token. The report router now trusts only the configured origin; a PostgreSQL regression proves the configured origin plus a valid token submits exactly once, while missing tokens and a foreign origin remain 403.

7. `e0b0e84c` — the overview balance shortcut targeted the nonexistent `/panel/balance` route. It now opens the existing `/shop/history` wallet surface, which includes available, transferable and reserved balances. The owning overview regression asserts the destination and absence of the stale URL; the browser checks the balance fields.

8. `87b625d9` — Chromium native report forms under `Referrer-Policy: no-referrer` sent `Origin: null`, which the CSRF gate correctly refused. These non-token report pages now use `same-origin`: report URLs remain suppressed for external destinations, and native same-origin submissions retain a usable Origin. The owning regression explicitly rejects null/foreign origins and missing tokens, asserts the header, and permits exactly one correctly bound submission. Null Origin was not allowlisted.

9. `ba3fa9f0` — the older document stylesheet gave shared shell elements intrinsic widths/margins, and a closed positioned navigation sheet could contribute off-screen layout. Guilds/calculators measured 643–662 px at a 390 px viewport; the open mobile sheet was only 49 px tall. Shell sizing now resets those document defaults, closed disclosures have no layout, the footer fills its containing width, and the open sheet uses the available viewport height. The owning web browser regression renders real shared components with the older stylesheet and checks PL/EN, light/dark preferences, closed/open states, widths and menu height. Both affected portal pages measure 390 px after the fix; no content-clipping rule was added.

10. `6814e401` — shell injection into HTML documents with omitted head/body tags placed the first stylesheet before the html opening tag closed. The token stylesheet was therefore parsed as malformed markup, causing Atlas header icons to use fallback geometry and keyboard focus styles to fail. The injector now closes the opening tag before inserting styles. The owning unit assertion and a native headless-document fixture verify loaded tokens, widths and focus in both locales and color preferences.

## Qualification boundaries and open failures

- Shared shell: the upstream UI merge now wraps module documents with the common header/footer. Final public checks target those shared components explicitly and still require no serious/critical axe violations, visible focus, intact links and no horizontal overflow.
- Editorial fixtures: a freshly migrated database has no published reserved/legal pages, while footer navigation emits their links. This produces broken public links. No legal copy was invented or published as product content.
- Credential change: `credentials_http::configured_router` still installs `UnavailableRevoker`. A successful global revocation transaction is required before password changes commit. The test expects a completed change and retains the failure; no permissive replacement was installed.
- E-mail change: `email_change_http::configured_router` still installs `DenySecurity`. Read rendering alone does not qualify request/confirmation/reversion. A transactional MFA/recovery proof and authorization revocation adapter remains necessary.
- Notifications/deletion: account notification preferences/unsubscribe and the complete deletion request/cancel browser lifecycle are not composed into the requested account UI. Domain tests do not substitute for those browser flows.
- Player data and commerce: several configured routers remain empty or use unavailable/deny authority adapters. Examples include character bazaar startup, dashboard defaults, item evidence, login-history authority, journal report authority and season pass. Enabling a flag does not provide ownership publications or game evidence.
- Character profile/services and parameterized journal/items/progress/deaths owner-versus-foreign reads need authoritative local projection fixtures and additional browser cases. They are not claimed complete.
- Staff panel: the mandatory-MFA gate is now usable, but visiting pages is not a proof of editorial CRUD/publish, player account mutations, wiki review or four-eyes execution. Pending support/control/commerce adapters remain closed. These positive workflows remain open.
- Recovery key: the upstream web crate already composes the domain/router, and `RecoveryKeys::new` selects the upstream integrated reset adapter. The suite exercises native form generation/confirmation, successful reset, password replacement, revocation of another browser session, one-time replay refusal, regeneration and revocation. No production adapter was replaced for the tests.
- Live game admissions, runtime commands, escrow, delivery and game-side completion are NOT_APPLICABLE locally because no game service or accepted live authority is available. Pending/refusal states must stay explicit. This does not turn unexecuted portal flows into PASS.

## Diagnostic attempts

The first SMTP experiment used an unsuitable self-signed server certificate and an unhandled sink error; the checked-in launcher uses a separate local CA and server certificate and handles transport errors. Node API requests needed IPv4 resolution for the loopback-only proxy. Fresh registration needed the repository's operator policy plus a synthetic local breach-range fixture; browser proof of work remains active.

Browser worker replacement initially repeated account creation and encountered real registration quotas. The suite now preserves one owner fixture per project outside the repository and waits for actual rate windows; it does not delete quota rows or change limits. The character form has a GET filter before its POST form; the test now selects the mutation form explicitly. Admin routes that reject unknown query parameters receive locale through their existing header/path mechanism.

A merged-build attempt exhausted disk while linking; obsolete test executables produced by this task were removed and the run restarted. No cache-wide purge or shared service deletion was used.

An early four-thread PostgreSQL run exhausted shared lock memory during concurrent schema creation. A subsequent combined Rust/browser run exhausted clients and produced invalid qualification evidence. Those attempts are diagnostic only. Acceptance runs use one Rust test thread and execute Rust and browser qualification sequentially. The staff-TOTP regression passed when rerun without the exhausted server; its assertions were not changed.

## Acceptance

The command summaries below and the final four-project case matrix are the acceptance record. No failed or unexecuted flow is counted as qualified.

The new browser cases also exercise a submitted private player report and a party post refused by the unavailable trusted ownership adapter. Party refusal is not a successfully published recruitment post.

## Requested flow coverage

The final case matrix below supplies the four-project result for each implemented case. PASS for a page means rendering and tokenless-POST refusal for its visible forms, not an unexecuted mutation.

| Requested flow | Status | Scope / reason |
| --- | --- | --- |
| Public header/footer pages | FAIL | Strict navigation, shell, linked-resource, focus, console, overflow and axe sweep retained; final findings below. |
| Register, verification, login/logout, wrong password | PASS | Real browser forms, proof of work and captured local mail. |
| Login rate limit | PASS | Sixth incorrect attempt returns 429 and positive Retry-After; actual windows observed. |
| Password reset | PASS | Captured mail link, changed credential and reset-token replay refusal. |
| Password change with sudo | FAIL | Required authorization revoker is unconfigured; positive browser assertion remains red. |
| E-mail change | FAIL | Page/CSRF covered; full request/confirm/revert requires the missing transactional proof/revocation adapter. |
| Sessions list/revoke | PASS | Separate browser session revoked and its cookie refused. |
| TOTP and recovery codes | PASS | Enrol, confirm, login challenge, recovery use and replay refusal. |
| Passkeys | PASS | Management page/form security only; virtual-authenticator registration/login not qualified. |
| Recovery key | PASS | Integrated reset changes the password, revokes an existing browser session, consumes the key once, refuses replay and permits replacement/revocation. |
| Privacy settings/data export | PASS | Persisted preferences and sudo-protected JSON containing the owner. |
| Deletion request/cancel | FAIL | Required browser lifecycle uncomposed. |
| Account overview | PASS | Owning account presentation. |
| Character creation | PASS | Browser request produces a PENDING operation with no game receipt; localized pending page. |
| Character profile/settings/services | FAIL | Complete owner/foreign projection fixtures and positive settings/service operations remain open. |
| Wallet balance/history, store/premium | PASS | Existing page rendering/CSRF and explicit zero balances; real payment and game delivery completion are not claimed. |
| Bazaar list/listing | FAIL | Character startup adapters uncomposed; item listing authority unavailable. |
| Journal/hunts/items/progress/deaths/logins/reports/dashboard | FAIL | Some pages render; parameterized owner/foreign lifecycle coverage incomplete; journal reports use their required closed-week filter; journal/dashboard authority returns its documented 503. |
| Support ticket | PASS | Create, reply, and foreign account 404 without ticket disclosure. |
| Player report | PASS | Synthetic submitted private report; moderation completion not claimed. |
| Party finder | PASS | Public page and exact mutation refusal with unavailable ownership authority; row count unchanged. |
| Puzzles/collection/hirelings | FAIL | Pages/CSRF covered; complete interactions not qualified. |
| Season pass | PASS | Exact documented 503/no-store/no owner disclosure from unavailable game authority. No award granted. |
| Notification preferences/unsubscribe | FAIL | Requested account UI/lifecycle not composed. |
| First staff/MFA/non-staff refusal | PASS | Repository test bootstrap SQL; enrolled and challenged MFA; pre-MFA and nonstaff 403. |
| Admin navigation | FAIL | Strict page sweep retained; pending/refusal adapters and route gaps listed below. |
| Admin editorial CRUD/publish, wiki review/world versions | FAIL | Read/CSRF sweep does not prove mutations; positive lifecycle remains open. |
| Commerce/support/sanctions/four-eyes/bazaar/controls | FAIL | Database unit/integration coverage passes, but complete positive browser lifecycle remains open. |
| Tokenless POST on touched forms | PASS | Shared helper asserts 403 before mutations and across discovered forms. |
| Game admissions, live delivery/escrow/runtime completion | NOT_APPLICABLE | No game service or accepted live authority locally; no completion fabricated. |

The final case matrix below is authoritative for executed cases. Every remaining FAIL row and every unexecuted required lifecycle prevents full acceptance.

A pre-browser binary regression caught duplicate recovery-key routes introduced during merge integration. The redundant adapter was removed in a normal corrective commit; the existing upstream web composition remains authoritative. The enabled-page and CSRF assertions remain in the binary regression.

## Rust acceptance evidence

```text
cargo build --locked -p oteryn-portal: PASS
cargo fmt --all --check: PASS
cargo clippy --locked --all-targets -- -D warnings: PASS
cargo test --locked: 1041 passed; 0 failed; 3 ignored
cargo test --locked -- --ignored: 3 passed; 0 failed; 0 ignored
Unique Rust cases executed: 1044
```

`PORTAL_TEST_DATABASE_URL` pointed to the disposable local PostgreSQL 18.6 test database and `RUST_TEST_THREADS=1` was set. The main log contains 472 successful cases whose final test-name component starts `postgres_`, `database_`, `db_`, `pg_`, or `composed_modules`; the three separately executed opt-in cases bring that explicitly named database-case count to **475**. Additional unlabelled identity, notification and contract fixtures also use PostgreSQL; 475 is the auditable naming-based count, not a claim that only 475 cases touched the database. No test was skipped for missing database configuration.

The three existing opt-in browser cases ran on installed Chromium, with no browser download: six account pages, eight shop pages, and PL/EN keyboard search at 360/1280 pixels in light/dark preferences. Each reported zero axe violations. Two pre-existing Cargo manifest-key notices remain (`base64.serde` and `base64.serde_json` in bazaar); strict Clippy itself passes. All acceptance commands and the three opt-in cases were rerun after the final production fixes.

`npm ci --ignore-scripts` from the lockfile and `npm audit`: PASS, zero reported vulnerabilities. Browser source syntax checks: PASS. No workflow files changed.

Journal reports require a closed-week timestamp (reports/README.md); the probe now supplies it. Its runtime `DenySource` returns `Unavailable`. Dashboard/README.md explicitly documents the default empty 503 when closed-session/timer producers are absent. These two cases now assert exactly 503, no-store and no owner disclosure, as required for game-dependent boundaries; no source, ownership or credential check was relaxed.

The final launcher cross-check verified all 82 literal `*_ENABLED` names found across Rust source: no missing or disabled suffix flag (`PORTAL_ATLAS_DELIVERY_ENABLED=1` and `PORTAL_WORLD_ENABLED=1`, all others `true`). Non-suffix identity/staff switches were also on. The inventory now covers 193 direct/helper configuration names, with names/locations only.

A diagnostic recovery test initially assumed the conservative adapter was the default. Source and browser readback showed that the merged upstream constructor already selects `integrated_security`; the suite was corrected to require the complete successful reset and its revocation/replay effects. This changed no production security policy.

Navigation probes use a fresh tab per destination so an error page cannot interrupt the next check. Staff discovery excludes the shared public menu, whose routes are covered separately. At 390 pixels logout opens the native mobile menu and submits its visible form, rather than attempting the hidden desktop control; CSRF is still checked first. No force-click, skipped test or retry was added.

Public non-HTML refusals remain failures. They are recorded before DOM checks, so Chromium’s internal error-page overflow/focus is not misreported as a portal layout defect. HTML refusals still undergo the shared-shell, locale, focus, overflow and axe checks.

The Atlas delivery refusal was ultimately an HTML portal page, not an internal browser error page. Its document omitted head/body tags and the shell injector inserted the first stylesheet before the opening html tag closed, swallowing the token stylesheet as malformed markup. The injector now closes html before adding style links; owning unit assertions and a real headless-document browser fixture verify the loaded tokens, width and keyboard focus. The Atlas module also requires `PORTAL_WORLD_ENABLED=1` (per its source and README); the launcher now supplies that value as well as the delivery flag’s `1`.

## Final browser acceptance — FAIL

Material source: `3180e9d48f008c0cee5eeadeb257648cde4cec1e`. The final report/checkpoint commit changes documentation/evidence only. Runtime flags were verified after startup: all 82 suffix flags enabled, including the two Atlas flags requiring `1`. This run started at `2026-11-06T03:20:15.834Z`; retries were disabled.

| Project | Passed | Failed | Skipped |
| --- | ---: | ---: | ---: |
| pl-1440 | 39 | 7 | 0 |
| pl-390 | 39 | 7 | 0 |
| en-1440 | 39 | 7 | 0 |
| en-390 | 39 | 7 | 0 |
| **Total** | **156** | **28** | **0** |

```text
184 cases: 156 passed; 28 failed; 0 skipped; 0 flaky; retries 0
Duration: 14.2 minutes
```

Every project has the same seven failed cases: bazaar, character listings, notification preferences, account deletion, admin navigation, public navigation, and positive password change. These assertions remain red. They were not skipped, retried into a pass, or reclassified as successful flows.

| Open failure | Root cause / observed outcome |
| --- | --- |
| Bazaar and character listings | Enabled character-bazaar startup remains an empty router; owning-page GETs return 404. Item-bazaar authority refusal is tested separately and is not a substitute for these pages. |
| Notification preferences/unsubscribe | Requested account browser UI/lifecycle is not composed; preferences GET returns 404. |
| Account deletion request/cancel | The domain has tested request/cancel behavior, but the requested browser lifecycle is absent; GET returns 404. |
| Positive password change | `UnavailableRevoker` remains installed; the required revocation cannot complete and the mutation returns 503. No permissive revoker was substituted. |
| Public navigation | Reserved/legal/help content is absent or routes are missing; cold game/public sources return 503, protected shortcuts refuse anonymous readers, and some navigation targets require filters absent from their URL. These remain failed navigation observations. No game evidence or legal publication was invented. |
| Staff navigation | `/admin/support/tickets`, `/admin/moderation/reports`, `/admin/moderation/enforcement`, and `/admin/control/step-up` return 503; `/console/dashboard` and `/admin/console/players` return 403 under the current inner adapters. The staff bootstrap, pre-MFA refusal, real MFA admission and non-staff refusal all completed before the navigation finding assertion failed. |

The staff sweep reached 20 paths per project and retained six failed destinations each. Public sweeps collected 45 distinct navigation URLs per project. Full normalized destination findings are in [qualification.json](qualification.json); the JSON contains no cookies, credentials, input values, raw mail or traces.

| Public project | Findings | Console errors | Serious/critical axe findings | Horizontal overflow | Invisible focus |
| --- | ---: | ---: | ---: | ---: | ---: |
| pl-1440 | 81 | 23 | 0 | 0 | 0 |
| pl-390 | 81 | 23 | 0 | 0 | 0 |
| en-1440 | 81 | 24 | 0 | 0 | 0 |
| en-390 | 81 | 23 | 0 | 0 | 0 |

The zero axe/layout/focus figures cover HTML documents that could be analyzed. Non-HTML refusals and failed navigation are explicit failures, not accessibility passes. Missing content and rejected requests still generate console errors; the public case therefore remains FAIL.

## Final executed-case matrix

PASS for a page-only case does not imply an unexecuted lifecycle. Documented game-authority refusals are named explicitly; they assert 503, no-store and no owner disclosure. Positive Game completion is NOT_APPLICABLE locally.

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
| bazaar: owner page and CSRF on every rendered mutation | FAIL | FAIL | FAIL | FAIL |
| character listings: owner page and CSRF on every rendered mutation | FAIL | FAIL | FAIL | FAIL |
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
| notification preferences: owner page and CSRF on every rendered mutation | FAIL | FAIL | FAIL | FAIL |
| data export: owner page and CSRF on every rendered mutation | PASS | PASS | PASS | PASS |
| account deletion: owner page and CSRF on every rendered mutation | FAIL | FAIL | FAIL | FAIL |
| support ticket create, reply and refusal to another account | PASS | PASS | PASS | PASS |
| character request reaches pending state without a game result | PASS | PASS | PASS | PASS |
| player report: create a private pending report | PASS | PASS | PASS | PASS |
| party post: unavailable ownership authority refuses the mutation | PASS | PASS | PASS | PASS |
| staff bootstrap, mandatory MFA, admin navigation and non-staff refusal | FAIL | FAIL | FAIL | FAIL |
| registration, mail verification, wrong password, login, logout, reset and single-use link | PASS | PASS | PASS | PASS |
| TOTP enrollment, challenge, recovery login and one-time recovery code | PASS | PASS | PASS | PASS |
| all public header and footer links: shell, links, keyboard, accessibility and mobile | FAIL | FAIL | FAIL | FAIL |
| privacy changes persist and account export contains the owning account | PASS | PASS | PASS | PASS |
| sessions: revoke other browser sessions and refuse the revoked cookie | PASS | PASS | PASS | PASS |
| password change requires sudo, revokes sessions and accepts the new password | FAIL | FAIL | FAIL | FAIL |
| login rate limit refuses the sixth wrong password and supplies Retry-After | PASS | PASS | PASS | PASS |
| recovery key: one-time reset revokes sessions; regenerate and revoke | PASS | PASS | PASS | PASS |

## Remaining qualification scope

Full e-mail-change request/confirmation/reversion, character profile/settings/services with owner-versus-foreign projections, complete journal/items/progress/deaths owner data, puzzle/collection/hireling interactions, notification unsubscribe, editorial CRUD/publish, wiki review/world versions and staff four-eyes execution remain unqualified. Read rendering or owning domain tests do not turn these into browser PASS. Their current boundaries and requested-flow statuses are listed above.

No PR or deployment was created. The branch is retained for review; acceptance is blocked, not ready for release. Jira mapping/synchronization remains unknown. Raw authentication traces and disposable runtime material are not publication artifacts.

Task serve/worker/SMTP/proxy processes were stopped after evidence extraction. The launcher removed its two random databases; the original two task databases were also dropped. Shared environment containers remain running.

Final publication checks: checkpoint validator PASS; changed-file/secret-field scan PASS; no workflow changes; `python3 tools/validation/legacy_token_ratchet.py` PASS (`ratchet ok`). Private runtime keys, authentication states and raw browser artifacts were removed after normalized evidence extraction. Native Node/package caches and recovery evidence remain for reproduction/recovery.
