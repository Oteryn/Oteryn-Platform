# Runtime environment inventory

Derived from `env::var`, `env::var_os`, and core `lookup` reads in Rust source. Names and locations only; no credentials are included. Indirect module flags found as string arguments to configuration helpers are enabled by the local launcher as well.

Required for the exercised configuration: `OTERYN_DATABASE_URL`, `OTERYN_PUBLIC_ORIGIN`, `OTERYN_ADMIN_HOST`, `OTERYN_MFA_KEY`, `OTERYN_RESET_DELIVERY_KEY`, `OTERYN_WEBAUTHN_RP_ID`, `OTERYN_SMTP_HOST`, `OTERYN_SMTP_USERNAME`, `OTERYN_SMTP_PASSWORD`, both `OTERYN_MAIL_FROM` and `OTERYN_SMTP_FROM`, `OTERYN_NOTIFY_UNSUBSCRIBE_KEY`, `OTERYN_NOTIFY_UNSUBSCRIBE_URL`, `PORTAL_ADMIN_CONTROL_AGENTS_JSON`. `OTERYN_ACTIVE_CHARACTER_LIMIT` and `OTERYN_TOTAL_CHARACTER_LIMIT` provide bounded overview policy. `OTERYN_STATIC_DIR`, `OTERYN_BIND`, `OTERYN_ENV` and `APP_ENV` select local runtime behavior. `SSL_CERT_FILE` trusts the generated local CA. The test-only database is selected with `PORTAL_TEST_DATABASE_URL`.

All values used for keys, mail credentials and accounts are disposable local values. Source authorities are local fixture labels, not evidence of a game integration. Game certificate/token/evidence configuration is intentionally absent; its enabled gates retain documented refusal behavior. An enable flag alone does not establish an authority contract.

