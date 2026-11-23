# GAP-35 World daily qualification

G-12 revision 3 was accepted as a wire and disclosure contract. Producer delivery, consumer qualification, and production activation remain open. See the [G-12 acceptance record](../contracts/OTERYN_V2_WORLD_DAILY_FAMILY_CONTRACT_PROPOSAL.md#acceptance-record-revision-3).

## Player and operator views

- Today in the Community footer shows editorial content alongside the World daily section. The World selector is limited to registered Worlds in the configured publication scope. The current view shows the fenced boost, published boost history and committed fixed-calendar raid occurrences from the current reset interval.
- The history date is a UTC calendar date (`YYYY-MM-DD`). Boost history is filtered by interval start and raids by committed firing time. Current boost information still describes the live reset interval. A date wholly before the boost coverage floor or after today is unavailable. Raid coverage stays limited to the current reset interval. Unknown slots and future raids are unavailable. A genuinely empty covered result has its own message.
- The JSON and HTML fragment routes under `/today/data` and `/today/widget` use the same selection and freshness rules as the page. Malformed input returns 400. Unavailable evidence or an out-of-scope World returns 503 on the fragments, and the editorial page remains readable.
- Administration → World daily projection (`/admin/today`) requires the existing staff host, session, MFA, `admin.access`, and `game.world.read`. It shows ingest and serving switches, source configuration, and eligibility for the selected World. The completeness timestamp appears only for eligible evidence. The page reads Platform state and offers no operational actions.

## Qualification and activation

Keep `PORTAL_PUBLIC_PROJECTIONS_ENABLED`, `PORTAL_WORLD_DAILY_INGEST_ENABLED`, and `PORTAL_WORLD_DAILY_PUBLIC_ENABLED` off by default. The existing restricted-environment checks allow testing/preproduction only. In a separately authorized qualification environment, configure the existing publisher identity, source authority, and publication World scope. Verify the exact peer revisions, baseline coverage for all Worlds, freshness and epoch rollover, catalogue approval, bounds and replay, withdrawal, and recovery using the accepted producer and shared golden fixtures. Confirm that withholding a watermark or invalidating a generation removes both the page and fragment data while editorial content stays readable.

## Game delivery still required

- Deliver the separate G-12 lane after V2 PUBPROJ and supply the exact producer revision and shared fixtures.
- Publish committed reset intervals using the matching calendar revision, bounded boost history and its coverage floor, and fixed-calendar raid occurrences classified against each firing's bundle. Publish exact committed slots only when supported by durable evidence.
- Extend the transactional V2 outbox, watermark, external recovery fence and operator resync to the full World scope, including withdrawals and a complete post-restore baseline.
- The accepted contract excludes future raid slots and prediction outcomes. Upcoming raids can be populated after Game and Platform accept and qualify a separate disclosure contract for them.
