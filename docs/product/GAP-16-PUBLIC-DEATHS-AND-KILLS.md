# GAP-16 · Public deaths and daily kills

| Feed | Contract decision | Configured portal behavior |
| --- | --- | --- |
| G-4 character deaths and killers | Revision 4 wire accepted. Q7 activation qualification is open. | The existing deaths serving flags expose the localized unavailable state while qualification is pending. Character and world lists use the same privacy and freshness rules. |
| G-8 UTC monster and boss kills | Revision 2 proposed. Q3 counting and source semantics and coordinator wire acceptance remain open. | World statistics shows daily kills as unavailable. A qualified composition can render complete and explicitly provisional counts from the existing V2 read model. |
| World character-death observations | Same-day observations under current privacy. | Counts remain labeled as partial observations, separate from physical monster and boss deaths. |

See [G-4 revision 4 and Q7](../contracts/OTERYN_V2_PUBLIC_GAME_FAMILIES_EXTENSION_CONTRACT_PROPOSAL.md) and [G-8 proposal and Q3](../contracts/OTERYN_V2_STATISTICS_FAMILIES_CONTRACT_PROPOSAL.md).

## Player and operator views

Players can open **Recent deaths** from Community and **World statistics** from World when those sections are enabled. The deaths list accepts an exact registered world name and an optional inclusive UTC date range within the current retention window. The world filter uses each character’s current eligible profile because the death record carries no world at occurrence. A character link opens that character’s list. World statistics accepts a registered world ID and up to 31 inclusive UTC dates ending today or earlier. A missing daily kill sample stays unavailable. An explicit zero is displayed only when the qualifying feed supplies it.

On the configured staff host, open **Public world projections** in the staff panel (`/admin/public-projections`) for the current qualification and switch status. This read-only view requires the existing staff session, MFA and game catalogue access. It reports categorical state and does not expose source identifiers or projection payloads. These switches do not certify a Game producer, privacy floor, watermark or coverage.

## Activation evidence still required

- G-4 requires an exact Game producer revision with the tagged-killer fixtures, committed receipt and privacy/lifecycle suppression triggers, purpose-separated mTLS, and a successful retained-set baseline plus tail. Measure the stopped-node resync bound, prove watermark stalls and privacy revocation, agree retention and record the Q7 decision before configuring serving. Keep the localized unavailable state while that evidence is missing.
- G-8 requires the Game owner to accept the wire and physical-death counting rules, then implement the durable authoritative count source and publisher. Qualify deduplication across channels, disjoint boss and non-boss totals, UTC corrections, tombstones, complete rebuild and watermark coverage. The configured portal has no accepted coverage adapter for this proposal. Platform must attach a trusted, current coverage adapter and an accepted configuration before enabling the view.
- Production requires separate approval of the exact producer/consumer revisions, PKI, freshness and rollback evidence. Disable presentation before rolling back ingest. Preserve monotonic epoch and privacy floors, and never reconstruct missing values from bestiary, character death counts or old snapshots.