| Variable | Source |
| --- | --- |
| `APP_ENV` | `crates/community/src/houses.rs:128` |
| `GAME_AUTH_DEVICE_SESSIONS_ENABLED` | `crates/gameauth/src/devices_http.rs:72` |
| `GAME_AUTH_NATIVE_EVIDENCE_ACTIVATED` | `crates/gameauth/src/runtime.rs:660` |
| `GAME_AUTH_NATIVE_EVIDENCE_CLOCK_UNCERTAINTY_SECONDS` | `crates/gameauth/src/runtime.rs:672` |
| `GAME_AUTH_NATIVE_EVIDENCE_ENABLED` | `crates/gameauth/src/runtime.rs:658` |
| `GAME_AUTH_NATIVE_EVIDENCE_FRESH_ACCOUNT_PURPOSE` | `crates/gameauth/src/runtime.rs:665` |
| `GAME_AUTH_NATIVE_EVIDENCE_FRESH_ACCOUNT_SCOPE` | `crates/gameauth/src/runtime.rs:667` |
| `GAME_AUTH_NATIVE_EVIDENCE_FRESH_KEY_PURPOSE` | `crates/gameauth/src/runtime.rs:669` |
| `GAME_AUTH_NATIVE_EVIDENCE_HIGH_WATER_DIRECTORY` | `crates/gameauth/src/runtime.rs:661` |
| `GAME_AUTH_NATIVE_EVIDENCE_MTLS_CLIENT_IDENTITY` | `crates/gameauth/src/runtime.rs:647` |
| `GAME_AUTH_NATIVE_EVIDENCE_SOURCE_AUTHORITY` | `crates/gameauth/src/runtime.rs:664` |
| `GAME_AUTH_NATIVE_RUNTIME_STATUS_ENABLED` | `crates/gameauth/src/runtime.rs:598` |
| `GAME_AUTH_NATIVE_RUNTIME_STATUS_IDENTITIES` | `crates/gameauth/src/runtime.rs:619` |
| `GAME_AUTH_NATIVE_SCOPE_ASSIGNMENT_ENABLED` | `crates/gameauth/src/scopes.rs:448` |
| `GAME_AUTH_NATIVE_SCOPE_ASSIGNMENT_REQUESTS_PER_MINUTE` | `crates/gameauth/src/scopes.rs:452` |
| `OTERYN_ACCOUNT_BADGES_ENABLED` | `crates/characters/src/badges/http.rs:79` |
| `OTERYN_ACCOUNT_BALANCE_ENABLED` | `crates/characters/src/overview/wallet.rs:37` |
| `OTERYN_ACCOUNT_CHARACTERS_AUTHORITY` | `crates/admin/src/players/projection.rs:42` |
| `OTERYN_ACCOUNT_CREDENTIALS` | `crates/identity/src/credentials_http.rs:48` |
| `OTERYN_ACCOUNT_EMAIL_CHANGE` | `crates/identity/src/email_change_mail.rs:57` |
| `OTERYN_ACCOUNT_EXPORT_ENABLED` | `crates/identity/src/export.rs:143` |
| `OTERYN_ACCOUNT_LOGIN` | `bin/oteryn-portal/src/main.rs:337` |
| `OTERYN_ACCOUNT_MFA` | `crates/identity/src/mfa_http.rs:44` |
| `OTERYN_ACCOUNT_OVERVIEW_ENABLED` | `crates/identity/src/navigation.rs:30` |
| `OTERYN_ACCOUNT_PASSKEYS` | `crates/identity/src/passkeys_http.rs:21` |
| `OTERYN_ACCOUNT_PASSWORD_RESET` | `crates/identity/src/reset_http.rs:21` |
| `OTERYN_ACCOUNT_PRIVACY_ENABLED` | `crates/identity/src/privacy_web.rs:24` |
| `OTERYN_ACCOUNT_SESSIONS` | `crates/identity/src/navigation.rs:22` |
| `OTERYN_ACTIVE_CHARACTER_LIMIT` | `crates/characters/src/overview/mod.rs:242` |
| `OTERYN_ADMIN_CONSOLE` | `crates/admin/src/console/mod.rs:320` |
| `OTERYN_ADMIN_CONTROL` | `crates/admin/src/control/controls_http.rs:54` |
| `OTERYN_ADMIN_HOST` | `crates/admin/src/web.rs:110` |
| `OTERYN_ADMIN_PANEL` | `crates/admin/src/web.rs:109` |
| `OTERYN_ADMIN_PLAYERS` | `crates/admin/src/players/http.rs:18` |
| `OTERYN_ADMIN_PLAYER_CHARACTERS` | `crates/admin/src/players/http.rs:19` |
| `OTERYN_ATLAS_BESTIARY_ENABLED` | `crates/atlas/src/bestiary/web.rs:63` |
| `OTERYN_ATLAS_DIRECTORY` | `crates/atlas/src/navigation.rs:9` |
| `OTERYN_ATLAS_ENABLED` | `crates/atlas/src/web.rs:28` |
| `OTERYN_BAZAAR_MODERATION` | `crates/admin/src/bazaar.rs:96` |
| `OTERYN_BAZAAR_PROFILES` | `crates/bazaar/src/moderation_web.rs:69` |
| `OTERYN_BAZAAR_PROMOTIONS` | `crates/bazaar/src/moderation_web.rs:70` |
| `OTERYN_BIND` | `crates/core/src/lib.rs:79` |
| `OTERYN_CHARACTER_COMMAND_SOURCE_AUTHORITY` | `crates/characters/src/create_web/http.rs:66` |
| `OTERYN_CHARACTER_CREATE_WEB_ENABLED` | `crates/characters/src/navigation.rs:6` |
| `OTERYN_CHARACTER_PROFILES_ENABLED` | `crates/characters/src/profile/http.rs:54` |
| `OTERYN_CHARACTER_RENAME_PRICE` | `crates/characters/src/services/browser.rs:307` |
| `OTERYN_CHARACTER_RENAME_PRODUCT_ID` | `crates/characters/src/services/browser.rs:302` |
| `OTERYN_CHARACTER_RENAME_PRODUCT_VERSION` | `crates/characters/src/services/browser.rs:303` |
| `OTERYN_CHARACTER_SERVICES_CLOCK_UNCERTAINTY_SECONDS` | `crates/characters/src/services/browser.rs:290` |
| `OTERYN_CHARACTER_SERVICES_COMMAND_AUTHORITY` | `crates/characters/src/services/browser.rs:285` |
| `OTERYN_CHARACTER_SERVICES_PROJECTION_AUTHORITY` | `crates/characters/src/services/browser.rs:287` |
| `OTERYN_COLLECTION_ENABLED` | `crates/minigames/src/collection/http.rs:98` |
| `OTERYN_COLLECTION_PVP_ENABLED` | `crates/minigames/src/collection_pvp/http.rs:21` |
| `OTERYN_COLLECTION_SETS_ENABLED` | `crates/minigames/src/collection_pvp/http.rs:22` |
| `OTERYN_DAILY_PUZZLES_ENABLED` | `crates/minigames/src/puzzles/http.rs:49` |
| `OTERYN_DATABASE_URL` | `crates/core/src/lib.rs:85` |
| `OTERYN_DOWNLOADS_ENABLED` | `bin/oteryn-portal/src/main.rs:90` |
| `OTERYN_ENV` | `bin/oteryn-portal/src/main.rs:100` |
| `OTERYN_JOURNAL_REPORTS_ENABLED` | `crates/playerdata/src/reports/http.rs:237` |
| `OTERYN_LCFA_ENABLED` | `crates/characters/src/overview/mod.rs:228` |
| `OTERYN_LCFA_SOURCE_AUTHORITY` | `crates/characters/src/overview/mod.rs:234` |
| `OTERYN_MAIL_FROM` | `bin/oteryn-portal/src/registration_delivery.rs:46` |
| `OTERYN_MFA_KEY` | `crates/identity/src/mfa.rs:143` |
| `OTERYN_NOTIFY_UNSUBSCRIBE_KEY` | `crates/support/src/tickets/web.rs:48` |
| `OTERYN_NOTIFY_UNSUBSCRIBE_URL` | `crates/support/src/tickets/web.rs:49` |
| `OTERYN_PLAYERDATA_HUNTS_READ` | `crates/playerdata/src/hunts_web.rs:65` |
| `OTERYN_PLAYERDATA_PLACES_AGGREGATE` | `crates/playerdata/src/places/http.rs:20` |
| `OTERYN_PRIVACY_VERSION` | `crates/web/src/routes/register.rs:133` |
| `OTERYN_PUBLIC_ORIGIN` | `bin/oteryn-portal/src/main.rs:98` |
| `OTERYN_REPORTS_ENABLED` | `crates/support/src/reports/web.rs:42` |
| `OTERYN_REPORTS_PENDING_LIMIT` | `crates/support/src/navigation.rs:16` |
| `OTERYN_REPORTS_UNJUSTIFIED_LIMIT` | `crates/support/src/navigation.rs:18` |
| `OTERYN_RESET_DELIVERY_KEY` | `crates/identity/src/reset_http.rs:24` |
| `OTERYN_SEASON_PASS_ENABLED` | `crates/minigames/src/pass/web.rs:137` |
| `OTERYN_SECURITY_WIZARD_ENABLED` | `crates/identity/src/security_wizard.rs:116` |
| `OTERYN_SHOP_ENABLED` | `crates/store/src/navigation.rs:6` |
| `OTERYN_SHOP_TEST_PAYMENTS_ENABLED` | `crates/store/src/web_payments.rs:109` |
| `OTERYN_SHOP_TOP_UP_PACKAGES` | `crates/store/src/web_payments.rs:111` |
| `OTERYN_SMTP_FROM` | `crates/identity/src/sessions_http.rs:362` |
| `OTERYN_SMTP_HOST` | `bin/oteryn-portal/src/registration_delivery.rs:23` |
| `OTERYN_SMTP_PASSWORD` | `bin/oteryn-portal/src/registration_delivery.rs:38` |
| `OTERYN_SMTP_USERNAME` | `bin/oteryn-portal/src/registration_delivery.rs:37` |
| `OTERYN_STATIC_DIR` | `bin/oteryn-portal/src/main.rs:76` |
| `OTERYN_SUPPORT_TICKETS` | `bin/oteryn-portal/src/worker.rs:227` |
| `OTERYN_TERMS_VERSION` | `crates/web/src/routes/register.rs:132` |
| `OTERYN_TOTAL_CHARACTER_LIMIT` | `crates/characters/src/overview/mod.rs:243` |
| `OTERYN_WEBAUTHN_RP_ID` | `crates/identity/src/passkeys_http.rs:25` |
| `OTERYN_WORLD_ADMIN` | `crates/world/src/admin.rs:32` |
| `OTERYN_WORLD_ENABLED` | `crates/atlas/src/web.rs:35` |
| `PANEL_PREMIUM_PAYMENTS` | `crates/payments/src/account.rs:54` |
| `PATH` | `bin/oteryn-portal/tests/startup.rs:39` |
| `PORTAL_ACCOUNT_TERMINATION_ENABLED` | `crates/identity/src/termination.rs:61` |
| `PORTAL_ACCOUNT_TERMINATION_GRACE_DAYS` | `crates/identity/src/termination.rs:67` |
| `PORTAL_ADMIN_CONTROL_AGENTS_JSON` | `crates/admin/src/control/http.rs:523` |
| `PORTAL_ADMIN_CONTROL_ENABLED` | `crates/admin/src/control/store.rs:59` |
| `PORTAL_ATLAS_DELIVERY_ENABLED` | `crates/atlas/src/navigation.rs:17` |
| `PORTAL_CALCULATORS_ENABLED` | `crates/wiki/src/calculators/web.rs:66` |
| `PORTAL_CHARACTER_STATS_ENABLED` | `crates/stats/src/character/http.rs:23` |
| `PORTAL_DEATHS_ENABLED` | `crates/community/src/deaths.rs:268` |
| `PORTAL_DEATHS_INGEST_REQUESTS_PER_MINUTE` | `crates/community/src/deaths.rs:253` |
| `PORTAL_DEATHS_V2_INGEST_ENABLED` | `crates/community/src/deaths.rs:256` |
| `PORTAL_GATEWAY_PREVIOUS_TOKEN_SHA256_FILE` | `crates/gameauth/src/admission/http.rs:178` |
| `PORTAL_GATEWAY_TOKEN_SHA256_FILE` | `crates/gameauth/src/admission/http.rs:177` |
| `PORTAL_GROWTH_RANKINGS_ENABLED` | `crates/stats/src/rankings_http.rs:23` |
| `PORTAL_GUILDS_ENABLED` | `crates/community/src/guilds.rs:219` |
| `PORTAL_GUILDS_INGEST_ENABLED` | `crates/community/src/guilds.rs:206` |
| `PORTAL_GUILD_STATS_ENABLED` | `crates/stats/src/guilds/http.rs:47` |
| `PORTAL_HIRELINGS_ENABLED` | `crates/hirelings/src/web.rs:31` |
| `PORTAL_HOUSES_INGEST_ENABLED` | `crates/community/src/houses.rs:149` |
| `PORTAL_HOUSES_SERVING_ENABLED` | `crates/community/src/houses.rs:154` |
| `PORTAL_HUNT_RENDER_DIR` | `crates/playerdata/src/hunts_web.rs:435` |
| `PORTAL_NATIVE_ADMISSIONS_ENABLED` | `crates/gameauth/src/admission/http.rs:176` |
| `PORTAL_NATIVE_ADMISSION_BIND` | `crates/gameauth/src/admission/http.rs:195` |
| `PORTAL_NATIVE_ADMISSION_REQUESTS_PER_MINUTE` | `crates/gameauth/src/admission/http.rs:187` |
| `PORTAL_OAUTH_CONFIG` | `crates/gameauth/src/oauth.rs:31` |
| `PORTAL_ONLINE_ENABLED` | `crates/community/src/online.rs:79` |
| `PORTAL_ONLINE_STALE_SECONDS` | `crates/community/src/online.rs:81` |
| `PORTAL_PARTY_ENABLED` | `crates/community/src/party.rs:70` |
| `PORTAL_PUBLIC_PROFILES_ENABLED` | `crates/community/src/profiles.rs:52` |
| `PORTAL_PUBLIC_PROJECTIONS_AUTHORITY` | `crates/community/src/houses.rs:150` |
| `PORTAL_PUBLIC_PROJECTIONS_ENABLED` | `crates/community/src/houses.rs:126` |
| `PORTAL_PUBLIC_PROJECTIONS_SAN` | `crates/community/src/houses.rs:131` |
| `PORTAL_PUBLIC_RANKINGS_ENABLED` | `crates/community/src/rankings.rs:54` |
| `PORTAL_STATS_ENABLED` | `bin/oteryn-portal/src/worker.rs:191` |
| `PORTAL_STATS_ONLINE_INTERVALS_ENABLED` | `crates/stats/src/online/http.rs:143` |
| `PORTAL_STATS_WATCHES_ENABLED` | `crates/stats/src/watch/http.rs:62` |
| `PORTAL_STATS_WORLD_TOTALS_ENABLED` | `crates/stats/src/online/http.rs:145` |
| `PORTAL_T46_RENDER_DIR` | `crates/characters/src/services/browser.rs:377` |
| `PORTAL_TEST_GAME_DIR` | `crates/world/tests/postgres.rs:476` |
| `PORTAL_TEST_RENDER_DIR` | `crates/admin/src/players/http.rs:288` |
| `PORTAL_TODAY_RENDER_PATH` | `crates/community/tests/today_contract.rs:312` |
| `PORTAL_WIKI_ENTITIES_ENABLED` | `bin/oteryn-portal/src/main.rs:272` |
| `PORTAL_WIKI_PLACES_ENABLED` | `crates/wiki/src/places/mod.rs:57` |
| `PORTAL_WORLDS_ENABLED` | `crates/community/src/worlds.rs:42` |
| `PORTAL_WORLDS_FRESHNESS_SECONDS` | `crates/community/src/navigation.rs:9` |
| `PORTAL_WORLD_DAILY_INGEST_ENABLED` | `crates/community/src/today.rs:48` |
| `PORTAL_WORLD_DAILY_PUBLIC_ENABLED` | `crates/community/src/today.rs:50` |
| `PORTAL_WORLD_DAILY_WORLDS` | `crates/community/src/today.rs:33` |
| `PORTAL_WORLD_ENABLED` | `crates/atlas/src/delivery/http.rs:25` |
| `PORTAL_WORLD_STATS_ENABLED` | `crates/stats/src/world/http.rs:52` |
| `UPDATE_DESIGN_GALLERY` | `crates/web/tests/design_system.rs:254` |

