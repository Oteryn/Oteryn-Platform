# Runtime environment inventory

Derived from `env::var`, `env::var_os`, and configuration-helper `lookup`/`get`/`enabled`/`value`/`flag` reads in Rust source. Names and locations only; no credentials are included. Indirect module flags found as string arguments to configuration helpers are enabled by the local launcher as well.

Required for the exercised configuration: `OTERYN_DATABASE_URL`, `OTERYN_PUBLIC_ORIGIN`, `OTERYN_ADMIN_HOST`, `OTERYN_MFA_KEY`, `OTERYN_RESET_DELIVERY_KEY`, `OTERYN_WEBAUTHN_RP_ID`, `OTERYN_SMTP_HOST`, `OTERYN_SMTP_USERNAME`, `OTERYN_SMTP_PASSWORD`, both `OTERYN_MAIL_FROM` and `OTERYN_SMTP_FROM`, `OTERYN_NOTIFY_UNSUBSCRIBE_KEY`, `OTERYN_NOTIFY_UNSUBSCRIBE_URL`, `PORTAL_ADMIN_CONTROL_AGENTS_JSON`. `OTERYN_ACTIVE_CHARACTER_LIMIT` and `OTERYN_TOTAL_CHARACTER_LIMIT` provide bounded overview policy. `OTERYN_STATIC_DIR`, `OTERYN_BIND`, `OTERYN_ENV` and `APP_ENV` select local runtime behavior. `SSL_CERT_FILE` trusts the generated local CA. The test-only database is selected with `PORTAL_TEST_DATABASE_URL`.

All values used for keys, mail credentials and accounts are disposable local values. Source authorities are local fixture labels, not evidence of a game integration. Game certificate/token/evidence configuration is intentionally absent; its enabled gates retain documented refusal behavior. An enable flag alone does not establish an authority contract.

