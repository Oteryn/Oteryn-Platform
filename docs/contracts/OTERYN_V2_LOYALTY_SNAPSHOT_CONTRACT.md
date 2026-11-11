# Oteryn Loyalty Snapshot v1

**Status: PROPOSED PLATFORM PRODUCER CONTRACT — oteryn.loyalty_snapshot.v1 — DEFAULT-OFF.**

The Platform produces this projection. The Game consumer is a separate Game task and is not enabled or deployed by this contract.

## Endpoint and peer authentication

The endpoint is `POST /internal/v1/products-entitlements/loyalty-snapshots/read`. It is available only over mutual TLS 1.3 and only to the exact client-certificate identity configured as `products-entitlements.loyalty_snapshot.mtls_client_identity`.

The loyalty identity is dedicated to this read. It must not equal the Premium snapshot identity, the native-evidence identity, or the character-bootstrap-intent identity. Missing or invalid configuration, a reused identity, or a disabled producer makes settings unavailable and the route answers 503. The producer is default-off.

Requests are bounded per peer. A request over the configured limit answers 429 and includes `Retry-After`.

## Request

The body is at most 256 bytes and has exactly these members:

```json
{"schema":"oteryn.loyalty_snapshot_request.v1","account_id":"<uuidv7>","nonce":"<32 lower hex>"}
```

`account_id` is a canonical lower-case UUIDv7 and `nonce` matches `^[0-9a-f]{32}$`. Duplicate, unknown, or missing members are malformed and answer 400. The request must have JSON content type, identity or absent content encoding, and no transfer encoding.

## Successful response

A successful response is `application/json`, is at most 512 bytes, and contains members in exactly this order:

1. `schema`: `"oteryn.loyalty_snapshot.v1"`;
2. `producer_revision`: 40 lower-case hexadecimal characters;
3. `nonce`: the request nonce, echoed exactly;
4. `account_id`: the request account identifier, echoed exactly;
5. `loyalty_points`: an integer from 0 through 2147483647;
6. `loyalty_revision`: an integer at least 1, allocated by advancing the account's Premium `authority_revision` for this read;
7. `computed_at`: the computation time formatted by `PremiumTimeContract::formatTime(now)`.

## Outcomes and caching

- 400: malformed request;
- 401: wrong or unverified peer, or a connection other than TLS 1.3;
- 404: unknown account; this outcome allocates no authority revision;
- 429: per-peer rate limit exceeded, with `Retry-After`;
- 503: disabled, misconfigured, or temporarily unavailable producer.

Every non-200 response has an empty body. Every response, successful or otherwise, is private and `no-store`.

## Loyalty computation

One loyalty point represents one whole day of Premium time already consumed by the account. For every distinct (`entitlement_id`, `effective_from`) group in the account's Premium event ledger, select the `effective_until` from the event with the greatest `lifecycle_revision`. Only groups whose `effective_from` is earlier than `now` participate.

For each participating group:

```text
interval_consumed_seconds = max(0, min(until_of_highest_lifecycle_revision_in_group, now) - effective_from)
```

Then:

```text
consumed_seconds = sum(interval_consumed_seconds)
loyalty_points = min(intdiv(consumed_seconds, 86400), 2147483647)
```

For a fixed ledger, points never decrease as time advances. Revocation stops future growth of the affected interval and never removes days already consumed.

## Consumer rule

Game keeps the snapshot with the highest `loyalty_revision` per account. It refuses a lower or equal revision when that snapshot carries different points. Because the loyalty read advances the same per-account Premium authority revision used by Premium snapshots, the projection has one monotonic ordering with Premium lifecycle and snapshot activity.

## Loyalty stream v1 (owner decision 2026-11-11)

This additive export is `POST /internal/v1/products-entitlements/loyalty-snapshots/stream`.
The snapshot endpoint and all of its existing members and revision semantics above
remain unchanged. The Game pulls this export over verified mutual TLS 1.3 and
HTTP/1.1. There is no shared database, outbound Game call, or ranking in Platform.
The Game builds its own loyalty highscores.

### Stream configuration and authentication

The stream is default-off, independently of the snapshot. Enable it only with
`PRODUCTS_ENTITLEMENTS_LOYALTY_STREAM_ENABLED=true`. Its exact certificate identity
is `PRODUCTS_ENTITLEMENTS_LOYALTY_STREAM_MTLS_CLIENT_IDENTITY`, bound to the dedicated
`products-entitlements-loyalty-stream` group. Build the identity map with **all**
internal peer bindings together: stream, snapshot, Premium, native evidence,
bootstrap intents and other internal peers must not reuse an identity. An absent
binding, overlapping identities, a disabled producer, or an invalid configuration
fails closed with 503. A wrong or unverified peer receives 401. Identity headers
are never accepted as certificate evidence.

`PRODUCTS_ENTITLEMENTS_LOYALTY_STREAM_REQUESTS_PER_MINUTE` defaults to 1200 and must
be an integer from 1 through 6000. The budget is shared by authenticated requests
in the route's process, including malformed bodies. Exhaustion gives 429 with
`Retry-After: 60`. Operate one route-group owner; multiple owners require an
external shared peer budget. All responses include `Cache-Control: no-store, private`.
Non-200 responses have empty bodies. Database or publication failures give 503.

