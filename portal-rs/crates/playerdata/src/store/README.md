# Private journal core

The governing wire proposals are G-9, G-10 revision 2 and G-11 revision 2 in
`docs/contracts/OTERYN_V2_GAME_JOURNAL_*_CONTRACT_PROPOSAL.md`.
The proposals' tighter privacy and retention rules take precedence over the plan's
older retention table. There is no corresponding Laravel journal endpoint; its
existing internal guard/canonical conventions are reused through `internal-api`.

`Store` commits canonical-payload storage, independent immutable identity checks,
and contiguous stored/applied cursors in one PostgreSQL transaction before ACK.
Delivery above the next revision is refused; bounded sequential delivery is used
instead of speculative staging. A watermark separately publishes a proven prefix,
including an explicitly proven empty baseline. An ACK never asserts freshness,
completeness, ownership, a game mutation or economic settlement.

G-10 logout BEGIN/PAGE/END remains hidden until all counts, page digests, global
ordering, roots, parent graph and custody coordinates validate. A new complete cut
replaces the previous one atomically. Source history is not combined across
families. Bank chaining and balances are checked without treating journal gold as
spendable currency. Catalog interpretation, stack rules, equipment slot mapping,
house eligibility and authoritative source coverage require `Lease::validate`.

Every feature flag defaults to false and the family allowlist defaults to empty.
`http::router` is a composition seam for the trusted private listener; it must
never be mounted on a public listener. It uses verified connection extensions,
the shared purpose IdentityMap, bounded transport/JSON and a shared request budget.
The TLS adapter must also enforce the contract's per-peer budget before transport
decoding. Responses, including errors, use no-store; errors have empty bodies.
No request headers can supply TLS provenance. No public read route is introduced.

T15 guards and the T24 public ingester are already in the base; neither has a
separate remote task branch. The guard is reused. Public projections cannot
establish private ownership: even the LCFA API explicitly says it never proves
ownership. The private seam is `Authority::acquire`, which must hold a current
fence until its lease is dropped.
It must prove source/account partition, accepted initial baseline or the entire
transfer-receipt cut, current recovery/epoch and privacy fences, source coverage,
and source/catalog interpretation. Pending, unknown or ambiguous facts deny.
The cutoff writer must serialize with that same fence. The default adapter and
clock always deny; ambient environment variables cannot replace these proofs.
T44 supplies account overview, not journal ownership authority.

The owner read API accepts an account resolved by authenticated session code,
never a browser-selected tenant. Anonymous, foreign-account and staff actors are
refused. Staff permissions alone do not create a read exception under these
proposals. Every read reacquires authority. G-10/G-11 old character cuts are hidden
from both seller and buyer; the buyer requires a separately proven new baseline.
No seller payload is rebound to a buyer. Bank history stays account-scoped.

The trusted transfer writer calls `erase_character` under its exclusive authority
fence to purge character payloads immediately. Leases are acquired before journal
transaction locks, so cutoff writers cannot deadlock with ingest/read leases.

`expire` purges expired payloads and ages G-10 digests into a durable replay floor;
G-11 digests remain while active and age out 90 days after erasure. `erase_account`
immediately hides streams and purges payloads. Lifecycle/retention adapters must
invoke these APIs with trusted time; no background job or account lifecycle is
implicitly enabled. Raw request archives are not retained. Infrastructure must
provide encrypted storage/backups and enforce the proposals' backup expiry and
restore-fence requirements before activation. Superseded current-state payloads
expire within seven days; historical G-11 payloads expire after 90 days and game
login payloads after 30 days. G-10 snapshots/history expire after 30 days and
incomplete staging after 24 hours. Expired content never becomes a proven empty
history, and exact replay cannot recreate an expired payload.

## Deliberate refusals and remaining activation prerequisites

G-9's supplied revision-1 schema conflicts with the task's revision-2 character
revision and entry/logout/area/inactivity boundaries. No revised closed schema has
been supplied. G-9 records therefore return unavailable rather than accepting the
old ownership field or inventing revision-2 fields. Party capture, replay commands,
sharing and public aggregates remain unavailable. The G-11 house-rent proposal
explicitly has no accepted payload schema; it is unavailable too.

Activation requires the revised G-9 schema, accepted per-family proposals,
producer fixtures, authoritative transfer/restore/baseline and clock adapters,
private-listener composition and lifecycle/retention scheduling. No game runtime
command or game-side producer is implemented or enabled by this crate.

Tests read `PORTAL_TEST_DATABASE_URL`; without it database tests print a skip notice.
Each database test creates an isolated schema and applies only this task's migration.

## Workspace qualification blocker

The scoped suite passes eleven tests, including eight PostgreSQL tests. Full
workspace formatting and Clippy pass. The full workspace test run currently has
four failing targets in gameauth/gateway: existing dependencies select both
Rustls crypto providers, while gateway's client builder selects neither explicitly.
The same conflicting feature graph exists with playerdata excluded. The isolated
gameauth listener test passes with its smaller feature graph. Changing the gateway
TLS builder or those dependency owners is outside this task's allowed paths.
The branch requires coordinator reconciliation before integration.
