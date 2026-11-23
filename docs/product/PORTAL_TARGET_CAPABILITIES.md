# Portal target capabilities (professional MMO server portal)

Status: working inventory, 2026-11-23. Top-down model of what the portal must offer, diffed against
portal-rs on `origin/main` 7847cf7d and Oteryn-Game at 139056a0 (read-only). It complements the
bottom-up `PORTAL_GAPS.md`, which could only list gaps next to existing code.

States: **DONE** reachable in production configuration · **UNUSABLE** code exists but is unreachable
(flag off, staff host absent, testing/preprod-only, CLI-only, adapter unavailable) · **ABSENT** · **GAME**
needs a Game producer or a contract that is not accepted · **OWNER** needs an owner rule.

Counts (57 rows): DONE 7 · UNUSABLE 25 · GAME 15 · ABSENT 10 · plus 3 rows that also need an owner rule.

## Cross-cutting facts

1. Every staff surface is behind `OTERYN_ADMIN_PANEL` + `OTERYN_ADMIN_HOST` (crates/admin/src/web.rs:154, 300-309);
   production sets neither, so no staff page is reachable in production. Staging enables them (deploy/oracle staging).
2. The first staff role can only be granted by SQL (crates/admin/src/rbac/storage.rs:219 needs an existing admin).
3. `restricted_environment` (crates/core/src/lib.rs:161) limits Game public-data ingest, guilds, houses, deaths,
   character commands/services, journal ingest and store purchases to testing/preprod; production pages render
   "unavailable".
4. Game already keeps receipts/ledgers for nearly every value movement (item mints, fee burns, bank ledger, NPC and
   player trades, market settlements, coin-market gold, bazaar burns, store deliveries, guild bank), but no exporter
   publishes them and there is no money-supply counter (Game ADM-02:16, :99).
