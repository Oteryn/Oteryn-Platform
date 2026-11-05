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
