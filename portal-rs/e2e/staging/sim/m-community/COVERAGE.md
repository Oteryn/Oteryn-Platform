# Community, minigames and downloads staging coverage

Source reviewed at `7847cf7de91957d4317e7693d31d049b8f1c5f8e`. The browser origin was `https://staging.oteryn.com`. The base commit describes the expected contracts. The live revision and private environment values were unavailable through browser or HTTP evidence. Staging briefly returned 502 and came back with Mailpit, puzzles, collection, PvP, league, badges and character profile routes enabled. The table records the later deployment.

`{lang}` is `pl` or `en`. `{prefix}` is the empty string, `/pl`, or `/en`. Explicit locale aliases ran in the corresponding projects. Axum adds HEAD for GET routes. A GET route below lists application methods, with HEAD implicit. Browser mutation form fields include the session token where applicable.

## Route inventory

The binary mounts the community and download routers in [main.rs](../../../../bin/oteryn-portal/src/main.rs#L253), puzzles at [main.rs](../../../../bin/oteryn-portal/src/main.rs#L383), and collection, PvP, pass, league and Today at [main.rs](../../../../bin/oteryn-portal/src/main.rs#L474). Web mounts account badges and the character profile editor in [lib.rs](../../../../crates/web/src/lib.rs#L256). Web's shell sends anonymous GET requests under `account/` and for `pass` to sign-in before feature-specific checks. [chrome.rs](../../../../crates/web/src/chrome.rs#L24)

| Method | Path | Actor and prerequisites | Query or form | Source |
|---|---|---|---|---|
| GET | `{prefix}/worlds`, `/api/v1/worlds` | Public, `PORTAL_WORLDS_ENABLED`, valid freshness configuration | — | [worlds.rs](../../../../crates/community/src/worlds.rs#L36) |
| GET | `{prefix}/online` | Public, `PORTAL_ONLINE_ENABLED` | Query `world`, `sort` | [online.rs](../../../../crates/community/src/online.rs#L66) |
| GET | `/{lang}/rankings/{kind}` | Public, `PORTAL_PUBLIC_RANKINGS_ENABLED`, fresh visible projection | Query `world`, `vocation`, `page` | [rankings.rs](../../../../crates/community/src/rankings.rs#L63) |
| GET | `/{lang}/characters` | Public, `PORTAL_PUBLIC_PROFILES_ENABLED` | Query `name` | [profiles.rs](../../../../crates/community/src/profiles.rs#L60) |
| GET | `/{lang}/characters/{name}` | Public, same flag, visibility and projection | — | [profiles.rs](../../../../crates/community/src/profiles.rs#L201) |
| GET | `{prefix}/characters/{name}/skills` | Public, restricted environment, projection prerequisite, serving flag `PORTAL_G4_SKILLS_SERVING_ENABLED` and valid authority | — | [families/web.rs](../../../../crates/community/src/families/web.rs#L64) |
| GET | `{prefix}/account/characters/{name}/achievements` | Character owner, same projection prerequisites, `PORTAL_G4_ACHIEVEMENTS_SERVING_ENABLED` | — | [families/web.rs](../../../../crates/community/src/families/web.rs#L105) |
| GET | `{prefix}/account/characters/{name}/equipment` | Character owner, same projection prerequisites, `PORTAL_G4_EQUIPMENT_SERVING_ENABLED` | — | [families/web.rs](../../../../crates/community/src/families/web.rs#L105) |
| GET | `{prefix}/account/characters/{name}/outfit` | Character owner, same projection prerequisites, `PORTAL_G4_OUTFIT_SERVING_ENABLED` | — | [families/web.rs](../../../../crates/community/src/families/web.rs#L105) |
| GET | `{prefix}/guilds`, `{prefix}/guilds/{name}` | Public, `PORTAL_GUILDS_ENABLED`, restricted environment and current projection/visibility | Query `q`, `page` | [guilds_pages.rs](../../../../crates/community/src/guilds_pages.rs#L56), [guilds.rs](../../../../crates/community/src/guilds.rs#L216) |
| GET | `{prefix}/houses`, `{prefix}/houses/{key}` | Public, restricted environment, `PORTAL_PUBLIC_PROJECTIONS_ENABLED` and `PORTAL_HOUSES_SERVING_ENABLED`. Owner names subject to visibility | Query `world` | [houses.rs](../../../../crates/community/src/houses.rs#L110) |
| GET | `{prefix}/deaths`, `{prefix}/characters/{name}/deaths` | Public, restricted environment, projection prerequisite and `PORTAL_DEATHS_ENABLED`. Character visibility applies | Query `world` | [deaths/web.rs](../../../../crates/community/src/deaths/web.rs#L65), [deaths.rs](../../../../crates/community/src/deaths.rs#L235) |
| GET | `{prefix}/today/data`, `{prefix}/today/widget` | Public, restricted environment, projection prerequisite, `PORTAL_WORLD_DAILY_PUBLIC_ENABLED`, configured worlds and authority | Query `world`, `lang` | [today/http.rs](../../../../crates/community/src/today/http.rs#L132), [today.rs](../../../../crates/community/src/today.rs#L31) |
| GET | `/{lang}/party` | Public with `PORTAL_PARTY_ENABLED`. Active users may see forms | Query `world`, `purpose`, `level`, `profession` | [party.rs](../../../../crates/community/src/party.rs#L279) |
| POST | `/{lang}/party` | Active player, owns an eligible character, party flag | `_csrf`, `character`, `purpose`, `min_level`, `max_level`, `profession`, `starts_at`, `expires_at`, `note` | [party.rs](../../../../crates/community/src/party.rs#L326) |
| POST | `/{lang}/party/{id}/report` | Active player other than the posting account, party flag | `_csrf`, `reason` | [party.rs](../../../../crates/community/src/party.rs#L438) |
| POST | `/{lang}/party/{id}/remove` | Active posting account, party flag | `_csrf` | [party.rs](../../../../crates/community/src/party.rs#L455) |
| GET | `/admin/{lang}/party` | Party flag, staff host guard, active MFA session, `admin.access` and `support.reports.manage` | — | [party.rs](../../../../crates/community/src/party.rs#L238) |
| POST | `/admin/{lang}/party/{id}/moderate` | Same staff guard and permission, current reports | `_csrf`, `report_ids`, `action`, `reason` | [party.rs](../../../../crates/community/src/party.rs#L480) |
| GET | `/{lang}/account/referrals` | Signed-in player, `PORTAL_REFERRALS_ENABLED` | — | [referrals/web.rs](../../../../crates/community/src/referrals/web.rs#L132) |
| POST | `/{lang}/account/referrals` | Signed-in player with campaign eligibility, same flag | `_csrf`, `kind` (`link`, `new`, `returning`), `email` for email invitations | [referrals/web.rs](../../../../crates/community/src/referrals/web.rs#L202) |
| GET | `/{lang}/referrals/{code}` | Public, valid invitation, same flag | — | [referrals/web.rs](../../../../crates/community/src/referrals/web.rs#L247) |
| POST | `/{lang}/referrals/{code}` | Anonymous session can proceed to registration/login. A signed-in account must meet campaign eligibility, same flag | `_csrf` | [referrals/web.rs](../../../../crates/community/src/referrals/web.rs#L267) |
| GET | `{prefix}/account/badges` | Signed-in account, `OTERYN_ACCOUNT_BADGES_ENABLED` | — | [badges/http.rs](../../../../crates/characters/src/badges/http.rs#L61) |
| POST | `{prefix}/account/badges` | Signed-in account, owned visible characters and earned achievements, same flag | `_csrf`, repeatable `badge`, repeatable `achievement`, `profile`, `ranking` | [badges/http.rs](../../../../crates/characters/src/badges/http.rs#L186) |
| GET | `{prefix}/account/characters/{name}/profile` | Character owner, `OTERYN_CHARACTER_PROFILES_ENABLED` | — | [profile/http.rs](../../../../crates/characters/src/profile/http.rs#L108) |
| POST, PUT | `{prefix}/account/characters/{name}/profile` | Same flag, current character owner and profile update budget | `_csrf`, `comment`, `show_guild`, `is_main_character`, `hidden`, `hide_all_characters`, `rendered_hide_all_characters`, `show_account`, `show_status`, `show_house`, `show_skills`, `show_deaths`, `show_kills`, `show_online_time` | [profile/http.rs](../../../../crates/characters/src/profile/http.rs#L167) |
| GET | `/{lang}/puzzles`, `/{lang}/puzzles/{id}/image` | Signed-in player, `OTERYN_DAILY_PUZZLES_ENABLED`. Image requires eligible published puzzle | — | [puzzles/http.rs](../../../../crates/minigames/src/puzzles/http.rs#L24) |
| POST | `/{lang}/puzzles/{id}/answer` | Signed-in player, same flag and puzzle eligibility/window | `_csrf`, `choice` | [puzzles/http.rs](../../../../crates/minigames/src/puzzles/http.rs#L263) |
| GET | `/{lang}/collection` | Signed-in player, `OTERYN_COLLECTION_ENABLED`. Game authority needed for character list | — | [collection/http.rs](../../../../crates/minigames/src/collection/http.rs#L89) |
| GET | `/{lang}/collection/{character}`, `/{lang}/collection/{character}/cards/{card}/image` | Current character owner, same flag and Game evidence | — | [collection/http.rs](../../../../crates/minigames/src/collection/http.rs#L103) |
| POST | `/{lang}/collection/{character}/decks` | Current character owner and owned cards, same flag | `_csrf`, `name`, `first`, optional `second`, `third`, `id` | [collection/http.rs](../../../../crates/minigames/src/collection/http.rs#L439) |
| POST | `/{lang}/collection/decks/{id}/battle` | Deck owner, same flag and eligible deck | `_csrf` | [collection/http.rs](../../../../crates/minigames/src/collection/http.rs#L475) |
| GET | `/{lang}/collection/battles/{id}` | Battle owner, same flag | — | [collection/http.rs](../../../../crates/minigames/src/collection/http.rs#L111) |
| POST | `/{lang}/collection/battles/{id}` | Battle owner with current turn and valid action, same flag | `_csrf`, `expected`, `action` | [collection/http.rs](../../../../crates/minigames/src/collection/http.rs#L567) |
| POST | `/{lang}/collection/battles/{id}/forfeit` | Battle owner with active battle, same flag | `_csrf` | [collection/http.rs](../../../../crates/minigames/src/collection/http.rs#L618) |
| POST | `/{lang}/collection/decks/{id}/delete` | Deck owner, same flag | `_csrf` | [collection/http.rs](../../../../crates/minigames/src/collection/http.rs#L632) |
| GET | `/{lang}/collection-pvp` | Signed-in player, `OTERYN_COLLECTION_PVP_ENABLED` or `OTERYN_COLLECTION_SETS_ENABLED` | — | [collection_pvp/http.rs](../../../../crates/minigames/src/collection_pvp/http.rs#L19) |
| POST | `/{lang}/collection-pvp` | Signed-in actor, corresponding PvP/sets flag and operation-specific ownership/state | `_csrf`, `op`, optional `id`, `opponent`, `deck`, `revision`, `action`, `capacity`, `version`, `price`, `request` | [collection_pvp/http.rs](../../../../crates/minigames/src/collection_pvp/http.rs#L164) |
| GET | `/{lang}/league` | Public, `OTERYN_LEAGUE_ENABLED`, public policy and fresh producer watermark | — | [league/web.rs](../../../../crates/minigames/src/league/web.rs#L154) |
| GET | `/{lang}/pass` | Signed-in player, `OTERYN_SEASON_PASS_ENABLED`, account authority | — | [pass/web.rs](../../../../crates/minigames/src/pass/web.rs#L105) |
| POST | `/{lang}/pass/{season}/buy` | Signed-in player, same flag, testing Wallet adapter and `OTERYN_SEASON_PASS_WALLET_ENABLED` | `_csrf`, `operation` | [pass/web.rs](../../../../crates/minigames/src/pass/web.rs#L146) |
| POST | `/{lang}/pass/{season}/claim` | Signed-in player eligible for reward, same flag and reward adapter | `_csrf`, `operation`, `reward` | [pass/web.rs](../../../../crates/minigames/src/pass/web.rs#L276) |
| GET | `/updates/{channel}/metadata/{filename}` | Public, `OTERYN_DOWNLOADS_ENABLED`, published signed metadata | — | [downloads/http.rs](../../../../crates/downloads/src/http.rs#L14) |
| GET | `/updates/{channel}/policy-v1.json` | Public, same flag and channel publication | — | [downloads/http.rs](../../../../crates/downloads/src/http.rs#L43) |
| GET | `/updates/{channel}/targets/{*target}` | Public, same flag and published target | — | [downloads/http.rs](../../../../crates/downloads/src/http.rs#L52) |
| GET | `/download`, `/download/{slug}` | Web legacy aliases redirect to Polish localized paths | Allowlisted redirect query parameters only | [redirects.rs](../../../../crates/web/src/redirects.rs#L24) |

The source defines no download publication route or download administration form. The `downloads.manage` permission belongs to platform-admin and operations, and signed release publication is a CLI operation. Party moderation is the browser staff surface. The catalog grants its required permissions to platform-admin and support. Moderator has enforcement permissions but lacks `support.reports.manage`. [downloads README](../../../../crates/downloads/README.md), [RBAC catalog](../../../../crates/admin/src/rbac/catalog.rs#L311)

### Producer routes

All paths below start with `/internal/{version}/game-public/`. There are no HTML forms. Body contracts are the typed producer JSON envelopes in the referenced handlers. Ordinary browser, player and staff credentials confer no producer role.

| Method | Version and suffix | Authority and gates | Source |
|---|---|---|---|
| POST | v1 `highscores`, `character-profile`, `character-deaths`, `guild`, `house`, `world-online`, `watermark` | Verified game-public publisher. Restricted environment, `PORTAL_PUBLIC_PROJECTIONS_ENABLED`, valid authority | [ingest.rs](../../../../crates/community/src/ingest.rs#L31) |
| POST | v2 `character-deaths` | Verified game-public publisher. Projection prerequisites and `PORTAL_DEATHS_V2_INGEST_ENABLED` | [deaths.rs](../../../../crates/community/src/deaths.rs#L53) |
| POST | v2 `guild` | Same publisher. Projection prerequisites and `PORTAL_GUILDS_INGEST_ENABLED` | [guilds.rs](../../../../crates/community/src/guilds.rs#L32) |
| POST | v2 `house` | Same publisher. Projection prerequisites and `PORTAL_HOUSES_INGEST_ENABLED` | [houses.rs](../../../../crates/community/src/houses.rs#L71) |
| POST | v2 `world-daily` | Same publisher. Projection prerequisites, `PORTAL_WORLD_DAILY_INGEST_ENABLED`, configured worlds | [today/http.rs](../../../../crates/community/src/today/http.rs#L32) |
| POST | v2 `character-skills`, `character-achievements`, `character-equipment`, `character-outfit` | Verified producer on internal listener, projection prerequisites and corresponding `PORTAL_G4_*_INGEST_ENABLED` | [families.rs](../../../../crates/community/src/families.rs#L138), [game_routes.rs](../../../../bin/oteryn-portal/src/game_routes.rs#L411) |
| POST | v2 `watermark` | Shared dispatcher, uses the selected family's producer role and flag | [v2_watermark.rs](../../../../crates/community/src/v2_watermark.rs#L52), [game_routes.rs](../../../../bin/oteryn-portal/src/game_routes.rs#L439) |
| POST | v1 `world-event-outcome`, `guild-season`, `league-watermark` | Dedicated verified league producer, `OTERYN_LEAGUE_ENABLED`, league audience and authority | [league/admission.rs](../../../../crates/minigames/src/league/admission.rs#L8), [league/config.rs](../../../../crates/minigames/src/league/config.rs#L195) |

## Route × actor results

Each row covers the listed methods and aliases. `Visitor` means no signed-in portal account. `Fresh / owner / other` refers to three distinct player states relevant to ownership checks. `Staff` means a seeded test role. A 404 paired with an absent feature mount or handler-level enabled check is evidence of the disabled surface. An anonymous 303 from the shell establishes the sign-in redirect only.

Test abbreviations refer to full titles in [community.spec.mjs](community.spec.mjs).

| Route(s) | Actor | Expected from code | Observed | Test |
|---|---|---|---|---|
| GET worlds and worlds API, online, rankings, character search and public profile | Visitor | Disabled 404. Links omitted where registered in navigation | 404. Omitted navigation links | community navigation and unavailable public projections |
| GET guild index/detail, house index/detail, world/character deaths, public skills | Visitor | Disabled 404 | 404 | community navigation and unavailable public projections |
| GET Today data and widget | Visitor | Unavailable 503 | 503 | community navigation and unavailable public projections |
| GET party | Visitor | Disabled 404 and localized fallback | 404 after load and refresh | community navigation and unavailable public projections |
| Public community projection routes and party reads | Fresh / owner / other / staff | Same feature availability gates as for the visitor, plus projection visibility where applicable | Visitor availability results above. Signed-in coverage blocked | community navigation and unavailable public projections |
| POST party create/report/remove | Visitor | Unmounted 404 | 404 with session form token, including malformed IDs | party posting, reporting, removal and staff queue are unavailable |
| GET staff party queue. POST moderate | Visitor | Unmounted 404 | 404 | party posting, reporting, removal and staff queue are unavailable |
| Party create/report/remove and staff moderation | Fresh / owner / other / platform-admin / support / moderator | Party flag must be on before eligibility or staff permissions can be evaluated | Blocked by unmounted routes | party posting, reporting, removal and staff queue are unavailable |
| GET account referrals, badges, profile and three owner-only family tabs | Visitor | Shell 303 to localized sign-in | 303 with localized destination | account routes direct visitors to sign in |
| POST account referrals | Visitor | Unmounted 404 | 404 | account routes direct visitors to sign in |
| POST account badges. POST/PUT account profile | Visitor | Enabled handler redirects unauthenticated requests after form admission | 303 to localized sign-in | account routes direct visitors to sign in |
| GET/POST referral invitation landing | Visitor | Unmounted 404 | 404 with absent valid-shape and malformed codes | account routes direct visitors to sign in |
| Account referrals and landing, badges, profile and owner family tabs | Fresh / owner / other | Signed-in account-specific paths, including ownership/referral eligibility when enabled | Player bootstrap blocked actor-level outcomes | fresh player signup and verified community entry points |
| GET puzzles and puzzle image | Visitor | Module redirects to sign-in before reading puzzle ID | 303 with valid-shape and malformed ID | minigame sign-in, unavailable league and disabled season pass |
| POST puzzle answer | Visitor | 303 for unauthenticated submission admitted by session middleware, 403 for missing form token | 303 / 403 | minigame sign-in, unavailable league and disabled season pass |
| GET collection index, character collection, card image and battle | Visitor | Redirect to localized sign-in | 303 | minigame sign-in, unavailable league and disabled season pass |
| POST deck save/start/delete, battle turn/forfeit | Visitor | 303 for unauthenticated submission admitted by session middleware, 403 for missing form token | 303 / 403 | minigame sign-in, unavailable league and disabled season pass |
| GET/POST collection PvP | Visitor | GET redirects to sign-in. POST refuses actor with localized response | 303 / 403 | minigame sign-in, unavailable league and disabled season pass |
| GET league | Visitor | 503 and localized unavailable results when producer evidence is unavailable | 503 and unavailable results | minigame sign-in, unavailable league and disabled season pass |
| GET pass. POST buy/claim | Visitor | Shell 303 to login. Unmounted mutation routes 404 | 303 / 404 | minigame sign-in, unavailable league and disabled season pass |
| Puzzle, collection, PvP and pass routes | Fresh / owner / other | Actor and owner requirements for enabled surfaces, unavailable without game evidence as applicable | Player bootstrap blocked authenticated and ownership outcomes | fresh player signup and verified community entry points |
| GET league | Fresh / staff | Same public result and producer prerequisites | Visitor unavailable result verified. Authenticated variants unverified | minigame sign-in, unavailable league and disabled season pass |
| GET three update documents. POST same paths | Visitor | Missing/disabled 404, invalid channel 400, unsupported publication method 405 | 404, 400, 405 | downloads origin returns absent documents and rejects publication |
| GET `/download`, `/download/{slug}` and localized destinations | Visitor | 301 to Polish alias. No download-page route in these modules | 301 and localized 404 | downloads origin returns absent documents and rejects publication |
| Downloads document and publication requests | Fresh / staff | Public read rules apply to all actors. No HTTP publication operation | Guest verified. Authenticated replay blocked by account bootstrap | downloads origin returns absent documents and rejects publication |
| POST v1 community producer routes. V2 deaths, guild, house, daily and shared watermark | Visitor | Guarded and unavailable without an enabled producer configuration | 503 | community and league producer routes are unavailable to browser actors |
| POST v2 four G-4 producer payloads. V1 three league producer routes | Visitor | No active public mount for these listeners and features | 404 | community and league producer routes are unavailable to browser actors |
| POST all producer routes | Player / staff / authorized producer | Browser/staff permissions cannot substitute for producer identity | Browser refusal verified. Authenticated or producer execution outside available staging actors | community and league producer routes are unavailable to browser actors |
