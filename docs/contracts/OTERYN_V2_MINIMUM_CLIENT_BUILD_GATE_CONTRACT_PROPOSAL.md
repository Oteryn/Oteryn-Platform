# Minimum client build gate v1

Status: **PROPOSED**. Owner decision (b), 2026-11-19: Platform gets this gate,
as a contract and then an implementation, before Game ADR-0024 (Game private
gameplay CA, Oteryn-Game #3327) is accepted. ADR-0024 rests on it.
Design notes, data model and rollout runbook:
`docs/architecture/MINIMUM_CLIENT_BUILD_GATE_DESIGN.md`.

This proposal authorizes no code, migration, route, flag, key or deployment.
The implementation is a separate PR after acceptance.

"Login contract" below means `OTERYN_V2_NATIVE_GATEWAY_LOGIN_CONTRACT.md`.

## 1. Why

Today the admission issuer cannot tell an old client from a new one:

- `offer.client_build` is diagnostic only (login contract §3.1). It selects
  nothing, authorizes nothing and has no order.
- `release_id` is an opaque name with no order.

So when a gameplay CA root or intermediate is rotated or compromised, a client
that does not trust the new chain gets a grant, connects, and fails the TLS
handshake. The user sees a connection error, not "update your client".

This contract adds two things:

- an ordered build identifier in the login offer;
- a minimum accepted build, checked before a grant is issued.

## 2. Ordered build identifier

The order is the one ADR 0035 already defines: the **release sequence**.

**Release sequence.**

- `release_sequence` is a positive integer, scoped to one **update channel**
  (`stable` or `beta`), strictly increasing per channel.
- The TUF-authenticated channel policy target (`policy-v1.json`) signs it as
  `current_release_sequence` for the current release, together with
  `minimum_supported_release_sequence`.

**What the build knows about itself.**

- Each client build has its `(update_channel, release_id, release_sequence)`
  compiled in as a build-time constant. There is no second manifest
  (ADR 0035 rejects one).
- The release pipeline refuses to activate a policy whose
  `current_release_id` names a build whose compiled constant differs from
  `current_release_sequence`.
- The client never derives the sequence from `current_version_display` or
  `client_build`.

**Comparing sequences.**

- Sequences of `stable` and `beta` are not comparable. Every minimum here is
  per update channel.
- "Update channel" (`stable`/`beta`) is distinct from the game channel
  (`channel_id`, a channel of a World). This contract always says which.

## 3. Wire change

### 3.1 Request: `offer.release`

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

**Member rules.**

- `release` is either absent or an object with exactly these three members.
- `update_channel`: exactly `stable` or `beta`.
- `release_id`: the Downloads release id grammar, 1..96 characters of
  `[A-Za-z0-9._-]`.
- `release_sequence`: integer in 1..2^53-1.
- Any other member, type or value: `NATIVE_LOGIN_REQUEST_MALFORMED`. The
  Gateway and the issuer both apply these rules.

**Offer digest and diagnostics.**

- `release` is part of the offer, so it enters the offer digest (login
  contract §6.1).
- A retry with the same `attempt_ref` and a different `release` is an offer
  change and conflicts like any other.
- `client_build` stays diagnostic. The gate reads only `release`.

**Gateway.** The Gateway decodes `offer` into its typed form and re-encodes
it in contract order. Its `Offer` type gains `release` with the same rules.

### 3.2 Responses

**New codes.** §5 adds two issuer codes. The Gateway passes an issuer error
through only when it knows the code and the exact member set (Gateway
`errors.rs`); anything else becomes `NATIVE_LOGIN_UNAVAILABLE`. So the
Gateway must learn both codes, and the `update` member, for protocol 2,
protocol 3 and recovery, and re-emit them byte for byte.

**Sections amended.** This contract amends these sections of the login
contract:

- §3.1, the `offer` member set;
- §11.1, which says the error body is exactly three members;
- §11.2, the code table;
- §19.1, which keeps protocol 2 byte for byte;
- §19.2.5, which says every other protocol 3 error body is exactly §11.1.

The amendment covers only requests that carry `offer.release`. A request
without `release` still gets exactly the §11.1 body (§5).

### 3.3 Deploy order and rollback floor

1. Platform deploys a version whose Gateway and issuer both accept
   `offer.release` and know the new codes. Flag
   `PORTAL_CLIENT_BUILD_GATE_ENABLED` is off.
2. A client release that sends `release` ships only after step 1 is confirmed
   in that environment.
3. From then on, that Platform version is a **rollback floor**. Rolling the
   Gateway or issuer back below it would turn every login from such clients
   into `NATIVE_LOGIN_REQUEST_MALFORMED`, a total lockout. The floor is
   recorded in login contract §14 and §19.11 and in the deploy runbook.
4. Only then is the flag turned on and a gate record set.

## 4. Minimum accepted build

### 4.1 Gate records

Gate records are kept per scope:

| Scope | Applies to |
|---|---|
| global | every World and game channel. One row always exists, seeded `open` |
| World | every game channel of one `world_id` |
| game channel | one `(world_id, channel_id)` |

Each record has:

- `min_release_sequence.stable` and `min_release_sequence.beta`, each a
  positive integer or absent;
- `unknown_release`, either `admit` or `refuse`;
- `state`, one of `open`, `enforcing` or `maintenance`;
- `revision`, the actor and the time.

### 4.2 Effective rule for one game channel

Take the global record, the World record and the game channel record. Only
records that are not `open` count.

**State.**

- If any counting record is `maintenance`, the state is maintenance.
- Otherwise, if any is `enforcing`, the state is enforcing.
- Otherwise the state is open.

**Minimum for update channel `u`.** The effective minimum is the maximum of:

- `minimum_supported_release_sequence` of the latest **activated, verified**
  Downloads policy generation for `u`. An expired head still contributes its
  minimum: minimums never fail open on expiry.
- `min_release_sequence.u` of every counting record.

**Unknown release.** `refuse` wins if any counting record says `refuse`.

**A release passes** when all three hold:

- the state is not maintenance;
- the release is not in `revoked_release_ids` of that latest generation;
- the release is at or above the effective minimum, or no minimum applies.

**An offer without `release`** passes only if the state is open, or the state
is enforcing and `unknown_release` is `admit`.

### 4.3 Where the check runs

**Maintenance runs first.** The issuer evaluates maintenance for the
requested scope (`channel_id`, or the character's World when it is null)
right after ticket authentication. That is before §7.4 candidate
qualification, so maintenance answers even when no node reports ready.

Maintenance is separate from `native_login_enabled` (§7.4 condition 2, U7).
A scope with login disabled keeps its existing `NATIVE_LOGIN_ROUTE_UNAVAILABLE`
behaviour. Gate maintenance is checked first.

**The build check runs after candidates are found.** It runs after §7.4
candidates are found and before the grant is prepared and signed, in the same
transaction (in code, between candidate selection and `prepare_on`).

- Candidates the release does not pass are removed. They also do not appear
  in the protocol 3 `channels` lists, so the client is never offered a
  channel it cannot enter.
- A requested `channel_id` that is a candidate but does not pass gets the
  gate refusal (§5). That takes precedence over
  `NATIVE_LOGIN_CHANNEL_UNAVAILABLE`.
- With `channel_id` null, selection runs over the passing candidates. If none
  passes and at least one was removed by the gate, the result is the gate
  refusal, not `NATIVE_LOGIN_ROUTE_UNAVAILABLE`.

The check runs the same way for protocol 2 and 3 and for §19.4 recovery.

**Ticket and retries.** A gate refusal rolls the transaction back, so the
ticket is still unused (as §19.2.5). Tickets are short-lived (§4.1), so in
practice the client signs in again after updating.

A §6.2 retry of an attempt committed before a gate change returns the stored
grant without running the gate again. That is by design: a committed grant
is not revoked by the gate.

**Sessions.** The gate decides only whether a new grant is issued. Live
sessions and fast reconnect never reach Platform (§19.6). Ending them is a
rollout step (§7), not part of the gate.

## 5. Refusals

Two new rows for login contract §11.2:

| Internal code | HTTP | Category | Progression | Next authority | Public class |
|---|---|---|---|---|---|
| `NATIVE_LOGIN_CLIENT_UPDATE_REQUIRED` | 409 | `UNSUPPORTED_REVISION` | `TERMINAL` | client update, then a fresh sign-in, ticket and attempt | `CLIENT_UPDATE_REQUIRED` |
| `NATIVE_LOGIN_MAINTENANCE` | 503 | `DEPENDENCY_UNAVAILABLE` | `RETRYABLE` | a fresh sign-in after `Retry-After` | `TEMPORARILY_UNAVAILABLE` |

### Which refusal a request gets

**A request with `offer.release`:**

- Maintenance applies and the latest generation's `current_release_sequence`
  for its update channel is above the offered one:
  `NATIVE_LOGIN_CLIENT_UPDATE_REQUIRED` with `update`. The client then
  updates during maintenance instead of retrying.
- Maintenance applies otherwise: `NATIVE_LOGIN_MAINTENANCE`, exact §11.1
  body, with `Retry-After`.
- The release does not pass (below the minimum, or revoked):
  `NATIVE_LOGIN_CLIENT_UPDATE_REQUIRED` with `update`.

**A request without `offer.release`.** That is every client released before
this contract, so the body must not change shape for it.

- Maintenance: `NATIVE_LOGIN_MAINTENANCE` is a new code an old client may not
  know. It therefore gets `NATIVE_LOGIN_ROUTE_UNAVAILABLE`, exact §11.1 body,
  class `TEMPORARILY_UNAVAILABLE`.
- Refused as unknown: `NATIVE_LOGIN_OFFER_UNSUPPORTED`, exact §11.1 body,
  class `CLIENT_UPDATE_REQUIRED`. That code already exists.

### The `update` member

`update` is added to the §11.1 body, with the request's `protocol_version`:

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

| member | value |
|---|---|
| `update_channel` | the offered update channel |
| `minimum_release_sequence` | the effective minimum, or null when only revocation or maintenance applies |
| `current_release_sequence` | from the latest non-expired generation for that update channel, or null |
| `manual_download_url` | a fixed, configured HTTPS URL of the public download page, never derived from the request |

No gate record, scope, World or channel list, key or chain identity appears
in any body.

## 6. Update path, independent of gameplay TLS

**Where the client goes to update.** `CLIENT_UPDATE_REQUIRED` with `update`
means "run the updater for `update_channel`". The updater uses:

- TUF metadata from `https://account.oteryn.com/updates/<u>/...`;
- packages from `https://downloads.oteryn.com`.

**Why gameplay TLS cannot block it.** Its trust is the channel TUF Root
shipped in the client, not the gameplay CA. Web TLS on those hosts is
WebPKI and transport only. A rotated or compromised gameplay CA therefore
never blocks updating.

**What the client installs.** The client installs only the release that the
verified policy names as current, and only if that release's sequence is at
or above `minimum_release_sequence`. Then it signs in again.

**A client with no usable update trust.** This covers:

- a build without the updater;
- a missing TUF Root, or one expired beyond recovery;
- a TUF Root past the bounded rotation profile;
- a threshold TUF Root compromise.

Such a client shows `manual_download_url` and stops; it does not retry in a
loop. The user installs a fresh installer, which carries a current pinned
Root.

A request without `release` gets no `update` member, so that client shows its
generic "update required" screen. A gameplay CA event never needs the manual
path. A TUF Root compromise always does (client update origin v1).

## 7. Rotation and compromise rollout

`R(A)`, `R(A+B)` and `R(B)` are client releases trusting chain A only, both A
and B (overlap), and B only. `s_AB` and `s_B` are their sequences. Every
client released before this contract is an `R(A)` client that sends no
`release`.

### Planned rotation (A to B)

1. Publish `R(A+B)`. It sends `release`. It is current in the latest
   verified generation of every update channel of the scope. No gate change
   yet.
2. B starts serving on every node of the scope while A still serves
   (design §4).
3. Overlap check: an `R(A+B)` client works against both chains (test 12).
4. Only now set `enforcing` with `min_release_sequence.u = s_AB` and
   `unknown_release = refuse`, with `chain_evidence` naming step 3. From now
   on no new grant goes to an `R(A)` client.
5. End live `R(A)` sessions. Wait for grant lifetime plus skew. Then drain
   the scope's nodes, or use a staff kick, so that every session is
   re-admitted through the gate. Fast reconnect never reaches Platform, so
   waiting alone is not enough.
6. Retire A on the nodes.
7. Later, publish `R(B)` and optionally raise the minimum to `s_B`.

### Compromise of root or intermediate A

1. Set the affected scopes to `maintenance` at once. This works with no node
   ready. Nothing else is locked out: website, account, updater and other
   scopes keep working.
2. Drain the scope's nodes, so no session remains on a chain an attacker
   can impersonate.
3. Bring B into service. Publish `R(A+B)`, or `R(B)` when A must not be
   trusted at all.
4. Move the scope from `maintenance` to `enforcing`, with the minimum at that
   release's sequence and `unknown_release = refuse`.

During step 3, a client that sends `release` and is older than the new
current release gets `CLIENT_UPDATE_REQUIRED` with `update`, so it updates
during the window.

### Platform-enforced preconditions

The operator command refuses, writing nothing, in these cases:

- **A minimum without an installable update.** A minimum `m` on update
  channel `u` is refused unless the latest non-expired verified generation
  for `u` has `current_release_sequence >= m`. That generation must also list
  an artifact for every `client_platform` the login contract admits
  (`windows`, x86_64 in v1).
- **A raise without evidence.** A move to `enforcing` with a minimum above
  the scope's previous effective minimum is refused without a
  `chain_evidence` reference, which is stored in the audit row.
- **A stale write.** The `expected_revision` given does not match the
  record's current revision.

`maintenance` needs no evidence and no policy, so issuance can always be
stopped at once. Lowering a minimum (rollback) is allowed and audited.

## 8. Acceptance tests

**Platform** (implementation PR, `crates/gameauth` and `crates/gateway`,
PostgreSQL where the admission store needs it):

1. **Old client refused before the grant.**
   - Setup: `enforcing`, minimum 43, offer with sequence 42.
   - Expect: 409 `NATIVE_LOGIN_CLIENT_UPDATE_REQUIRED`, with the exact
     `update` body, through the Gateway for protocol 2 and 3.
   - Also: no attempt record and no signature; the ticket is still unused.
2. **At the minimum.** A sequence equal to the minimum gets a grant. A higher
   one gets a grant too.
3. **No `release`.**
   - With `refuse` on any counting record, the request gets the exact §11.1
     body `NATIVE_LOGIN_OFFER_UNSUPPORTED`, byte-identical to today's.
   - With `admit` on every counting record, it gets a grant.
4. **Revoked release.** A revoked `release_id` gets the update refusal, even
   above the minimum.
5. **Scope combination.**
   - Global enforcing 40, World enforcing 43, game channel open 50: the
     effective minimum is 43.
   - With `channel_id` null, a gated candidate is skipped and missing from
     the `channels` lists.
   - A requested gated channel gets the update refusal, not
     `CHANNEL_UNAVAILABLE`.
   - With every candidate gated, the result is the update refusal, not
     `ROUTE_UNAVAILABLE`.
6. **Separate update channels.** A beta sequence is never compared with a
   stable minimum.
7. **Maintenance.**
   - With no ready node, a request with `release` gets
     `NATIVE_LOGIN_MAINTENANCE`, exact §11.1 body, with `Retry-After`.
   - When a newer current release exists, it gets the update refusal
     instead.
   - A request without `release` gets `NATIVE_LOGIN_ROUTE_UNAVAILABLE`.
   - Maintenance never produces a grant.
8. **Recovery.** Rows 1, 3 and 7 also hold for §19.4 recovery.
9. **Wire.**
   - A malformed `release` is refused by the Gateway and by the issuer: an
     extra member, a sequence of 0, an unknown channel, or a 97-character id.
   - A changed `release` with the same `attempt_ref` is an offer conflict.
   - The Gateway relays both new codes and `update` byte for byte. It still
     turns an unknown issuer code into `NATIVE_LOGIN_UNAVAILABLE`.
10. **Operator command.**
    - These are refused, with nothing written: a raise above the current
      sequence, a raise without `chain_evidence`, and a stale revision.
    - A change committed concurrently with issuance is either seen by that
      issuance or ordered after its commit; never a grant signed after the
      raise committed. This includes the first `maintenance` on a scope with
      no row.
    - Each change writes one audit row.

**Joint** (Game client and node, preproduction smoke, named in ADR-0024):

11. **Old client refused before the grant.**
    - An `R(A)` client after step 4 gets the update refusal; no TLS
      handshake is attempted.
    - A pre-contract client gets `NATIVE_LOGIN_OFFER_UNSUPPORTED` and shows
      its "update required" screen.
12. **Overlap client works against both chains.** An `R(A+B)` client
    completes the handshake and a session against a node serving A and
    against one serving B (step 3).
13. **Next-only client after retirement.** An `R(B)` client works after
    step 6, when A is no longer served anywhere. No `R(A)` session remains.

## 9. Not covered

- **The gate is not client attestation.** A client can lie about `release`.
  The gate is a compatibility and UX gate that turns a TLS failure into an
  update prompt. It authorizes nothing. Authentication stays with the ticket
  and grant; gameplay trust stays with the client's pinned CA and the node.
- **No per-platform or per-architecture minimums**, following ADR 0035.
- **Not decided here:** which roots a release trusts, and how a node picks a
  chain. Game ADR-0024 decides those (design §4).
