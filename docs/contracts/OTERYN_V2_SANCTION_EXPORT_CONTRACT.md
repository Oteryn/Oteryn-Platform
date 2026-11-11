# Sanction export V1

Owner decision: 2026-11-11, N009. Platform owns decisions. Game pulls a
player-visible record projection; it does not enforce decisions from this feed.
There is no shared database, outbound game call, game credential or SQL projection
writer in this implementation. The transport described in the read-only Game
reference `PLATFORM_SANCTION_LOYALTY_EXPORT_V1.md` is superseded for this producer
by the owner's HTTP pull decision. Its record fields and privacy rules are retained.

## Transport and configuration

`POST /internal/v1/support/sanctions/stream/read`, on the existing private
TLS 1.3 / HTTP/1.1 listener only. The listener verifies the client certificate
chain and extracts its single DNS SAN. Headers cannot supply peer provenance.

`PORTAL_SANCTION_EXPORT_ENABLED=true` enables reading; absent or any other value
keeps it off. `PORTAL_SANCTION_EXPORT_IDENTITY` is the exact lowercase DNS SAN of
its dedicated client certificate. It must differ from every other internal
identity. The production factory includes all environment variables ending in
`_IDENTITY`, `_IDENTITIES` (JSON object keys), or `_SAN` in overlap detection;
explicit composition must supply the complete shared `IdentityMap`.

Disabled, missing, malformed or overlapping identity configuration returns empty
503 before reading the body. Missing or wrong verified identity returns empty
401. The public listener supplies no verified identity, so cannot read this feed.
The existing `OTERYN_INTERNAL_TLS_*` listener configuration remains required.
Storage/encoding failures and a three-second read timeout return empty 503.

Requests require a single `Content-Type: application/json`, no transfer encoding,
and absent or identity content encoding. Maximum body: 256 bytes, JSON depth 4.
Malformed, oversized, duplicate/extra-member or otherwise invalid requests return
empty 400. No query parameters are accepted. A shared per-process peer budget
allows 1200 authenticated attempts per minute, including invalid bodies that
passed the shared JSON guard; exhaustion returns empty 429 with `Retry-After: 60`.
All responses carry `Cache-Control: no-store, private`. Success is application/json,
maximum 262144 bytes (256 KiB). No browser surface, translations or page changes are involved.

## Ordered JSON

Request members are `schema`, `after`, `limit`; insignificant whitespace and member
order are accepted. Schema is `oteryn.sanction_stream_request.v1`; `after` is null
or a cursor string; `limit` is an integer from 1 through 500. Floats, exponent
notation, duplicate members and unknown members are refused.

Response member order is exactly `schema`, `items`, `next_cursor`. Schema is
`oteryn.sanction_stream_page.v1`. Items are ordered by publication position, ascending.
Every item has these members in this exact order:

| Member | Rule |
| --- | --- |
| sanction_stream_id | UUIDv7, stable sanction ID |
| decision_revision | positive signed 64-bit JSON integer, strictly increasing per stream |
| account_id | UUIDv7, immutable per stream |
| character_id | game character UUIDv7 or null, immutable per stream |
| kind | integer 1 mute, 2 jail, 3 char_ban, 4 acct_ban, 5 namelock, 6 notation; immutable per stream |
| reason_code | integer 1..65535, public reason category |
| reason_text | 1..120 UTF-8 bytes, no Unicode control characters |
| starts_at_ms | positive Unix milliseconds |
| ends_at_ms | null or Unix milliseconds greater than starts_at_ms |
| revoked | boolean; true remains true for subsequent records in this stream |

Kinds 1, 2, 3 and 5 require a character; kind 4 requires null. Kinds 1 and 2
require an end. Kinds 5 and 6 have no end. These constraints match the reference
consumer. UUIDs use lowercase canonical spelling. Output has no whitespace or
trailing newline, unescaped UTF-8 and slashes, integer JSON numbers.

`next_cursor` is the last emitted publication cursor. It is non-null for every
non-empty page and null for an empty page. Cursors are 1..256 printable ASCII
characters; this producer emits `v1:<positive canonical signed-64-bit decimal>`
(4..22 bytes). Persist a non-null cursor only after durably applying its page;
on an empty page retain the previous request cursor for subsequent polling.
Retry identical input safely, apply revisions in ascending order, and use
`(sanction_stream_id, decision_revision)` for idempotency. An empty page is caught
up, not evidence that no sanction exists. Cursors contain no identity, reason or
timestamp, have no expiry, and history is not pruned.

