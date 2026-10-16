---
task_id: OTERYN-20261015-public-gamedata-native-1
governing_issue: 1474
required_reads: []
search_first:
  - app/GameProjections/PublicProjectionReadModel.php
optional_reads: []
---

# OTERYN-20261015 public online and highscores on native data

## Goal

Governing GitHub Issue: #1474 (native-only migration) is the canonical lifecycle authority for this task. The top-level `governing_issue` frontmatter field is machine-required while this packet remains active.

Decision input first, then the Platform page readers (packets C5 and C7), approved by the owner on 2026-10-15. `https://oteryn.com/pl/online` and `/pl/highscores` answer 503 because `PublicGameDataController` reads the obsolete legacy-engine MySQL tables through the legacy game-data repository class. This packet records what native data exists, why a Platform-only reader is not possible, the minimal cross-repository change set that makes both pages serve real data, and (after the owner decisions below) implements the Platform reader for both pages.

## Acceptance criteria

- [x] The existing native data and contracts are inventoried with evidence.
- [x] The minimal contract, ownership, order of changes and rollback are stated.
- [x] The owner decides the open questions below (2026-10-15, see Owner decisions).
- [x] `/{locale}/online` and `/{locale}/highscores` read only the native public projections (`NativePublicGameData` over `PublicProjectionReadModel`); neither page opens the legacy connection or reads `PUBLIC_GAME_DATA_SOURCE`, in PL and EN.
- [x] A fresh projection answers 200 with rows; a stale, missing, epoch-fenced or disabled projection answers 503 with the unavailable view; a fresh but empty online list answers 200 with the empty state. No row is ever invented.
- [x] Highscores rows join `character_profile` in the same World by character id (never by name), are shown under the profile name, are left out without a servable profile keeping the published rank, and serve nothing while `character_profile` is stale.
- [x] The category and vocation filters select the published (World, category, filter) list; an unpublished list is unavailable, not empty.
- [x] Feature tests cover fresh, stale, missing, empty, profile join and filters (`tests/Feature/PublicGameData/NativePublicGameDataTest.php`); the legacy online and highscores tests are removed.
- [ ] Real HTTP check of both pages against the Oracle stack with live rows, and the unavailable state within S (30 s) after the producer stops (after the Game producer and the deployment steps).
- [x] `/characters/{name}` is served only from `character_profile` (plus the `guild` family for the guild and rank names), with the carried fields only: name, level, vocation, guild. A stale or missing family is 503, an unknown, hidden or other-World character is 404. Skills, deaths, comment, status and house are not shown and have no placeholders. The guild row is shown only when Game's `guild_visible` and the owner preference (keyed by the native CharacterId through the account-character projection) both allow it; an unbindable character hides it.
- [x] Deploy wiring in repo: `deploy/synology/compose.yml` forwards `GAME_PROJECTIONS_PUBLIC_*` and `PUBLIC_GAME_DATA_WORLD_ID` fail-closed, `deploy/oracle/prepare-platform.sh` seeds them off, and `deploy/synology/nginx/internal-mtls.conf` routes `/internal/v1/game-public/*` with its own 512 KiB body bound (`deploy/synology/tests/test_platform_mtls_contract.py`).

## Findings

PROVEN (read-only, Platform `origin/main` 0628d22 and Game `main` 88d6b8885):

- Platform consumer exists. Packet C4 (`OTERYN-20261011-proj-ingest-1`) ships `POST /internal/v1/game-public/{highscores,character-profile,character-deaths,guild,house,world-online,watermark}`, the read-model tables `game_public_projection_snapshots` and `game_public_projection_watermarks`, and the serve-time freshness gate `App\GameProjections\PublicProjectionReadModel`. It is default-off (`GAME_PROJECTIONS_PUBLIC_ENABLED`, testing and preproduction only).
- No page reads that read model. `PublicGameDataController` and `PublicGameDataSource` still point at the legacy game-data repository; with `PUBLIC_GAME_DATA_SOURCE=native` the pages only show "unavailable". Removal-plan packets C5 (highscores, profiles) and C7 (online) are not built.
- Game has no producer. `rg` over `apps/` and `crates/` finds no `game-public`, `world-online` or publication code. `docs/contracts/OTERYN_GAME_PUBLIC_PROJECTIONS_V1.md` §11 states "Nothing is implemented" and its status is CANDIDATE (§12 acceptance pending); `OTERYN_PLATFORM_2_DESIGN_DRAFT.md` records "ingest exists, producer in game: none".
- Game has no stored highscores or online list the Platform could query. Online presence is runtime state, and the accepted ownership boundary (`DATA_OWNERSHIP.md`, ADR-0004 §6-7, contract Q3=A) is that Game owns the facts and pushes projections; Platform keeps read models and never queries Game on page render. Platform must not read the Game database directly.
- The only Game-to-Platform feed with a producer today is the account-character projection (LCFA). It carries no level, experience, skills or presence, so it cannot feed these pages.

