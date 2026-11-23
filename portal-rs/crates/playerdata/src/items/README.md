# Private item projections (T76)

Authority: `OTERYN_V2_GAME_JOURNAL_ITEMS_CONTRACT_PROPOSAL.md`, revision 3.
G-9 and G-11 remain separate families and are never interpreted as item custody.
The Laravel application has no equivalent private item-journal route. The Game
reference was inspected read-only at `857f15f376907c592bb28d854fccff18a15cdf87`;
fixtures in the tests are synthetic proposal mappings, not copied game data.

## Integration seam

T14b was merged before implementation. T72 and T74 had no remote branches at
admission. `Evidence` is the deliberately narrow seam for their public APIs:

- Resolve the current source/world/character receipt cut server-side.
- Lock authoritative account, ownership, privacy and restore evidence in the
  supplied PostgreSQL transaction until commit. There is no staff override.
- Prove registration, initial baseline, contiguous retained coverage and safe UTC.
  BANK belongs to the account/world independently of a character transfer.
- Resolve exact definition family/key/revision, quantity and container rules;
  supply optional, source-labelled estimates through T74. Unknown prices remain
  unknown and invalidate the total; an unknown definition denies activation.
- Send trusted erasure/transfer events to `Projection::erase`, and schedule
  `cleanup` using the latest plausible trusted UTC. Erasure fences and removes
  payloads atomically; normal retention keeps minimal replay metadata for 31 days
  after payload expiry, then advances a durable floor before removing it.

G-10 records reach the platform only through T72's journal listener
(`crate::journal`, `journal/{records,cursor}` on the internal mTLS listener);
this module has no HTTP ingest of its own. With `PORTAL_PLAYERDATA_G10_SOURCE_AUTHORITY`
set, reads use T72's ownership evidence (`crate::authority::JournalEvidence`),
which proves the current owner and cut and still answers 503 until a G-10
baseline is verified and the catalogue mapping is accepted.
The ingest flag defaults to false. No gameplay command or synchronous Game query
exists. Cursor responses acknowledge durable storage, not snapshot visibility.

Public reads use the existing `BrowserSession` middleware. Presentation flags
`PLAYERDATA_G10_ITEMS_ENABLED` and `PLAYERDATA_G10_HISTORY_ENABLED` default false.
All responses use private no-store, including denials and malformed queries.
Items are labelled as logout state with UTC time, never live inventory. Estimates
include every custody row once and never add historical bank/market/trade flows
or held credit. Search filters the displayed rows, not the asset estimate.

## Fail-closed choices and activation prerequisites

- House keys, eligible custody and slot/definition mappings await joint producer
  acceptance. House rows and nonempty house manifests are refused. An authoritative
  proof is required even for snapshots declaring no eligible houses.
- Revision 2's five receipt-cut fields are used together, including recovery
  generation. Initial ownership needs an accepted equivalent baseline. Unknown,
  pending, sold, erased and restoring states deny both ingestion and serving.
- BANK v1 accepts only DEPOSIT, WITHDRAW and TRANSFER, with the stated arithmetic,
  ceilings and native chain. Without an accepted partial-ledger baseline shape,
  the first stored entry must prove the native ledger origin.
- A source gap durably fences its stream without advancing its cursor. Recovery
  needs T72's verified replacement generation; no administrative cursor skipping.
- Browser history reads use bounded pages and revalidate the cursor in the
  current owner, world, family and retained coverage interval.
- The optional Bazaar link is navigation only, requires a tradeable catalogue
  definition plus its independent default-off flag, and conveys no current custody,
  reservation or listing authority. Bazaar history remains owned by Bazaar;
  no fabricated receipt family is created from market or trade events.
- Production activation, journal peer binding, cleanup scheduling, backup expiry,
  accepted baseline/transfer fencing and real producer qualification remain T72 /
  coordinator integration prerequisites. This implementation does not close G-10.

## Validation

Tests cover canonical positive and negative proposal mappings, complete/empty
logout cuts, page digests, roots and parent graphs, bank arithmetic and chains,
market held credit and two-leaf trades, owner/foreign/unknown access, no-store
HTTP, exact retries and concurrent publication, source gaps, retention, unpriced
assets and erasure that preserves account bank history across character transfer.
PostgreSQL tests use isolated schemas and `PORTAL_TEST_DATABASE_URL`; when absent,
each prints an explicit skip notice. No sqlx query macros are added.

## Operator status

At startup, the portal emits one `G-10 journal` status line. It reports whether inventory and history reads are enabled, whether ingest was requested, and whether the source setting is present. `readiness=awaiting_qualified_game_evidence` is the activation status. The line contains neither journal contents nor peer identities. An ingest request still goes through the internal listener's environment, identity and family checks.

`PLAYERDATA_G10_ITEMS_ENABLED` and `PLAYERDATA_G10_HISTORY_ENABLED` default off. Keep them off in production until the consumer qualification is complete. `PORTAL_PLAYERDATA_G10_INGEST_ENABLED` also defaults off. The account landing page remains available and displays the unavailable state for disabled views.

## Qualification still required

- Game must supply repeatable durable logout snapshots with complete equipment, depot and inbox coverage, and confirm the supported house custody and placement mapping.
- Game must supply accepted typed item definitions and the source evidence required to resolve valuation inputs to the same definition revision. Missing estimates can remain unavailable independently of inventory coverage.
- Game must qualify committed BANK chains, MARKET updates and both private trade leaves, including retained coverage, replay, transfer and recovery fencing. BANK baseline and replacement evidence must follow the accepted generation rules and the jointly agreed completeness protocol.
- Against an authorized producer, qualify the production T72-to-T76 read adapter on the accepted baseline and catalogue mapping. Exercise a retained bank entry, a complete and an empty logout, and a transfer that hides earlier character history while preserving the original account's bank history. Confirm lost or ambiguous evidence returns unavailable. Obtain the activation decision before changing production flags.
