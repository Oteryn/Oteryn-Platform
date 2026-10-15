---
task_id: OTERYN-20261015-public-gamedata-native-1
governing_issue: 1474
required_reads: []
search_first:
  - app/GameProjections/PublicProjectionReadModel.php
optional_reads:
  - docs/agents/programs/DECANARY_REMOVAL_PLAN.md
---

# OTERYN-20261015 public online and highscores on native data

## Goal

Governing GitHub Issue: #1474 (native-only migration) is the canonical lifecycle authority for this task. The top-level `governing_issue` frontmatter field is machine-required while this packet remains active.

Decision input only (docs, no code). `https://oteryn.com/pl/online` and `/pl/highscores` answer 503 because `PublicGameDataController` reads the obsolete Canary MySQL tables through `CanaryGameDataRepository`. This packet records what native data exists, why a Platform-only reader is not possible, and the minimal cross-repository change set that makes both pages serve real data. It implements nothing.

## Acceptance criteria

- [x] The existing native data and contracts are inventoried with evidence.
- [x] The minimal contract, ownership, order of changes and rollback are stated.
- [ ] The owner decides the open questions below.

## Findings

PROVEN (read-only, Platform `origin/main` 0628d22 and Game `main` 88d6b8885):

- Platform consumer exists. Packet C4 (`OTERYN-20261011-proj-ingest-1`) ships `POST /internal/v1/game-public/{highscores,character-profile,character-deaths,guild,house,world-online,watermark}`, the read-model tables `game_public_projection_snapshots` and `game_public_projection_watermarks`, and the serve-time freshness gate `App\GameProjections\PublicProjectionReadModel`. It is default-off (`GAME_PROJECTIONS_PUBLIC_ENABLED`, testing and preproduction only).
- No page reads that read model. `PublicGameDataController` and `PublicGameDataSource` still point at `CanaryGameDataRepository`; with `PUBLIC_GAME_DATA_SOURCE=native` the pages only show "unavailable". Removal-plan packets C5 (highscores, profiles) and C7 (online) are not built.
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
- Never roll back by reconnecting the Canary database or by serving zero rows.

## Owner questions

1. Approve Game contract acceptance and a producer allocation for `highscores`, `world_online` and the watermark? Recommendation: yes; it is the only route that respects the ownership boundary.
2. Highscores page needs `character_profile` as well (joined by character id, contract §4.1). Options: (a) also build the profile family in the first slice, (b) amend the contract before acceptance so highscores rows carry the public name and vocation. Recommendation: (a), because it needs no contract change and unlocks the profile page.
3. Until the producer ships, should the two pages return the unavailable view with 503 as now (recommended, honest) or a "coming soon" page with an explicit non-200 status?
4. U-PP2: is coarse online public by default? The online list stays empty or hidden until the privacy preference default is decided.

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-10-15T12:00:00Z
head: UNKNOWN
branch: fix/public-gamedata-native
pr: 1645
status: waiting
context_routes:
  - game-auth
owned_paths:
  - docs/agents/tasks/active/OTERYN-20261015-public-gamedata-native-1.md
proven:
  - Platform consumer and read model for Public Projections v1 exist on main (C4).
  - Game has no producer for game-public routes; the contract is CANDIDATE and states nothing is implemented.
derived:
  - A Platform-only native reader cannot serve real data; it would only render unavailable.
unknown:
  - Whether the owner accepts the contract and allocates a Game producer.
conflicts: []
first_failure:
  marker: none
  evidence: none
rejected_hypotheses:
  - Read Game Postgres directly from Platform; rejected by the ownership boundary (ADR-0004 6-7, contract Q3=A).
changed_paths:
  - docs/agents/tasks/active/OTERYN-20261015-public-gamedata-native-1.md
validation:
  - command: python tools/agents/checkpoint.py docs/agents/tasks/active/OTERYN-20261015-public-gamedata-native-1.md --require-checkpoint
    result: NOT_RUN
    evidence: run before push
blockers:
  - Owner decision on Game contract acceptance and producer allocation.
next_action: Owner answers the questions above; then open the Game producer allocation and the Platform C5/C7 packet.
```

## Source branch closeout

```yaml
source_branch_disposition: auto_delete_after_merge
source_branch_reason: ordinary same-repository PR
source_branch_evidence: pending
```