DERIVED: with the current code no Platform-only change can serve real data. A Platform reader would read empty tables and must show "unavailable" (correct fail-closed), which is today's outcome with a different cause. Fabricated zero rows, a reconnect of the obsolete database, or a 200 that hides the failure are all excluded.

## Minimal contract

Accept `oteryn-game-public-projections-v1` as written, and implement only two of its six families for the first slice: `highscores` (`PublishWorldHighscoresV1`) and `world_online` (`PublishWorldOnlineV1`), plus `PublishPublicProjectionWatermarkV1` for those two families. Profiles, deaths, guilds and houses stay unavailable until their slices. Transport, identity (a dedicated public-projection client certificate), ordering, epoch fence, watermark and privacy follow the contract; no new wire is proposed.

Open point inside the slice: `highscores` is published per (World, category) from the HIGHSCORES-0 job, and the contract joins each row to `character_profile` (a row is not shown while that family is stale, §9 Content). A highscores page therefore also needs the `character_profile` family, or the contract must allow names inside the highscores rows. This is owner question 2.

## Ownership

| Part | Owner repository | Change |
|---|---|---|
| Contract acceptance | Oteryn-Game | Move the contract from CANDIDATE to accepted for testing and preproduction (exact-head validation, independent review, owner or architect acceptance) |
| Producer | Oteryn-Game | Source queries, outbox, publisher with mTLS, watermark, 10 s `world_online` scan, operator resync command, epoch fence, flag default off |
| Consumer ingest | Oteryn-Platform | Done (C4), unchanged |
| Page reader | Oteryn-Platform | A native `PublicGameDataSource` reader over `PublicProjectionReadModel` for online and highscores (packets C5 and C7), feature tests, pages 503 or a clear unavailable state when a family is stale |
| Deployment and PKI | existing deployment owner | Public-projection certificate, `GAME_PROJECTIONS_PUBLIC_*` env, producer flag; not part of this packet |

## Order of changes

1. Game: accept the contract (docs, needs owner or architect acceptance).
2. Game: producer for `highscores`, `world_online` and the watermark, behind a default-off flag, with tests shared with the Platform wire fixtures.
3. Platform: native reader and page wiring (C5 and C7). It can merge before step 2 because it renders "unavailable" for a missing or stale watermark; it must be merged before the producer is enabled so no data is published unseen.
4. Deployment owner: issue the certificate, set the Platform consumer env, deploy the Game build with the producer disabled, then enable the Platform consumer, then the producer.
5. Acceptance: real HTTP check of `/pl/online` and `/pl/highscores` against the Oracle stack showing live rows and, after stopping the producer, the unavailable state within S (30 s).

## Rollback

- Disable the Game producer flag: the watermark stops, each family goes stale within S and the pages show unavailable. No data loss, because the read models are rebuildable by an operator resync.
- Set `GAME_PROJECTIONS_PUBLIC_ENABLED=false` on Platform: ingest answers 503.
- The Platform reader is read-only and has no migration; revert the PR. The C4 tables are untouched, and their rollback is already refused while Game-published rows exist.
- Never roll back by reconnecting the legacy-engine database or by serving zero rows.

## Owner decisions

Decided by the owner on 2026-10-15 ("zbuduj to"):

1. Q1 yes: Game contract acceptance and a producer for `highscores`, `character_profile`, `world_online` and the watermark are being built in parallel in Oteryn-Game.
2. Q2 (a): highscores rows join `character_profile`.
3. Q3: until a family is fresh the pages show the unavailable view with status 503.
4. Q4: coarse online presence (name, level, vocation) is public by default (U-PP2 default).

## Implementation notes (C5 and C7)

