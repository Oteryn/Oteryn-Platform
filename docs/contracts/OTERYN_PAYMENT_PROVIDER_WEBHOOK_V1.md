# OTERYN_PAYMENT_PROVIDER_WEBHOOK_V1

Status: implemented behind a default-off flag (owner-approved design,
2026-11-14). It is not yet wired to a specific payment provider. That wiring
waits until a provider is chosen; the provider's own signature scheme and
event names will then be mapped onto this contract, or this contract
will be revised.

A payment provider reports refunds and chargebacks of native coin purchases
to the portal through one signed HTTP webhook. Each verified event drives the
existing coin purchase reversal (`crates/payments/src/coin_store.rs`,
`CoinPurchases::webhook`). Code: `portal-rs/crates/payments/src/provider_webhook.rs`.
Durable record: table `payment_provider_webhook_events` (migration
`20261114100003`).

## Route and configuration

`POST /api/v1/payments/provider-webhook`

| variable | meaning |
|---|---|
| `PORTAL_PAYMENTS_PROVIDER_WEBHOOK_ENABLED` | `true` turns the route on; anything else (the default) leaves it off |
| `PORTAL_PAYMENTS_PROVIDER_WEBHOOK_SECRET` | current HMAC secret, 32..=1024 bytes |
| `PORTAL_PAYMENTS_PROVIDER_WEBHOOK_SECRET_PREVIOUS` | optional previous secret, accepted as well during rotation, same limits |

The route is also off when native coin purchases are off
(`PORTAL_COIN_PURCHASES_ENABLED`, sandbox only). Off, or with any value missing
or malformed, every request gets `503` with an empty body, before the body is
read. Secrets are never logged or printed.

## Headers and signature

| header | value |
|---|---|
| `Oteryn-Webhook-Timestamp` | Unix time in seconds, decimal ASCII digits only (1..=12) |
| `Oteryn-Webhook-Signature` | `v1=` followed by 64 lowercase hex digits |

Each header appears exactly once. The signature is HMAC-SHA256 with the
secret over the signature base string:

```text
<Oteryn-Webhook-Timestamp value> "." <exact raw request body bytes>
```

The portal compares in constant time against each configured secret. A
timestamp more than 300 seconds before or after the portal clock is refused.
The raw body is limited to 16384 bytes, checked from `Content-Length` and
while reading.

## Body

JSON object, UTF-8. Every event has:

| member | type | rule |
|---|---|---|
| `version` | integer | `1` |
| `event_id` | string | 1..=128 printable ASCII bytes, no spaces; the provider's unique event ID |
| `type` | string | 1..=64 bytes of `a-z`, `0-9`, `_`, `.` |

`refund` and `chargeback` events have exactly these members, and also:

| member | type | rule |
|---|---|---|
| `order_reference` | string | the provider's reference of the paid order, as stored when the payment was created (1..=128 printable ASCII bytes) |
| `amount` | object | `{"minor_units": <positive integer>, "currency": "<3 uppercase letters>"}`; must equal the order's full price |

```json
{"version":1,"event_id":"evt_123","type":"refund","order_reference":"test:0193...","amount":{"minor_units":499,"currency":"EUR"}}
```

Any other `type` is stored and ignored; members beyond the three common
ones are then not checked.

## Responses

| status | body | meaning |
|---|---|---|
| `200` | `{"version":1,"event_id":"…","outcome":"applied"\|"ignored","replay":false}` | first delivery applied, or an unknown type ignored |
| `200` | `{"version":1,"event_id":"…","outcome":"<stored outcome>","replay":true}` | same `event_id`, same body: nothing applied again |
| `400` | empty | signature valid, body malformed; nothing stored |
| `401` | empty | header missing, repeated or malformed, signature wrong, timestamp outside 300 s |
| `409` | empty | same `event_id`, different body; nothing applied, an `audit_log` row `payment_provider_webhook.event_replay_refused` is written |
| `413` | empty | body over 16384 bytes |
| `422` | `{"version":1,"event_id":"…","code":"<code>"}` | refused and stored with that outcome (table below) |
| `503` | empty | off or misconfigured, or the outcome is unknown (database or wallet unavailable, timeout); nothing stored, the provider should retry |

| `code` | stored outcome | when |
|---|---|---|
| `PROVIDER_WEBHOOK_UNKNOWN_ORDER` | `unknown_order` | no coin order is paid through `order_reference` |
| `PROVIDER_WEBHOOK_EVENT_REJECTED` | `rejected` | amount or currency differs from the order (a partial refund included) |
| `PROVIDER_WEBHOOK_ILLEGAL_TRANSITION` | `illegal_transition` | the order cannot take this reversal (a second refund, a chargeback after a chargeback, a failed order) |

A body is the same when its SHA-256 matches the stored one byte for byte.
"Same body" replays of a refused event answer `200` with the stored outcome.

## Effect and idempotency

- One event ID is processed at a time (a transaction-scoped lock on the ID).
  Of two parallel deliveries of one event, exactly one applies; the other is
  answered as a replay.
- A `refund` or `chargeback` maps onto the coin purchase reversal: the
  credited coins are debited once, never below zero; when the balance no
  longer covers them, the order is flagged for review and the payer's
  purchasing is held (rules of `coin_store.rs`).
- The reversal reaches the coin purchase service as a verified sandbox
  event whose ID is derived from `event_id`
  (`provider-webhook:<hex SHA-256 of event_id>`). The event row is committed
  only after that reversal has committed. If the commit of the row is lost, a
  retry is answered by the coin purchase service as its own replay and records
  the outcome, so nothing is applied twice.
- A timeout or any unknown result is neither success nor failure: nothing is
  stored and the answer is `503`.
