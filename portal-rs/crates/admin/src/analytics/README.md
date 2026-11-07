# Admin analytics consumer — ACCEPTED contract revision 2

The sole wire authority is
[`OTERYN_V2_ADMIN_ANALYTICS_CONTRACT_PROPOSAL.md`](../../../../../docs/contracts/OTERYN_V2_ADMIN_ANALYTICS_CONTRACT_PROPOSAL.md),
including its §10 acceptance record. This consumer never calls Game. Export has
no route (§10.4). No data is read from the opaque `analytics_cache`.

Configuration uses the existing `OTERYN_*` environment style:

| Setting | Default | Meaning |
|---|---|---|
| `OTERYN_ADMIN_ANALYTICS_INGEST` | `false` | Three purpose-scoped POST handlers |
| `OTERYN_ADMIN_ANALYTICS_PRESENT` | `false` | Permit qualified reads, subject to every gate below |
| `OTERYN_ADMIN_ANALYTICS_AUTHORITY` | absent | Exact configured Game source authority |
| `OTERYN_ADMIN_ANALYTICS_PRIVACY` | absent | Exact accepted policy identifier; must match durable floor |
| `OTERYN_ADMIN_ANALYTICS_PEER_SAN` | absent | Dedicated SAN `game-admin-analytics` from verified TLS evidence |
| `OTERYN_ADMIN_ANALYTICS_RATE` | `1200` | Per-peer requests/minute, restricted to 1..6000 |
| `OTERYN_ADMIN_ANALYTICS_STAGING_BYTES` | `268435456` | Global staged payload budget, minimum 8 MiB |

The existing admin host/panel switches, server-side session identity, current
account state, confirmed MFA and effective grants remain mandatory. Only the
built-in `platform_admin` role may hold either new permission. Both API role
editing and SQL constraints enforce this; grants are read on every request.
The shared panel host guard returns 404 before session or audit work on a wrong or
duplicate Host header, disabled panel or disabled presentation. Successful reads
and denials with a resolved staff actor are audited through `audit::append`;
anonymous probes do not write audit rows.
No aggregate values or free query text enter audit metadata. Audit failure denies
release. Every response is `Cache-Control: no-store, private`.

## Activation is gated, even when presentation is switched on

Under §§5.3 and 10.6, Game must propose and obtain acceptance for the exact resync
manifest/control envelope, coverage/absence proof and recovery SLA in its producer
PR. **No such envelope is implemented or invented here.** Pages, commits,
watermarks, a process restart or an empty database can never accept a baseline.
Generations begin off-path with no accepted baseline. Reads join the durable epoch
and privacy fence, the active generation and its explicit accepted-baseline
provenance, the subject revision and the fresh watermark. With presentation
enabled but without that evidence, the pages and JSON report **unavailable**.
Disabled presentation returns 404 before session or audit work. There is no runtime endpoint or setter that accepts a baseline.
The future accepted resync verifier owns baseline acceptance and the atomic switch;
it must use these tables and locks, not infer coverage from received pages.

A reconciled external analytics epoch fence and accepted privacy floor must already
exist in `admin_analytics.fence` for the configured authority. Missing fence evidence
returns 503; ingestion does not initialize or reconstruct it. Accepted higher authenticated epochs fence old generations; rate-limited or
privacy-policy-refused messages never move the fence or active generation. No
handler resets or lowers the high-water mark.
No fence row is created until the contract resync envelope and baseline are
accepted (§§5.3, 10.6); activation remains blocked and missing evidence fails closed.
Operator provisioning/restoration of that evidence belongs to the accepted resync
procedure, not to a seed or mock producer in application startup. Within-epoch
rebuilds also require accepted manifest evidence. Consumer test fixtures that mark
an accepted baseline are explicitly test-only and do not qualify that procedure.

## Storage, cleanup and qualification

Migration `20261107140001_admin_analytics.up.sql` is additive and revokes public
schema/table access. Its explicit inverse is
`portal-rs/migrations/20261107140001_admin_analytics.down.sql`, run by an
operator only after disabling presentation and ingestion. The inverse removes
this consumer, its grants and its schema; ordinary rollback of a feature switch
preserves fences. This follows T45's reversible SQLx migration layout. Integration
fixtures skip down migrations when building a schema; the rollback/reapply test
explicitly executes this down file and then the up file.
The application database principal owns the private schema; public readers must
not receive schema USAGE or SELECT. Production least-privilege principal mapping
still requires qualification (§8).

Hourly maintenance removes 24-hour staging and expired payloads, independently
of traffic. Reads enforce 30-day hourly/400-day daily expiry, the 24-hour embargo,
current privacy, epoch, baseline, full selected bucket coverage and <=120-second
watermark freshness without a stale grace period. Empty/suppressed/incomplete
rows never imply zero. Corrections replace entire subjects under a serialized
transaction; acknowledged snapshots are never evicted for staging capacity.
Non-payload receipts/fences survive payload cleanup and generation replacement.

Remaining activation evidence is defined by §§8, 10.6–10.8: accepted producer and
resync envelope, producer/consumer fixture matrix, measured full retained-set
replay and disk capacity, PKI/purpose mapping and clock/SLA qualification,
deletion/restore, payload-backup expiry within 30 days after purge and the
400-day access-audit retention procedure through the protected audit boundary.
These operational obligations do not become production-qualified from unit tests.
Nothing here enables ingestion, presentation or deployment by default.

For the unavailable-state panel E2E, start the existing test-only local launcher
with `OTERYN_ADMIN_ANALYTICS_PRESENT=true`, a synthetic
`OTERYN_ADMIN_ANALYTICS_AUTHORITY` and `OTERYN_ADMIN_ANALYTICS_PRIVACY`. Leave
ingestion off and create no fence/baseline. Production defaults remain OFF.

## Revision 3 economy

The five economy members share the existing ingestion switches, envelope,
transactions, fences, retention and baseline gate. `low_n` retains its numeric
values only for economy; economy `suppressed` is rejected. Fixed row sets and
paired Store currencies are checked across the entire committed snapshot.
The economy page is `/admin/analytics/economy` (and `/data`), limited to one
world and UTC day. It never sums rows or combines currency units with gold.
The four contract §11.6 example bodies are consumer-owned byte-exact vectors;
the test records all four SHA-256 digests. No Game source files are included.

Migration `20261107170001_admin_analytics_economy` changes only the private
aggregate row-state constraint. Disable both switches before rollback. Its
inverse preserves fences, receipts and rev-2 data, and refuses to erase or
relabel existing `low_n` payloads: those must first drain through the retention
or an explicitly authorized operator procedure. Rollback/reapply and refusal
with existing `low_n` data are covered by PostgreSQL tests.

### Platform purchased source gap

The base wallet is one unnamed transferable coin. Its operation kinds are
`transfer`, `adjustment`, `reserve`, `release`, `capture`, and `store_purchase`;
the last is a negative in-game purchase debit. There is no purchase-credit
classification or currency dimension. Payments' `TopUpJob` explicitly describes
a future wallet handler; payment intents and requested top-ups do not prove
wallet credit. Consequently the page labels `purchased` as **incomplete** with
this named Platform source gap, including for empty days. It does not invent a
zero or count operator adjustments as real-money purchases. The wallet schema
and ledger remain untouched. A qualified credit classification, coin currency
mapping and capture coverage are needed before numeric purchased aggregation
and its empty-day test can be delivered. Owner clarification is pending.