- `App\PublicGameData\NativePublicGameData` is the only reader of both pages. `PublicProjectionReadModel` gained `worlds()` and `entries()` with the same serve-time epoch and watermark checks as `entry()`.
- World: `PUBLIC_GAME_DATA_WORLD_ID` (config `public_game_data.world_id`) or, when empty, the only World that has published. Several Worlds without a value fail closed (503). The World id is never rendered.
- Category ids follow the contract (1 experience, 2 magic, 3-9 fist, club, sword, axe, distance, shielding, fishing). The `level` category reads category 1 and shows the level, `experience` shows the value.
- Vocation ids: the contract only says `vocation` is the A13 build-state id (0..255) and `filter` 1..5 a base vocation, without numbers. Platform follows Game's reference enum (`apps/game-server/src/content/reference_playable.rs`): Druid 1, Knight 2, Monk 3, Paladin 4, Sorcerer 5, None 6; filter n lists base vocation n. The names are `CharacterPresentation::publishedVocationName()`; the legacy ids of `vocationName()` stay for the legacy pages. The producer must pin the same numbers.
- The online page no longer shows the channel (the contract carries none).
- Profile: `PublicProjectionReadModel::entryByName()` counts present holders of the case-folded name (`lookup_folded`, indexed per family) at the fresh epoch with no World filter, requires exactly one, and serves it only when its World is the served World (contract §7.3); otherwise 404. A configured World with no data is 503.
- Round 4: owner preferences are keyed by the native CharacterId (`CharacterGuildVisibility`, `CharacterProfilePreferenceService` over `NativeAccountCharactersReadModel`); the form keeps only the guild toggle and main character. Legacy rows stay unbound (no mapping exists, so no backfill). The three pages carry `Cache-Control: no-store, private` and the `public-game-data` throttle (120 per minute per IP). The unused legacy repository methods are deleted. `PublicCharacterProfileService` and the legacy profile tests are removed.
- Round 5 (Codex on 3173b2d): `CharacterGuildVisibility` proves ownership through `NativeAccountCharactersReadModel::viewForAccount()` (enabled consumer, fresh watermark at the highest epoch, valid snapshot at that epoch listing the character) and requires exactly one live owner and one live Identity; a disabled, stale or epoch-fenced feed, an invalid snapshot or an ambiguous mapping hides the guild even while old rows remain. The native account roster links each listed character to the preference form and shows the saved main-character marker (preferences keyed by `character_id`, only for listed characters); the legacy roster no longer links to the native-only form or reads preferences keyed by the legacy player id.
- Deployment wiring is in this PR (compose, Oracle prepare script, mTLS listener). Remaining on oteryn-vps: certificate, secrets and the flag flip (see the PR comment).

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-10-16T12:00:00Z
head: 3173b2df81f942fe9a1343df872ff20a9b479ccd
branch: fix/public-gamedata-native
pr: 1645
status: validating
context_routes:
  - game-auth
owned_paths:
  - docs/agents/tasks/active/OTERYN-20261015-public-gamedata-native-1.md
  - app/GameProjections/PublicProjectionReadModel.php
  - app/PublicGameData/NativePublicGameData.php
  - app/PublicGameData/CommunityDataPolicy.php
  - app/Http/Controllers/PublicGameData/PublicGameDataController.php
  - app/PublicGameData/CharacterPresentation.php
  - app/PublicGameData/PublicCharacterProfileService.php
  - config/public_game_data.php
  - resources/views/game/highscores.blade.php
  - resources/views/game/online.blade.php
  - resources/views/game/character.blade.php
  - lang/en/community.php
  - lang/pl/community.php
  - deploy/synology/compose.yml
  - deploy/synology/nginx/internal-mtls.conf
  - deploy/synology/README.md
  - deploy/synology/tests/test_platform_mtls_contract.py
  - deploy/oracle/prepare-platform.sh
  - tests/Feature/CharacterProfiles/CharacterProfilePreferenceTest.php
  - tests/Feature/PublicGameData/CharacterProfilePresentationTest.php
  - tests/Feature/PublicGameData/NativePublicGameDataTest.php
  - tests/Feature/PublicGameData/PublicGameDataSourceTest.php
  - tests/Feature/PublicGameData/PublicGameDataTest.php
  - tests/Feature/PublicGameData/CommunityDataCompletenessTest.php
  - tests/Feature/PublicGameData/OnlinePaginationTest.php
  - .env.example
  - app/CharacterProfiles
  - app/GameProjections/PublicProjectionIngestion.php
  - app/Http/Controllers/CharacterProfiles
  - app/Http/Middleware/NoStorePublicGameData.php
  - app/Http/Requests/CharacterProfiles
  - app/Providers/AppServiceProvider.php
  - app/PublicGameData/CanaryGameDataRepository.php
  - config/public_game_data.php
  - database/migrations
  - docs/contracts/CHARACTER_PROFILE_PREFERENCES_CONTRACT.md
  - docs/operations/CHARACTER_PROFILE_PREFERENCES.md
  - docs/testing/product-completeness-benchmark.json
  - resources/views/identity/account/character-profile-preferences.blade.php
  - routes/web.php
  - scripts/acceptance
  - tests
  - docs/agents/LEGACY_TOKEN_BASELINE.json
  - app/Http/Controllers/Accounts/AccountOverviewController.php
  - resources/views/identity/account/overview.blade.php
  - resources/views/identity/account/partials/native-roster.blade.php