## Enabled local flags

The final launcher set the following names to `true` (`PORTAL_ATLAS_DELIVERY_ENABLED` uses its code-defined value `1`). These are configuration values, not claims that external game dependencies exist.

```text
GAME_AUTH_DEVICE_SESSIONS_ENABLED
GAME_AUTH_NATIVE_EVIDENCE_ENABLED
GAME_AUTH_NATIVE_RUNTIME_STATUS_ENABLED
GAME_AUTH_NATIVE_SCOPE_ASSIGNMENT_ENABLED
OTERYN_ACCOUNT_BADGES_ENABLED
OTERYN_ACCOUNT_BALANCE_ENABLED
OTERYN_ACCOUNT_CREDENTIALS
OTERYN_ACCOUNT_EMAIL_CHANGE
OTERYN_ACCOUNT_EXPORT_ENABLED
OTERYN_ACCOUNT_LOGIN
OTERYN_ACCOUNT_MFA
OTERYN_ACCOUNT_OVERVIEW_ENABLED
OTERYN_ACCOUNT_PASSKEYS
OTERYN_ACCOUNT_PASSWORD_RESET
OTERYN_ACCOUNT_PRIVACY_ENABLED
OTERYN_ACCOUNT_SESSIONS
OTERYN_ADMIN_COMMERCE
OTERYN_ADMIN_CONSOLE
OTERYN_ADMIN_CONTROL
OTERYN_ADMIN_PANEL
OTERYN_ADMIN_PLAYERS
OTERYN_ADMIN_PLAYER_CHARACTERS
OTERYN_ATLAS_BESTIARY_ENABLED
OTERYN_ATLAS_ENABLED
OTERYN_ATLAS_SPAWNS_ENABLED
OTERYN_BAZAAR_MODERATION
OTERYN_BAZAAR_PROFILES
OTERYN_BAZAAR_PROMOTIONS
OTERYN_CHARACTER_COMMANDS_ENABLED
OTERYN_CHARACTER_COMMAND_CREATE_ENABLED
OTERYN_CHARACTER_CREATE_WEB_ENABLED
OTERYN_CHARACTER_PROFILES_ENABLED
OTERYN_CHARACTER_SERVICES_CANCEL_ENABLED
OTERYN_CHARACTER_SERVICES_ENABLED
OTERYN_CHARACTER_SERVICES_EXCHANGE_ENABLED
OTERYN_CHARACTER_SERVICES_PROJECTION_ENABLED
OTERYN_CHARACTER_SERVICES_RENAME_ENABLED
OTERYN_CHARACTER_SERVICES_SCHEDULE_ENABLED
OTERYN_COLLECTION_ENABLED
OTERYN_COLLECTION_PVP_ENABLED
OTERYN_COLLECTION_SETS_ENABLED
OTERYN_DAILY_PUZZLES_ENABLED
OTERYN_DOWNLOADS_ENABLED
OTERYN_JOURNAL_REPORTS_ENABLED
OTERYN_LCFA_ENABLED
OTERYN_PAYMENTS_ENABLED
OTERYN_PLAYERDATA_GAME_LOGIN_HISTORY_ENABLED
OTERYN_PLAYERDATA_HUNTS_READ
OTERYN_PLAYERDATA_PLACES_AGGREGATE
OTERYN_PLAYERDATA_PROGRESS_READ_ENABLED
OTERYN_PREMIUM_ENABLED
OTERYN_REFUNDS_ENABLED
OTERYN_REPORTS_ENABLED
OTERYN_SEASON_PASS_ENABLED
OTERYN_SECURITY_WIZARD_ENABLED
OTERYN_SHOP_ENABLED
OTERYN_SHOP_TEST_PAYMENTS_ENABLED
OTERYN_SHOP_TOP_UPS_ENABLED
OTERYN_SUPPORT_TICKETS
OTERYN_WALLET_ENABLED
OTERYN_WORLD_ADMIN
OTERYN_WORLD_ENABLED
PANEL_PREMIUM_PAYMENTS
PORTAL_ACCOUNT_TERMINATION_ENABLED
PORTAL_ADMIN_CONTROL_ENABLED
PORTAL_ATLAS_DELIVERY_ENABLED
PORTAL_CALCULATORS_ENABLED
PORTAL_CHARACTER_STATS_ENABLED
PORTAL_DEATHS_ENABLED
PORTAL_DEATHS_V2_INGEST_ENABLED
PORTAL_GROWTH_RANKINGS_ENABLED
PORTAL_GUILDS_ENABLED
PORTAL_GUILDS_INGEST_ENABLED
PORTAL_GUILD_STATS_ENABLED
PORTAL_HIRELINGS_ENABLED
PORTAL_HOUSES_INGEST_ENABLED
PORTAL_HOUSES_SERVING_ENABLED
PORTAL_NATIVE_ADMISSIONS_ENABLED
PORTAL_ONLINE_ENABLED
PORTAL_PARTY_ENABLED
PORTAL_PUBLIC_PROFILES_ENABLED
PORTAL_PUBLIC_PROJECTIONS_ENABLED
PORTAL_PUBLIC_RANKINGS_ENABLED
PORTAL_STATS_ENABLED
PORTAL_STATS_ONLINE_INTERVALS_ENABLED
PORTAL_STATS_WATCHES_ENABLED
PORTAL_STATS_WORLD_TOTALS_ENABLED
PORTAL_WIKI_ENTITIES_ENABLED
PORTAL_WIKI_PLACES_ENABLED
PORTAL_WORLDS_ENABLED
PORTAL_WORLD_DAILY_INGEST_ENABLED
PORTAL_WORLD_DAILY_PUBLIC_ENABLED
PORTAL_WORLD_ENABLED
PORTAL_WORLD_STATS_ENABLED
```
