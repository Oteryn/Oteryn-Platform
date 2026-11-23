# G-10 inventory and economic history

The [revision 3 contract](../contracts/OTERYN_V2_GAME_JOURNAL_ITEMS_CONTRACT_PROPOSAL.md) accepts the wire and privacy semantics. Production activation requires separately qualified producer and consumer evidence.

## Platform behavior

- **Account → Inventory and history** opens `/{lang}/account/journal/items` in Polish and English. Character destinations are derived from current receipt-backed ownership. Bank history uses the account and world independently of character ownership.
- Inventory shows a timestamped logout snapshot. Search matches item definition keys across equipment, depots and inbox, or within the selected location. Item rows show their slot, depot town and box, or immediate container when present. House custody remains unavailable pending its mapping.
- A snapshot estimate uses source-labelled unit prices and checked arithmetic across all rows, including rows outside the current search. A missing price leaves the overall estimate unavailable. Bank, market and trade entries do not contribute to the estimate.
- When Bazaar is enabled, an item whose accepted catalogue definition is tradeable can link to Bazaar. Bazaar applies its own eligibility and current-custody checks.
- History shows committed bank entries, market events and player trades within the proven UTC interval. A next-page cursor identifies the last record on the page. Each request checks the current scope and verifies the cursor against the retained, covered records for that scope.
- Empty results from verified evidence have their own messages. When coverage, ownership evidence or catalogue mapping is unavailable, the corresponding view remains unavailable.

## Operator status

At startup, the portal emits one `G-10 journal` status line. It reports whether inventory and history reads are enabled, whether ingest was requested, and whether the source setting is present. `readiness=awaiting_qualified_game_evidence` is the activation status. The line contains neither journal contents nor peer identities. An ingest request still goes through the internal listener's environment, identity and family checks.

`PLAYERDATA_G10_ITEMS_ENABLED` and `PLAYERDATA_G10_HISTORY_ENABLED` default off. Keep them off in production until the consumer qualification is complete. `PORTAL_PLAYERDATA_G10_INGEST_ENABLED` also defaults off. The account landing page remains available and displays the unavailable state for disabled views.

## Qualification still required

- Game must supply repeatable durable logout snapshots with complete equipment, depot and inbox coverage, and confirm the supported house custody and placement mapping.
- Game must supply accepted typed item definitions and the source evidence required to resolve valuation inputs to the same definition revision. Missing estimates can remain unavailable independently of inventory coverage.
- Game must qualify committed BANK chains, MARKET updates and both private trade leaves, including retained coverage, replay, transfer and recovery fencing. BANK baseline and replacement evidence must follow the accepted generation rules and the jointly agreed completeness protocol.
- Against an authorized producer, qualify the production T72-to-T76 read adapter on the accepted baseline and catalogue mapping. Exercise a retained bank entry, a complete and an empty logout, and a transfer that hides earlier character history while preserving the original account's bank history. Confirm lost or ambiguous evidence returns unavailable. Obtain the activation decision before changing production flags.