proven:
  - Platform consumer and read model for Public Projections v1 exist on main (C4).
  - Game has no producer for game-public routes yet; the contract is CANDIDATE. The owner decided Q1-Q4 on 2026-10-15 and the producer is being built in parallel in Oteryn-Game.
  - The native reader and the online, highscores and character pages, and the deploy wiring, are implemented with feature tests (fresh 200, stale or missing 503, empty 200, profile join, filters).
derived:
  - Until the producer publishes, both pages answer 503 unavailable, which is the honest fail-closed state.
unknown:
  - Whether the Game producer pins the vocation ids to the reference enum (the contract gives no numbers).
conflicts: []
first_failure:
  marker: none
  evidence: none
rejected_hypotheses:
  - Read Game Postgres directly from Platform; rejected by the ownership boundary (ADR-0004 6-7, contract Q3=A).
changed_paths:
  - docs/agents/tasks/active/OTERYN-20261015-public-gamedata-native-1.md
  - app/GameProjections/PublicProjectionReadModel.php
  - app/PublicGameData/NativePublicGameData.php
  - app/PublicGameData/CommunityDataPolicy.php
  - app/Http/Controllers/PublicGameData/PublicGameDataController.php
  - app/PublicGameData/CharacterPresentation.php
  - app/PublicGameData/PublicCharacterProfileService.php
  - config/public_game_data.php
  - resources/views/game/highscores.blade.php
  - resources/views/game/online.blade.php
  - resources/views/game/character.blade.php
  - lang/en/community.php
  - lang/pl/community.php
  - deploy/synology/compose.yml
  - deploy/synology/nginx/internal-mtls.conf
  - deploy/synology/README.md
  - deploy/synology/tests/test_platform_mtls_contract.py
  - deploy/oracle/prepare-platform.sh
  - tests/Feature/CharacterProfiles/CharacterProfilePreferenceTest.php
  - tests/Feature/PublicGameData/CharacterProfilePresentationTest.php
  - tests/Feature/PublicGameData
  - tests/Feature/CharacterProfiles/CharacterProfilePreferenceTest.php
  - deploy
  - lang
  - resources/views/game/character.blade.php
  - .env.example
  - docs/agents/LEGACY_TOKEN_BASELINE.json
  - app/CharacterProfiles/CharacterGuildVisibility.php
  - app/Http/Controllers/Accounts/AccountOverviewController.php
  - resources/views/identity/account/overview.blade.php
  - resources/views/identity/account/partials/native-roster.blade.php
  - tests/Feature/Accounts/NativeAccountOverviewTest.php
  - docs/contracts/CHARACTER_PROFILE_PREFERENCES_CONTRACT.md
  - docs/operations/CHARACTER_PROFILE_PREFERENCES.md
validation:
  - command: python tools/agents/checkpoint.py docs/agents/tasks/active/OTERYN-20261015-public-gamedata-native-1.md --require-checkpoint
    result: PASS
    evidence: run locally before push; legacy token ratchet, task liveness, Control Room and source branch closeout also pass locally
blockers:
  - Game producer (parallel), the certificate and secrets and the flag flip on oteryn-vps before live rows can be accepted.
next_action: Exact-head CI and independent architect review; the CP marks ready; after the Game producer and the deployment steps, run the acceptance HTTP check.
```

## Source branch closeout

```yaml
source_branch_disposition: auto_delete_after_merge
source_branch_reason: ordinary same-repository PR
source_branch_evidence: pending
```

Round 3: `character_profile` carries a required boolean `guild_visible` (ingest rejects a profile without it); the profile page shows the guild row only when it is true. The Oracle SOURCE/image pin cannot name its own head, so it is bumped to the merge commit in the CP slot commit after merge.
