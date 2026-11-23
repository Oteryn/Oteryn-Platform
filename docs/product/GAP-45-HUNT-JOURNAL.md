# G-9 hunt journal qualification

The [accepted G-9 contract](../contracts/OTERYN_V2_GAME_JOURNAL_SESSIONS_CONTRACT_PROPOSAL.md) governs the wire and privacy scope. Game owns measured journal facts. Platform reads the verified prefix and calculates private estimates. Production activation requires evidence from both sides.

## Player and operator views

- Account → Hunt session analyzer opens `/{lang}/account/hunts` when `OTERYN_PLAYERDATA_HUNTS_READ=true`. The history filters by character, place, start interval and party participation. Select two results for comparison. Place totals describe time spent at each measured place. Segment-wide experience and loot cannot be attributed to individual places.
- A recap requires durable logout, every segment index, every sealed rare-drop occurrence and application of the enclosing stream prefix. The view rechecks current character ownership, admission, restore, deletion and retention evidence. The list shows unavailable when it has no verified rows, including while capture is in progress. The browser bounds each recap to 1,024 records and 2 MiB of source data. Over-budget recaps are withheld intact.
- A rare drop appears in the session's history and remains part of measured loot. Party ledgers use anonymous slots within each capture segment. The gross-loot split requires a price for every item in the ledger. It calculates advice in gold and never transfers gold.
- The detail view accepts one price choice per item and purpose. A custom price applies to this request. NPC quotes require the active published content commit to match the accepted journal catalogue revision and the NPC offer to match the item definition and trade direction. Bazaar estimates use the median from seven completed UTC days of consented, completed gold item sales for the same world and definition revision. The estimate needs at least five distinct buyers and sellers. Missing price evidence keeps estimates unavailable.
- The share URL is an owner-only link. It rechecks access to the recap on each visit. G-9 v1 leaves bearer disclosure disabled.
- Staff with `admin.access` and current staff MFA can open `/admin/hunt-journal` on the configured staff host. The page shows which journal settings are present. Configuration alone cannot qualify a source.

## Acceptance before enabling production reads

1. Keep `PORTAL_PLAYERDATA_G9_INGEST_ENABLED`, `PORTAL_PLAYERDATA_G9_REPLAY_COMMANDS_ENABLED` and `OTERYN_PLAYERDATA_HUNTS_READ` off in production during qualification. In isolated testing or preproduction, bind `PORTAL_PLAYERDATA_JOURNAL_SESSIONS_SAN` to the dedicated peer and `PORTAL_PLAYERDATA_G9_SOURCE_AUTHORITY` to the accepted authority. Use the existing stream and catalogue admission process, including the current ownership and restore cut. Set `PORTAL_PLAYERDATA_G9_PARTY_PRIVACY=accepted` only in an environment whose allocated source has documented Game-owner and privacy acceptance for anonymous party capture. Leave it unset by default.
2. Publish an ongoing hunt from the qualified producer, complete it with durable logout, and deliver its sealed rare-drop records. Verify the owner's exact totals, drop history and anonymous party quantities. Compare two completed sessions. Reopen the owner link and confirm another signed-in account cannot open it. Repeat after an ownership fence changes.
3. Retry the exact accepted bytes and confirm one set of totals. Send conflicting bytes using the same segment identity and confirm conflict refusal and that the affected prefix becomes unavailable. Exercise out-of-order delivery, an expired replay range and a restore.
4. Confirm price provenance and unavailable states for incomplete evidence in Polish and English at desktop width and 390 px. Keep reads disabled if producer, privacy, retention, restore or consumer evidence fails qualification.

## Remaining Game evidence

The Game owner must supply a qualified producer with durable logout across disconnects and crashes, area-entry/exit and kill-idle boundaries, attributable counters and rare drops tied to a pinned catalogue, and approved anonymous party capture. Producer qualification also covers stable identities, atomic capture and outbox, purpose-bound mTLS, exact retries, replay receipts and restore-generation fencing. Any sharing beyond the owner requires a separately accepted scope and contract.