The Rust private route factory is `loyalty::stream::http::router`.
`loyalty::http::router_with_stream` mounts snapshot and stream together with
independent flags, identities and budgets. Both factories must only be mounted
on the verified internal listener; neither is mounted on the public web router.
The stream runtime registration uses the existing `OTERYN_INTERNAL_TLS_*`
listener configuration. It resolves account existence in the Platform database
under a shared row lock in the publication transaction. Every configured
`*_MTLS_CLIENT_IDENTITY` / `*_SAN` setting and SAN key in an
`*_IDENTITIES` map participates in the reuse check, including
`PORTAL_GAME_COINS_IDENTITIES`.

### Stream request and response

The request is a JSON object of at most 256 bytes, with exactly these members:

```json
{"schema":"oteryn.loyalty_stream_request.v1","after":null,"limit":1}
```

`after` is null for an initial history read, otherwise the opaque `next_cursor` string
from the preceding successful response. `limit` is an integer from 1 through 500.
Member order and insignificant whitespace are accepted. Unknown, duplicate,
missing or wrongly typed members, fractions, exponents, invalid cursors and
cursors beyond committed history give 400. Content-type/encoding normalization
and transport validation are the same as for the snapshot.

The Game requests 200 items and limits response bodies to 256 KiB. Even the
maximum 500-item response is below 256 KiB: UUIDs are fixed at 36 characters,
day/point totals at most 10 digits each, revisions at most 19 digits, and the
producer cursor at most 42 ASCII characters. This bound is tested with 500
items at the maximum numeric values.

A 200 response is compact JSON with exactly these members in this order:

```json
{"schema":"oteryn.loyalty_stream_page.v1","items":[{"account_id":"01890f4e-7c00-7000-8000-000000000001","spent_premium_days":2,"loyalty_points":2,"platform_revision":1}],"next_cursor":"v1:1:0"}
```

Each item contains, in order:

- `account_id`: a canonical lower-case UUIDv7;
- `spent_premium_days`: an integer from 0 through 2147483647, the absolute consumed
  Premium-day total using the existing loyalty computation above (including its
  saturation, lifecycle grouping and flooring after summing fractional days);
- `loyalty_points`: consumed Premium-day points plus durable referral loyalty
  credits, saturated at 2147483647, matching the snapshot computation;
- `platform_revision`: an integer from 1 through 9223372036854775807, strictly
  increasing per account when a different total is published.

The stream discovers accounts through Platform's Premium authority and referral
loyalty credits, including accounts with zero consumption. A credited account
without Premium history exports zero days and its credit total. Accounts with
neither Premium authority/history nor referral credits have nothing exported.
Unknown accounts are omitted; ambiguous account authority fails the publication
with 503. An initial export has revision 1. Polling unchanged day and point totals
does not allocate another revision. A credit-only change allocates a new stream
revision even when days are unchanged. Snapshot reads still advance
their existing shared Premium authority revision and do not advance the stream
revision. A higher stream revision can carry a lower corrected total. Game applies
higher revisions, accepts identical replays and refuses conflicting equal revisions,
matching the record arguments of `game_account_loyalty_project_premium_days`.
That Game function is a consumer-side operation; Platform never connects to it.

### Durable pagination and polling

Tail polls compute Premium consumption from one PostgreSQL statement snapshot, using the
current database time. Before publication, each known account's Premium-only
consumption is passed to referral qualification. Before taking the referral
advisory lock, the stream locks all candidate accounts and their currently linked
inviters in UUID order. Publication reuses that authority set without locking
additional accounts, preventing a cycle with concurrent purchases and requiring
only one pool connection. Durable credits are then read in the same transaction,
including newly credited inviters without Premium history. Accounts appearing
after the authority set is captured are observed on the next tail poll. This works
with the snapshot endpoint disabled. Referral credits never
qualify for further referral credits. Continuation pages replay their immutable
window without qualification. This also detects whole-day growth without a Premium
mutation or a prior snapshot read. The stream exports observed changes when
polled, rather than every intermediate value between polls. State, new events
and the global head commit atomically. A locked transactional head serializes
publishers through commit; sequence allocation alone would permit skipped late
commits. A failure rolls the entire publication back.

Events are retained and immutable. Items follow global publication order, never
rank order. A page can contain multiple revisions of the same account. Consumers
must persist `next_cursor` only after applying every item, and may replay on failure.
Both events and cursors survive service restarts. Do not prune history or reset
the head/state independently; database restore requires consumer reconciliation.

The versioned cursor currently encodes `v1:<position>:<through>` as canonical
non-negative decimal integers within the signed 64-bit range. A positive
`through` freezes the upper boundary of a pagination window. Continuations read
only that window, so concurrent writes cannot change a replayed bounded page,
move an account past the cursor or skip its old value. The final page returns
`v1:<through>:0`; zero denotes a checkpoint for the next poll, which opens a new
window at the committed head after refreshing totals. A checkpoint poll may
therefore return newly published items on retry. The initial empty stream returns
`items:[]` and `next_cursor:null`. Null is returned only for empty pages;
every non-empty page has a cursor of 1..256 printable ASCII characters. An
empty page is caught up, not a permanent end of history: keep the preceding
non-null cursor and poll it again after a delay. If no checkpoint has yet been
received, continue polling with `after:null`. A null request cursor restarts
retained history from the beginning. Consumers must otherwise treat cursor
strings as opaque.

Golden byte fixtures and malformed request cases are under
`docs/contracts/fixtures/loyalty-stream-v1/`. The earlier snapshot tests remain
unchanged and are required alongside the stream tests.

The additive golden fixture `loyalty-stream-v1/credit-response.json` demonstrates
2 consumed Premium days plus 1 referral credit at stream revision 2. The four
item members and their order remain unchanged.
