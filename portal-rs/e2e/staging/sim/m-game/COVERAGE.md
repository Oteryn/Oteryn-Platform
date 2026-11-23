# Staging game module coverage

The source inventory uses origin/main at `7847cf7de91957d4317e7693d31d049b8f1c5f8e`. The public target is `https://staging.oteryn.com`. The deployment's release SHA was unavailable through the exercised routes.

The platform wires character overview, badges, profiles, services and creation in [web/src/lib.rs](../../../../crates/web/src/lib.rs#L476). The portal binary merges the public world, atlas, statistics and staff routers, plus player data surfaces and the language shell. The signed-out shell runs before the account-page handlers and redirects GET requests even when the module behind the page is disabled. [Portal assembly](../../../../bin/oteryn-portal/src/main.rs#L234) · [Player data mounts](../../../../bin/oteryn-portal/src/main.rs#L376) · [Signed-out navigation](../../../../crates/web/src/chrome.rs#L23)

`{lang}` is `pl` or `en`. `[/{lang}]` includes the unprefixed path. Fields in GET rows are query inputs. Fields in POST and DELETE rows are bodies unless listed in the path. The table groups methods and language aliases with a shared outcome.

## Characters and player data

| Method and route | Actor and enablement | Input |
| --- | --- | --- |
| GET `[/{lang}]/account` | Signed-in account; overview flag and configured character projection | None |
| GET, POST `[/{lang}]/account/badges` | Signed-in account; `OTERYN_ACCOUNT_BADGES_ENABLED` | POST `_csrf`, repeated `badge` and `achievement`, `profile`, `ranking` |
| GET, POST `[/{lang}]/account/characters/{name}/profile` | Character owner; `OTERYN_CHARACTER_PROFILES_ENABLED` | POST `_csrf`, `comment`, `show_guild`, `is_main_character`, `hidden`, `hide_all_characters`, `rendered_hide_all_characters`, `show_account`, `show_status`, `show_house`, `show_skills`, `show_deaths`, `show_kills`, `show_online_time` |
| GET, POST `/{lang}/account/characters/{character}/services` | Character owner; services, per-action and projection/exchange flags. POST also checks recent verification where applicable | `_csrf`, `request_id`, `action`, `confirm`; cancel adds `deletion_operation_id`; rename adds `requested_name`, `accepted_price` |
| GET, POST `/{lang}/account/characters/new` | Signed-in eligible player; `OTERYN_CHARACTER_CREATE_WEB_ENABLED`, `OTERYN_CHARACTER_COMMANDS_ENABLED`, `OTERYN_CHARACTER_COMMAND_CREATE_ENABLED` | GET `pvp`, `location`; POST `_csrf`, `name`, `world`, `submission`, `sex`, `vocation` |
| GET `/{lang}/account/characters/requests/{operation}`, `/{lang}/account/characters/suggest` | Account that submitted the request, or eligible signed-in player for a suggestion; creation flags | Operation ID in path; suggestion takes no form |
| GET `[/{lang}]/account/characters/{name}/{achievements,equipment,outfit}` | Owner; public projections and the matching private family serving flag | None |
| GET `/{lang}/account/dashboard` | Owner context; runtime uses disabled flags and unavailable source | None |
| GET `[/{lang}]/account/game-logins` | Owner; `OTERYN_PLAYERDATA_PROGRESS_READ_ENABLED`, `OTERYN_PLAYERDATA_GAME_LOGIN_HISTORY_ENABLED`, current login projection authority | None |
| GET `/{lang}/panel/hirelings` | Active, verified player; `PORTAL_HIRELINGS_ENABLED` | None |
| GET `/{lang}/account/hunts`, `/{lang}/account/hunts/{id}`, `/{lang}/account/hunts/compare` | Owner; `OTERYN_PLAYERDATA_HUNTS_READ` | Hunt ID in path; comparison query `ids` |
| GET, POST `/{lang}/account/hunts/{id}/share` | Owner; the public runtime always responds unavailable | Hunt ID in path |
| GET `/static/playerdata/hunts.css` | Public asset; hunts read flag | None |
| GET `/{lang}/panel/items/{world}/{character}` | Character owner; `PLAYERDATA_G10_ITEMS_ENABLED` | World and character in path |
| GET `/{lang}/panel/history/{world}/{family}`, `/{lang}/panel/history/{world}/{family}/{character}` | Owner; `PLAYERDATA_G10_HISTORY_ENABLED` | World, family and optional character in path |
| GET `/{lang}/account/journal/reports` | Account owner; `OTERYN_JOURNAL_REPORTS_ENABLED` | `week` |
| GET, POST `/{lang}/account/journal/reports/notes` | Account owner; journal reports flag | POST `_csrf`, `body` |
| GET `/{lang}/account/journal/reports/notes/export` | Account owner; journal reports flag | None |
| POST `/{lang}/account/journal/reports/email` | Account owner; journal reports flag | `_csrf`, optional `enabled=true` |
| GET `/{lang}/account/journal/{character}/deaths` | Trusted owner context; library router with no browser mount in portal assembly | Character ID in path |

[Character creation](../../../../crates/characters/src/create_web/http.rs#L57) · [Profile form](../../../../crates/characters/src/profile/http.rs#L162) · [Badges form](../../../../crates/characters/src/badges/http.rs#L186) · [Service forms](../../../../templates/services/manage.html) · [Character families](../../../../crates/community/src/families/web.rs#L62) · [Dashboard](../../../../crates/playerdata/src/dashboard/mod.rs#L212) · [Game logins](../../../../crates/playerdata/src/logins/web.rs#L20) · [Hirelings](../../../../crates/hirelings/src/web.rs#L15) · [Hunts](../../../../crates/playerdata/src/hunts_web.rs#L32) · [Items and history](../../../../crates/playerdata/src/items/web.rs#L100) · [Reports](../../../../crates/playerdata/src/reports/http.rs#L223) · [Death detail router](../../../../crates/playerdata/src/deaths/http.rs#L121)

## Public world, statistics and atlas

| Method and route | Actor and enablement | Input |
| --- | --- | --- |
| GET `[/{lang}]/worlds`, `/api/v1/worlds` | Public; `PORTAL_WORLDS_ENABLED` and nonzero freshness window | None |
| GET `[/{lang}]/online` | Public; `PORTAL_ONLINE_ENABLED` | `world`, `sort` |
| GET `/{lang}/characters`, `/{lang}/characters/{name}` | Public; `PORTAL_PUBLIC_PROFILES_ENABLED`; profile privacy applies | Search `name` or path `name` |
| GET `[/{lang}]/characters/{name}/skills` | Public; public projections and `PORTAL_G4_SKILLS_SERVING_ENABLED`; profile privacy applies | Name in path |
| GET `/{lang}/rankings/{kind}` | Public; `PORTAL_PUBLIC_RANKINGS_ENABLED` | `world`, `vocation`, `page`; level, experience, magic, fist, club, sword, axe, distance, shielding, fishing |
| GET `/{lang}/stats/rankings/{kind}` | Public; `PORTAL_GROWTH_RANKINGS_ENABLED` | `world`, `vocation`, `pvp`, `page`; yesterday, week, month, record, oldest, anniversaries, name-changes, transfers, sold |
| GET `[/{lang}]/stats/world` | Public; `PORTAL_WORLD_STATS_ENABLED` | `world`, `start`, `end` |
| GET `[/{lang}]/stats/guilds` | Public; `PORTAL_GUILD_STATS_ENABLED` | `guild`, `compare`, `days` |
| GET `/{lang}/stats/characters/{id}/{tab}` | Public; `PORTAL_CHARACTER_STATS_ENABLED` and public projection policy | `start`, `end`, `target`; history, experience, rankings, deaths, progress, online |
| GET `/{lang}/stats/compare` | Public; character stats flag and public projection policy | `start`, `end`, `ids`, `metric` |
| GET, POST, DELETE `/api/v1/stats/watches` | Bearer with independent watch authority; `PORTAL_STATS_WATCHES_ENABLED`; configured authority denies | POST/DELETE JSON `target_kind`, `target_id`, `event_kind` |
| GET `/{lang}/atlas`, `/atlas/tiles/{version}/{floor}/{zoom}/{x}/{y}` | Public; `OTERYN_ATLAS_ENABLED`, configured atlas directory and ready world projection | Map `x`, `y`, `z`, `zoom`; tile coordinates and file suffix in path |
| GET `/{lang}/atlas/delivery`, `/atlas/delivery.css` | Public; delivery flag and world projection. CSS only mounts when enabled | Page filters `item`, `creature`, `q`, `level`, `floor`, `city`, `rarity`, `count`, `tab` |
| GET `/atlas/spawns-text/{lang}`, `/atlas/spawns` | Public; `OTERYN_ATLAS_SPAWNS_ENABLED`; spawn data requires ready world source | Viewport `version`, `floor`, `min_x`, `min_y`, `max_x`, `max_y` |
| GET, POST `/atlas/private-markers`, DELETE `/atlas/private-markers/{id}` | Active account; spawns flag; owner-scoped markers | POST JSON `x`, `y`, `floor`, `text`; delete ID in path |
| POST `/atlas/map-reports` | Active player; spawns flag and report limits | JSON `x`, `y`, `floor`, `text` |
| GET `/{lang}/atlas/bestiary/{character}` | Character owner; `OTERYN_ATLAS_BESTIARY_ENABLED` and bestiary projection | `class`, `difficulty`, `stars` |
| GET `/{lang}/atlas/places`, GET and POST `/{lang}/atlas/places/participation` | Signed-in account; `OTERYN_PLAYERDATA_PLACES_AGGREGATE` | POST `_csrf`, `participating` |
| GET `/servers`, `/characters`, `/characters/{name}`, `/highscores` | Public legacy aliases to Polish routes | Only recognized migration query keys survive |

[Worlds](../../../../crates/community/src/worlds.rs#L36) · [Online](../../../../crates/community/src/online.rs#L72) · [Profiles](../../../../crates/community/src/profiles.rs#L48) · [Highscores](../../../../crates/community/src/rankings.rs#L63) · [Growth](../../../../crates/stats/src/rankings_http.rs#L20) · [World stats](../../../../crates/stats/src/world/http.rs#L31) · [Guild stats](../../../../crates/stats/src/guilds/http.rs#L19) · [Character stats](../../../../crates/stats/src/character/http.rs#L20) · [Watches](../../../../crates/stats/src/watch/http.rs#L12) · [Atlas](../../../../crates/atlas/src/web.rs#L27) · [Delivery](../../../../crates/atlas/src/delivery/http.rs#L19) · [Spawns](../../../../crates/atlas/src/spawns.rs#L18) · [Bestiary](../../../../crates/atlas/src/bestiary/web.rs#L60) · [Places](../../../../crates/playerdata/src/places/http.rs#L16) · [Aliases](../../../../crates/web/src/redirects.rs#L8)

## Staff game tools

Staff routes require the configured administration host, MFA, `admin.access` and the route permission. When the panel or module is disabled or the host differs, 404 is expected. Game console pages render unavailable when the permitted role can open the page but an agent is absent. The concern for signed-in staff is whether each action can be offered or dispatched, which remains unobserved here.

| Method and route | Additional permission and enablement | Input or dispatch |
| --- | --- | --- |
| GET `/admin` | `OTERYN_ADMIN_PANEL` | Staff navigation |
| GET, POST `/admin/world` | `OTERYN_WORLD_ADMIN`; catalogue access and snapshot view, plus activate/profile/translation permission per action | GET version selector; POST `_csrf`, `action`, `version`, `kind`, `key`, `allowed`, `pl`, `en`; publish, visibility, names |
| GET, POST `/console/dashboard` | `OTERYN_ADMIN_CONSOLE`; `game.ops.read` | `ops.overview.get` |
| GET, POST `/console/worlds` | Console flag; `game.world.read` | `world.list`, `world.get` |
| GET, POST `/console/nodes` | Console flag; `game.node.read` | `node.list`, `node.get` |
| GET, POST `/console/database` | Console flag; `game.db.read` | `db.status`, `db.migrations.list` |
| GET, POST `/admin/console/players` | Console flag; `game.player.read` for search/detail; sanction read for the sanction panel | GET `lang`, `q`, `character`; POST `_csrf`, `q`, `lang` |
| GET `/admin/console/audit` | Console flag; view sections use portal audit and Game audit permissions | `lang`, `request_id`, `admin`, `operation`, `target` |
| GET `/admin/console/access`, `/admin/console/approvals` | Console flag; manage roles; Game sections use Game permissions | `lang`; policy lookup and pending approvals are reads |
| GET `/admin/players`, GET and POST `/admin/players/{id}` | `OTERYN_ADMIN_PLAYERS`; respective read/action permission and player-character flag for Game data | GET `q`; POST `_csrf`, `action`, `body`, `reason`, `evidence`, `request`, `hold`, `ticket`; note, logout, kick, release hold, verify identity, request reset, approve reset |
| GET, POST `/admin/control/step-up` | `OTERYN_ADMIN_CONTROL`; verified staff session | POST `_csrf`, `code` |
| GET, POST `/admin/control/approvals/{id}` | Admin control flag; operation permission and qualifying approval context | POST `_csrf` and approval ID in path |
| GET, POST `/admin/{lang}/bazaar` | `OTERYN_BAZAAR_MODERATION`; manage marketplace | GET pending cursor; POST `_csrf`, `action`, `id`, `listing`, `reason`, `coin_bps`, `gold_bps`, `listing_fee`, `promotion_price`, `world`, `family`, `production_key`, `revision_ref`, `minimum_value`, `accepted` |

The four dashboard forms share `operation`, `agent`, `target`, `lang`, and `_csrf`. Their GET filters are `agent`, `target`, `request`, and `lang`. The RBAC catalogue grants those four read permissions to game master senior, game admin, game operator and game support. Game security has Game audit and sanction read access, but lacks those four read permissions. Game support and game master senior have Game player read and moderate permissions. The named Game roles have no catalogue administration, role management or marketplace management grants in the built-in catalogue. The Game operation registry defines permissions beyond the mounted browser forms.

[Staff guard](../../../../crates/admin/src/web.rs#L203) · [Staff role catalogue](../../../../crates/admin/src/rbac/catalog.rs#L397) · [World staff form](../../../../crates/world/src/admin.rs#L282) · [Console routes](../../../../crates/admin/src/console/mod.rs#L388) · [Console metadata](../../../../crates/admin/src/console/access.rs#L425) · [Player staff router](../../../../crates/admin/src/players/http.rs#L63) · [Approval controls](../../../../crates/admin/src/control/controls_http.rs#L37) · [Bazaar staff router](../../../../crates/admin/src/bazaar.rs#L138)

## Game service routes

These router entries require configured Game service identity and authority, feature enablement and the appropriate internal ingress. Browser players and browser staff are outside their caller model. The QA environment had no Game identity or connected agent, so these contracts are inventoried without an execution result. Wire schemas are defined at their respective router sources.

| Method and path | Authority or mount |
| --- | --- |
| POST `/internal/v1/game-auth/character-commands/{pending,read,receipt}` | Character command exchange |
| POST `/internal/v2/game-auth/character-commands/{pending,read,receipt}` | Character services command exchange |
| POST `/internal/v2/game-auth/character-services/{state,watermark}` | Character services projection |
| POST `/internal/v1/game-auth/character-bootstrap-intents/read` | Bootstrap reader |
| POST `/internal/v1/game-auth/game-restore/notice` | Restore notice authority |
| POST `/internal/v1/game-auth/native-account-characters`, `/internal/v1/game-auth/native-account-characters/watermark` | Account character projection |
| POST `/internal/v1/playerdata/journal/{records,cursor}`, `/internal/v1/playerdata/journal/commands/{pending,read,receipt}` | Journal sessions, items and replay authority |
| POST `/internal/v1/playerdata/progress/{record,watermark}` | Journal progress authority; a separate death library router declares the same paths |
| POST `/internal/v2/game-public/{character-online-session,world-daily-kills,watermark}` | Game public statistics authority |
| POST `/internal/v1/admin-control/{hello,channel,rollups,heartbeat,stream,results}` | Verified agent channel |

[Internal assembly](../../../../bin/oteryn-portal/src/game_routes.rs#L348) · [Archive and restore](../../../../crates/characters/src/restore/http.rs) · [Command protocol](../../../../crates/characters/src/commands/http.rs) · [Services protocol](../../../../crates/characters/src/services/transport.rs) · [Journal contracts](../../../../crates/playerdata/src/journal.rs) · [Statistics ingress](../../../../crates/stats/src/online/http.rs#L21) · [Agent protocol](../../../../crates/admin/src/control/http.rs#L178)

## Expected and observed by route and actor

Repeated locale and width checks share a row. The scope of each observation is the public staging host. Authenticated behavior is unobserved where the prerequisite could not be completed.

| Method and route | Actor | Expected from source | Observed | Test |
| --- | --- | --- | --- | --- |
| GET `[/{lang}]/worlds`, `/api/v1/worlds`, `[/{lang}]/online` | Anonymous | 404 with public serving disabled | 404 | Public worlds and character directory |
| GET `/{lang}/characters`, `/{lang}/characters/{name}`, `[/{lang}]/characters/{name}/skills` | Anonymous | 404 while disabled | 404 including bounded invalid search | Public worlds and character directory |
| GET `/servers`, `/characters`, `/characters/{name}` | Anonymous | 301 to Polish route | 301 and target checked | Public worlds and character directory |
| GET `/highscores` | Anonymous | 301 to Polish level listing with supported filters | 301 and target checked | Highscores and statistics |
| GET `/{lang}/rankings/{kind}`, `/{lang}/stats/rankings/{kind}`, `[/{lang}]/stats/{world,guilds}`, `/{lang}/stats/compare`, `/{lang}/stats/characters/{id}/{tab}` | Anonymous | 404 while disabled | 404 including invalid filters and all character tabs | Highscores and statistics |
| GET, POST, DELETE `/api/v1/stats/watches` | Anonymous | 404 while unmounted | 404 | Highscores and statistics |
| GET `/{lang}/atlas`, `/atlas/tiles/{version}/{floor}/{zoom}/{x}/{y}`, `/{lang}/atlas/delivery`, `/atlas/delivery.css` | Anonymous | 404; delivery has explanatory copy | 404, localized delivery unavailable | Atlas and map services |
| GET `/atlas/spawns-text/{lang}`, `/atlas/spawns`, GET and POST `/atlas/private-markers`, DELETE `/atlas/private-markers/{id}`, POST `/atlas/map-reports`, GET `/{lang}/atlas/bestiary/{character}` | Anonymous | 404 while unmounted | 404 | Atlas and map services |
| GET and POST `/{lang}/atlas/places/participation` | Anonymous | 503 while disabled | 503 | Atlas and map services |
| GET `/{lang}/atlas/places` | Anonymous | 303 to localized login | 303 | Player navigation and signed-out account views |
| GET `[/{lang}]/account`, `[/{lang}]/account/badges`, `[/{lang}]/account/characters/{name}/{profile,achievements,equipment,outfit}`, `/{lang}/account/characters/{character}/services` | Anonymous | 303 to localized login | 303; login heading differs from source | Player navigation and signed-out account views |
| GET `/{lang}/account/characters/{new,suggest}`, `/{lang}/account/characters/requests/{operation}`, `/{lang}/account/dashboard`, `[/{lang}]/account/game-logins`, `/{lang}/panel/hirelings` | Anonymous | 303 to localized login | 303 | Player navigation and signed-out account views |
| GET `/{lang}/account/hunts`, `/{lang}/account/hunts/{compare,id}`, `/{lang}/account/hunts/{id}/share`, `/{lang}/panel/items/{world}/{character}`, `/{lang}/panel/history/{world}/{family}[/{character}]` | Anonymous | 303 to localized login | 303 | Player navigation and signed-out account views |
| GET `/{lang}/account/journal/reports`, `/{lang}/account/journal/reports/notes`, `/{lang}/account/journal/reports/notes/export`, `/static/playerdata/hunts.css` | Anonymous | 303 for account views, 404 for disabled CSS | 303 and 404 respectively | Player navigation and signed-out account views |
| GET or POST as inventoried for `/admin`, `/admin/world`, `/console/{dashboard,worlds,nodes,database}`, `/admin/console/{players,audit,access,approvals}`, `/admin/players[/{id}]`, `/admin/control/{step-up,approvals/{id}}`, `/admin/{lang}/bazaar` | Anonymous | 404 when disabled or served on non-admin host | 404 | Staff game surfaces on the public staging host |
| GET and POST `/{lang}/register`, GET sign-in form | New unverified player | Empty form remains on page; eligible registration redirects to inbox, then verification is required to sign in | Browser validation observed; completion subject to shared registration budget and Mailpit | Fresh player registration and sign-in |
| POST badges, profile, services, creation, hunt share and journal forms listed above; owner reads listed above | Owner, other player, fresh player | Owner-scoped outcome or disabled response as specified by each handler | Authenticated checks blocked before account establishment; hunt share has a disabled runtime branch | Blocked |
| GET and mutations for private atlas, participation and bestiary | Owner, other player, fresh player | Disabled response or owner-scoped outcome | Atlas routes return disabled status for anonymous attempts; authenticated outcomes unobserved | Atlas and map services; authenticated checks blocked |
| GET public world, profile, atlas and statistics paths listed above | Signed-in player or staff | Same disabled/public behavior subject to per-view privacy and authority | Signed-in contexts unavailable | Blocked |
| Staff routes listed above | Game master senior, game admin, game operator, game support, game security | Per-route permission, missing-agent unavailable state where enabled, or disabled/host refusal | Anonymous host checks only; seeded-account reset, staff MFA and action results unobserved | Staff game surfaces on the public staging host; authenticated checks blocked |
| Internal Game service routes and the unmounted death detail router | Trusted Game or owner context as specified above | Internal authority required, or route absent from browser assembly | External QA setup cannot exercise these handler contracts | Blocked |
