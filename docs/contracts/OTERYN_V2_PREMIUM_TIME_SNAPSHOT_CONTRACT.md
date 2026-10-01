# Oteryn-v2 Premium time entitlement and snapshot contract

## Status

`CANDIDATE PLATFORM PRODUCER CONTRACT — PRODUCT oteryn.premium_time VERSION 1, PROFILE B — ACCEPTED ON OWNER-REVIEWED MERGE`

- Cross-repository coordination id: **`OTV2-PREMIUM-DELIVERY`**; Platform producer slice **PREM-P**.
- Governing Issue: Oteryn/Oteryn-Platform#1431 (parent #322).
- Consumer proposal: Oteryn-Game `PREMIUM-DELIVERY-0` (Oteryn/Oteryn-Game#1369, `docs/architecture/reviews/OTERYN_GAME_PREMIUM_DELIVERY0_PREMIUM_EVIDENCE_TRANSPORT_DECISION_2026-09-30.md`), as relayed to Platform. Platform did not read that file. Section 9 records every point where this contract accepts, amends or overrides the proposal.
- Subordinate to `OTERYN_V2_ENTITLEMENT_GAME_DELIVERY_CONTRACT.md` (the generic contract). This document selects the product/version values and the one wire transport that the generic contract defers. It does not relax any generic Profile-B rule.

This contract authorizes no payment, voucher, production activation, deployment, certificate issuance, secret handling, Canary mutation or Oteryn-Game write.

## 1. Product

| Field | Value |
|---|---|
| `product_id` | `oteryn.premium_time` |
| `product_version` | `1` |
| Delivery profile | B: a game-consumed account entitlement |
| Target scope | one canonical Platform `AccountId` (`identities.account_id`, lower-case UUIDv7) |
| Issuance sources in v1 | `OPERATOR_GRANT` (production-capable), plus test fixtures in non-production test code only |
| Paid / voucher issuance | **not available** in v1. Commercial activation of #322 stays blocked on #1236 |
| Post-delivery reversal class | `deny-future-use`: revocation ends future benefit and never mutates game state |
| Premium gameplay benefits | owned by Oteryn-v2. Platform does not define them |

## 2. Entitlement model

### 2.1 One entitlement per account

Each account has at most one `oteryn.premium_time` v1 entitlement. It is created by the first accepted grant and keeps a stable `entitlement_id` (lower-case UUIDv7) for its whole life. The entitlement is **not** deleted or replaced after expiry or revocation, because later grants re-activate the same identity. This keeps `lifecycle_revision` a single monotonic sequence per account and product.

### 2.2 Grants merge into one interval

An operator grant carries a whole number of days, `duration_days`, from 1 through 366. Platform evaluates it with Platform server time `now`, truncated to whole seconds:

```text
if entitlement exists and state = ACTIVE and now < effective_until:
    effective_until := effective_until + duration_days * 86400      (extend; effective_from unchanged)
else:
    effective_from  := now
    effective_until := now + duration_days * 86400                  (new interval; also after EXPIRED or REVOKED)
state := ACTIVE
lifecycle_revision := lifecycle_revision + 1
```

- A grant never creates a second interval or a gap. Future-dated grants are not supported in v1, so v1 never emits `NOT_YET_EFFECTIVE` (see 5.3).
- `effective_until` may not exceed `now + 3660 days`. A grant that would exceed this cap is refused as a whole and no partial extension is applied.
- Each grant carries an operator request identity (a lower-case UUID). An exact retry with the same identity, target, duration and reason hash returns the stored result and does not change the entitlement. Reusing the identity with any other value fails closed as an idempotency conflict.

### 2.3 Revocation

An operator revocation ends the current interval of an `ACTIVE` entitlement:

```text
state := REVOKED
effective_until := min(effective_until, now)
lifecycle_revision := lifecycle_revision + 1
```

- If revocation happens in the same second the interval started, `effective_until` becomes `effective_from + 1 s`, so that `effective_from < effective_until` still holds. `REVOKED` still wins classification, so this second grants no benefit.
- Revocation removes all remaining time. It does not remove one grant's days while keeping the rest. An operator who needs a shorter term revokes and then re-grants.
- Revoking an entitlement that is not `ACTIVE`, or an absent one, is refused and changes nothing.
- A later grant starts a new interval (2.2) at a higher `lifecycle_revision`. The generic contract permits this: a higher lifecycle revision supersedes every lower one. The revision fence is never lowered.

### 2.4 Expiry

Expiry is not a stored transition. The entitlement is `EXPIRED` when `now >= effective_until` and its stored state is `ACTIVE`. `lifecycle_revision` does not change on expiry. The absolute `effective_until` already fences it.

### 2.5 Operator controls

A grant or revocation requires all of the following:

- an authenticated Platform session with **confirmed MFA** (`mfa.confirmed`);
- the exact admin permission `products.premium.manage`, deny by default and not implied by any other permission;
- a reason of 10 to 500 characters, stored only as a SHA-256 hash;
- one transaction that locks the account's authority row (6.4). The same transaction updates the entitlement, appends an immutable grant/revocation record and writes an admin audit event (`products.premium_granted` or `products.premium_revoked`). The event records the actor, target AccountId, entitlement id, lifecycle revision, duration, reason hash and request id. It never records a reason in plaintext.

## 3. Product/version Profile-B authority policy

These values are **mandatory, fixed for `oteryn.premium_time` v1, and not deployment configuration**. Changing any of them requires a new product version or an amendment to this contract. Increasing the lease or skew widens authorization, as the generic contract notes.

| Policy | Game request | Platform decision | Reason |
|---|---|---|---|
| `max_authority_lease` | 60 min | **Accepted: 3600 s** | A 60-minute lease bounds how long a revocation can go unseen. With refresh at two thirds of the lease, one Game pull per online account every 40 minutes is a negligible load. |
| Refresh point (`refresh_before` in the generic contract, `refresh_after` on the wire) | 40 min after issue | **Amended:** `refresh_after = authority_issued_at + floor(2 × (authority_valid_until − authority_issued_at) / 3)` seconds. This equals exactly 40 min for a full lease. | A fixed 40-minute value would land after `authority_valid_until` when the commercial end clips the lease, and the generic contract requires the refresh point to precede the cutoff. |
| `max_clock_skew` | 5 s | **Accepted: 5 s** | Both producer and consumer run on NTP-disciplined servers. The consumer fails closed when its trusted-time uncertainty exceeds 5 s. |
| `stale_within_bound` | not permitted | **Accepted: DENY** | Zero stale grace. Once `refresh_after` of the newest accepted snapshot is reached without newer accepted evidence, the refresh is due and the evidence classifies as `STALE_WITHIN_BOUND` until `authority_valid_until`. Benefit is denied throughout that interval. `authority_valid_until` stays the absolute cutoff after which the evidence is `EXPIRED`. |
| `authority_expired` | — | **Deny** benefit until a fresh snapshot is accepted and trusted time is safe again | Required by the generic contract. |

Because stale use is denied, `CURRENT_AUTHORITY` is the only classification that permits benefit for this product. `STALE_WITHIN_BOUND`, `NOT_YET_EFFECTIVE`, `EXPIRED`, `REVOKED` and `AUTHORITY_UNAVAILABLE` all deny it. Without fresh evidence, benefit therefore ends at `refresh_after`, never at the later `authority_valid_until`.

## 4. Snapshot read transport

### 4.1 Endpoint

```text
POST /internal/v1/products-entitlements/premium-snapshots/read
Content-Type: application/json
```

The endpoint is served only on Platform's private internal route group. It is never exposed to browsers, the public edge or the native client.

### 4.2 Service identity (mutual TLS)

The existing Platform mTLS peer pattern applies. It is the same pattern used for character-bootstrap intents and native evidence:

- the trusted TLS terminator on the private network must report `SSL_CLIENT_VERIFY=SUCCESS` and `SSL_PROTOCOL=TLSv1.3`;
- the verified client-certificate subject (`SSL_CLIENT_S_DN`) must match `PRODUCTS_ENTITLEMENTS_PREMIUM_SNAPSHOT_MTLS_CLIENT_IDENTITY` exactly, using a constant-time comparison;
- that identity is **dedicated to this one read**. It must not equal any other configured Platform peer identity, and no other internal endpoint accepts it. A misconfiguration that reuses another endpoint's identity is treated as unavailable (`503`);
- configuration is required and has no default. A missing, empty or malformed value (non-printable ASCII or over 128 bytes) returns `503`.

Platform owns which subject is authorized. Issuing and custody of the Game server's client certificate and private key, and configuration of the terminator's trust anchor, are deployment and operations tasks. This contract does not perform or authorize them.

### 4.3 Activation

`PRODUCTS_ENTITLEMENTS_PREMIUM_SNAPSHOT_ENABLED` defaults to `false`. While it is not exactly `true`, every request returns `503`. `PLATFORM_BUILD_REVISION`, the 40-character lower-case hex commit SHA of the running build, is also required. If it is missing or malformed, every request returns `503`.

### 4.4 Request

The body is one JSON object of at most 256 bytes. It has exactly these members, and member order does not matter:

```json
{"schema":"oteryn.premium_snapshot_request.v1","account_id":"01890f4e-7c00-7000-8000-000000000001","nonce":"4f1c2a9be07d4c3a8b6e5d4c3b2a1908"}
```

| Member | Rule |
|---|---|
| `schema` | exactly `oteryn.premium_snapshot_request.v1` |
| `account_id` | canonical lower-case hyphenated RFC 9562 UUIDv7 (version 7, RFC variant), 36 characters; any other UUID is rejected with `400` |
| `nonce` | 128 bits as exactly 32 lower-case hexadecimal characters, generated fresh by Game for each request |

Platform rejects duplicate, unknown, missing, nested or non-string members, and malformed or oversized JSON, with `400`. Platform echoes the nonce and does not store it. Freshness and matching belong to the consumer (8.1). The mTLS channel supplies transport replay protection, and the echoed nonce binds each response to one Game request.

### 4.5 Outcomes

| Condition | Status | Body |
|---|---|---|
| Snapshot issued | `200` | snapshot (section 5) |
| Malformed request | `400` | empty |
| Missing or wrong mTLS peer provenance | `401` | empty |
| No Platform Identity has this `account_id` | `404` | empty |
| Rate limit exceeded | `429` | empty |
| Disabled, misconfigured, database unavailable or invalid durable state | `503` | empty |

Every response is `Cache-Control: no-store, private`. Failure bodies never carry entitlement data or diagnostics. A `404` means the AccountId is unknown to Platform. It is not a Premium state. The consumer treats it, like every non-200 status, as no fresh authority. Existing accepted evidence then stops permitting benefit at its own `refresh_after` (section 3) and expires at its absolute cutoff.

A `404` is decided before any authority revision is allocated, so probing unknown ids creates no rows. The request rate per peer is bounded by the named limiter `products-entitlements-premium-snapshot`, which defaults to 1200 requests per minute.

## 5. Snapshot response

### 5.1 Wire

A `200` body is one UTF-8 JSON object of **at most 1,024 bytes**. It has exactly these members:

| Member | Type | Value |
|---|---|---|
| `schema` | string | `oteryn.premium_snapshot.v1` |
| `producer_revision` | string | `PLATFORM_BUILD_REVISION`, 40 lower-case hex characters |
| `producer_profile` | string | `oteryn.entitlement.profile_b.v1`, the generic Profile-B delivery profile |
| `nonce` | string | the request nonce, byte for byte |
| `account_id` | string | the request `account_id` |
| `product_id` | string | `oteryn.premium_time` |
| `product_version` | integer | `1` |
| `entitlement_id` | string or null | lower-case RFC 9562 UUIDv7. `null` only when the state is `NONE` |
| `entitlement_state` | string | `ACTIVE`, `NOT_YET_EFFECTIVE`, `EXPIRED`, `REVOKED` or `NONE` |
| `lifecycle_revision` | integer | at least 1 when an entitlement exists, 0 for `NONE` |
| `authority_revision` | integer | at least 1, strictly increasing per account (6.4) |
| `effective_from` | string or null | RFC 3339 UTC. `null` only for `NONE` |
| `effective_until` | string or null | RFC 3339 UTC. `null` only for `NONE` |
| `authority_issued_at` | string | RFC 3339 UTC, Platform time when the snapshot was issued |
| `authority_valid_until` | string | RFC 3339 UTC, the finite cutoff (5.2) |
| `refresh_after` | string | RFC 3339 UTC (section 3) |

- Every timestamp uses exactly the form `YYYY-MM-DDTHH:MM:SSZ`: UTC, whole seconds, no fractional part, no offset other than `Z`.
- The two revision integers are JSON numbers with no more than 2^53 − 1 (9007199254740991), so every JSON parser reads them exactly. The consumer may store them as u64.
- The machine-readable schema and examples are in `docs/contracts/fixtures/premium-snapshot-v1/` (section 7).

### 5.2 Cutoff and refresh values

With `issued = authority_issued_at` and `L = 3600 s`:

```text
ACTIVE, NOT_YET_EFFECTIVE:  authority_valid_until = min(issued + L, effective_until)
EXPIRED, REVOKED, NONE:     authority_valid_until = issued + L
refresh_after = issued + floor(2 × (authority_valid_until − issued) / 3)
```

Therefore `issued <= refresh_after < authority_valid_until <= issued + 3600 s`, and for every usable active snapshot `authority_valid_until <= effective_until`. For non-active states the cutoff only bounds how long the negative snapshot counts as current evidence. It never grants anything.

### 5.3 State classification

Platform classifies the state at `authority_issued_at`, in this order:

1. No entitlement exists: `NONE`.
2. The stored state is `REVOKED`: `REVOKED`.
3. `issued >= effective_until`: `EXPIRED`.
4. `issued < effective_from`: `NOT_YET_EFFECTIVE`. This is defined for wire compatibility and is unreachable in v1 (2.2).
5. Otherwise: `ACTIVE`.

`entitlement_state` is Platform's commercial classification at issue time. **It is not by itself authorization to grant benefit** (8.2).

## 6. Ordering and fencing guarantees (producer side)

### 6.1 `lifecycle_revision`

`lifecycle_revision` increases by exactly 1 on each accepted grant or revocation of the account's entitlement, and on nothing else. It never decreases.

### 6.2 Same lifecycle revision

Within one lifecycle revision, `entitlement_id`, the stored state, `effective_from` and `effective_until` are immutable. Two snapshots with the same `lifecycle_revision` can therefore differ only in state derived from time (`ACTIVE` changing to `EXPIRED`) and in the authority fields.

### 6.3 Monotonic cutoffs

Replaying a snapshot never extends a cutoff. Only a newly issued snapshot with a higher `authority_revision` can carry a later `authority_valid_until`, and the consumer's fence (8.3) rejects replays.

Within one lifecycle revision, the derived state may move only from `ACTIVE` to `EXPIRED`, never the reverse. `REVOKED` and `NONE` do not change without a new lifecycle revision.

### 6.4 `authority_revision`

Each account has one Platform authority row. Every issued snapshot, grant and revocation first locks that row with `SELECT … FOR UPDATE`. Every issued snapshot increments `authority_revision` by 1 in the same transaction that reads the entitlement. As a result:

- `authority_revision` is **strictly increasing per account across all lifecycle revisions**. This is stronger than the generic contract's "within one lifecycle revision" requirement, and it is the ordering the Game proposal asks for;
- a snapshot with a higher `authority_revision` never reflects an older lifecycle revision than a lower one, because grants and snapshot issuance serialize on the same row;
- every `NONE` snapshot for a known account also allocates a revision (the row is created on first use).

### 6.5 Database rollback

Restoring the Platform database to an earlier point can make Platform issue an `authority_revision` at or below one that Game has already accepted. The consumer's high-water fence (8.3) rejects those snapshots, so Premium benefit for the affected accounts fails closed until newly issued revisions exceed the retained high-water mark. The fence never fails open. Platform v1 does not detect its own rollback. Recovering from a restore of every Platform database copy requires operator reconciliation against the consumer's retained high-water marks. This is the same limit the character-bootstrap-intent contract records.

## 7. Shared fixtures (reusable by Game)

`docs/contracts/fixtures/premium-snapshot-v1/` contains:

- `request.schema.json` and `snapshot.schema.json`: JSON Schema (draft 2020-12) for the exact wire;
- `request.valid.json`;
- `snapshot.active.json`, `snapshot.active-clipped.json`, `snapshot.expired.json`, `snapshot.revoked.json`, `snapshot.none.json`: valid examples that obey every rule in sections 5 and 6;
- `manifest.json`, which lists each example with its expected classification, plus invalid request and response cases the consumer must reject.

Platform's own contract tests check real endpoint responses against these schemas and the cross-field rules. Game may copy the directory, pinned to a Platform commit, into its end-to-end test. The fixture set is versioned with `schema`. Any wire change requires a new schema id.

JSON Schema alone cannot express the cross-field rules: nullability by state, the timestamp arithmetic in 5.2, and `effective_from < effective_until`. The manifest states them as `cross_field_rules`, and both producer and consumer tests must enforce them.

## 8. Consumer obligations (Game)

These restate generic Profile-B rules as they apply to this wire. Game owns their implementation.

### 8.1 Response binding

Game accepts a `200` only when all of these hold:

- the `nonce` is identical to the request's;
- `account_id` is identical;
- `schema`, `product_id` and `product_version` are exact;
- the body parses and passes every rule in section 5, including the size limit.

Anything else is treated as no fresh authority.

### 8.2 Benefit rule

Premium benefit is permitted only when **all** of the following hold:

- `entitlement_state` is `ACTIVE` or `NOT_YET_EFFECTIVE`. A future-start `ACTIVE`-lifecycle representation authorizes only once start is proven reached;
- the earliest plausible trusted time is at or after `effective_from`;
- the latest plausible trusted time is before `min(effective_until, refresh_after)` of the newest accepted snapshot. At or after `refresh_after` without newer accepted evidence the snapshot is `STALE_WITHIN_BOUND`, which this product denies (section 3); `refresh_after < authority_valid_until` always holds (5.2);
- trusted-time uncertainty is at most 5 s.

Platform's `entitlement_state` of `ACTIVE` never overrides these time checks. Game must evaluate the absolute times itself.

### 8.3 Fence

Game keeps a durable per-account high-water mark of the accepted `authority_revision` and rejects any snapshot at or below it. It also rejects a snapshot whose `lifecycle_revision` is lower than the accepted one. For an equal `lifecycle_revision`, it rejects a snapshot whose `entitlement_id`, `effective_from` or `effective_until` differs, or whose state contradicts 6.2 (for example `REVOKED` becoming `ACTIVE`), as conflicting authority evidence.

### 8.4 No stale use

After `authority_valid_until`, an accepted active snapshot classifies as `EXPIRED`. Restart, reconnect, outage or snapshot restore never moves the start earlier, extends the cutoff or resets it.

### 8.5 Refresh

Game may pull at any time within the rate limit, and should pull on login or reconnect. To keep benefit continuous it should pull early enough that a newer snapshot is accepted before the current `refresh_after`; benefit stops at `refresh_after` otherwise (8.2). Pull failures never extend authority.

## 9. Decisions on the Game proposal

| # | Game proposal | Platform decision |
|---|---|---|
| D1 | Profile B account entitlement, operator/test grants only, no payment (Game D69) | **Accepted.** Consistent with #322, which keeps commercial activation blocked on #1236. |
| D2 | Several grants merge into one account-level interval | **Accepted** with the extend-or-restart rule in 2.2 and the 3660-day cap. |
| D3 | Expiry and revocation are Platform's | **Accepted.** Revocation removes all remaining time (2.3). Removing a single grant is not supported in v1. |
| D4 | mTLS on the private network with a Platform-issued service identity scoped to this one read | **Accepted as the existing trusted-terminator mTLS peer pattern with a dedicated configured subject (4.2).** Certificate issuance is deployment work and out of scope. |
| D5 | Request carries `account_id` and a fresh 128-bit nonce | **Accepted.** The wire encoding is 32 lower-case hex characters inside a versioned request `schema` member (4.4). |
| D6 | Response of at most 1,024 bytes with exactly the listed fields | **Accepted.** The field set and names are unchanged, including `producer_profile` (constant `oteryn.entitlement.profile_b.v1`), which the consumer requires. |
| D7 | `effective_from` / `effective_until` as absolute RFC 3339 UTC | **Amended:** both are `null` when `entitlement_state = NONE`, because there is no interval. Whole-second `Z` format. |
| D8 | `refresh_after` = 40 min after issue | **Amended:** two thirds of the actual lease (40 min for a full lease), so the refresh point always precedes a clipped cutoff (section 3). |
| D9 | `authority_valid_until` ≤ `issued_at + max_authority_lease` | **Accepted and tightened:** also ≤ `effective_until` for `ACTIVE` and `NOT_YET_EFFECTIVE` (generic contract rule). |
| D10 | `authority_revision` u64, monotonic per account | **Accepted.** Strictly increasing per account (6.4). Values stay ≤ 2^53 − 1 on the wire. |
| D11 | `lifecycle_revision` u64, monotonic per entitlement | **Accepted.** There is one entitlement per account and product (2.1). `NONE` uses 0. |
| D12 | `entitlement_state` includes `NOT_YET_EFFECTIVE` | **Accepted** on the wire but unreachable in v1 (5.3). |
| D13 | Policy values: 60 min lease, 5 s skew, no stale use | **Accepted** (section 3). No stale use means benefit stops at `refresh_after` without newer accepted evidence. |

### 9.1 Conflicts with Platform contracts

**Point of conflict:** if the Game proposal treats `entitlement_state = ACTIVE` from Platform as enough to grant benefit, that conflicts with the generic contract. The generic contract requires the consumer to evaluate the not-before and not-after boundaries conservatively on trusted server time. Section 8.2 is binding.

No other part of the relayed proposal conflicts with an accepted Platform contract. The amendments D7 and D8 fill in details the proposal left open or that would otherwise break the generic contract's requirement that refresh precede the cutoff.

## 10. Validation required before Game consumption

Platform implementation PRs under #1431 must prove, with tests:

- grant extend-or-restart, the cap, idempotent retry and conflicting reuse;
- revoke of an active entitlement; refusal of a revoke on a non-active one; re-grant after revoke at a higher lifecycle revision;
- the `mfa.confirmed` and exact-permission gates, and one audit event per mutation;
- request validation (every `400` class), mTLS `401` and `503` cases, `404` for unknown accounts without allocating a revision, disabled `503` and misconfigured build revision `503`;
- every response validates against `snapshot.schema.json` and the cross-field rules, and stays at most 1,024 bytes;
- `authority_revision` strictly increases across snapshots and grants, and concurrent grants and snapshots serialize;
- expiry classification at the boundary (`issued == effective_until` gives `EXPIRED`), the clipped cutoff and the refresh arithmetic.

Joint Platform/Game end-to-end qualification is a Game-side task using the section 7 fixtures, followed by a real endpoint run. Production activation needs its own authority.

## 11. Non-authorization

This contract authorizes no payment or voucher issuance, no production or staging activation, no certificate or secret operation, no Canary or Oteryn-Game write, and no Premium gameplay benefit definition.
