# DECANARY removal plan (v2)

Task: DECANARY-PLAN-1. Mode: PLAN ONLY. This document changes no code, schema, configuration, deployment or data and grants no runtime, DDL, production, Game-repository or merge authority.

- Owner decision (2026-10-08): **everything about Canary leaves Oteryn-Platform; every capability is re-pointed at Oteryn-Game (native server).**
- Governing Issues: [#1474](https://github.com/Oteryn/Oteryn-Platform/issues/1474) (native-only migration, Jira `KAN-41`) and [#1476](https://github.com/Oteryn/Oteryn-Platform/issues/1476) (ARCH-PLATFORM-DECANARY-0, decoupling plan v1).
- This v2 supersedes the packet table of #1476 §3 where they differ. #1476 §1 (C1–C18 inventory) and §2 (target boundary) stay valid and are referenced, not repeated.
- Inspected Platform head: `db1ddec` (`main`). Measured with `git grep -i canary`: **609 files, 5521 lines**.
- Game repository content was **not read** (no exact-repository authorization for this task). Game contract names and numbers below come from the owner task statement and are treated as `UNVERIFIED` until a packet with Game read authority confirms them.

## 0. What changes from v1 (#1476)

| v1 position | v2 position (this plan) | Why |
|---|---|---|
| P14: retire Canary docs by *supersede, not delete* | Zero-occurrence target for the whole tree, history rule §5 | Owner decision 2026-10-08 |
| Q2 rec A (disable create/transfer until Game impl) | Kept, now mandatory for removal ordering | Canary writes cannot remain as a fallback (#1475: no automatic Canary fallback) |
| Q6 open | **D963 Q6=A**: archive then drop; no mapping to `AccountId` (as stated in #1477) | Owner ruling cited by #1477 |
| Name collisions (deploy/MQ "canary" rollout) out of scope | In scope, renamed | "everything containing 'canary'" |
| Historical migration files untouched | Removed by a final migration squash (§4 D3) | Only way to reach zero occurrences without editing applied migrations |
| No guard | Ratchet guard first (§4 A1) | Prevents regressions while ~25 serial PRs land |

## 1. Inventory (grouped)

Counts are files containing the token (case-insensitive) at `db1ddec`.

### G1 Runtime code (PHP, Go, views, lang) — ~120 files

| Sub-group | Paths | v1 row |
|---|---|---|
| Integration adapters | `app/CanaryIntegration/*` (8: provisioner, creator, transfer, 4 privilege verifiers, runtime Redis reader) | C2, C3, C7, C8, C10 |
| Account provisioning | `app/Accounts/Actions/ProvisionCanaryAccount.php`, `Contracts/CanaryAccountProvisioningGateway.php`, `Exceptions/CanaryAccountProvisioning*.php` (3), `Models/IdentityCanaryAccount.php`, `app/Identity/Actions/RegisterIdentity.php`, `app/Console/Commands/FinalizeIdentityTerminations.php` (description text) | C3 |
| Account read model | `app/Accounts/ReadModels/AccountOverviewReadModel.php`, `app/Http/Controllers/Accounts/AccountOverviewController.php` | C9 |
| Game auth v1 | `app/GameAuth/Context/GameLoginContextProvider.php`, `Tickets/*` (4), `Sessions/*` (2), `Worlds/*` (3), `NativeLogin/NativeGameLoginTickets.php`, `app/Http/Controllers/GameAuth/GameLogin*` (3), `routes/internal.php` (`{canaryAccountId}`) | C4, C5 |
| Characters | `app/Characters/Actions/CreateCharacter.php`, `Contracts/CanaryCharacterCreationGateway.php`, `Policies/CharacterNamePolicy.php`, `app/CharacterProfiles/*` (2) | C7, C11 |
| Bazaar | `app/Marketplace/Actions/{CreateCharacterAuction,PlaceAuctionBid,ReconcileCharacterAuctions,RecoverCharacterAuction}.php`, `Contracts/CanaryCharacterTransferGateway.php`, `Http/MarketplaceListingController.php`, `Models/CharacterAuction.php`, `resources/views/admin/marketplace/index.blade.php` | C8, C11 |
| Public data | `app/PublicGameData/*` (6: `CanaryGameDataRepository`, `CanaryChannelRuntimeService`, `CanaryRuntimeSnapshot`, `CanaryRuntimeStatus`, `GuildIndexQuery`, `PublicCharacterProfileService`), `app/PublicPortal/HomePageQuery.php`, `app/Http/Controllers/PublicGameData/PublicGameDataController.php` | C9, C10 |
| Catalog | `app/GameCatalog/Application/Import/{CatalogImportService,ValidatedCatalogSnapshot}.php`, `Http/Admin/AdminGameCatalogController.php`, `resources/views/game-catalog/admin/snapshot.blade.php`, `resources/schemas/game-catalog/v1*/` (3), `tools/game-catalog/validate_contract_fixture.py` | C12 |
| Wiring / ops | `app/Providers/AppServiceProvider.php` (3 bindings), `app/Operations/ProductionConfigurationVerifier.php`, `app/Audit/SecurityEventRecorder.php` (4 event constants `identity.canary_account_provisioning_*`), `routes/console.php` (5 `canary:*` commands) | C13 |
| User-visible text | `lang/{en,pl}/{community,game,identity,public}.php` (8), `app/Wiki/Content/WikiLaunchContentCatalog.php` | — |
| Gateway (Go) | `services/game-gateway/**` (17: legacy session client, protocol v1 branch, tests, README) | C6 |

### G2 Database, migrations, provisioning — 10 files

| Path | Canary content | Durable data impact |
|---|---|---|
| `database/migrations/2026_07_20_011000_create_identity_canary_accounts_table.php` | whole table `identity_canary_accounts` | binding rows per identity |
| `..._2026_07_22_000200_create_game_login_tickets_table.php`, `..._2026_09_30_000100_add_native_game_login_ticket_redemption.php` | `game_login_tickets.canary_account_id` (nullable since 09-30) | short-lived tickets (TTL 60 s default) |
| `..._2026_07_28_070100_create_character_bazaar_tables.php` | `character_auctions.seller_canary_account_id`, `escrow_canary_account_id` | auction history, money-adjacent |
| `..._2026_07_28_110000_create_game_catalog_tables.php` | `game_catalog_snapshots.canary_commit_sha` | snapshot provenance |
| `..._2026_07_29_165500_create_character_profile_preferences.php` | `character_profile_preferences.canary_player_id` (+ unique, index) | user preferences |
| `database/provisioning/canary-{readonly,provisioning,character-create,character-transfer}.sql.template` | GRANT templates into the Canary DB | none in Platform DB |

Not in the tree but affected: rows in `security_events` whose type is `identity.canary_account_provisioning_*` (append-only audit data, see §3 R6).

### G3 Configuration and environment — 5 files

`config/database.php` (4 MySQL connections `canary*`, Redis `canary_runtime`), `config/marketplace.php` (`escrow_canary_account_id`), `.env.example` (42 lines `CANARY_*`), `deploy/synology/.env.example` (38), `phpunit` / test bootstrap use of the sqlite `canary` connection (via tests in G4).

### G4 Tests — ~65 files

- Feature: `tests/Feature/{Accounts,Characters,CharacterProfiles,GameAuth,Marketplace,PublicGameData,Identity,Operations,Payments}/**` (38), `HomeTest.php`.
- Unit: `tests/Unit/CanaryIntegration/*` (5), `tests/Unit/PublicGameData/CanaryChannelRuntimeServiceTest.php`, `tests/Unit/Characters/CharacterNamePolicyTest.php`.
- Integration/fixtures: `tests/Integration/GameCatalog/CrossRepositoryGameCatalogTest.php`, `tests/Integration/REGISTRY.json`, `tests/Fixtures/GameCatalog/v1*/` (3).
- E2E: `tests/e2e/native_auth_ephemeral_cutover/*` (3).
- CI contract tests: `tests/ci/test_synology_*` (4, Canary image in release/rollback identity), `tests/ci/test_workflow_trigger_economy.py` (name collision: `native-auth-canary-cache-build.yml`).

### G5 CI, acceptance, deploy — ~70 files

| Sub-group | Paths |
|---|---|
| Acceptance scripts | `scripts/acceptance/**` (23: `bootstrap-production-like.sh` simulated schema, `docker/compose.yml`, `seed*.php`, `assert-*.php`, `tests/*.spec.mjs`, `coverage/**`, `visual-acceptance*.js`) |
| Acceptance workflows | 14 `*-acceptance.yml` / `*-contract.yml` with a MariaDB `canary_acceptance` service, plus `ci.yml`, `game-catalog-contract.yml`, `native-protocol-contract-audits.yml`, `phase7-production-like-validation.yml` |
| Synology deploy | `deploy/synology/{compose.yml,compose.marketplace.yml,nginx/internal.conf,tls/init.sh,README.md,BUILD_ROUTING.md,PUBLIC_ENDPOINTS.md}`, `scripts/{deploy,deploy-impact,lib,health-check,rollback,release-state,upgrade-legacy-release-state,marketplace-staging,production-target-preflight}.sh`, `tests/test_fresh_baseline_contract.py` |
| Synology workflows | `build-synology-staging-images.yml` (Canary image digest pin), `deploy-synology-staging.yml`, `character-bazaar-staging-{control,validation}.yml`, `recover-synology-staging-schema.yml`, `repair-synology-*`, `synology-{container-hygiene,diagnostics,production-target-preflight}.yml` |

### G6 Documentation — ~330 files

| Class | Paths | Count |
|---|---|---|
| Normative contracts | `docs/contracts/*` (incl. `CANARY_DATA_CONTRACT`, `GAME_SESSION_CANARY_CONTRACT`, `IDENTITY_CANARY_ACCOUNT_BINDING_CONTRACT`, `PLATFORM_CANARY_ACCOUNT_PROVISIONING_CONTRACT`, `AUTH_GAME_LOGIN`, `CHARACTER_{CREATION,TRANSFER,DELETION}`, catalog, gateway, OTCLIENT, V2 contracts that mention Canary as legacy) | 35 |
| Architecture + ADRs | `docs/architecture/*.md` (25), `adr/*` (37, incl. 0002 *separate-platform-and-canary-repositories*, 0021 *require-canary-owned-character-deletion-lifecycle*), `migration/`, `character-lifecycle/` | 67 |
| Operations / testing / misc live docs | `docs/operations/*` (17), `docs/testing/**` (19), `docs/{acceptance,design,maps,evidence}` (4), `README.md`, `THIRD_PARTY_NOTICES.md` | ~42 |
| Agent control plane (live) | `docs/agents/{ACTIVE_WORK,CONTEXT_HANDOFF,CONTEXT_ROUTING,OTERYN_PLATFORM_PROGRAM_SCOPE,PROJECT_STATE,REPOSITORY_MAP,SHORT_PROGRAM_INVOCATIONS}.md`, `CI_WORKFLOW_LIFECYCLE.json`, `GOVERNANCE_CONTRACT.json`, `programs/*` (2), `prompts/*` (12), `tools/validation/adr_registry.py` | ~25 |
| Historical record | `docs/agents/tasks/archive/*` (172), `reports/*` (23), `evidence/*` (11), `handovers/*` (2), `CHANGELOG.md` | ~210 |
| Name collision (deploy/MQ "canary" rollout, not the game engine) | `docs/maintenance/{DELEGATED_MQ_CANARY_20260913,OTERYN_CI_MQ_CANARY_V2}.md`, workflow name `native-auth-canary-cache-build.yml` referenced in `tests/ci/test_workflow_trigger_economy.py` | 3 |

The task IDs of this programme (`DECANARY-*`) and this file contain the token too; see §4 F1.

## 2. Native replacement per group

Contracts in flight (status as given by the owner; Game-side items `UNVERIFIED` here):

| Ref | Contract | Covers |
|---|---|---|
| Game #1936 | `CHARACTER_AUTHORITY_COMMANDS_V1` | Create, TransferOwnership (+ receipts, idempotency key); expected to cover Delete/Rename later |
| Game #1937 | `PUBLIC_PROJECTIONS_V1` | Highscores, character profile, guilds/membership, deaths, online list |
| Platform #1477 | DECANARY-IDS-1 | Additive `char(36)` `AccountId` / `CharacterId` beside Canary ids in `character_auctions`, `character_profile_preferences` |
| Platform #1479 | DECANARY-LOGIN-DEFAULT-1 | LCFA (native account characters) default-on in `APP_ENV=preproduction`; gateway opt-in per deployment |
| Platform #1478 | DECANARY-DOCS-1 v2 | Routes and Canary contracts labelled LEGACY; target ownership table in `DATA_OWNERSHIP.md` |
| — | DECISION-INDEX-1 v2 | Owner decision index; must record this plan's history rule (§5) and ADR retirements. Not yet visible as a Platform PR at inspection time |
| Platform (merged) | LCFA consumer #1465, GATEWAY-NATIVE-CONFIG-1 #1472, native admission (N4-P), native runtime status ingest | Login, char list, runtime status |
| Platform | `docs/contracts/OTERYN_V2_CHARACTER_AUTHORITY_COMMAND_CONTRACT.md` (accepted semantic contract; transport deferred) | Platform-side semantics that #1936 must realise |

| Group | Replaced by | Gap until Game delivers |
|---|---|---|
| G1 login / char list / tickets (C4, C5, C6) | Native admission + LCFA + gateway native-only mode | None on Platform; needs preprod soak (#1479) |
| G1 account provisioning (C3) | Nothing: Platform already owns `AccountId` (ADR 0028); Game copies it via LCFA | None |
| G1 character creation (C7) | Game #1936 `CreateCharacter` via a Platform command client | Feature **disabled** (Q2=A) until #1936 is accepted and implemented |
| G1 Bazaar transfer (C8) | Game #1936 `TransferCharacterOwnership` + receipt inside the existing Platform escrow saga | New listings **frozen**, in-flight auctions drained (§3 R3) |
| G1 public data + online (C9, C10) | Game #1937 projections pushed into Platform read models (Q3=A, LCFA style); node/world status from native runtime status | Pages render explicit "unavailable" state, never fake data |
| G1 catalog (C12) | Game catalog export (native envelope, already validated in Platform); provenance field renamed | Needs #1460 / ADR 0034 disposition and a Game export field name without the token |
| G1 lang / wiki text | Neutral native wording | None |
| G2 provisioning templates, verifiers | Nothing (no Platform credential into Game, ADR-0004/0013) | None |
| G2 Canary id columns | Native id columns from #1477; Canary columns archived then dropped (Q6=A) | Preferences keyed by Canary player id are **not** carried over |
| G3 config / env | Native config already in `config/game-auth.php`; new projection/command client config per packet | None |
| G4 tests | Native tests (`tests/Feature/GameAuth/Native*`, LCFA tests) + new fixtures per packet | Public-data native fixtures do not exist yet |
| G5 acceptance / deploy | Native acceptance fixtures; Platform-only Synology topology with a separately operated Game endpoint | Owner Q4; Game deploy is out of Platform scope |
| G6 docs | Native contracts + DECISION-INDEX-1 v2; history rule §5 | None |

## 3. Risk rules (apply to every packet)

- **R1 No fallback.** No packet may add an automatic Canary fallback or keep a Canary write behind a flag after its cutover packet merges.
- **R2 Disable before delete.** A capability whose Game wire is missing is switched off (feature returns a typed unavailable state) in its own PR before adapter code is deleted.
- **R3 Money.** Bazaar cutover requires: new listings frozen, every in-flight auction settled or cancelled with refund, reconciliation job reports zero non-terminal sagas, then adapter switch. Concurrency/idempotency/recovery suites stay green at every step.
- **R4 Additive first, destructive last.** Columns are added (#1477) before reads switch; drops happen only after every environment has run the switched code and the archive (D1) is evidenced.
- **R5 Migrations are durable contracts.** Applied migration files are never edited or renamed. The token leaves migration files only through drop migrations followed by a schema squash (D3).
- **R6 Audit data is append-only.** Existing `security_events` rows keep their stored type strings; only the code constants are removed. No data migration rewrites audit history.
- **R7 Auth.** Login changes keep CSRF, rate limits and session rotation; the v1 ticket path is removed only after native login has run as default in a prod-like environment.
- **R8 Contract versions.** Removing catalog schemas `v1`, `v1.1`, `v1.2` is a wire change: Platform must first accept the native export and stop accepting legacy snapshots in a separate, announced PR.
- **R9 Required checks.** Renaming or deleting a workflow that is a required status check is coordinated with branch protection by the owner; the PR names the check explicitly.

## 4. PR sequence

Small, serial, one domain per PR. Target diff per PR: ≤ ~400 changed lines excluding pure deletions; pure-deletion PRs may be larger but contain nothing else. `Dep` lists hard prerequisites (merged, unless stated).

### Phase 0 — in flight (not re-planned)

#1475 baseline, #1477 IDS-1, #1478 DOCS-1 v2, #1479 LOGIN-DEFAULT-1, this plan, DECISION-INDEX-1 v2.

### Phase A — guard

| # | Packet | Change | Dep |
|---|---|---|---|
| A1 | DECANARY-GUARD-1 | CI ratchet: per-group occurrence baseline (JSON); fails when a group count rises; regex written as `c[a]nary` so the guard itself does not count. Report-only for docs groups at first | — |

### Phase B — cutover per domain (switch reads/writes to native, or disable)

| # | Packet | Change | Dep (Platform) | Dep (Game) | Risk |
|---|---|---|---|---|---|
| B1 | DECANARY-REG-1 | Stop creating `identity_canary_accounts` at registration; unschedule/disable `canary:provision-pending-accounts`; account page stops showing binding state | A1 | — | Medium (account) |
| B2 | DECANARY-ACCOUNT-1 | Account overview reads LCFA read model instead of Canary `players` | #1479, B1 | LCFA producer live in env | Medium |
| B3 | DECANARY-CREATE-OFF-1 | Character creation returns typed unavailable; remove Canary creator binding | A1 | — | Low |
| B4 | DECANARY-BAZAAR-FREEZE-1 | Freeze new listings + drain command/report for in-flight auctions (R3) | #1477 | — | High (money) |
| B5 | DECANARY-PUBLIC-OFF-1 | Public pages (highscores, profiles, guilds, deaths, online) render unavailable when the Canary connection is absent; no Canary queries when `PUBLIC_GAME_DATA_SOURCE=native` | A1 | — | Low |
| B6 | DECANARY-CATALOG-1 | Additive `game_catalog_snapshots.source_commit_sha`, copy from old column, code reads new field; accept native export only behind config | #1460 disposition | catalog export field name | Medium (wire) |
| B7 | DECANARY-LOGIN-NATIVE-ONLY-1 | Synology/compose gateway in native-only mode; Platform v1 login-context route returns 410 | #1479 soak in preprod, native E2E | — | High (auth) |

### Phase C — native capability (needs Game)

| # | Packet | Change | Dep (Platform) | Dep (Game) | Risk |
|---|---|---|---|---|---|
| C1 | DECANARY-CMD-CLIENT-1 | Platform Character Authority command client (transport, auth, idempotency key, receipt store) | #1477 | #1936 accepted | High (wire) |
| C2 | DECANARY-CREATE-1 | Re-enable creation through C1 | B3, C1 | #1936 Create implemented | High |
| C3 | DECANARY-BAZAAR-1 | Escrow saga uses C1 `TransferCharacterOwnership`; writes native ids; unfreeze | B4 drained, C1 | #1936 Transfer implemented | High (money) |
| C4 | DECANARY-PROJ-INGEST-1 | Projection ingest endpoint (mTLS, watermark, LCFA style) + read-model tables | A1 | #1937 accepted | High (wire) |
| C5 | DECANARY-PUBLIC-1a | Highscores + profiles from C4 | B5, C4 | #1937 producer | Low |
| C6 | DECANARY-PUBLIC-1b | Guilds + deaths from C4 | C5 | #1937 producer | Low |
| C7 | DECANARY-PUBLIC-1c | Online list from C4; channel status from native runtime status; drop Redis reader usage | C5 | #1937 online | Low |
| C8 | DECANARY-PREFS-1 | Profile preferences keyed by `character_id`; old rows ignored (Q6=A, no mapping) | #1477, C5 | — | Medium (user-visible reset) |

### Phase D — remove code, config, CI, deploy

| # | Packet | Change | Dep |
|---|---|---|---|
| D-1 | DECANARY-ACCEPT-1a | Acceptance bootstrap/seed/assert on native fixtures | C2–C7 (or B-phase unavailable states) |
| D-2 | DECANARY-ACCEPT-1b | Playwright specs + coverage manifests | D-1 |
| D-3 | DECANARY-ACCEPT-1c | Acceptance workflows drop `canary_acceptance` service (R9) | D-2 |
| D-4 | DECANARY-DEPLOY-1a | `deploy/synology/compose*.yml`, `.env.example`, nginx, tls: remove Canary container, DB, env | B7, owner Q4 |
| D-5 | DECANARY-DEPLOY-1b | Deploy/rollback/health/preflight scripts + `tests/ci/test_synology_*` + `deploy/synology/tests` | D-4 |
| D-6 | DECANARY-DEPLOY-1c | Synology workflows (image pin, staging, bazaar staging, schema recovery) (R9) | D-5 |
| D-7 | DECANARY-GATEWAY-1 | Go gateway: delete legacy session client and protocol v1 branch | B7, D-4 |
| D-8 | DECANARY-DELETE-LOGIN-1 | Delete `GameAuth/{Context,Tickets,Sessions}` v1, controllers, internal route, tests | B7, D-7 |
| D-9 | DECANARY-DELETE-CODE-1 | Delete `app/CanaryIntegration/**`, `Accounts/*Canary*`, gateway contracts, bindings, console commands, verifier entries, audit constants (R6), tests | B1–B5, C2/C3 or their off-states, D-8 |
| D-10 | DECANARY-DELETE-CONFIG-1 | `config/database.php` connections, `config/marketplace.php`, `.env.example`, `database/provisioning/canary-*` | D-9, D-6 |
| D-11 | DECANARY-TEXT-1 | `lang/**`, wiki catalog text, views | C5–C7 or B5 |

### Phase E — data (protected; owner authority per environment)

| # | Packet | Change | Reversibility | Dep |
|---|---|---|---|---|
| E1 | DECANARY-ARCHIVE-1 (operation, not a code PR) | Export `identity_canary_accounts`, Canary id columns and `canary_commit_sha` per environment to an access-controlled backup outside the repo; record row counts and checksums as evidence (no data in repo) | Read-only | D-9 deployed everywhere |
| E2 | DECANARY-DROP-1a | Migration: drop `game_login_tickets.canary_account_id` (after all v1 tickets expired) and `identity_canary_accounts` | `down()` recreates structure empty; data only from E1 | E1 |
| E3 | DECANARY-DROP-1b | Migration: drop `character_auctions.{seller,escrow}_canary_account_id`, `character_profile_preferences.canary_player_id` (+ index/unique) | as E2; refuse `down()` never needed (no data loss on recreate) | E1, C3, C8 |
| E4 | DECANARY-DROP-1c | Migration: drop `game_catalog_snapshots.canary_commit_sha` | as E2 | E1, B6 |
| E5 | DECANARY-SQUASH-1 | `php artisan schema:dump --prune` (MariaDB) once E2–E4 ran in every environment; delete historical migration files; rewrite migration-specific rollback tests | Not reversible by `down()`; restore = revert PR (old files) on a DB that already has the post-drop schema | E2–E4 confirmed in all envs, CI has `mysql` client |

### Phase F — documentation and final gate

| # | Packet | Change | Dep |
|---|---|---|---|
| F-1 | DECANARY-DOCS-2a | Contracts: delete Canary contracts, rewrite remaining contracts to native terms | D-9 |
| F-2 | DECANARY-DOCS-2b | Architecture docs + ADR retirement (0002, 0021 deleted, `adr_registry.py` + ADR README + backlog updated, recorded in DECISION-INDEX-1) | F-1 |
| F-3 | DECANARY-DOCS-2c | Operations, testing, README, THIRD_PARTY_NOTICES, agent control-plane docs, `CONTEXT_ROUTING.md` (`canary-integration` route removed) | F-2 |
| F-4 | DECANARY-HISTORY-1 | Apply §5 to archive/reports/evidence/handovers/prompts/CHANGELOG; IA catalog + reconciliation registry paths | F-3, owner Q-H |
| F-5 | DECANARY-RENAME-1 | Name collisions: MQ rollout docs and workflow renamed to `pilot` (R9) | A1 |
| F-6 | DECANARY-FINAL-1 | Guard becomes blocking at zero for all groups; this plan and any remaining `DECANARY-*` packet text moved out of tree under §5 | everything above |

Parallel lanes: A1 first; then B1, B3, B4, B5, F-5 in parallel (disjoint paths); B2/B6/B7 as their deps land; C-phase follows Game; D after B/C per domain; E strictly serial; F last.

## 5. History rule (proposal, owner decision Q-H)

| Class | Rule |
|---|---|
| H1 Live normative docs (contracts, architecture, ADRs, operations, testing, routing, prompts marked reusable) | Rewrite in native terms or delete. A doc whose subject is Canary is deleted; its supersession is recorded once in DECISION-INDEX-1 v2 by ADR/contract id, without the token |
| H2 Historical records (`tasks/archive`, `reports`, `evidence`, `handovers`, one-shot prompts, `CHANGELOG.md`) | **Recommended:** tag `archive/pre-native-only` at the commit before F-4 (immutable provenance), then a scripted deterministic redaction: token → `legacy engine` (identifiers: `legacy_engine`), plus one header line `Redacted 2026-10 (native-only); original at tag archive/pre-native-only`. The script and its mapping are committed and re-runnable; the PR contains nothing else |
| H2 alternative | Delete H2 files from the tree and keep them only at the tag. Smaller risk of rewording evidence, but loses in-tree history for unrelated work |
| H3 Git history, closed Issues/PRs, Jira | Untouched; out of the zero-occurrence scope |
| H4 Data at rest (audit rows, backups from E1) | Untouched (R6); not repository content |

Rationale for the recommendation: redaction keeps 170+ unrelated task records searchable, the tag preserves exact evidence, and evidence-integrity tooling (`HISTORICAL_WORK_RECONCILIATION_REGISTRY.json`, `DOCUMENTATION_IA_CATALOG.json`) keeps valid paths.

## 6. Owner questions

| # | Question | Recommendation |
|---|---|---|
| Q-H | History rule for H2 | Redaction + tag (§5) |
| Q-ID | Programme ID contains the token | New packets use prefix `NATIVE-ONLY-` from F-phase onward; existing `DECANARY-*` stay in git history |
| Q4 (v1) | Remove Canary container and legacy login rollback port in D-4 | Yes (v1 rec A) |
| Q-PREFS | Accept loss of profile preferences keyed by Canary player id | Yes, unless a production row count says otherwise (E1 evidence) |
| Q-CAT | Stop accepting catalog schemas v1–v1.2 | Yes, after the native export is qualified (B6) |

## 7. Worker packets (Sonnet-tier)

Packets marked **Opus** in the tier column stay with a top-tier architect/worker (auth, wire, persistence, money). Every packet: one branch, one PR, active packet from `TASK_TEMPLATE.md`, Conventional Commit, ratchet count for its group must drop and no other group may rise. Common validation for PHP packets: `vendor/bin/pint --test`, `composer analyse`, focused `php artisan test --filter=...`, required CI on exact head.

| Packet | Tier | owned_paths | Validation (in addition to common) |
|---|---|---|---|
| A1 GUARD-1 | Sonnet | `tools/validation/legacy_token_ratchet.py`, `tools/validation/test_legacy_token_ratchet.py`, `docs/agents/LEGACY_TOKEN_BASELINE.json`, one step in `.github/workflows/ci.yml` | `python -m pytest tools/validation/test_legacy_token_ratchet.py`; baseline equals `git grep -ic` per group |
| B1 REG-1 | **Opus** | `app/Identity/Actions/RegisterIdentity.php`, `routes/console.php` (provision command only), `tests/Feature/Identity/RegistrationTest.php` | Registration test without any Canary connection configured |
| B2 ACCOUNT-1 | Sonnet | `app/Accounts/ReadModels/**`, `app/Http/Controllers/Accounts/AccountOverviewController.php`, `tests/Feature/Accounts/AccountOverviewTest.php` | Account overview from LCFA fixture; owner-only read test |
| B3 CREATE-OFF-1 | Sonnet | `app/Characters/**`, `app/Providers/AppServiceProvider.php` (creator binding line), `tests/Feature/Characters/CharacterCreationTest.php` | Creation returns typed unavailable; no write attempted |
| B4 BAZAAR-FREEZE-1 | **Opus** | `app/Marketplace/**`, `config/marketplace.php`, `tests/Feature/Marketplace/**` | Settlement recovery, idempotency, concurrency (MariaDB) suites |
| B5 PUBLIC-OFF-1 | Sonnet | `app/PublicGameData/**`, `app/PublicPortal/HomePageQuery.php`, `app/Http/Controllers/PublicGameData/**`, `tests/Feature/PublicGameData/**`, `tests/Feature/HomeTest.php` | Pages 200 with unavailable state and no Canary connection |
| B6 CATALOG-1 | **Opus** | `app/GameCatalog/**`, new additive migration, `resources/schemas/game-catalog/**`, `tests/Feature/GameCatalog/**`, `tests/Fixtures/GameCatalog/**` | Up/down on MariaDB with non-empty snapshots; import of native fixture |
| B7 LOGIN-NATIVE-ONLY-1 | **Opus** | `deploy/synology/compose.yml` (gateway service env only), `routes/internal.php`, `config/game-auth.php` | `tests/e2e/native_auth_ephemeral_cutover`, gateway CI |
| C1 CMD-CLIENT-1 | **Opus** | new `app/CharacterAuthority/**`, `config/character-authority.php`, tests | Timeout, retry with same idempotency key, receipt replay |
| C2 CREATE-1 | **Opus** | `app/Characters/**`, tests | MariaDB concurrency; duplicate-name rejection via receipt |
| C3 BAZAAR-1 | **Opus** | `app/Marketplace/**`, tests | Full Marketplace suites; drain report zero before switch |
| C4 PROJ-INGEST-1 | **Opus** | new `app/GameProjections/**`, migration (read models), `routes/internal.php` (ingest route), tests | mTLS identity, watermark monotonicity, replay rejection |
| C5 PUBLIC-1a | Sonnet | `app/PublicGameData/**` (highscores/profile), tests | Feature tests on projection fixtures; acceptance pages |
| C6 PUBLIC-1b | Sonnet | `app/PublicGameData/GuildIndexQuery.php` + deaths, tests | as C5 |
| C7 PUBLIC-1c | Sonnet | `app/PublicGameData/*Runtime*`, `app/PublicGameData/*Channel*`, tests | Online/status pages without Redis runtime connection |
| C8 PREFS-1 | Sonnet | `app/CharacterProfiles/**`, `tests/Feature/CharacterProfiles/**` | Concurrency test on `character_id` |
| D-1..D-3 ACCEPT-1a/b/c | Sonnet | `scripts/acceptance/**` (a, b); acceptance workflows (c) | Acceptance runs green without `canary_acceptance`; workflow inventory test |
| D-4..D-6 DEPLOY-1a/b/c | Sonnet (Opus review) | `deploy/synology/**`, `tests/ci/test_synology_*`, Synology workflows | `python -m pytest tests/ci deploy/synology/tests`; staging deploy + rollback dry run (protected, owner) |
| D-7 GATEWAY-1 | **Opus** | `services/game-gateway/**` | `go test ./...`, `go vet`; native E2E |
| D-8 DELETE-LOGIN-1 | **Opus** | `app/GameAuth/{Context,Tickets,Sessions}/**`, `app/Http/Controllers/GameAuth/GameLogin*`, `routes/internal.php`, matching tests | Full suite; no route references `canaryAccountId` |
| D-9 DELETE-CODE-1 | Sonnet (Opus review) | `app/CanaryIntegration/**`, `app/Accounts/**/*Canary*`, `app/Characters/Contracts/*`, `app/Marketplace/Contracts/*`, `app/Providers/AppServiceProvider.php`, `app/Audit/SecurityEventRecorder.php`, `app/Operations/ProductionConfigurationVerifier.php`, `routes/console.php`, `tests/Unit/CanaryIntegration/**` and Canary feature tests | Full suite; `composer analyse`; config verifier test |
| D-10 DELETE-CONFIG-1 | Sonnet | `config/database.php`, `config/marketplace.php`, `.env.example`, `database/provisioning/**` | `php artisan config:cache` in CI; full suite |
| D-11 TEXT-1 | Sonnet | `lang/**`, `app/Wiki/Content/WikiLaunchContentCatalog.php`, `resources/views/**` | Lang key parity en/pl; wiki content tests |
| E2..E4 DROP-1a/b/c | **Opus** | one new migration each | Up/down on MariaDB with seeded data; E1 evidence linked |
| E5 SQUASH-1 | **Opus** | `database/schema/**`, `database/migrations/**`, migration rollback tests | Fresh `migrate` from dump equals post-drop schema (diff of `SHOW CREATE TABLE`) |
| F-1..F-3 DOCS-2a/b/c | Sonnet | listed docs per §4 F; `tools/validation/adr_registry.py` (F-2) | `python -m pytest tools/validation tools/agents`; docs IA tests |
| F-4 HISTORY-1 | Sonnet | `tools/agents/redact_legacy_token.py`, H2 paths | Script idempotent (second run no diff); IA catalog + reconciliation tests |
| F-5 RENAME-1 | Sonnet | `docs/maintenance/*MQ*`, `.github/workflows/native-auth-*-cache-build.yml`, `tests/ci/test_workflow_trigger_economy.py` | Workflow inventory/trigger-economy tests |
| F-6 FINAL-1 | Sonnet | guard baseline, this file | `git grep -i` for the token returns nothing; guard blocking |

## 8. Closeout

Product runtime E2E for this task: `NOT_APPLICABLE` (planning document only). Jira mapping for this plan: pending (programme Story `KAN-41` per #1475; not updated by this task).
