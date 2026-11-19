# Minimum client build gate v1

Status: **PROPOSED**. Owner decision (b), 2026-11-19: Platform gets this gate,
as a contract and then an implementation, before Game ADR-0024 (Game private
gameplay CA, Oteryn-Game #3327) is accepted. ADR-0024 rests on it.
Design notes, data model and rollout runbook:
`docs/architecture/MINIMUM_CLIENT_BUILD_GATE_DESIGN.md`.

This proposal authorizes no code, migration, route, flag, key, certificate or
deployment. The implementation is a separate PR after acceptance.

## 1. Why

Today the admission issuer cannot tell an old client from a new one:

- `offer.client_build` is diagnostic only (native gateway login contract §3.1).
  It selects nothing, authorizes nothing and has no order.
- `release_id` (`rel-a`, ...) is an opaque name with no order.

So when a gameplay CA root or intermediate is rotated or compromised, a client
that does not trust the new chain gets a grant, connects, and fails the TLS
handshake. The user sees a connection error, not "update your client". This
contract adds an ordered build identifier to the login offer and a minimum
accepted build, checked before a grant is issued.

## 2. Ordered build identifier

The order is the one that already exists in the signed updater policy
(client update origin v1, ADR 0035): the **release sequence**.

- `release_sequence` is a positive integer, scoped to one **update channel**
  (`stable` or `beta`), strictly increasing per channel.
- It is signed: the TUF-authenticated channel policy target (`policy-v1.json`)
  carries `current_release_sequence` and `minimum_supported_release_sequence`,
  and each release's signed targets name its `release_id`.
- A client release knows its own `(update_channel, release_id,
  release_sequence)` from the release manifest it was installed from. It does
  not derive the sequence from `current_version_display` or `client_build`.

Sequences of `stable` and `beta` are not comparable with each other. Every
minimum in this contract is per update channel.

"Update channel" (`stable`/`beta`) is distinct from the game channel
(`channel_id`, a channel of a World). This contract always says which.

## 3. Wire change: `offer.release`

The `offer` object of `POST /v1/login` (protocol 2 and 3) and of the §19.4
recovery request gains one optional member:

```json
"offer": {
  "client_build": "0.4.2+abc123",
  "client_platform": "windows",
  "release": {
    "update_channel": "stable",
    "release_id": "rel-2026-11-30",
    "release_sequence": 42
  },
  "transports": [ { "protocol_major": 1, "transport_profile": 1, "alpn": "oteryn-game/1" } ]
}
```

- `release` is either absent or an object with exactly these three members.
- `update_channel`: exactly `stable` or `beta`.
- `release_id`: 1..64 visible ASCII, the signed release id of the installed
  release.
- `release_sequence`: integer in 1..2^53-1.
- Any other member, type or value: `NATIVE_LOGIN_REQUEST_MALFORMED`.
- `release` is part of the offer, so it is in the offer digest (§5 of the
  login contract). A retry with the same `attempt_ref` and a different
  `release` is an offer change and conflicts like any other offer change.
- `client_build` stays diagnostic. The gate reads only `release`.

The Gateway passes `offer` through unchanged and validates `release` with the
same rules, so a malformed `release` is refused at the Gateway.

**Ordering.** Platform first deploys the version that accepts `offer.release`.
A client sends `release` only after Platform confirms that deployment, because
an older issuer refuses an unknown offer member as malformed.

## 4. Minimum accepted build

### 4.1 Gate records

A gate record holds, per update channel, a minimum release sequence, together
with a policy for offers that carry no `release`. Each gate record applies to
one scope:

| Scope | Applies to |
|---|---|
| global | every World and game channel |
| World | every game channel of one `world_id` |
| game channel | one `(world_id, channel_id)` |

Each gate record has:

- `min_release_sequence.stable` and `min_release_sequence.beta`, each a
  positive integer or absent (no minimum for that update channel);
- `unknown_release`, either `admit` or `refuse`;
- `state`, one of `open`, `enforcing` or `maintenance` (§4.3);
- `revision`, the actor and the time.

### 4.2 Effective minimum

For a candidate game channel and the offered update channel `u`, the
effective minimum is the maximum of:

1. `minimum_supported_release_sequence` of the **active, verified** policy
   for `u` in Platform Downloads (client update origin v1);
2. `min_release_sequence.u` of the global, World and game-channel gate
   records that exist and are not `open`.

A record in state `open` contributes nothing (a staged value). An absent
minimum contributes nothing.

If no active verified policy exists for `u`, item 1 is absent.

### 4.3 States

| State | Effect at the gate |
|---|---|
| `open` | no check from this record |
| `enforcing` | the release must be at or above the record's minimum. An offer without `release` is decided by `unknown_release` |
| `maintenance` | no grant for this scope, whatever the release |

The most restrictive state among the global, World and game-channel records
wins: `maintenance` beats `enforcing`, which beats `open`.

### 4.4 Where the check runs

The issuer runs the gate inside the admission transaction:

- after the request and `offer` validation;
- after ticket authentication and route selection;
- before an attempt record is written and before anything is signed.

It runs the same way for protocol 2 and 3 logins and for §19.4 recovery.

The gate applies to each candidate game channel.

- When `channel_id` is given, its candidate is checked.
- When `channel_id` is null, candidates the release does not pass are removed
  before selection.
- If no candidate is left and at least one was removed by the gate, the
  result is the gate refusal (§5), not `NATIVE_LOGIN_ROUTE_UNAVAILABLE`.

A gate refusal rolls the transaction back, so the ticket is still unused.
That is the same rule as `NATIVE_LOGIN_CHANNEL_UNAVAILABLE`, §19.2.5. After
updating, the client may retry with a new `attempt_ref` while the ticket
lasts.

The gate decides only whether a grant is issued. A grant already issued
keeps its normal lifetime; the gate does not revoke grants or sessions.
Disconnecting sessions is a separate staff action.

## 5. Refusal codes

Two new rows for the §11.2 table of the native gateway login contract:

| Internal code | HTTP | Category | Progression | Next authority | Public class |
|---|---|---|---|---|---|
| `NATIVE_LOGIN_CLIENT_UPDATE_REQUIRED` | 409 | `UNSUPPORTED_REVISION` | `TERMINAL` | client update, then a new `attempt_ref` | `CLIENT_UPDATE_REQUIRED` |
| `NATIVE_LOGIN_MAINTENANCE` | 503 | `DEPENDENCY_UNAVAILABLE` | `RETRYABLE` | same request after `Retry-After`, or update if `update` is present | `TEMPORARILY_UNAVAILABLE` |

**`NATIVE_LOGIN_CLIENT_UPDATE_REQUIRED`** is returned when:

- the offered `release_sequence` is below the effective minimum; or
- `release` is absent and the `unknown_release` of an `enforcing` record is
  `refuse`; or
- `release_id` is listed in `revoked_release_ids` of the active verified
  policy for its update channel.

**`NATIVE_LOGIN_MAINTENANCE`** is returned when a `maintenance` record covers
every candidate game channel.

Both refusal bodies are the §11.1 body plus one member, `update`. The body
uses the request's `protocol_version`:

```json
{
  "protocol_version": 3,
  "error": { "code": "NATIVE_LOGIN_CLIENT_UPDATE_REQUIRED", "public_class": "CLIENT_UPDATE_REQUIRED" },
  "attempt_ref": "<echo or null>",
  "update": {
    "update_channel": "stable",
    "minimum_release_sequence": 43,
    "current_release_sequence": 44,
    "manual_download_url": "https://oteryn.com/download"
  }
}
```

**Members of `update`:**

- `update_channel`: the offered update channel, or `stable` when `release`
  was absent.
- `minimum_release_sequence`: the effective minimum. It is null when no
  minimum applies, for example in maintenance.
- `current_release_sequence`: from the active verified policy for that
  update channel. It is null when there is none.
- `manual_download_url`: a fixed, configured HTTPS URL of the public download
  page. It is not derived from the request.

**When `update` is present:**

- `NATIVE_LOGIN_CLIENT_UPDATE_REQUIRED` always carries `update`.
- `NATIVE_LOGIN_MAINTENANCE` carries `update` only when the active policy's
  `current_release_sequence` is above the offered one. In every other case
  it carries `Retry-After` and no `update`.

**What the body never contains:** the gate records, the scope, the World or
channel list, or any key or chain identity.

## 6. Update path, independent of gameplay TLS

**The update path.** The refusal sends the client to the existing updater:

- TUF metadata from `https://account.oteryn.com/updates/<u>/...`;
- packages from `https://downloads.oteryn.com`.

**Its trust is separate from gameplay TLS:**

- The update trust is the channel TUF Root shipped in the client. It is not
  the gameplay CA.
- Web TLS on those hosts uses the public WebPKI. It is transport only.

So a rotated or compromised gameplay CA never blocks updating.

**The client must:**

- treat `CLIENT_UPDATE_REQUIRED` with `update` as "run the updater for
  `update_channel`";
- install only a release that the verified policy names as current, and
  whose sequence is at or above `minimum_release_sequence`;
- then sign in again.

**A client with no usable update trust.** This covers:

- a build without the updater;
- a TUF Root that is missing, expired beyond recovery or past the bounded
  rotation profile;
- a threshold TUF Root compromise.

Such a client cannot update automatically. It shows `manual_download_url` and
stops; it does not retry login in a loop. The user installs a fresh installer
from the download page, and that installer carries a current pinned Root. A
gameplay CA event never needs this path. A TUF Root compromise always does
(client update origin v1, "Initial trust and rotation").

## 7. Rotation and compromise rollout

`R(A)`, `R(A+B)` and `R(B)` are client releases trusting chain A only, both A
and B (overlap), and B only. `s_AB` and `s_B` are their release sequences on
the update channel.

**Planned rotation (A to B), in order:**

1. Publish `R(A+B)` through the updater. It is current in the active verified
   policy for every update channel of the scope. The minimum is unchanged.
2. B starts serving on every node of the scope while A still serves. The
   node presents the chain that matches the client (design doc §4).
3. Overlap check: an `R(A+B)` client works against both chains.
4. Only now set `enforcing` with `min_release_sequence.u = s_AB`. Clients
   below `s_AB` get `NATIVE_LOGIN_CLIENT_UPDATE_REQUIRED`, never a TLS error.
5. Wait until no grant issued under the old minimum can still be used (grant
   lifetime plus skew).
6. Retire A on the nodes.
7. Later, publish `R(B)` and optionally raise the minimum to `s_B`.

**Compromise of root or intermediate A:**

1. Set the affected scopes to `maintenance` at once. No grant is issued for a
   chain an attacker can impersonate. Every other path keeps working: the
   website, account, updater and other scopes.
2. Bring B into service. Publish `R(A+B)`, or `R(B)` directly when A must not
   be trusted at all.
3. Switch the scope from `maintenance` to `enforcing` with the minimum set to
   that release's sequence.

While a scope is in maintenance, a client older than the published fix gets
`update` in the maintenance body. It can update during the maintenance window
and is admitted as soon as the scope enforces.

**Platform-enforced preconditions.** The operator command (design doc §3)
refuses, writing nothing, to:

- set a minimum `m` on update channel `u` unless the active verified policy
  for `u` has `current_release_sequence >= m` and lists an artifact for every
  supported `(platform, architecture)`. A raised minimum must always have an
  installable update;
- move a scope from `maintenance` or `open` to `enforcing` with a minimum
  above the previous one without a `chain_evidence` reference. That reference
  points to the recorded overlap check of step 3, or to the B-serving check
  in a compromise. It is stored in the audit record.

**Lowering a minimum.** Lowering a minimum is allowed (rollback). It is
audited like a raise.

## 8. Acceptance tests

**Platform tests** (implementation PR, `crates/gameauth`, PostgreSQL where the
admission store needs it):

1. Old client refused before the grant. Setup: `enforcing`, minimum 43,
   offer sequence 42. Expect 409 `NATIVE_LOGIN_CLIENT_UPDATE_REQUIRED` with
   the exact `update` body. No attempt record, no signature, and the ticket is
   still usable: a retry with sequence 43 and a new `attempt_ref` gets a grant.
2. At the minimum admitted. Sequence equal to the minimum gets a grant. A
   higher sequence gets a grant.
3. Unknown release. With `unknown_release = refuse`, an offer without
   `release` gets the update refusal with `update_channel: stable`. With
   `admit`, it gets a grant.
4. Revoked release. A `release_id` in `revoked_release_ids` gets the update
   refusal, even above the minimum.
5. Scope precedence. With global 40, World 43 and game channel `open` 50, the
   effective minimum is 43. With `channel_id` null and only one candidate
   below its minimum, that candidate is skipped. With all candidates gated,
   the result is the update refusal, not `ROUTE_UNAVAILABLE`.
6. Update channels are separate. A beta sequence is never compared with a
   stable minimum.
7. Maintenance. Maintenance gives 503 `NATIVE_LOGIN_MAINTENANCE`. It has
   `update` only when a newer current release exists. It never gives a
   grant.
8. Recovery path. Rows 1, 3 and 7 also hold for §19.4 recovery.
9. Wire. A malformed `release` (an extra member, a sequence of 0, an unknown
   channel) is refused by the Gateway and by the issuer. A changed `release`
   with the same `attempt_ref` is an offer conflict.
10. Operator preconditions. Raising the minimum above the active policy's
    current sequence is refused with nothing written. So is raising it
    without `chain_evidence`. Each change writes one audit row.

**Joint tests** (Game client and node, preproduction smoke, named in
ADR-0024):

11. Old client refused before the grant. An `R(A)` client after step 4 gets
    the update refusal, and no TLS handshake is attempted.
12. Overlap client works against both chains. An `R(A+B)` client completes
    the handshake and the session on a node serving A and on one serving B
    (step 3).
13. Next-only client after retirement. An `R(B)` client works after step 6,
    when A is no longer served anywhere.

## 9. Not covered

- A client can lie about `release`. The gate is a compatibility and UX gate
  that turns a TLS failure into an update prompt. It is not client
  attestation and authorizes nothing. Authentication stays with the ticket
  and grant. Gameplay trust stays with the client's pinned CA and the node.
- Per-platform or per-architecture minimums. This follows ADR 0035: one
  sequence per update channel.
- Which roots a release trusts, and how a node picks a chain. That is Game
  ADR-0024.