5. Game logs are key=value lines on stderr with coded errors (E####) and a panic hook; a metrics registry with
   `render_prometheus` exists but nothing serves it; health is a root-only Unix socket (`tick_ms` null).
6. Game has an outbound admin agent that long-polls `/internal/v1/admin-control/*` with a 15-operation catalog
   (world, node, player, economy, moderation; risk classes R0-R4) - contract ADM-01 CANDIDATE, only
   `admin.policy.get` executed. This is the intended pipe for every server-operation command below.
7. Game has no world state (open/closed/maintenance), no rates config, no hot reload, no save-now command;
   MOTD is static config; GM in-game commands exist (inspect, mute, kick, ban, teleport, bring, notify, broadcast).

## A. Server and world operations

| id | capability | state | next |
|---|---|---|---|
| A1 | world registry list/add | UNUSABLE (seed-only) | Platform: staff world registry editor once ADM-01 `world.*` accepted |
| A2 | world enable/disable/hide/retire | GAME | Game: ADM-01 `world.state.set` |
| A3 | states open / closed / staff-only / maintenance (scheduled) / drain | GAME | Game: world state machine + ADM-01 op; Platform: scheduler + approvals |
| A4 | maintenance notice, status page, incidents, uptime | UNUSABLE/ABSENT | Platform lane **w2-status** |
| A5 | scheduled restart / save / shutdown with countdown broadcast | GAME | Game: ADM-01 `node.*` + countdown |
| A6 | in-game broadcast, MOTD edit | GAME | Game: ADM-01 broadcast op, MOTD from config to runtime |
| A7 | per-world config (rates, pvp type) with history and rollback | GAME | Game: `world.configure` + config revisions |
| A8 | feature flags view | ABSENT | Platform lane **w2-sysops** (read-only; runtime toggle = OWNER) |
| A9 | log viewers + search/export (audit, staff, login, Game, economy) | UNUSABLE | Platform: enable audit console; Game logs need a log shipping contract |
| A10 | deploy history / changelog / version per world | ABSENT | changelog in GAP-33 lane; version per world from runtime status |
| A11 | system health (DB, worker, outbox, mail, gateway, Game link) | UNUSABLE | Platform lane **w2-sysops** |
| A12 | mail outbox failures + resend | ABSENT | Platform lane **w2-mailops** |
| A13 | registration policy management | UNUSABLE (SQL only) | Platform lane **w2-regpolicy** |
| A14 | staff management: bootstrap, invitations, roles, MFA, sessions | UNUSABLE | Platform lane **w2-bootstrap** |
| A15 | four-eyes approvals | UNUSABLE | enable; rules per action = OWNER |
| A16 | backups / ops export | UNUSABLE | ops runbook, not portal UI |

## B. Player administration

| id | capability | state | next |
|---|---|---|---|
| B1 | one dossier per player (account, characters, sanctions, payments, logins, notes, tickets) | UNUSABLE | Platform: after GAP-11 lands |
| B2 | account edit, password reset, MFA reset, session revoke | UNUSABLE | GAP-11 lane (in progress) |
| B3 | character edit (name, level/skills, position, outfit, items, gold, house) | GAME | Game: ADM-01 player ops; OWNER: who may edit |
| B4 | sanctions with expiry, reason, appeal; IP/device bans | UNUSABLE | enable enforcement transport (contract implemented, default off) |
| B5 | compensation (items, premium, currency) | UNUSABLE + OWNER | OWNER rule: who grants, limits, four-eyes |
| B6 | character restore, ownership repair | GAME | Game op needed |
| B7 | multi-account detection | ABSENT + OWNER | OWNER: privacy rule for IP/device linking |
| B8 | bulk actions | ABSENT | after B1 |
| B9 | staff notes | UNUSABLE | enable |
| B10 | support staff side, canned replies, assignment | UNUSABLE/ABSENT | after GAP-26/27 land |
| B11 | player reports queue | UNUSABLE | enable; GAP-59 adds atlas reports |
| B12 | CMS (news, pages, FAQ, events, media, drafts, schedule) | UNUSABLE | enable; GAP-28/33/34 lanes |
| B13 | store offers, prices, promotions, refunds | UNUSABLE | offer editor (Platform) after money rules = OWNER |
| B14 | analytics dashboards | UNUSABLE | Platform lane **w2-charts** (local data) |

## C. Player self-service

| id | capability | state | next |
|---|---|---|---|
| C1 | register, login, verify, reset, change password/email | DONE | |
| C2 | MFA TOTP/WebAuthn, recovery | DONE (rotation missing) | GAP-21 lane |
| C3 | sessions, login history, security notifications | DONE | |
| C4 | account deletion, data export | DONE (final deletion waits on Game closure proof) | |
| C5 | character create | UNUSABLE | production ingest gate |
| C6 | character delete / undelete | GAME | |
| C7 | rename, sex change, world transfer | GAME | character services v2 PROPOSED |
| C8 | character visibility, comment | DONE | |
| C9 | public character page (deaths, achievements, equipment, skills) | UNUSABLE | v2 projections ACCEPTED; production gate |
| C10 | guilds (create, invite, ranks, wars, page) | GAME | Game guilds contract CANDIDATE |
| C11 | houses (list, auction, transfer, leave) | GAME | |
| C12 | highscores, online, worlds, kill stats | UNUSABLE | ACCEPTED; production gate |
| C13 | bazaar / character trade | GAME | bazaar authority CANDIDATE |
| C14 | purchase history, coin balance, gifts | UNUSABLE | store purchases off in code |
| C15 | sanctions visible + appeal | UNUSABLE | GAP-27 lane |
| C16 | support tickets | DONE (no staff in production) | |
| C17 | notification and newsletter preferences | partial | Platform lane **w2-notify** |
| C18 | linked external accounts (Discord) | ABSENT | next wave |
| C19 | forum | ABSENT | Platform lane **w2-forum** |

## D. Observability and economy

| id | capability | state | next |
|---|---|---|---|
| D1 | charts (online, registrations, logins, revenue) | ABSENT | **w2-charts** (local), Game parts via D2 |
| D2 | Game analytics ingest | GAME | accepted rev 3 but blocked on resync baseline; Game must publish |
| D3 | economy monitoring: gold supply, sources/sinks, market prices, top earners, dupe alerts | GAME | Game: ANL-03 exporter over existing ledgers + `gold_supply` queries |
| D4 | economy management (rates, sinks, store prices) | UNUSABLE | store prices CLI only; rates need A7 |
| D5 | error detection and alerting | ABSENT | Platform lane **w2-obs** (portal); Game: error/crash push or log shipping |
| D6 | Game event logs per player (trades, deaths, drops, chat reports) | GAME | Game: audit outbox publisher |
| D7 | performance (tick, memory, players per world, uptime) | UNUSABLE | Game: serve metrics, fill `tick_ms`/connections |
| D8 | portal `/metrics` | ABSENT | Platform lane **w2-obs** |

## Game-side tasks (for the Game coordinator; Platform does not implement from non-accepted contracts)

- G-ADM: accept ADM-01 and implement agent operations: world state set (open/closed/staff-only/maintenance/drain),
  scheduled save/restart/shutdown with countdown broadcast, broadcast/MOTD, world configure with revisions,
  player ops (teleport, kick, ban/unban, rename, restore, item/gold grant with receipts).
- G-ECO: ANL-03 exporter: publish item/bank/market/house/mail/guild-bank audit outboxes to the portal; money-supply
  snapshots per world (`gold_supply`, `gold_sources_sinks`, `market_volume`, `npc_trade_flow`).
- G-OBS: serve the metrics registry to the portal (or push), fill `tick_ms`, connections, players per world, memory;
  push coded errors/panics (E####, E4001) with build id; log shipping decision (backend open in the logging review).
- G-PUB: production enablement of accepted public projections v1/v2 (highscores, profiles, deaths, skills, online).

## Owner decisions needed

1. Accept ADM-01 (admin control plane) and ANL-03 (economy telemetry) so both sides may implement.
2. Staff power rules: who may grant items/gold/premium/coins, edit characters, change world state; which need
   four-eyes; limits per action.
3. Multi-account detection: may staff see IP/device links between accounts.
4. Runtime feature-flag toggles from the panel vs env-only.
5. Production: open the staff panel on a separate host (and bootstrap the first admin).
6. Production: lift the testing/preprod-only gate for Game public data once Game publishes.

## Platform build lanes running (Codex Cloud, each followed by a Codex review-and-fix lane)

Wave 1: GAP-11, 14, 21, 26, 27, 28, 33, 34, 59, UI. Wave 2: w2-bootstrap, w2-regpolicy, w2-mailops, w2-obs,
w2-sysops, w2-charts, w2-status, w2-forum, w2-notify.