| Variable | Source |
| --- | --- |
| `APP_ENV` | `crates/community/src/houses.rs:128`, `crates/community/src/today.rs:30`, `crates/community/src/ingest.rs:270`, `crates/community/src/navigation.rs:42`, `crates/community/src/guilds.rs:186`, `crates/community/src/deaths.rs:232`, `crates/gameauth/src/devices_http.rs:73`, `crates/stats/src/online/http.rs:124` |
| `DATABASE_URL` | `bin/oteryn-portal/src/import/mod.rs:98` |
| `GAME_AUTH_DEVICE_SESSIONS_ENABLED` | `crates/gameauth/src/devices_http.rs:72` |
| `GAME_AUTH_NATIVE_EVIDENCE_ACTIVATED` | `crates/gameauth/src/runtime.rs:660` |
| `GAME_AUTH_NATIVE_EVIDENCE_CLOCK_UNCERTAINTY_SECONDS` | `crates/gameauth/src/runtime.rs:672` |
| `GAME_AUTH_NATIVE_EVIDENCE_ENABLED` | `crates/gameauth/src/runtime.rs:658` |
| `GAME_AUTH_NATIVE_EVIDENCE_FRESH_ACCOUNT_PURPOSE` | `crates/gameauth/src/runtime.rs:665` |
| `GAME_AUTH_NATIVE_EVIDENCE_FRESH_ACCOUNT_SCOPE` | `crates/gameauth/src/runtime.rs:667` |
| `GAME_AUTH_NATIVE_EVIDENCE_FRESH_KEY_PURPOSE` | `crates/gameauth/src/runtime.rs:669` |
| `GAME_AUTH_NATIVE_EVIDENCE_HIGH_WATER_DIRECTORY` | `crates/gameauth/src/runtime.rs:661` |
| `GAME_AUTH_NATIVE_EVIDENCE_MTLS_CLIENT_IDENTITY` | `crates/gameauth/src/runtime.rs:647`, `crates/gameauth/src/scopes.rs:470` |
| `GAME_AUTH_NATIVE_EVIDENCE_SOURCE_AUTHORITY` | `crates/gameauth/src/runtime.rs:664` |
| `GAME_AUTH_NATIVE_RUNTIME_STATUS_ENABLED` | `crates/gameauth/src/runtime.rs:598` |
| `GAME_AUTH_NATIVE_RUNTIME_STATUS_IDENTITIES` | `crates/gameauth/src/runtime.rs:619` |
| `GAME_AUTH_NATIVE_SCOPE_ASSIGNMENT_ENABLED` | `crates/gameauth/src/scopes.rs:448` |
| `GAME_AUTH_NATIVE_SCOPE_ASSIGNMENT_REQUESTS_PER_MINUTE` | `crates/gameauth/src/scopes.rs:452` |
| `GATEWAY_LISTEN_ADDR` | `crates/gateway/src/config.rs:41` |
| `GATEWAY_NATIVE_ADMISSION_TIMEOUT` | `crates/gateway/src/config.rs:50` |
| `GATEWAY_NATIVE_LOGIN_ENABLED` | `crates/gateway/src/config.rs:36` |
| `GATEWAY_REQUEST_TIMEOUT` | `crates/gateway/src/config.rs:51` |
| `GATEWAY_VERSION` | `crates/gateway/src/config.rs:52` |
| `OTERYN_ACCOUNT_BADGES_ENABLED` | `crates/characters/src/badges/http.rs:79` |
| `OTERYN_ACCOUNT_BALANCE_ENABLED` | `crates/characters/src/overview/wallet.rs:37` |
| `OTERYN_ACCOUNT_CHARACTERS_AUTHORITY` | `crates/admin/src/players/projection.rs:42` |
| `OTERYN_ACCOUNT_CREDENTIALS` | `crates/identity/src/credentials_http.rs:48`, `crates/identity/src/navigation.rs:14` |
| `OTERYN_ACCOUNT_EMAIL_CHANGE` | `crates/identity/src/email_change_mail.rs:57`, `crates/identity/src/email_change_http.rs:22` |
| `OTERYN_ACCOUNT_EXPORT_ENABLED` | `crates/identity/src/export.rs:143` |
| `OTERYN_ACCOUNT_LOGIN` | `bin/oteryn-portal/src/main.rs:340`, `crates/identity/src/login_http.rs:21`, `crates/identity/src/mfa_http.rs:47` |
| `OTERYN_ACCOUNT_MFA` | `crates/identity/src/mfa_http.rs:44` |
| `OTERYN_ACCOUNT_OVERVIEW_ENABLED` | `crates/identity/src/navigation.rs:30`, `crates/web/src/lib.rs:369` |
| `OTERYN_ACCOUNT_PASSKEYS` | `crates/identity/src/passkeys_http.rs:21` |
| `OTERYN_ACCOUNT_PASSWORD_RESET` | `crates/identity/src/reset_http.rs:21`, `crates/identity/src/reset_delivery.rs:30` |
| `OTERYN_ACCOUNT_PRIVACY_ENABLED` | `crates/identity/src/privacy_web.rs:24`, `crates/identity/src/navigation.rs:6` |
| `OTERYN_ACCOUNT_RECOVERY_KEYS` | `crates/identity/src/navigation.rs:38`, `crates/web/src/lib.rs:264` |
| `OTERYN_ACCOUNT_SESSIONS` | `crates/identity/src/navigation.rs:22`, `crates/identity/src/sessions_http.rs:24`, `crates/identity/src/sessions_http.rs:353` |
| `OTERYN_ACTIVE_CHARACTER_LIMIT` | `crates/characters/src/overview/mod.rs:242` |
| `OTERYN_ADMIN_COMMERCE` | `crates/admin/src/commerce/http.rs:60` |
| `OTERYN_ADMIN_CONSOLE` | `crates/admin/src/console/mod.rs:320`, `crates/admin/src/console/access.rs:309` |
| `OTERYN_ADMIN_CONTROL` | `crates/admin/src/control/controls_http.rs:54` |
| `OTERYN_ADMIN_HOST` | `crates/admin/src/web.rs:110`, `crates/admin/src/console/access.rs:313` |
| `OTERYN_ADMIN_PANEL` | `crates/admin/src/web.rs:109`, `crates/admin/src/console/access.rs:312` |
| `OTERYN_ADMIN_PLAYERS` | `crates/admin/src/players/http.rs:18` |
| `OTERYN_ADMIN_PLAYER_CHARACTERS` | `crates/admin/src/players/http.rs:19` |
| `OTERYN_ATLAS_BESTIARY_ENABLED` | `crates/atlas/src/bestiary/web.rs:63` |
| `OTERYN_ATLAS_DIRECTORY` | `crates/atlas/src/navigation.rs:9`, `crates/atlas/src/web.rs:29` |
| `OTERYN_ATLAS_ENABLED` | `crates/atlas/src/spawns.rs:23`, `crates/atlas/src/web.rs:28` |
| `OTERYN_ATLAS_SPAWNS_ENABLED` | `crates/atlas/src/spawns.rs:22` |
| `OTERYN_BAZAAR_MODERATION` | `crates/admin/src/bazaar.rs:96`, `crates/bazaar/src/moderation_web.rs:68` |
| `OTERYN_BAZAAR_PROFILES` | `crates/bazaar/src/moderation_web.rs:69` |
| `OTERYN_BAZAAR_PROMOTIONS` | `crates/bazaar/src/moderation_web.rs:70` |
| `OTERYN_BIND` | `crates/core/src/lib.rs:79` |
| `OTERYN_CHARACTER_COMMANDS_ENABLED` | `crates/characters/src/create_web/http.rs:64` |
| `OTERYN_CHARACTER_COMMAND_CREATE_ENABLED` | `crates/characters/src/create_web/http.rs:65` |
| `OTERYN_CHARACTER_COMMAND_SOURCE_AUTHORITY` | `crates/characters/src/create_web/http.rs:66` |
| `OTERYN_CHARACTER_CREATE_WEB_ENABLED` | `crates/characters/src/navigation.rs:6`, `crates/characters/src/create_web/http.rs:62` |
| `OTERYN_CHARACTER_PROFILES_ENABLED` | `crates/characters/src/profile/http.rs:54`, `crates/characters/src/profile/http.rs:80`, `crates/web/src/lib.rs:210` |
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
| `OTERYN_DOWNLOADS_ENABLED` | `bin/oteryn-portal/src/main.rs:93` |
| `OTERYN_ENV` | `bin/oteryn-portal/src/main.rs:103`, `crates/admin/src/commerce/effects.rs:134`, `crates/admin/src/players/projection.rs:32`, `crates/characters/src/overview/mod.rs:229`, `crates/characters/src/services/browser.rs:274`, `crates/core/src/lib.rs:78`, `crates/store/src/web_payments.rs:108`, `crates/store/src/purchase.rs:75` |
| `OTERYN_JOURNAL_REPORTS_ENABLED` | `crates/playerdata/src/reports/http.rs:237` |
| `OTERYN_LCFA_ENABLED` | `crates/characters/src/overview/mod.rs:228` |
| `OTERYN_LCFA_SOURCE_AUTHORITY` | `crates/characters/src/overview/mod.rs:234` |
| `OTERYN_MAIL_FROM` | `bin/oteryn-portal/src/registration_delivery.rs:46`, `crates/identity/src/reset_delivery.rs:50`, `crates/identity/src/mfa_http.rs:563` |
| `OTERYN_MFA_KEY` | `bin/oteryn-portal/src/import/identity.rs:42`, `crates/identity/src/mfa.rs:143` |
| `OTERYN_NOTIFY_UNSUBSCRIBE_KEY` | `crates/support/src/tickets/web.rs:48` |
| `OTERYN_NOTIFY_UNSUBSCRIBE_URL` | `crates/support/src/tickets/web.rs:49` |
| `OTERYN_PAYMENTS_ENABLED` | `crates/admin/src/commerce/http.rs:63` |
| `OTERYN_PLATFORM_BASE_URL` | `crates/gateway/src/config.rs:28` |
| `OTERYN_PLATFORM_SERVICE_TOKEN_FILE` | `crates/gateway/src/config.rs:32` |
| `OTERYN_PLAYERDATA_GAME_LOGIN_HISTORY_ENABLED` | `crates/playerdata/src/logins/web.rs:27` |
| `OTERYN_PLAYERDATA_HUNTS_READ` | `crates/playerdata/src/hunts_web.rs:65` |
| `OTERYN_PLAYERDATA_PLACES_AGGREGATE` | `crates/playerdata/src/places/http.rs:20` |
| `OTERYN_PLAYERDATA_PROGRESS_READ_ENABLED` | `crates/playerdata/src/logins/web.rs:26` |
| `OTERYN_PREMIUM_ENABLED` | `crates/admin/src/commerce/http.rs:61` |
| `OTERYN_PRIVACY_VERSION` | `crates/web/src/routes/register.rs:133` |
| `OTERYN_PUBLIC_ORIGIN` | `bin/oteryn-portal/src/main.rs:101`, `bin/oteryn-portal/src/main.rs:162`, `bin/oteryn-portal/src/registration_delivery.rs:26`, `crates/atlas/src/spawns.rs:26`, `crates/community/src/party.rs:116`, `crates/identity/src/credentials_http.rs:76`, `crates/identity/src/privacy_web.rs:32`, `crates/identity/src/reset_http.rs:31`, `crates/identity/src/reset_delivery.rs:35`, `crates/identity/src/login_http.rs:23`, `crates/identity/src/mfa_http.rs:52`, `crates/identity/src/sessions_http.rs:28`, `crates/identity/src/passkeys_http.rs:24`, `crates/identity/src/email_change_http.rs:23`, `crates/store/src/web.rs:166`, `crates/support/src/tickets/web.rs:67`, `crates/support/src/reports/web.rs:17`, `crates/web/src/lib.rs:94`, `crates/web/src/routes/register.rs:137`, `crates/playerdata/src/logins/web.rs:34`, `crates/playerdata/src/places/http.rs:26`, `crates/minigames/src/puzzles/http.rs:58`, `crates/minigames/src/collection/http.rs:116`, `crates/minigames/src/collection_pvp/http.rs:25`, `crates/minigames/src/pass/web.rs:142` |
| `OTERYN_REFUNDS_ENABLED` | `crates/admin/src/commerce/http.rs:64` |
| `OTERYN_REPORTS_ENABLED` | `crates/support/src/reports/web.rs:48` |
| `OTERYN_REPORTS_PENDING_LIMIT` | `crates/support/src/navigation.rs:16`, `crates/support/src/reports/web.rs:42` |
| `OTERYN_REPORTS_UNJUSTIFIED_LIMIT` | `crates/support/src/navigation.rs:18`, `crates/support/src/reports/web.rs:44` |
| `OTERYN_RESET_DELIVERY_KEY` | `crates/identity/src/reset_http.rs:24`, `crates/identity/src/reset_delivery.rs:33` |
| `OTERYN_SEASON_PASS_ENABLED` | `crates/minigames/src/pass/web.rs:137` |
| `OTERYN_SECURITY_WIZARD_ENABLED` | `crates/identity/src/security_wizard.rs:116` |
| `OTERYN_SHOP_ENABLED` | `crates/store/src/navigation.rs:6`, `crates/store/src/web.rs:162` |
| `OTERYN_SHOP_TEST_PAYMENTS_ENABLED` | `crates/store/src/web_payments.rs:109` |
| `OTERYN_SHOP_TOP_UPS_ENABLED` | `crates/store/src/web.rs:163` |
| `OTERYN_SHOP_TOP_UP_PACKAGES` | `crates/store/src/web_payments.rs:111` |
| `OTERYN_SMTP_FROM` | `crates/identity/src/sessions_http.rs:362` |
| `OTERYN_SMTP_HOST` | `bin/oteryn-portal/src/registration_delivery.rs:23`, `crates/identity/src/reset_delivery.rs:44`, `crates/identity/src/mfa_http.rs:562`, `crates/identity/src/sessions_http.rs:359` |
| `OTERYN_SMTP_PASSWORD` | `bin/oteryn-portal/src/registration_delivery.rs:38`, `crates/identity/src/reset_delivery.rs:47`, `crates/identity/src/mfa_http.rs:590`, `crates/identity/src/sessions_http.rs:361` |
| `OTERYN_SMTP_USERNAME` | `bin/oteryn-portal/src/registration_delivery.rs:37`, `crates/identity/src/reset_delivery.rs:46`, `crates/identity/src/mfa_http.rs:589`, `crates/identity/src/sessions_http.rs:360` |
| `OTERYN_STATIC_DIR` | `bin/oteryn-portal/src/main.rs:79` |
| `OTERYN_SUPPORT_TICKETS` | `bin/oteryn-portal/src/worker.rs:227`, `crates/support/src/tickets/web.rs:45` |
| `OTERYN_TERMS_VERSION` | `crates/web/src/routes/register.rs:132` |
| `OTERYN_TOTAL_CHARACTER_LIMIT` | `crates/characters/src/overview/mod.rs:243` |
| `OTERYN_WALLET_ENABLED` | `crates/admin/src/commerce/http.rs:62` |
| `OTERYN_WEBAUTHN_RP_ID` | `crates/identity/src/passkeys_http.rs:25` |
| `OTERYN_WORLD_ADMIN` | `crates/world/src/admin.rs:32` |
| `OTERYN_WORLD_ENABLED` | `crates/atlas/src/spawns.rs:23`, `crates/atlas/src/web.rs:35`, `crates/minigames/src/puzzles/http.rs:54`, `crates/minigames/src/collection/http.rs:103` |
| `PANEL_PREMIUM_PAYMENTS` | `crates/payments/src/account.rs:54` |
| `PATH` | `bin/oteryn-portal/tests/startup.rs:39` |
| `PLATFORM_BUILD_REVISION` | `crates/store/src/premium_http.rs:35`, `crates/store/src/loyalty_http.rs:36` |
| `PLAYERDATA_G10_HISTORY_ENABLED` | `crates/playerdata/src/items/web.rs:131` |
| `PLAYERDATA_G10_ITEMS_ENABLED` | `crates/playerdata/src/items/web.rs:130` |
| `PORTAL_ACCOUNT_TERMINATION_ENABLED` | `crates/identity/src/termination.rs:61` |
| `PORTAL_ACCOUNT_TERMINATION_GRACE_DAYS` | `crates/identity/src/termination.rs:67` |
| `PORTAL_ADMIN_CONTROL_AGENTS_JSON` | `crates/admin/src/control/http.rs:523` |
| `PORTAL_ADMIN_CONTROL_ENABLED` | `crates/admin/src/control/store.rs:59` |
| `PORTAL_ATLAS_BROWSER_NODE` | `crates/atlas/src/spawns_tests.rs:407` |
| `PORTAL_ATLAS_DELIVERY_ENABLED` | `crates/atlas/src/navigation.rs:17`, `crates/atlas/src/delivery/http.rs:22` |
| `PORTAL_BADGES_RENDER_OUTPUT` | `crates/characters/src/badges/tests.rs:195` |
| `PORTAL_BESTIARY_TEST_OUTPUT` | `crates/atlas/src/bestiary/tests.rs:233` |
| `PORTAL_BROWSER_NODE` | `crates/payments/src/account_tests.rs:420`, `crates/store/src/web_tests.rs:648`, `crates/web/src/search/tests.rs:354` |
| `PORTAL_CALCULATORS_ENABLED` | `crates/wiki/src/calculators/web.rs:66` |
| `PORTAL_CHARACTER_STATS_ENABLED` | `crates/stats/src/character/http.rs:23` |
| `PORTAL_COLLECTION_HTML_DIR` | `crates/minigames/src/collection/tests.rs:482`, `crates/minigames/src/collection/tests.rs:564` |
| `PORTAL_COMMERCE_RENDER_PATH` | `crates/admin/src/commerce/tests.rs:491` |
| `PORTAL_CONSOLE_RENDER_DIR` | `crates/admin/src/console/players_tests.rs:372` |
| `PORTAL_CONTROL_HTML_DIR` | `crates/admin/src/control/controls_tests.rs:477`, `crates/admin/src/control/controls_tests.rs:692` |
| `PORTAL_DEATHS_ENABLED` | `crates/community/src/deaths.rs:268` |
| `PORTAL_DEATHS_INGEST_REQUESTS_PER_MINUTE` | `crates/community/src/deaths.rs:253` |
| `PORTAL_DEATHS_TEST_RENDER_DIR` | `crates/community/src/deaths/tests.rs:409` |
| `PORTAL_DEATHS_V2_INGEST_ENABLED` | `crates/community/src/deaths.rs:256` |
| `PORTAL_DELIVERY_PREVIEW_DIR` | `crates/atlas/src/delivery/tests.rs:371` |
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
| `PORTAL_IMPORT_CHARACTERS_ENABLED` | `bin/oteryn-portal/src/import/mod.rs:90` |
| `PORTAL_IMPORT_COMMERCE_ENABLED` | `bin/oteryn-portal/src/import/mod.rs:84` |
| `PORTAL_IMPORT_LARAVEL_KEY` | `bin/oteryn-portal/src/import/identity.rs:73` |
| `PORTAL_IMPORT_MEDIA_ROOT` | `bin/oteryn-portal/src/import/content.rs:197` |
| `PORTAL_NATIVE_ADMISSIONS_ENABLED` | `crates/gameauth/src/admission/http.rs:176` |
| `PORTAL_NATIVE_ADMISSION_BIND` | `crates/gameauth/src/admission/http.rs:195` |
| `PORTAL_NATIVE_ADMISSION_REQUESTS_PER_MINUTE` | `crates/gameauth/src/admission/http.rs:187` |
| `PORTAL_OAUTH_CONFIG` | `crates/gameauth/src/oauth.rs:31` |
| `PORTAL_ONLINE_ENABLED` | `crates/community/src/online.rs:79` |
| `PORTAL_ONLINE_STALE_SECONDS` | `crates/community/src/online.rs:81` |
| `PORTAL_PARTY_ENABLED` | `crates/community/src/party.rs:70` |
| `PORTAL_PUBLIC_PROFILES_ENABLED` | `crates/community/src/profiles.rs:52` |
| `PORTAL_PUBLIC_PROJECTIONS_AUTHORITY` | `crates/community/src/houses.rs:150`, `crates/community/src/today.rs:51`, `crates/community/src/ingest.rs:273`, `crates/community/src/guilds.rs:200`, `crates/community/src/deaths.rs:237`, `crates/stats/src/online/http.rs:148` |
| `PORTAL_PUBLIC_PROJECTIONS_ENABLED` | `crates/community/src/houses.rs:126`, `crates/community/src/today.rs:47`, `crates/community/src/ingest.rs:268`, `crates/community/src/deaths.rs:236`, `crates/stats/src/online/http.rs:122` |
| `PORTAL_PUBLIC_PROJECTIONS_SAN` | `crates/community/src/houses.rs:131`, `crates/community/src/ingest.rs:274`, `crates/community/src/guilds.rs:189`, `crates/community/src/deaths.rs:238`, `crates/community/src/today/http.rs:193`, `crates/stats/src/online/http.rs:127` |
| `PORTAL_PUBLIC_RANKINGS_ENABLED` | `crates/community/src/rankings.rs:54` |
| `PORTAL_REPORTS_RENDER_DIR` | `crates/playerdata/src/reports_tests.rs:122`, `crates/playerdata/src/reports_tests.rs:128` |
| `PORTAL_SPRITE_ASSETS_DIR` | `crates/world/src/sprites/tests.rs:320` |
| `PORTAL_STATS_ENABLED` | `bin/oteryn-portal/src/worker.rs:191` |
| `PORTAL_STATS_ONLINE_INTERVALS_ENABLED` | `crates/stats/src/online/http.rs:143` |
| `PORTAL_STATS_WATCHES_ENABLED` | `crates/stats/src/watch/http.rs:62` |
| `PORTAL_STATS_WORLD_TOTALS_ENABLED` | `crates/stats/src/online/http.rs:145` |
| `PORTAL_T46_RENDER_DIR` | `crates/characters/src/services/browser.rs:377` |
| `PORTAL_T59_RENDER_DIR` | `crates/bazaar/src/characters/tests.rs:465` |
| `PORTAL_TEST_ARTIFACT_DIR` | `crates/community/src/guilds_tests.rs:174` |
| `PORTAL_TEST_DATABASE_URL` | `bin/oteryn-portal/src/worker_tests.rs:14`, `bin/oteryn-portal/tests/connection_budget.rs:30`, `bin/oteryn-portal/src/import/tests/mod.rs:24`, `crates/admin/src/audit_tests.rs:7`, `crates/admin/src/support_tests.rs:148`, `crates/admin/src/web_tests.rs:142`, `crates/admin/src/bazaar_tests.rs:33`, `crates/admin/src/control/tests.rs:17`, `crates/admin/src/control/controls_tests.rs:22`, `crates/admin/src/control/http_tests.rs:18`, `crates/admin/src/console/dashboard_tests.rs:148`, `crates/admin/src/console/players_tests.rs:198`, `crates/admin/src/console/players_tests.rs:337`, `crates/admin/src/commerce/tests.rs:123`, `crates/admin/src/players/tests.rs:97`, `crates/admin/src/rbac/tests.rs:121`, `crates/atlas/src/spawns_tests.rs:69`, `crates/atlas/src/web_tests.rs:29`, `crates/atlas/src/bestiary/tests.rs:310`, `crates/atlas/src/delivery/tests.rs:286`, `crates/characters/src/bootstrap/tests.rs:155`, `crates/characters/src/profile/tests.rs:5`, `crates/characters/src/commands/tests.rs:88`, `crates/characters/src/create_web/tests.rs:195`, `crates/characters/src/lcfa/tests.rs:106`, `crates/characters/src/restore/tests.rs:230`, `crates/characters/src/badges/tests.rs:123`, `crates/characters/src/overview/tests.rs:47`, `crates/characters/src/services/tests.rs:47`, `crates/community/src/profiles_tests.rs:115`, `crates/community/src/houses_tests.rs:176`, `crates/community/src/referrals_tests.rs:104`, `crates/community/src/guilds_tests.rs:112`, `crates/community/tests/rankings.rs:70`, `crates/community/tests/today_contract.rs:96`, `crates/community/tests/online.rs:25`, `crates/community/tests/worlds.rs:25`, `crates/community/src/ingest/ingest_tests.rs:126`, `crates/community/src/deaths/tests.rs:224`, `crates/community/src/party/party_tests.rs:48`, `crates/content/src/media_tests.rs:45`, `crates/content/src/news_tests.rs:25`, `crates/content/src/announcements_tests.rs:62`, `crates/content/src/pages/tests.rs:23`, `crates/content/src/events/tests.rs:105`, `crates/downloads/tests/common/mod.rs:220`, `crates/gameauth/src/runtime_tests.rs:116`, `crates/gameauth/src/devices_tests.rs:71`, `crates/gameauth/src/scopes_tests.rs:104`, `crates/gameauth/src/oauth/oauth_tests.rs:16`, `crates/gameauth/src/admission/tests.rs:132`, `crates/gameauth/src/keys/tests.rs:397`, `crates/identity/src/privacy_web.rs:279`, `crates/identity/src/login_test_db.rs:10`, `crates/identity/src/recovery_key_tests.rs:19`, `crates/identity/src/passkeys_test_db.rs:10`, `crates/identity/src/security_wizard_tests.rs:113`, `crates/identity/src/sessions_test_db.rs:10`, `crates/identity/src/privacy.rs:71`, `crates/identity/src/mfa_tests.rs:426`, `crates/identity/src/reset_test_db.rs:10`, `crates/identity/src/credentials_test_db.rs:10`, `crates/identity/src/export_tests.rs:27`, `crates/identity/src/mfa_test_db.rs:10`, `crates/identity/src/email_change_test_db.rs:11`, `crates/identity/src/termination_tests.rs:11`, `crates/identity/tests/registration.rs:25`, `crates/identity/src/abuse/tests.rs:11`, `crates/notify/tests/notifications.rs:71`, `crates/payments/src/account_tests.rs:94`, `crates/payments/tests/payments.rs:35`, `crates/player-api/src/tests.rs:24`, `crates/player-api/tests/account_feed.rs:118`, `crates/store/src/purchase_tests.rs:271`, `crates/store/src/purchase_tests.rs:1319`, `crates/store/src/catalog_tests.rs:108`, `crates/store/src/premium_tests.rs:192`, `crates/store/src/codes_tests.rs:65`, `crates/store/src/loyalty_tests.rs:35`, `crates/store/src/web_tests.rs:18`, `crates/store/src/delivery_tests.rs:149`, `crates/store/src/web_payments_tests.rs:26`, `crates/support/src/sanctions/tests.rs:21`, `crates/support/src/appeals/tests.rs:5`, `crates/support/src/appeals/tests.rs:232`, `crates/support/src/tickets/tests.rs:17`, `crates/support/src/reports/tests.rs:103`, `crates/wallet/tests/postgres.rs:61`, `crates/web/src/surface.rs:578`, `crates/web/src/session_tests.rs:18`, `crates/web/tests/ratelimit.rs:21`, `crates/web/src/search/tests.rs:56`, `crates/wiki/src/entities/tests.rs:29`, `crates/wiki/src/web/tests.rs:40`, `crates/wiki/src/calculators/tests.rs:256`, `crates/world/src/admin_tests.rs:103`, `crates/world/tests/postgres.rs:59`, `crates/playerdata/src/hunts_tests.rs:126`, `crates/playerdata/src/reports_tests.rs:174`, `crates/playerdata/src/items/tests.rs:359`, `crates/playerdata/src/reminders/tests.rs:173`, `crates/playerdata/src/progress/tests.rs:59`, `crates/playerdata/src/store/tests.rs:116`, `crates/playerdata/src/logins/tests.rs:99`, `crates/playerdata/src/deaths/tests.rs:267`, `crates/playerdata/src/drops/tests.rs:171`, `crates/playerdata/src/pricing/tests.rs:188`, `crates/playerdata/src/places/tests.rs:65`, `crates/bazaar/src/search_tests.rs:165`, `crates/bazaar/src/moderation_tests.rs:16`, `crates/bazaar/src/items/tests.rs:137`, `crates/bazaar/src/characters/tests.rs:243`, `crates/bazaar/src/game/game_tests.rs:110`, `crates/bazaar/src/game/game_tests.rs:221`, `crates/bazaar/src/notify/tests.rs:82`, `crates/bazaar/tests/support/mod.rs:201`, `crates/stats/src/rankings_tests.rs:19`, `crates/stats/src/tests.rs:9`, `crates/stats/tests/world_kills.rs:59`, `crates/stats/tests/online.rs:39`, `crates/stats/src/watch/tests.rs:47`, `crates/stats/src/character/tests.rs:104`, `crates/stats/src/guilds/tests.rs:125`, `crates/stats/src/world/tests.rs:22`, `crates/minigames/src/puzzles/tests.rs:88`, `crates/minigames/src/collection/tests.rs:113`, `crates/minigames/src/collection_pvp/tests.rs:63`, `crates/minigames/src/pass/tests.rs:126`, `crates/hirelings/tests/flow.rs:95`, `crates/hirelings/tests/commands.rs:89` |
| `PORTAL_TEST_GAME_DIR` | `crates/world/tests/postgres.rs:476` |
| `PORTAL_TEST_RENDER_DIR` | `crates/admin/src/players/http.rs:288`, `crates/minigames/src/collection_pvp/tests.rs:413` |
| `PORTAL_TODAY_RENDER_PATH` | `crates/community/tests/today_contract.rs:312` |
| `PORTAL_WIKI_ENTITIES_ENABLED` | `bin/oteryn-portal/src/main.rs:275`, `crates/wiki/src/entities.rs:66` |
| `PORTAL_WIKI_PLACES_ENABLED` | `crates/wiki/src/places/mod.rs:57` |
| `PORTAL_WORLDS_ENABLED` | `crates/community/src/worlds.rs:42` |
| `PORTAL_WORLDS_FRESHNESS_SECONDS` | `crates/community/src/navigation.rs:9`, `crates/community/src/worlds.rs:37` |
| `PORTAL_WORLD_DAILY_INGEST_ENABLED` | `crates/community/src/today.rs:48` |
| `PORTAL_WORLD_DAILY_PUBLIC_ENABLED` | `crates/community/src/today.rs:50` |
| `PORTAL_WORLD_DAILY_WORLDS` | `crates/community/src/today.rs:33` |
| `PORTAL_WORLD_ENABLED` | `crates/atlas/src/delivery/http.rs:25`, `crates/wiki/src/calculators/web.rs:62` |
| `PORTAL_WORLD_STATS_ENABLED` | `crates/stats/src/world/http.rs:52` |
| `PRODUCTS_ENTITLEMENTS_LOYALTY_SNAPSHOT_ENABLED` | `crates/store/src/loyalty_http.rs:34` |
| `PRODUCTS_ENTITLEMENTS_LOYALTY_SNAPSHOT_MTLS_CLIENT_IDENTITY` | `crates/store/src/loyalty_http.rs:46` |
| `PRODUCTS_ENTITLEMENTS_LOYALTY_SNAPSHOT_REQUESTS_PER_MINUTE` | `crates/store/src/loyalty_http.rs:37` |
| `PRODUCTS_ENTITLEMENTS_PREMIUM_SNAPSHOT_ENABLED` | `crates/store/src/premium_http.rs:33` |
| `PRODUCTS_ENTITLEMENTS_PREMIUM_SNAPSHOT_MTLS_CLIENT_IDENTITY` | `crates/store/src/premium_http.rs:43` |
| `PRODUCTS_ENTITLEMENTS_STORE_DELIVERY_ENABLED` | `crates/store/src/purchase.rs:73` |
| `PRODUCTS_ENTITLEMENTS_STORE_ENABLED_UNIT_KINDS` | `crates/store/src/purchase.rs:80` |
| `PRODUCTS_ENTITLEMENTS_STORE_MTLS_CLIENT_IDENTITY` | `crates/store/src/purchase.rs:78` |
| `PRODUCTS_ENTITLEMENTS_STORE_PURCHASE_ENABLED` | `crates/store/src/purchase.rs:71` |
| `PRODUCTS_ENTITLEMENTS_STORE_SOURCE_AUTHORITY` | `crates/store/src/purchase.rs:76` |
| `UPDATE_DESIGN_GALLERY` | `crates/web/tests/design_system.rs:254` |