The consumer requests limit 200 and caps bodies at 256 KiB. `limit` is a record
ceiling, not a promise of a full page: this producer also applies a 256 KiB byte
budget and can return fewer records with a non-null continuation cursor. A
500-item page with 120-byte unescaped ASCII reasons fits. With every reason byte
requiring JSON escaping and every identifier/number at its maximum wire width,
500 items would require at most 289093 bytes; 200 require at most 115693 bytes.
The producer reserves 128 envelope bytes, measures each canonical item, and stops
before exceeding its budget, advancing the cursor only through returned records.
Tests prove a full 500-item unescaped page and byte-bounded escaped pages, including
a full worst-case 200-item consumer page. Clients continue after every non-empty
page even when it contains fewer records than requested.

## Mapping, publication and privacy

| Existing source | Kind | Public reason code |
| --- | --- | --- |
| T34 category warning | 6 (player-visible record) | 1 |
| T34 category suspension | 4 | 3 |
| Typed ban / unban | 4 | explicit metadata |
| Typed namelock / lift | 5 | explicit metadata |
| Typed warning | 6 | explicit metadata |

Generic T34 `restriction` is not exported: it does not identify a mute, jail or
character ban. Staff-only typed `notation` is never exported, even if reason
metadata is supplied. There are no current producers for kinds 1, 2 or 3.
Pending four-eyes decisions are private until confirmed (or revoked). Current
confirmed T34 records are backfilled once by the migration. Historical T34
changes predating this journal cannot be reconstructed; the initial record is
its current state. Existing typed history is published in revision order when
its public reason metadata first becomes available.

T34 public reasons replace control characters with spaces, trim surrounding
spaces and shorten to at most 120 bytes at a UTF-8 boundary. Empty resulting text
is refused transactionally; existing such mapped records must be corrected before
migration. No fallback reason is invented. Typed decisions have no public reason
in their existing intent: authorized callers use `set_public_reason` in the same
transaction as the decision and audit. It accepts only an explicit valid reason
code and 1..120-byte control-free text. Until this metadata is supplied, typed
records remain private. Metadata corrections emit a new current-state revision.

Revocation is sticky for each `sanction_stream_id`. The typed sanction service
refuses Ban after Unban, or Namelock after LiftNamelock, under the same sanction
ID with `Conflict`, before allocating a decision revision. This applies even
before public reason metadata is registered. To impose a sanction again, callers
must create a new UUIDv7 sanction ID (and public reason); its export starts at
decision revision 1 with `revoked=false`. The lifted stream remains revoked.
Idempotent replay of the current lift remains valid.

Triggers write the immutable journal in the transaction of every mapped T34
mutation, typed decision revision, and appeal review/outcome change. Accepted
appeals publish revoked=true; rejected/reviewing appeals publish a higher revision
retaining the current record. Appeal text, outcomes, moderator notes, evidence,
staff IDs and unrelated account data never appear on the wire. Exporting an
accepted appeal does not replace the existing appeal adapter's obligation to
revoke authoritative Platform enforcement in that same transaction.

A single counter row is locked and incremented transactionally before each
journal insert. The lock survives until commit. A concurrent writer cannot
publish a larger position before a smaller position commits or rolls back.
A bare database sequence would not provide this guarantee. Rollbacks undo both
counter and journal writes. Revisions are allocated per stream under this same
lock, independently of account-level revisions and sanction lock versions.
Confirmed stream identity changes are refused. Reads use one ordered statement
and never mutate source decisions or contact Game. Journal capture continues
while HTTP export is disabled, so reenabling does not lose changes.

## Fixtures and qualification

`fixtures/sanction-export-v1/request.json`, `page.json` and `empty.json` are
synthetic byte-for-byte golden vectors, without a terminal newline. Tests cover
HTTP success, disabled and misconfigured routes, wrong identity/header spoofing,
bad bodies, identity overlap, 429, UTF-8 boundaries, transactional revocation and
appeal outcomes, unmapped/private exclusions, rollback and concurrent publication.
Database tests use `PORTAL_TEST_DATABASE_URL`; when absent each prints a skip
notice. No SQL query macros or offline query metadata are introduced.