## Enabled local flags

The launcher enables every source suffix flag and the following non-suffix module switches. `PORTAL_ATLAS_DELIVERY_ENABLED` and `PORTAL_WORLD_ENABLED` use their source-required value `1`; other enable values are `true`. Native authority activation is not fabricated.

```text
GAME_AUTH_DEVICE_SESSIONS_ENABLED
GAME_AUTH_NATIVE_EVIDENCE_ENABLED
GAME_AUTH_NATIVE_RUNTIME_STATUS_ENABLED
GAME_AUTH_NATIVE_SCOPE_ASSIGNMENT_ENABLED
GATEWAY_NATIVE_LOGIN_ENABLED
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
OTERYN_ACCOUNT_RECOVERY_KEYS
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
PLAYERDATA_G10_HISTORY_ENABLED
PLAYERDATA_G10_ITEMS_ENABLED
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
PORTAL_IMPORT_CHARACTERS_ENABLED
PORTAL_IMPORT_COMMERCE_ENABLED
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
PRODUCTS_ENTITLEMENTS_LOYALTY_SNAPSHOT_ENABLED
PRODUCTS_ENTITLEMENTS_PREMIUM_SNAPSHOT_ENABLED
PRODUCTS_ENTITLEMENTS_STORE_DELIVERY_ENABLED
PRODUCTS_ENTITLEMENTS_STORE_PURCHASE_ENABLED
```


## League — owner decision 2026-11-06

`OTERYN_LEAGUE_ENABLED` is the sole feature flag; the launcher enables it.
The private listener and worker use the same real admission path with disposable
Game publisher authority. Keys and certificates are generated into scratch.

| Variable | Purpose |
| --- | --- |
| `OTERYN_LEAGUE_ENABLED` | Public routes, private ingress, worker, navigation |
| `OTERYN_LEAGUE_BASE_URL` | HTTPS Platform admission audience (push transport; no Game pull API) |
| `OTERYN_LEAGUE_BIND` | Dedicated private TLS listener |
| `OTERYN_LEAGUE_SOURCE_AUTHORITY` | Exact dedicated Game purpose namespace |
| `OTERYN_LEAGUE_PUBLIC_KEYS` | JSON key-id to Ed25519 public-key lowercase hex |
| `OTERYN_LEAGUE_CLIENT_CA_DER` | Trusted client CA certificate file |
| `OTERYN_LEAGUE_CERTIFICATE_DER` | Server certificate file |
| `OTERYN_LEAGUE_PRIVATE_KEY_DER` | External PKCS8 key file; never committed |
| `OTERYN_LEAGUE_PEER_CERTIFICATE_SHA256` | Pinned dedicated Game certificate digest |
| `OTERYN_LEAGUE_RECOVERY_FILE` | Operator-owned nonrollback epoch/generation file |
| `OTERYN_LEAGUE_REQUESTS_PER_MINUTE` | Aggregate publisher cap (default 120) |
| `PORTAL_E2E_LEAGUE_CONTROL` | Test-only producer control URL, never a production endpoint |
| `OTERYN_LEAGUE_POLICY` | Optional accepted_* policy JSON; defaults follow owner acceptance |
| `OTERYN_LEAGUE_MAX_ENVELOPE_BYTES` | Conservative ANL ceiling, default/max 4096; lower overrides allowed |
| `OTERYN_LEAGUE_CONNECTION_LIMIT` | Default/max 16 simultaneous private TLS connections; lower overrides allowed |
| `OTERYN_LEAGUE_BATCH_LIMIT` | Default/max 64 queued requests per worker tick; lower overrides allowed |
| `OTERYN_LEAGUE_SUBJECT_LIMIT` | Default/max 1000 subjects in a complete cut; lower overrides allowed |
