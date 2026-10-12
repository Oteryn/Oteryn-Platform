# Oteryn-v2 Native Gateway Login Contract (N4-P)

## Status

`CANDIDATE — ACCEPTANCE BY GAME ARCHITECT AND OWNER REQUIRED (Oteryn-Game #162, Q15a); IMPLEMENTATION, MIGRATION, DEPLOYMENT AND ACTIVATION NOT AUTHORIZED BY THIS DOCUMENT`

- Contract ID: `oteryn-native-gateway-login-v1`
- Governing Issue: Oteryn/Oteryn-Platform#1419 (delivery item 1). Coordination: Oteryn/Oteryn-Game#162.
- Owner decisions (Oteryn-Game #162, 2026-09-29): Q14 (cross-repository authority, comment 5899892092), Q15a, Q16b, Q17a, Q18a (comment 5899942821).
- Pinned sources at authoring: Platform `main@db9ce9703b0f5af0ce6dc93ca279d4e2621c27bf`; Game `main@50d75c6573344be02984b38ea79e32fc9a9caf1f`.
- Game authorities consumed read-only: `docs/architecture/ADR-0003-platform-identity-game-gateway-and-admission-boundary.md`, `docs/architecture/ADR-0020-native-client-gameplay-entry.md` (§3, §7 N4-P), `docs/contracts/FND-04_PRE_ADMISSION_GRANT_PROFILE_V1.md` (profile `oteryn-pre-admission-v1`), `docs/architecture/FND-04A_AUTHORITY_FRESH_ADMISSION_CONTRACT.md` (§11 error subset), `docs/contracts/CHARACTER_AUTHORITY_PLATFORM_BOUNDARY.md`.
- Game-side companion candidates (same coordination, Oteryn-Game PR that references this file): `docs/contracts/OTERYN_GAME_NATIVE_RUNTIME_STATUS_PRODUCER_V1.md` and `docs/contracts/OTERYN_GAME_LIST_CHARACTERS_FOR_ACCOUNT_PROJECTION_V1.md`. Game owns their wire; this contract defines only the Platform consumer rules.
- Platform contracts refined, not replaced: `OTERYN_V2_PRE_ADMISSION_HANDOFF_CONTRACT.md`, `OTERYN_V2_RUNTIME_STATUS_PROJECTION_CONTRACT.md`, `GAME_GATEWAY_IDENTITY_CONTRACT.md`, `OTERYN_V2_WORLD_TOPOLOGY_CONTRACT.md`, `OTERYN_V2_NATIVE_EVIDENCE_PRODUCER_CONTRACT.md`.

This contract fixes the exact native login exchange that ADR-0020 N4 (Rust client) needs from Platform. It changes no accepted semantics of the contracts above. Where it chooses among options those contracts left open, the choice is marked **Decision** and needs acceptance; every fact not yet established is marked `UNKNOWN` with an identifier (U1…U14, list in §15).

## 1. Scope

In scope:

- the native branch of the public Gateway `POST /v1/login` (request, response, errors, limits);
- native Game Login Ticket issuance and redemption returning canonical `AccountId` (UUIDv7);
- Character selection against Platform's read model of the Game-owned `ListCharactersForAccount` projection (Q17a);
- native route selection over `NativeTopologyRegistry` identities and World Registry route records, gated by automatically reported and ownership-bound Game runtime status (Q16b);
- issuance of the FND-04 fresh-entry grant and custody of its signing key;
- `attempt_ref` idempotency and replay protection;
- the error mapping to FND-04 classes;
- what stays legacy-engine-only.

Out of scope: game admission itself (Game FND-04A), reconnect/recovery grants (`oteryn-reauth-recovery-v1`; candidate amendment §19), channel switch, public status pages, capacity-aware balancing, production PKI, deployment. Multiple channels per World and reauthenticated recovery are proposed by the candidate amendment §19 (protocol 3), which leaves protocol 2 unchanged.

**Q18a internal-build allowance.** An internal test build of the Rust client may use this login path before ADR-0020 N6 (creatures and other players) and N7 (chat) land. That allowance is a Game release rule only: Platform does not know or check which client features exist, grants no extra authority to internal builds, and the release entry still needs the full ADR-0020 §7 set. Internal-build use happens only in `testing`/`preproduction` topology (see U8). The push projection transport with its liveness watermark is accepted on the basis of this allowance; the release entry needs Decision P1 (§5.2).

## 2. End-to-end flow

```text
Rust client (native OAuth client, U1)
 1. OAuth Authorization Code + PKCE S256, scope game:ticket          (existing Platform OAuth)
 2. GET  /v1/game-auth/native-characters   Bearer <access token>     (Platform, §5.3; Decision D2)
      -> CharacterSummary[] from Platform's projection read model
 3. POST /v1/game-auth/tickets             Bearer <access token>     (existing route, native ticket kind §4)
      -> one-time native Game Login Ticket; access + refresh token revoked (existing behaviour)
 4. POST Gateway /v1/login  {protocol_version: 2, ticket, attempt_ref, character_id, offer}
 5. Gateway -> Platform private issuer (§3.3), one call:
      native redeem + attempt binding + Character check + route selection + claim assembly
      -> commit -> deterministic Ed25519 signature
 6. Gateway -> client {Registry route endpoint, TLS identity, ALPN oteryn-game/1, grant}
 7. client -> game node: TLS 1.3, ALPN oteryn-game/1, FND-04 admission with the grant
```

No step falls back to password login, OAuth-token authentication at the game server, the legacy engine Game Session issuer or a legacy engine route.

## 3. Public Gateway native branch

### 3.1 Request

`POST /v1/login`, `Content-Type: application/json`, body at most 2048 bytes, exact member set (unknown, duplicate, missing or `null` members reject), UTF-8, nesting at most 3:

```json
{
  "protocol_version": 2,
  "game_login_ticket": "<opaque native ticket>",
  "attempt_ref": "0192b3c4-5d6e-7f80-9a1b-2c3d4e5f6a7b",
  "character_id": "0192b3c4-5d6e-7f80-8a1b-2c3d4e5f6a7c",
  "channel_id": null,
  "offer": {
    "client_build": "0.1.0+abc123",
    "client_platform": "windows",
    "transports": [ { "protocol_major": 1, "transport_profile": 1, "alpn": "oteryn-game/1" } ]
  }
}
```

- `protocol_version: 2` selects the native branch. Version 1 keeps its current legacy-compatible meaning (§12). Any other value: `NATIVE_LOGIN_UNSUPPORTED_VERSION`.
- `game_login_ticket`: the opaque ticket, 1..256 ASCII bytes; never logged.
- `attempt_ref`: canonical lowercase RFC 9562 UUIDv7 (version 7, RFC variant, non-nil). The client generates it once per logical login attempt and reuses it byte-for-byte on every retry of that attempt (§6). It is the FND-04 `attempt_ref` claim.
- `character_id`: canonical lowercase UUIDv7 chosen from step 2. It is a claim to validate, never authority.
- `channel_id`: optional (`null` or a canonical lowercase UUIDv7); a preference that must itself be an eligible candidate (§7.4). It is the only nullable member.
- `offer.transports`: 1..4 entries, each exact `{protocol_major, transport_profile, alpn}`. v1 supports only `protocol_major = 1`, `transport_profile = 1` (TCP + TLS 1.3, Game `PROTOCOL_OTERYN_TRANSPORT_POLICY.json`) and `alpn = "oteryn-game/1"`. Unknown combinations are ignored for selection; if none is supported: `NATIVE_LOGIN_OFFER_UNSUPPORTED`.
- `offer.client_build` (1..64 visible ASCII) and `offer.client_platform` (`windows` in v1; set closed, extended by revision) are diagnostics only. They select nothing and authorize nothing.

### 3.2 Success response

`200`, `Cache-Control: no-store, private`, exact member set:

```json
{
  "protocol_version": 2,
  "attempt_ref": "0192b3c4-5d6e-7f80-9a1b-2c3d4e5f6a7b",
  "world_id": "<canonical UUIDv7>",
  "channel_id": "<canonical UUIDv7>",
  "endpoint": {
    "host": "game-eu1.example.invalid",
    "port": 7172,
    "tls_server_name": "game-eu1.example.invalid",
    "alpn": "oteryn-game/1",
    "protocol_major": 1,
    "transport_profile": 1
  },
  "grant": {
    "profile": "oteryn-pre-admission-v1",
    "token": "<JWS compact, at most 4096 ASCII bytes>",
    "valid_for_seconds": 20
  }
}
```

- `endpoint.host`: DNS name (1..253, LDH labels) or IP literal; `port` 1..65535. The client connects there and nowhere else; it never retargets the grant.
- `endpoint` comes from the World Registry route record of the selected scope (§7.3), never from a game node.
- **TLS identity:** the client verifies the server certificate for `tls_server_name` (SNI and name check) against the gameplay trust anchor and requires the negotiated ALPN to be exactly `alpn`; a mismatch terminates before any admission byte (FND-02). The trust anchor is Decision U3, a prerequisite of §7.3; until it is decided the dev/test path keeps the explicitly configured root used by `oteryn-dev-client`.
- `grant.valid_for_seconds` = `exp - server_now` in whole seconds, rounded down, at the time of this response (so a retry reports the remaining validity, not the original TTL). It is at least 1; a retry with less than 1 s left is answered `NATIVE_LOGIN_GRANT_EXPIRED`. The client never uses its own clock to extend or judge validity (FND-04 §7).
- `world_id`, `channel_id` and every revision are also signed inside the token; the plain copies are for the client's routing and display only.

### 3.3 Gateway to Platform

**Decision D1 (issuer placement).** The Go Gateway validates, rate-limits and forwards the native request to one private Laravel operation, `POST /internal/v1/game-auth/native-admissions` (the *native admission issuer*). The issuer performs redemption, Character check, route selection, claim assembly and signing inside one database transaction (§6). The Gateway keeps no Platform database or signing credentials, holds no grant state and never assembles claims.

Rationale: one atomic Platform decision removes the three-call ambiguity (redeem, context, sign) and keeps the private key in one process. This is the "dedicated Platform authority invoked by Game Gateway" that `OTERYN_V2_PRE_ADMISSION_HANDOFF_CONTRACT.md` allows; route/selection ownership stays with the Platform Gateway boundary of ADR-0003.

- Request and response bodies are the §3.1/§3.2 shapes plus nothing; the issuer is the only place that parses the ticket.
- Service authentication reuses the existing Gateway service credential mechanism (`RequireGatewayServiceCredential`, rotated hash-compared credential, TLS). mTLS workload identity is preferred for production (U14).
- The issuer route is private ingress only and is not reachable from the public edge.

## 4. Native ticket and redemption (#1419 item 2)

### 4.1 Native ticket kind

A native ticket is a Game Login Ticket with:

- audience `oteryn-native-game-gateway` (distinct from the legacy engine audience `oteryn-game-gateway`);
- binding to the persisted Identity, its canonical `identities.account_id` (lowercase UUIDv7, non-null) and its `native_security_generation` at issuance;
- the existing properties: at least 256 bits of CSPRNG entropy, hash-at-rest, single use, TTL at most 60 seconds (default 60), never logged.

Issuance is the existing `POST /v1/game-auth/tickets` under the existing OAuth checks (valid bearer, allowed first-party client, scope `game:ticket`, current `game_auth_generation`, Identity enabled and not terminated), with these native differences:

- the ticket kind is selected by the authenticated OAuth client, never by a request parameter: the native Rust client receives native tickets only (U1: whether this is a separate Passport client or the existing native public client);
- native issuance requires a canonical non-null `account_id` and does **not** require an `IdentityCanaryAccount` binding;
- `canary_account_id` is not stored on a native ticket.

A legacy engine ticket never redeems on the native branch and a native ticket never redeems on the legacy engine branch (audience mismatch → `NATIVE_LOGIN_TICKET_REJECTED`).

### 4.2 Native redemption

Inside the issuer transaction (§6), under row locks, equivalent to:

1. derive the ticket hash and lock the ticket row;
2. require audience exactly `oteryn-native-game-gateway`, `used_at` null, `expires_at > server_now`;
3. lock the Identity; require it exists, is enabled and not terminated;
4. require `identities.account_id` equals the ticket-bound `account_id` and is canonical lowercase UUIDv7;
5. require the current `native_security_generation` equals the ticket-bound generation (a revocation in between advances it and fails redemption);
6. mark the ticket used and record `attempt_ref` on it;
7. yield `AccountId` and `account_security_generation` (decimal of `native_security_generation`).

`AccountId` is Platform-owned (Platform ADR 0028) and is never derived from legacy engine ids or integer Identity ids. Any failure in steps 1–5 is `NATIVE_LOGIN_TICKET_REJECTED` or `NATIVE_LOGIN_ACCOUNT_SECURITY_DENIED` and rolls the transaction back.

## 5. Character selection (Q17a)

**Status (PLATFORM-LCFA-1, Oteryn/Oteryn-Platform#1465):** the §5.2 ingestion, the §5.3 read and the §5.4 issuance check are implemented for `testing`/`preproduction` only, default off (`GAME_AUTH_NATIVE_ACCOUNT_CHARACTERS_ENABLED`); enabled in any other environment or misconfigured, ingestion and the read answer `503` and issuance fails closed with `NATIVE_LOGIN_UNAVAILABLE`. Decision P1 and U12 stay open for the release entry.

### 5.1 Source

Platform stores a read model of the Game-owned projection `ListCharactersForAccount(AccountId) -> CharacterSummary[]`, pushed by Game over mTLS as defined in the Game candidate `OTERYN_GAME_LIST_CHARACTERS_FOR_ACCOUNT_PROJECTION_V1.md`. Platform is read-only toward Character Authority: it never creates, edits or deletes a Character and never writes native character tables.

### 5.2 Ingestion rules (Platform consumer)

- private route `POST /internal/v1/game-auth/native-account-characters`, TLS 1.3 client certificate with trusted-terminator provenance as for `/internal/v1/game-auth/native-evidence`;
- **identity per purpose:** only the configured Character Authority projection identities (one per Character Authority host) may publish; a runtime-status or native-evidence identity is refused (`401`) here, and a projection identity is refused on every other route;
- one accepted snapshot per `AccountId` replaces the previous one; ordering is `(projection_epoch, projection_revision)` compared numerically; a lower pair is acknowledged as `superseded` and ignored; an equal pair with different content marks that account's read model `invalid` until a higher pair arrives;
- **epoch:** when Platform first sees a `projection_epoch` higher than the highest it has seen (in a snapshot or a watermark), every entry with a lower epoch becomes `invalid` at once; an account becomes usable again only when its snapshot in the new epoch arrives (the producer's full resync);
- **liveness watermark:** the producer sends a watermark to `POST /internal/v1/game-auth/native-account-characters/watermark` (same identity rules) at least every 10 s, carrying `projection_epoch` and `complete_through` (every Character change committed at or before that source time has been delivered). The feed is `live` while `platform_now - complete_through + clock_uncertainty <= S` (S proposed 30 s, U17) and the watermark's epoch equals the highest epoch seen; otherwise the whole feed is `stale`;
- a snapshot for an `AccountId` Platform does not know is stored but authorizes nothing;
- the read model is never proof of current ownership (CHARACTER_AUTHORITY_PLATFORM_BOUNDARY §3–§4; FND-04 step 13 revalidates).

**Decision P1 (transport).** Push with the watermark above is acceptable for Q18a internal test builds. For the release entry the architect must either accept push on the basis of the watermark bound S or require pull (Platform queries Game at issuance); recommendation: keep push only if measured S holds, else pull (U12).

### 5.3 Character list read (Decision D2)

`GET /v1/game-auth/native-characters` with the OAuth bearer from step 1 (scope `game:ticket`, same client and generation checks as ticket issuance, but it does **not** revoke the token). It returns the authenticated account's current read-model entries only:

```json
{ "protocol_version": 2, "characters": [ { "character_id": "…", "world_id": "…", "name": "…", "availability": "AVAILABLE" } ] }
```

Empty list when the account has no snapshot. `503` (empty body) when the projection feed is `stale` (§5.2), the account's read model is `invalid`, or its snapshot is in an epoch below the highest seen (after an epoch raise, until its resync arrives). Never another account's data; never public. Rate limit §10.

Alternative considered: a two-phase Gateway call (redeem, return characters and a one-time selection handle, then select). Rejected here because it adds a second bearer credential; the architect may choose it instead.

### 5.4 Issuance check

At issuance the issuer requires, for the redeemed `AccountId`:

- the projection feed is `live` (§5.2);
- the account read model exists, is in the highest seen epoch and is not `invalid`;
- `character_id` is listed with `availability = AVAILABLE`;
- the grant's `world_id` is that entry's `world_id` (the Character's current world).

Otherwise `NATIVE_LOGIN_CHARACTER_CONFLICT` (not listed, not available) or `NATIVE_LOGIN_ROUTE_UNAVAILABLE` (feed stale, account invalid or in an old epoch). This is the fail-closed staleness rule of `OTERYN_V2_PRE_ADMISSION_HANDOFF_CONTRACT.md` ("issuance fails closed rather than relying on final game rejection"). Within the bound S a stale entry can at worst produce a grant that Game admission rejects with `ADMISSION_ACCOUNT_CHARACTER_CONFLICT` or `ADMISSION_GRANT_WORLD_STALE`, never an admission.

## 6. `attempt_ref` idempotency and replay protection

### 6.1 Attempt record

The issuer keeps one durable attempt record per `attempt_ref` (schema not frozen): `attempt_ref` (unique), ticket hash, `AccountId`, `character_id`, requested `channel_id`, offer digest, the exact JWS signing input (`base64url(header) "." base64url(payload)`), `kid`, `iat`, `exp`, created time. It stores no signature and no complete token.

Offer digest: lowercase hex SHA-256 of the RFC 8785 (JCS) serialization of the validated `offer` object, with `transports` kept in request order. Equal digests mean an identical offer.

### 6.2 Algorithm

In one transaction:

1. Lock the attempt row for `attempt_ref`.
2. **Row exists** (a retry):
   - if ticket hash, `character_id`, `channel_id` and offer digest all equal the stored values:
     - `exp - server_now >= 1 s`: re-sign the stored signing input with the stored `kid` and return the §3.2 response. Ed25519 (RFC 8032) is deterministic, so the token is byte-identical to the first one: one logical issuance yields one capability and one `jti`;
     - otherwise: `NATIVE_LOGIN_GRANT_EXPIRED`. The attempt is **retired** only at `exp + 5 s + clock_uncertainty` (maximum verifier skew plus Platform's configured clock uncertainty), the first moment no verifier can still accept its grant;
   - any difference: `NATIVE_LOGIN_ATTEMPT_CONFLICT`, no mutation.
3. **Row absent**: redeem (§4.2), check the Character (§5.4), select the route (§7), assemble claims (§8), sign, insert the attempt row, commit. The response is returned only after commit.
4. Any rejection in step 3 rolls back everything: the ticket stays unused and no attempt row exists, so the same request can be retried until the ticket expires.
5. Unknown commit outcome (connection loss during commit), a response lost after commit, or a retry whose stored `kid` cannot be re-signed: `ADMISSION_ATTEMPT_RECONCILIATION_REQUIRED`. A signer failure before commit rolls back and is `NATIVE_LOGIN_UNAVAILABLE`. The client retries the **same** request (same ticket, same `attempt_ref`): step 2 returns the committed grant, or step 3 runs normally because nothing committed.

This satisfies FND-04 §9: a lost response or crash never lets Platform mint a second independently usable capability for one attempt, and a new independent attempt needs a new ticket (the old one is consumed once an attempt commits).

### 6.3 Replay and retention

- A ticket consumed by attempt A is rejected for any other `attempt_ref`.
- The same `attempt_ref` with another ticket is `NATIVE_LOGIN_ATTEMPT_CONFLICT`.
- Game consumes `jti` at most once (FND-04 §8); the Platform attempt record is issuance idempotency only and is never replay authority.
- The signing input is retained at least until retirement (`exp + 5 s + clock_uncertainty`) and then may be reduced to an audit row without `jti` or claims. The audit retention period is `UNKNOWN` (U13).
- Re-signing needs the stored `kid`'s private key; key rotation keeps the retiring key loaded until every attempt it signed has passed `exp` (§9.3). If it is not loaded, the result is `ADMISSION_ATTEMPT_RECONCILIATION_REQUIRED` until `exp`, then `NATIVE_LOGIN_GRANT_EXPIRED`.

## 7. Native route selection (Q16b)

### 7.1 Ownership of the route

Per ADR-0003 and `OTERYN_V2_PRE_ADMISSION_HANDOFF_CONTRACT.md` ("World Registry controls which route Platform may select; Oteryn-v2 controls runtime source facts"):

- the **World Registry** owns each scope's route record: `endpoint.host`, `endpoint.port`, `endpoint.tls_server_name`, ALPN, `transport_profile` and the `route_revision` bound to them (§7.3);
- **game nodes** report only readiness and their runtime facts and revisions (Q16b). A node never supplies an endpoint, SNI name or route record.

### 7.2 Runtime status input

Game nodes report their scope status automatically to `POST /internal/v1/game-auth/native-runtime-status` as defined by the Game candidate `OTERYN_GAME_NATIVE_RUNTIME_STATUS_PRODUCER_V1.md`. There is no operator-entered runtime status. Platform consumer rules (refining `OTERYN_V2_RUNTIME_STATUS_PROJECTION_CONTRACT.md`):

- **identity per node and purpose:** each game node host has its own runtime-status client certificate; Platform configures, per identity, the scopes that node host may serve. A report for a scope outside that list, or from a native-evidence or projection identity, is refused (`401`). Runtime-status identities are refused on every other route;
- **ownership binding:** Platform does not trust a node-asserted `scope_ownership_generation` by itself. The Game scope ownership authority (`oteryn-game-ops`, the #415 assignment) reports each committed assignment `(assignment_epoch, world_id, channel_id, ownership_generation, node identity)` to `POST /internal/v1/game-auth/native-scope-assignments` with its own ownership-authority identity. A node report is accepted only if its `(assignment_epoch, scope_ownership_generation)` equals the latest accepted assignment for the scope **and** its client identity is the one that assignment names;
- ordering per scope is `(assignment_epoch, scope_ownership_generation, source_revision)` numerically; lower is rejected as superseded; an equal triple with different `decision_identity` or content marks the scope `invalid`; for an equal triple and identical content, a later `observed_at` refreshes freshness only;
- **restore reset:** a Game restore of the durability root raises `assignment_epoch` (operator action, U16). On the first assignment with a higher epoch, Platform invalidates all runtime state of lower epochs for every scope; scopes route again only after a new assignment and a matching node report;
- `observed_at` later than Platform `now + clock_uncertainty` is `invalid`;
- evidence state is `fresh` only while `platform_now - observed_at + clock_uncertainty <= F` (F proposed 15 s, heartbeat proposed 5 s; U5); otherwise `stale`; never converted to `offline`;
- Platform acknowledges with the result class only; it stores `scope_ownership_generation` and `decision_identity` and never logs them or exposes them outside the grant (FND-04 §15).

### 7.3 Registry route record (Decision D3)

For each Registry-issued `(WorldId, ChannelId)` the Registry holds one current route record: `host`, `port`, `tls_server_name`, `alpn = oteryn-game/1`, `protocol_major = 1`, `transport_profile = 1`, and `route_revision`, where

```text
route_revision = "rt." <registry route version, decimal> "." <first 32 hex of SHA-256(JCS(route descriptor))>
route descriptor = {world_id, channel_id, host, port, tls_server_name, alpn, protocol_major, transport_profile}
```

(at most 64 characters, FND-04 grammar). The digest binds the revision to one scope and one endpoint: a `route_revision` copied from another scope or another endpoint never matches, so it cannot redirect a client or be replayed elsewhere. The game node is deployed with the Registry's `route_revision` for its scope (node-boot D5 declared value) and reports it; Game admission compares the grant's `route_revision` with the node's (`ADMISSION_GRANT_ROUTE_STALE` on mismatch). Changing any descriptor field creates a new `route_revision`, which invalidates outstanding grants.

**U3 is a prerequisite of D3.** The client can trust a Registry-published `tls_server_name` only if the gameplay certificate chains to an anchor the client already holds. Recommendation: an Oteryn-operated gameplay CA, name-constrained to the gameplay domain, shipped with the client (pinned), with the server certificate's name equal to the Registry `tls_server_name`. Alternative: public WebPKI for the gameplay domain. D3 cannot be implemented until U3 is decided.

### 7.4 Candidate set

For the Character's current `world_id` W, a channel C is a candidate only if all hold at issuance time:

1. (W, C) is a canonical Registry-issued pair (`NativeTopologyRegistry`) with a current route record (§7.3);
2. Platform native login policy for (W, C) allows login (not maintenance, not disabled). Its representation is `UNKNOWN` (U7); legacy engine `game_worlds.status`/`login_enabled` alone is not native readiness;
3. the latest accepted runtime report for (W, C) is `fresh`, ownership-bound (§7.2), not superseded, not `invalid`, and `ready = true`;
4. the report's `route_revision` **equals** the Registry route record's `route_revision`;
5. the route's `protocol_major`/`transport_profile`/ALPN appear in `offer.transports`, and the report's `protocol_major`/`transport_profile` equal the route's.

Gameplay revisions (`ruleset`, `content`, `map`, `world_policy`, `offer`) and `runtime_observation_revision` are taken from the report (Q16b: reported automatically, no manual source). Whether the Registry must also pin expected values for them is an architect question (U18); pinning would reintroduce a manual source, which Q16b excludes.

Selection: if `channel_id` is given it must be a candidate, else `NATIVE_LOGIN_ROUTE_UNAVAILABLE`; otherwise the candidate with the lowest canonical `channel_id` string is selected (deterministic; not an authority order). Capacity-aware selection is `UNKNOWN` (U6, Game OPS-CHANNEL-01). No candidate: `NATIVE_LOGIN_ROUTE_UNAVAILABLE` when a supported transport exists but no channel is ready, `NATIVE_LOGIN_OFFER_UNSUPPORTED` when no route matches any offered transport.

## 8. FND-04 grant issuance (#1419 item 4)

### 8.1 Header

Exactly, in this member order, no whitespace: `{"alg":"Ed25519","kid":"<kid>","typ":"oteryn-admission+jwt"}`. `kid` is 1..64 characters of `[A-Za-z0-9._-]`. No other member (`jku`, `x5u`, `x5c`, `jwk`, `crit`, `cty`, `zip`, `b64`) is ever emitted.

### 8.2 Claims

Exactly these claims, serialized in this order without whitespace, so the signing input is deterministic and fixtures are byte-stable:

| Claim | Value source |
|---|---|
| `iss` | constant `urn:oteryn:platform:game-admission` |
| `aud` | constant `urn:oteryn:game:admission` (string, never an array) |
| `iat` | issuer server time, whole seconds |
| `nbf` | `= iat` |
| `exp` | `iat + TTL`; TTL configured 5..30 s, proposed default 20 s (U4); never above 30 |
| `jti` | 32 bytes from a CSPRNG, base64url without padding (43 chars), fresh per attempt |
| `profile` | constant `oteryn-pre-admission-v1` |
| `purpose` | constant `fresh_entry` |
| `attempt_ref` | request `attempt_ref` |
| `account_id` | redeemed `AccountId` (§4.2) |
| `character_id` | request `character_id`, validated (§5.4) |
| `world_id` | Character's current world from the read model (§5.4) |
| `channel_id` | selected channel (§7.4) |
| `account_security_generation` | decimal string of the redeemed `native_security_generation`, non-zero |
| `route_revision` | Registry route record, equal to the selected report (§7.3, §7.4) |
| `runtime_observation_revision` | selected report |
| `scope_ownership_generation` | selected report, decimal non-zero uint64 string |
| `protocol_major` | integer `1` |
| `transport_profile` | integer `1` |
| `ruleset_revision`, `content_revision`, `map_revision`, `world_policy_revision`, `offer_revision` | selected report, each separately |

The response `endpoint` is the same Registry route record the `route_revision` digest covers. Every string value must already satisfy the FND-04 §5 grammar; a report value that does not is rejected at ingestion, and the issuer re-checks before signing (fail closed with `NATIVE_LOGIN_UNAVAILABLE`). No `compatibility_revision`, no `schema_revision`, no legacy engine id, no claim not listed. Token at most 4096 bytes, header at most 512 and payload at most 3072 bytes decoded.

### 8.3 Signing

The signature is RFC 8032 Ed25519 (pure) over the ASCII signing input, encoded as base64url without padding. The issuer signs only after every check in §4–§7 passed inside the transaction and returns the token only after commit.

## 9. Signing-key custody

### 9.1 Where the private key lives

- The Ed25519 private key for `oteryn-pre-admission-v1` exists only inside the Platform native admission issuer runtime: a dedicated process pool (for example its own PHP-FPM pool) running as its own Unix user, serving only `/internal/v1/game-auth/native-admissions` on private ingress. Public Laravel web workers, queue workers and other pools run as other users and cannot read the key. It is loaded from an externally injected secret file (proposed setting names `GAME_AUTH_NATIVE_ADMISSION_SIGNING_KEY_FILE`, `GAME_AUTH_NATIVE_ADMISSION_SIGNING_KEY_ID`), readable only by the service user, outside the repository, database, backups of the application database, logs, environment dumps and images.
- The Gateway, Game nodes, clients, CI and the NativeEvidence producer never hold it.
- The key is dedicated to issuer `urn:oteryn:platform:game-admission`, profile `oteryn-pre-admission-v1` and the configured fresh key purpose (`GAME_AUTH_NATIVE_EVIDENCE_FRESH_KEY_PURPOSE`). It is never shared with Passport/OAuth, the recovery profile (`urn:oteryn:platform:game-recovery`), Game Login Ticket hashing or any service credential.
- A KMS/HSM may replace the file only if it produces pure Ed25519 signatures over the exact signing input; vendor and delivery are `UNKNOWN` (U9). No secret value is part of any contract, fixture or repository file; the node-boot qualification key is test-only.

### 9.2 Public side: NativeSigningTrustRegistry

The public key is published with `NativeSigningTrustRegistry::publishTrustedKey` under the fixed scope (issuer, profile, fresh key purpose) and served to Game through `ReadFreshSigningTrustV1` (`OTERYN_V2_NATIVE_EVIDENCE_PRODUCER_CONTRACT.md`). Game verifies with trusted public keys only, selected by `kid` inside that fixed scope, with trust evidence at most 5 s old (FND-04 §13).

Registry rules for this scope, required before activation: a `kid` is bound to one public key forever (publishing an existing `kid` with a different key is rejected, trusted or not); at most two keys are trusted at once (a third `publishTrustedKey` is rejected until one is revoked); a revoked `kid` is never re-trusted.

Issuer self-check, at startup and before signing (result cached at most 5 s): the configured `kid` must be currently trusted in the registry for that scope, and its registered public key must equal the public key derived from the loaded private key. Otherwise the issuer signs nothing and returns `NATIVE_LOGIN_UNAVAILABLE`.

### 9.3 Rotation

1. Generate the new key pair inside the custody boundary.
2. `publishTrustedKey` a never-used `kid` (at most two trusted keys: current and retiring; the registry rejects a third).
3. Wait at least 5 s plus clock uncertainty so every verifier can observe the new trust revision.
4. Switch the signer to the new `kid`; keep the retiring private key loaded for re-signing (§6.3).
5. After the last attempt signed by the retiring key is retired (`exp + 5 s + clock_uncertainty`), `revokeKey` the retiring `kid` and destroy its private key.

Emergency: `revokeKey` immediately (Game rejects within the 5 s bounded-staleness window, FND-04 §13.3), and the self-check stops the signer. Profile-wide compromise: `revokeProfile`, then `publishNextProfileVersion` with a never-used `kid`. A revoked `kid` is never re-trusted. Rotation cadence is `UNKNOWN` (U9).

## 10. Rate limits and abuse controls

Initial values, to be tuned from measurement (U10). Exhaustion returns `NATIVE_LOGIN_RATE_LIMITED` with `Retry-After`, and changes no state; limits never change the exactly-once behaviour of §6.

| Surface | Key | Limit |
|---|---|---|
| Gateway `POST /v1/login` (protocol 2) | source IP | 30 / minute |
| Gateway `POST /v1/login` (protocol 2) | (`attempt_ref`, ticket hash) | 6 requests in total |
| `GET /v1/game-auth/native-characters` | bearer | 10 / minute |
| `POST /v1/game-auth/tickets` | bearer | 5 / minute (existing) |
| issuer `/internal/v1/game-auth/native-admissions` | Gateway identity | 120 / minute |
| issuer | `AccountId` | 10 committed attempts / minute |
| `native-runtime-status` ingestion | client identity | 600 / minute |
| `native-scope-assignments` ingestion | ownership-authority identity | 120 / minute |
| `native-account-characters` ingestion | client identity | 600 / minute |

Body limits: public request 2048 bytes; issuer request 2048 bytes; issuer response 6144 bytes; `native-characters` response 16384 bytes; projection publication 16384 bytes (Game candidate §8 worst case).

## 11. Errors

### 11.1 Response shape

Non-2xx responses from the native branch are `Cache-Control: no-store, private` with exactly:

```json
{ "protocol_version": 2, "error": { "code": "<public code>", "public_class": "<FND-04 public class>" }, "attempt_ref": "<echo or null>" }
```

`attempt_ref` is echoed only when it parsed as canonical. No ticket, token, hash, generation, key or internal exception text appears. For `SECURITY_TERMINAL` rows the public `code` collapses to `NATIVE_LOGIN_AUTHENTICATION_REQUIRED`; the internal code stays in metrics and audit only.

### 11.2 Mapping

Each code maps to exactly one Foundation category, progression and FND-04 public class. The client acts on `public_class` exactly as ADR-0020 §2 requires for FND-04 codes.

| Internal code | HTTP | Category | Progression | Next authority | Public class | FND-04A counterpart |
|---|---|---|---|---|---|---|
| `NATIVE_LOGIN_REQUEST_MALFORMED` | 400 | `INVALID_INPUT` | `TERMINAL` | corrected new attempt | `RETRY_LOGIN` | class of `ADMISSION_GRANT_MALFORMED` |
| `NATIVE_LOGIN_UNSUPPORTED_VERSION` | 400 | `UNSUPPORTED_REVISION` | `TERMINAL` | client update | `CLIENT_UPDATE_REQUIRED` | class of `ADMISSION_GRANT_REVISION_UNSUPPORTED` |
| `NATIVE_LOGIN_OFFER_UNSUPPORTED` | 409 | `UNSUPPORTED_REVISION` | `TERMINAL` | client update | `CLIENT_UPDATE_REQUIRED` | class of `ADMISSION_GRANT_REVISION_UNSUPPORTED` |
| `NATIVE_LOGIN_TICKET_REJECTED` | 401 | `AUTHENTICATION_FAILED` | `SECURITY_TERMINAL` | new OAuth sign-in and ticket | `AUTHENTICATION_REQUIRED` | class of `ADMISSION_GRANT_AUTHENTICATION_FAILED` |
| `NATIVE_LOGIN_ACCOUNT_SECURITY_DENIED` | 401 | `SESSION_REJECTED` | `SECURITY_TERMINAL` | new sign-in after account security permits | `AUTHENTICATION_REQUIRED` | `ADMISSION_GRANT_SECURITY_STATE_REVOKED` |
| `NATIVE_LOGIN_ATTEMPT_CONFLICT` | 409 | `INVALID_INPUT` | `TERMINAL` | new `attempt_ref` and new ticket | `RETRY_LOGIN` | issuance-only |
| `NATIVE_LOGIN_CHARACTER_CONFLICT` | 409 | `CONFLICT` | `TERMINAL` | new attempt after ownership/lifecycle change | `SESSION_UNAVAILABLE` | `ADMISSION_ACCOUNT_CHARACTER_CONFLICT` |
| `NATIVE_LOGIN_ROUTE_UNAVAILABLE` | 503 | `DEPENDENCY_UNAVAILABLE` | `RETRYABLE` | same request with backoff while the ticket is valid | `TEMPORARILY_UNAVAILABLE` | issuance-only |
| `NATIVE_LOGIN_RATE_LIMITED` | 429 | `CAPACITY_EXCEEDED` | `RETRYABLE` | same request after `Retry-After` | `TEMPORARILY_UNAVAILABLE` | class of `ADMISSION_CAPACITY_EXCEEDED` |
| `ADMISSION_ATTEMPT_RECONCILIATION_REQUIRED` | 503 | `DEPENDENCY_UNAVAILABLE` | `RETRYABLE` | same `attempt_ref` and same ticket only | `TEMPORARILY_UNAVAILABLE` | identical code (FND-04 §9) |
| `NATIVE_LOGIN_GRANT_EXPIRED` | 409 | `SESSION_REJECTED` | `TERMINAL` | fresh sign-in, ticket and attempt | `RETRY_LOGIN` | `ADMISSION_GRANT_EXPIRED` |
| `NATIVE_LOGIN_UNAVAILABLE` | 503 | `DEPENDENCY_UNAVAILABLE` | `RETRYABLE` | same request with backoff | `TEMPORARILY_UNAVAILABLE` | issuance-only |

"Class of" means the row has the same category, progression and public class as that FND-04A code, but a different cause at a different boundary. Game admission errors after the client presents the grant are FND-04A codes returned by the game node (ADR-0020 N8), not by this contract.

## 12. What stays legacy-engine-only

Unchanged and never used by the native branch:

- `POST /v1/login` with `protocol_version: 1`, the legacy engine `gameplay_offer`, `canary_account_id`, integer world/channel ids;
- `/internal/v1/game-auth/tickets/redeem` and its legacy engine binding requirement, audience `oteryn-game-gateway`;
- `/internal/v1/game-auth/accounts/{canaryAccountId}/login-context` and `GameLoginContextProvider`;
- the Game Session v1 issuer and the historical Game Session v2 producer (`GAME_SESSION_CANARY_CONTRACT.md`, `OTERYN_NATIVE_GAMEPLAY_PROTOCOL_CONTRACT.md`);
- `game_worlds.status`/`login_enabled` as compatibility state;
- the OTClient OAuth client and OTClient tickets.

A native failure never falls back to any of these, and a legacy engine request never reaches the native issuer.

## 13. Security and logging

- Never logged or traced: tickets, ticket hashes, grants, signing inputs, `jti`, private keys, OAuth tokens, `account_security_generation`, `scope_ownership_generation`, `assignment_epoch`, runtime `decision_identity` (it embeds the generation), Character names.
- Character names are accepted only as NFC UTF-8 without control (Cc), format (Cf, including bidi overrides U+202A–U+202E and isolates U+2066–U+2069, zero-width characters), `"` or `\\` characters (Game candidate §2.1); Platform rejects other snapshots as malformed.
- May be logged: `attempt_ref`, internal error code, `kid`, profile, `world_id`/`channel_id`, `route_revision`, timings. `AccountId`/`CharacterId` only pseudonymously where authorized; never as metric labels.
- All native responses are `no-store, private`.
- Platform authorization is never proof of admission; Game revalidates every bound fact (FND-04 §12).

## 14. Rollout, rollback and mixed versions

Classification per step (`CROSS_REPO_CONTRACTS.md` vocabulary):

1. accept this contract and the two Game candidates — documentation;
2. Game runtime-status producer, scope-assignment reporter and projection producer, reporting to Platform endpoints that may not exist yet (report failures never affect gameplay) — `server-first-safe`;
3. Platform ingestion endpoints and read models — `backward-compatible`;
4. native ticket kind, issuer and Gateway native branch behind a default-off switch — `backward-compatible`;
5. Game N4 client (`crates/platform-client`) — `client-first-safe` (fails closed while the branch is off);
6. joint E2E: Game node-boot qualification through the real Gateway and issuer (#1419 item 5);
7. enablement for internal builds in `testing`/`preproduction` only (Q18a); production needs separate authority and U8.

Rollback: switch the native branch off; outstanding grants expire within 30 s; legacy engine paths are untouched. A new claim, header member, profile or algorithm needs a new Game profile revision first; Platform never emits a claim the pinned Game verifier does not accept.

## 15. UNKNOWN register

| ID | Unknown | Blocks | Owner |
|---|---|---|---|
| U1 | Native Rust OAuth client: separate Passport public client or the existing native client; redirect URI | #1419 item 2 | Platform + owner |
| U2 | (Decision D2) character list before ticket vs two-phase Gateway selection | item 3, Game N4 | architect |
| U3 | Client trust anchor for the gameplay TLS certificate; prerequisite of D3. Recommended: Oteryn-operated, name-constrained gameplay CA pinned in the client; alternative WebPKI | N4, item 3 | architect + owner |
| U4 | Grant TTL default (proposed 20 s, bound 5..30 s) | item 4 | architect |
| U5 | Runtime-status freshness F and heartbeat H (proposed 15 s / 5 s) | item 3 | architect, measured |
| U6 | Channel selection beyond lowest ChannelId (capacity, load) | later | Game OPS-CHANNEL-01 |
| U7 | Representation of Platform native login policy per (WorldId, ChannelId) | item 3 | Platform |
| U8 | Production native topology issuance (`NativeTopologyRegistry` is restricted to `testing`/`preproduction`) | production | Platform + owner |
| U9 | Production key custody (file vs KMS/HSM), secret delivery and rotation cadence | production | owner |
| U10 | Rate-limit values | tuning | Platform, measured |
| U11 | (Decision D3) Registry route record and `route_revision` digest form (§7.3); U3 first | item 3 | architect |
| U12 | Push vs pull for the release entry (Decision P1, §5.2) | release | architect |
| U13 | Attempt audit retention period | item 4 | Platform + owner |
| U14 | Gateway→issuer auth (existing credential vs mTLS) | item 4 | Platform |
| U15 | Client-identity separation per purpose: per-node runtime-status certificates with scope lists, Character Authority projection certificates, ownership-authority certificate, all distinct from native evidence; PKI issuing them | items 3, 5 | Platform + Game + owner |
| U16 | Runtime-status restore reset: how `assignment_epoch` is stored and raised after a Game durability-root restore. Platform finding (#1426 review): the epoch is global, so any ownership-authority identity may raise it for every scope; either restrict epoch raises to an identity holding every scope or keep it an operator-only action. Two first reports for different scopes can gap-lock deadlock; this fails safe as `503` and the producer retries | item 3, Game | Game architect |
| U17 | Projection liveness bound S (proposed 30 s) and watermark period (10 s) | item 3 | architect, measured |
| U18 | Whether the Registry also pins gameplay revisions (conflicts with Q16b "no manual source") | item 3 | architect + owner |

## 16. Required contract tests before activation

- request/response exact-schema acceptance and rejection (unknown member, duplicate, non-canonical UUID, wrong version);
- native vs legacy engine ticket audience isolation in both directions;
- redemption races: generation advanced, Identity disabled, `account_id` changed, concurrent redeem (one winner);
- `attempt_ref` retry returns a byte-identical token with decreasing `valid_for_seconds`; conflict on changed ticket/character/channel/offer (JCS digest); unknown-commit reconciliation; retry after `exp`; retirement at `exp + 5 s + uncertainty`;
- Character not listed, not available, read model invalid;
- route: no fresh report, superseded generation, generation not matching the ownership-authority assignment, report from an identity not named by the assignment or outside its scope list, lower `assignment_epoch` after a reset, `ready=false`, maintenance, reported `route_revision` not equal to the Registry's, copied `route_revision` from another scope, offer mismatch, requested channel not eligible;
- projection: feed watermark stale, entry in an older epoch after an epoch raise, name with bidi/format characters rejected;
- identity per purpose: each ingestion route refuses the other purposes' certificates;
- claim fixtures verified by the pinned Game verifier (`apps/game-server/src/foundation/fnd04_verifier.rs`) at an exact Game commit, including each revision dimension mutated independently;
- signer self-check: untrusted `kid`, public-key mismatch, revoked `kid`; registry refuses re-keying a `kid` and a third trusted key; rotation overlap re-sign; key file unreadable from public workers;
- logs and responses free of every §13 secret;
- joint E2E through the real Gateway, issuer and game node (Game node-boot job).

## 17. Amendment N4P-3: testing/preproduction issuance modes

Owner decisions D171 and D172 (Oteryn-Game #162; answers 33c/34b, comment 5905264083) allow the private issuer (§3.3) to run before two prerequisites exist. Both modes are limited to the `testing` and `preproduction` environments, default off, and change no claim, header, error or wire shape.

**Mode 33a (D171): Character ownership not verified.** Until the Character read model exists (§5), the issuer does not verify `AccountId -> CharacterId` ownership, availability or current world. It is enabled only by `GAME_AUTH_NATIVE_ADMISSION_UNVERIFIED_CHARACTER_OWNERSHIP=true` together with one configured `GAME_AUTH_NATIVE_ADMISSION_UNVERIFIED_CHARACTER_WORLD_ID` (canonical UUIDv7) that serves as the Character's `world_id`. With the mode off, issuance fails closed with `NATIVE_LOGIN_ROUTE_UNAVAILABLE` (no Character source, §5.4). With the mode on outside `testing`/`preproduction`, or with an invalid world, issuance fails closed with `NATIVE_LOGIN_UNAVAILABLE`. The request `character_id` is copied into the grant unvalidated. Game FND-04A §5 admission stays the fail-closed guard: a foreign, unavailable or moved Character is rejected there (`ADMISSION_ACCOUNT_CHARACTER_CONFLICT`, `ADMISSION_GRANT_WORLD_STALE`). **Release gate:** CHAR-NAME-1, then LCFA-1, then the full §5.4 issuance check replaces this mode. The mode is never enabled for a release entry.

**Mode 34a (D172): Registry route record for testing/preproduction.** The §7.3 route record is stored on `game_channels` (`native_route_host`, `native_route_port`, `native_route_tls_server_name`, `native_route_version`, `native_route_revision`) with the §7.3 `route_revision` digest. It is published only through `NativeTopologyRegistry::publishRouteForPreproduction`, which is restricted to `testing`/`preproduction` like topology issuance (U8), and every read of the records refuses outside those environments on its own. An unchanged endpoint keeps its revision, and any endpoint change advances the version. `native_login_enabled` (default false) is the per-scope native login policy of §7.4 condition 2 in this mode. A stored record whose revision does not equal the one recomputed from its descriptor routes nowhere. The client verifies `tls_server_name` against the explicitly configured root used by `oteryn-dev-client` (§3.2). **Release gate:** U3 (gameplay CA), then D3 acceptance and U7 (production login policy), before any production route record exists.

The operator path of this mode is `game-auth:native-route:publish` and, for the fresh admission key it trusts, `game-auth:native-trust:publish-key` (ARCH-PREPROD-ROUTE-PUBLISH-AUTH-V1 §2): both run only in `testing`/`preproduction` behind the shared `DisposableNativeStore` guard of topology issuance, on the retained per-run SQLite file, and refuse with nothing written otherwise.

**Status (owner decision D1006, NATIVE-TRUST-PUBLISH-1):** a deployed `preproduction` stack with owner key custody trusts its issuer key in its own MariaDB trust registry with `game-auth:native-admission:publish-trust`. The command takes no input: it trusts only the public half of the configured current key loaded through the issuer keyring, runs only in `testing`/`preproduction` with native admission on and `GAME_AUTH_NATIVE_EVIDENCE_FRESH_KEY_PURPOSE` set, writes only through `publishTrustedKey` (§9.2: a kid bound to one key forever, no re-trust after revocation, at most two trusted keys, the high-water witness on every write), is idempotent, and prints only the trust scope, key id and public key. Synology deploy runs it after migration and before the signing self-check, on every route that (re)creates Platform, while the internal proxy and Gateway (the only holders of the Gateway service credential that admission issuance requires) are stopped; it starts them only after publication passes, and a failed publication removes the Platform, internal proxy and Gateway containers. No route record or production trust is published by it.

**Status (ADR 0044, ARCH-PREPROD-ROUTE-PUBLISH-AUTH-V1 amendment, owner option a, 2026-10-11):**

- `game-auth:native-topology:issue` and `game-auth:native-route:publish` gain a separate preproduction-store mode for the deployed `preproduction` MariaDB store. The `DisposableNativeStore` guard above stays unchanged for every other path.
- The mode is entered only by `GAME_AUTH_NATIVE_ROUTE_PREPRODUCTION_STORE_ENABLED` (default off, malformed refuses). Synology sets it from `OTERYN_NATIVE_ROUTE_PUBLISH_ENABLED`.
- It runs only with `APP_ENV=preproduction`, on the code-allowlisted store (`mysql`, host `mariadb`, port 3306, database `oteryn_platform`), with no outer transaction.
- It publishes exactly one route record: the one for the World whose WorldId equals the configured native World (`GAME_AUTH_NATIVE_ADMISSION_UNVERIFIED_CHARACTER_WORLD_ID`). It refuses a foreign World or a foreign route.
- It is idempotent under the version and revision rules above, and audit-logged in `admin_audit_events`.
- It prints identifiers and route fields only, never a credential or host, and fails closed with a named `NATIVE_PREPROD_STORE_*` reason and nothing written.
- No production route record is authorized by it.

Selection follows §7.4 over these records, using the runtime-status read model under the shared epoch lock. The issuer takes that lock before redeeming the ticket and before any non-locking read, so its snapshot sees every committed epoch raise (lock order: attempt row, epoch, ticket, Identity; ingestion: epoch, assignment, report). A report that is not `fresh` (stale, invalid, superseded or not ownership-bound), not ready, or whose `route_revision` differs from the record routes nowhere (`NATIVE_LOGIN_ROUTE_UNAVAILABLE`). The response `endpoint` is the record whose `route_revision` is bound in the grant. If that record changed after issuance, the retry answers `NATIVE_LOGIN_ROUTE_UNAVAILABLE` until `exp`, then `NATIVE_LOGIN_GRANT_EXPIRED`.

Issuer limits in this amendment: every InnoDB lock wait inside the issuer transaction is bounded by `GAME_AUTH_NATIVE_ADMISSION_LOCK_WAIT_TIMEOUT_SECONDS` (default 3 s, 1..10), and a timeout rolls back as `NATIVE_LOGIN_UNAVAILABLE`. The per-Gateway-credential limit is 120/minute (§10). The `native-scope-assignments` ingestion default is now 120/minute, matching §10. The Go Gateway forwarding of the native branch is a separate later change.

**Status (PLATFORM-LCFA-1, Oteryn/Oteryn-Platform#1465):** with the §5 read model enabled, the issuer applies the full §5.4 check and mode 33a is not consulted; with it disabled, mode 33a applies as above. The release gate of mode 33a is unchanged.

## 18. Non-authorization

This candidate authorizes no code, migration, route, configuration, secret, key, certificate, deployment, production change or Game repository change. Each #1419 delivery item needs its own task packet and PR after acceptance.

## 19. Amendment N4P-4 (candidate): protocol 3, multiple channels per World and reauthenticated recovery

Status: `CANDIDATE — OWNER DECISIONS RECORDED 2026-10-12; GAME ARCHITECT ACCEPTANCE REQUIRED; IMPLEMENTATION NOT AUTHORIZED`. Decision record: ADR 0045 (Proposed). Governing Issue: Oteryn/Oteryn-Platform#1559 (parent workstream #1419). §18 applies to this section unchanged.

Owner decisions of 2026-10-12:

- **U19:** the first multichannel World has three channels with the keys `antica`, `obscura` and `pythera` (§19.2.2). The display label is the key with its first letter capitalised. These three channels are also added to the staging World (§19.11 step S). The existing staging channel `oteryn-staging-1` keeps its key and route.
- **U20:** the handle lifetime H is 12 h (§19.3).
- **U21:** a §19.3 handle counts as fresh authentication for the recovery profile **only while it is bound to a device** (§19.15). Device binding is required scope, not a follow-up. It blocks enabling recovery anywhere beyond `testing`.
- **U22:** the Game recovery locator feed (G1) is built now and not deferred. §19.6 is its contract, and §19.14 is the ready-to-dispatch Game packet.

This amendment adds two things to the native branch:

- multiple channels per World: per-channel route records, channel selection reported in the login response, and a channel-specific refusal;
- a recovery path: a client that lost its game connection gets a short-lived `oteryn-reauth-recovery-v1` grant from the Gateway without a new browser sign-in or Game Login Ticket.

Protocol 2 (§3–§17) stays byte-for-byte as it is. Game authorities consumed read-only, in addition to the header list: `docs/contracts/FND-04_REAUTHENTICATED_RECOVERY_GRANT_PROFILE_V1.md` (the recovery profile), `docs/architecture/FND-04B_RECONNECT_RECOVERY_CONTINUITY_CONTRACT.md` (§9, §17, §20–§23), `docs/architecture/reviews/OTERYN_GAME_RECOVERY_SOURCE_TRANSPORT_AMENDMENT_2026-09-06.md` and `docs/architecture/MULTICHANNEL_SYSTEM_SCOPE_MATRIX.md`.

### 19.1 Versioning and routes

| Public Gateway route | Private issuer operation | Protocol |
|---|---|---|
| `POST /v1/login` | `POST /internal/v1/game-auth/native-admissions` | 2 (§3) or 3 (§19.2, §19.3) |
| `POST /v1/recover` | `POST /internal/v1/game-auth/native-recoveries` | 3 only |
| `POST /v1/recover/revoke` | `POST /internal/v1/game-auth/native-recoveries/revoke` | 3 only |

- `protocol_version: 3` on `POST /v1/login` uses the §3.1 request member set unchanged, with `3` in place of `2`. Only the success response (§19.2.4) and one error body (§19.2.5) differ.
- A protocol 2 request never receives a protocol 3 member. A protocol 3 member never appears in a protocol 2 response.
- The new private operations use the §3.3 rules: Gateway service credential, private ingress only, the Gateway holds no state and assembles no claims.
- Protocol 3 is a separate default-off switch, limited to `testing`/`preproduction` like §17. With the switch off, every protocol 3 request is `NATIVE_LOGIN_UNSUPPORTED_VERSION`.

### 19.2 Multiple channels per World

#### 19.2.1 Per-channel route records

Each Registry-issued `(WorldId, ChannelId)` holds its own §7.3 route record and `route_revision`. The digest already covers `channel_id`, so a revision never matches another channel.

**Decision M1 (distinct endpoints).** In protocol 3, two route records never share `(host, port)`. A record whose `(host, port)` equals another current record routes nowhere (`NATIVE_LOGIN_ROUTE_UNAVAILABLE` for both). Reason: the game node listens per channel scope and checks the grant's `channel_id` and `route_revision` against its own. Sharing one endpoint between channels would need the node to dispatch by grant, which is a Game change this amendment does not request.

Mode 34a (§17) widens from one route record to the route records of every Channel of the one configured World. Each publish still touches exactly one Channel row. `NATIVE_PREPROD_STORE_FOREIGN_ROUTE` then means a route record on a Channel of another World. This widening needs ADR 0045 acceptance, because it replaces the one-record clause of ADR 0044. It still authorizes no second World and no production record.

#### 19.2.2 First multichannel World (owner decision U19)

The Registry issues three Channels for the first multichannel World. The keys use the existing Registry grammar `[a-z0-9][a-z0-9-]{0,63}` (`NativeTopologyRegistry`), so no schema change is needed.

| `channel_key` | Display label | Example route record (Decision M1: each has its own endpoint) |
|---|---|---|
| `antica` | `Antica` | `host` `antica.<gameplay domain>`, `port` `<gameplay port>`, `tls_server_name` `antica.<gameplay domain>`, `route_revision` `rt.1.<digest>` |
| `obscura` | `Obscura` | `host` `obscura.<gameplay domain>`, `port` `<gameplay port>`, `tls_server_name` `obscura.<gameplay domain>`, `route_revision` `rt.1.<digest>` |
| `pythera` | `Pythera` | `host` `pythera.<gameplay domain>`, `port` `<gameplay port>`, `tls_server_name` `pythera.<gameplay domain>`, `route_revision` `rt.1.<digest>` |

- Every record has `alpn = oteryn-game/1`, `protocol_major = 1` and `transport_profile = 1` (§7.3).
- Each `<digest>` is computed from that channel's own descriptor, so the three revisions all differ.
- Placeholders stand for real values: no host, port or identifier is fixed here.
- **Staging.** The three channels are added to the staging World as three more Channels, under the widened mode 34a. Each gets its own route record on the NAS preproduction store (§19.11 step S).
- The existing channel `oteryn-staging-1` and its route are not renamed, re-keyed or reused. It was issued first, so it has the lowest canonical `channel_id`. Automatic selection therefore picks it while it is a candidate, unless its login flag is turned off.

**Display label:** the `label` is the key with its first ASCII letter upper-cased (`antica` becomes `Antica`). It is computed by Platform when the response is built, and nothing stores it. A later Registry label column may replace the rule without changing the protocol, because the client shows `label` and never derives it.

#### 19.2.3 Selection

§7.4 applies per channel, unchanged. The outcome is one of:

- `channel_id` given and a candidate: that channel is selected, and `channel_selection` is `requested`;
- `channel_id` given and not a candidate, while at least one other channel is a candidate: `NATIVE_LOGIN_CHANNEL_UNAVAILABLE` (§19.2.5);
- `channel_id` given, and no channel is a candidate: `NATIVE_LOGIN_ROUTE_UNAVAILABLE`, as in protocol 2;
- `channel_id` null: the lowest canonical candidate, with `channel_selection` `automatic`. U6 (capacity-aware selection) is unchanged.

#### 19.2.4 Protocol 3 success response

`200`, `Cache-Control: no-store, private`, exact member set:

```json
{
  "protocol_version": 3,
  "attempt_ref": "<request attempt_ref>",
  "world_id": "<canonical UUIDv7>",
  "channel_id": "<canonical UUIDv7>",
  "channel_selection": "automatic",
  "channels": [ { "channel_id": "<canonical UUIDv7>", "key": "antica", "label": "Antica" } ],
  "endpoint": { "...": "exactly as §3.2" },
  "grant": { "...": "exactly as §3.2, profile oteryn-pre-admission-v1" },
  "recovery": { "handle": "otr1.<43 base64url characters>", "expires_in": 43200 }
}
```

- `channel_selection` is `requested` or `automatic`.
- `channels` lists the §7.4 candidates of the Character's World at response time:
  - 1..16 entries in canonical `channel_id` order;
  - each entry is exactly `{channel_id, key, label}`;
  - `key` is the Registry `channel_key` and `label` is its display label (§19.2.2). Both are for display only;
  - no runtime fact, ownership generation, revision, load or endpoint appears in it;
  - it grants nothing. A §6.2 retry recomputes it, and the stored selected channel may be absent from a recomputed list.
- `recovery` is defined in §19.3. `expires_in` is whole seconds until the handle's absolute expiry.
- The grant, its claims and its `channel_id` are exactly §8. Protocol 3 adds no claim.

#### 19.2.5 Channel refusal

`NATIVE_LOGIN_CHANNEL_UNAVAILABLE` uses the §11.1 body plus one member, `channels`. It has the §19.2.4 entry shape and holds at least one entry. The issuer transaction rolled back (§6.2 step 4), so the ticket is still unused. The client may send the same ticket with a new `attempt_ref` and a listed `channel_id`, or `null`, until the ticket expires. Every other protocol 3 error body is exactly §11.1 with `protocol_version: 3`.

#### 19.2.6 Not covered

Changing channel after admission is a Game channel transition (ADR-0003 §7, FND-04B §23). It is not part of this amendment, and the `channels` list does not authorize it.

### 19.3 Recovery handle

The handle is Platform's play-session credential for recovering one Character. It is issued only in a protocol 3 login response.

- **Format and storage:** `otr1.` followed by the unpadded base64url encoding of 32 CSPRNG bytes (48 ASCII characters). Only its SHA-256 hash is stored. It is never logged, and it is never sent to a game node. The client keeps it in process memory only and drops it on logout, Character change or exit.
- **Binding:** created inside the §6.2 fresh issuer transaction and committed with it. It binds:
  - `AccountId`, `CharacterId` and `WorldId`;
  - the selected `ChannelId`, as a routing hint only (§19.6);
  - the redeemed `native_security_generation`;
  - the new `native_play_generation` (below);
  - the originating `attempt_ref`;
  - the `device_family_id` and `device_key_thumbprint` of the device family that issued the redeemed ticket (§19.15). These are required wherever §19.15 binding is on;
  - `issued_at` and `expires_at = issued_at + H`. By owner decision U20, H is 12 h (43200 s). Configuration may shorten H to between 1 h and 12 h, but never lengthen it. Any other value refuses protocol 3 issuance and does not fall back.
- **Play generation fence:** Platform keeps one positive `native_play_generation` per `AccountId`.
  - Every committed fresh attempt advances it by one in the same transaction, under protocol 2 or 3. The new value is bound to the new handle.
  - Every older handle of that account then stops working.
  - At most one handle per account is live at any time.
  - This fence is issuance-only. Game's own fences stay authoritative: `connection_generation`, `ControlLossEpoch`, the RecoveryGrantNonce and the healthy-controller rule.
- **Retry of the fresh attempt:** a §6.2 replay re-signs the identical grant and replaces the handle. It mints a new handle with the same bindings and expiry, and revokes the handle that attempt issued before. No plaintext is ever stored. Replays serialize on the attempt row, and only the last handle returned is live.
- **Fresh authentication (owner decision U21):** a device-bound handle counts as Platform fresh authentication for the recovery profile §1 ("Platform freshly authenticated AccountId"). It does so for these reasons:
  - it is created only inside a fresh issuer transaction, after a Game Login Ticket was redeemed against the current `native_security_generation`;
  - §19.4 step 2 checks it again against the current generations and Identity state;
  - every use needs a fresh proof from the device key held by the family that issued the ticket (§19.15).

  Game records this reading in the profile clarification G2.
- **Without device binding:** where binding is required and the redeemed ticket names no device family, the login response carries `"recovery": null`, and no handle is created.
- **Authority:** the handle authorizes only §19.4 for its bound Character and World. It never yields a fresh grant, a ticket, a character list, an OAuth token, or access to another Character or World.
- **No rotation (Decision R2):** using the handle does not rotate it or extend its expiry.
- **The handle stops working on any of:**
  - terminal revocation or expiry of its device family (§19.15 step DB-6);
  - absolute expiry;
  - a newer play generation;
  - a change of `native_security_generation` (every `RevokeIdentityGameAuthorizations` path);
  - Identity disabled or terminated;
  - §19.7 revocation;
  - the Character no longer listed for the account (§19.4 step 4).

### 19.4 Recovery issuance

`POST /v1/recover`, body at most 2048 bytes, the §3.1 parsing rules, exact member set:

```json
{
  "protocol_version": 3,
  "recovery_handle": "otr1.<43 base64url characters>",
  "attempt_ref": "<canonical UUIDv7>",
  "character_id": "<canonical UUIDv7>",
  "offer": { "...": "exactly as §3.1" },
  "device_proof": { "kid": "<device_key_thumbprint>", "sig": "<base64url Ed25519 signature, 86 characters>" }
}
```

`device_proof` is required wherever §19.15 binding is on. Where it is off, which is allowed only in `testing`, the member is absent. The signature input is defined in §19.15.

The issuer runs one transaction. Lock order: recovery attempt row, epoch (§17), handle, Identity, device family (shared lock), play generation. Each lock wait is bounded as in §17.

1. **Retry.** If an attempt row exists for `attempt_ref`, apply §6.2 step 2, with the handle hash in place of the ticket hash.
   - An `attempt_ref` already used by a fresh attempt is `NATIVE_RECOVERY_ATTEMPT_CONFLICT`, and the converse is `NATIVE_LOGIN_ATTEMPT_CONFLICT`. Fresh and recovery attempts share one `attempt_ref` namespace.
2. **Handle.** Every one of these must hold, otherwise `NATIVE_RECOVERY_HANDLE_REJECTED`:
   - the hash is known, not revoked, and `server_now < expires_at`;
   - the Identity is enabled, not terminated, and its `account_id` equals the bound one;
   - `native_security_generation` equals the bound one (otherwise `NATIVE_RECOVERY_ACCOUNT_SECURITY_DENIED`);
   - the account's `native_play_generation` equals the bound one;
   - the request `character_id` equals the bound one;
   - where binding is on, the bound device family must also be live. Every one of these must hold, otherwise `NATIVE_RECOVERY_HANDLE_REJECTED`:
     - it is not revoked, and neither its absolute nor its idle deadline has passed;
     - its generations equal the Identity's current ones;
     - its `device_key_thumbprint` equals both the handle's and `device_proof.kid`.

     These checks only read the family. They never rotate its bearer and never touch its idle deadline.
   - where binding is on, `device_proof.sig` must verify under the family's public key, otherwise `NATIVE_RECOVERY_DEVICE_PROOF_REJECTED`.
3. **One unretired attempt per account.** If another recovery attempt of the same `AccountId` is not yet retired, the result is `NATIVE_RECOVERY_ATTEMPT_PENDING`, with `Retry-After` until that retirement. Retirement is `exp + 5 s + clock_uncertainty`. This is the recovery profile §11 rule: a new independent attempt needs deterministic retirement of any grant that may still be usable.
4. **Character.** Ownership is checked before World, following recovery profile §15.
   - With the §5 read model enabled: the feed must be `live` and the account's read model valid in the highest epoch, otherwise `NATIVE_RECOVERY_ROUTE_UNAVAILABLE`.
   - `character_id` must be listed for the account, with either `availability` value (an online Character is usually `UNAVAILABLE`), otherwise `NATIVE_RECOVERY_CHARACTER_CONFLICT`.
   - The listed `world_id` must equal the bound World, otherwise `NATIVE_RECOVERY_WORLD_STALE`.
   - Under mode 33a (§17), ownership is not verified and the bound World is used.
5. **Routing target.**
   - The target is the Character's current locator entry (§19.6) when one exists, otherwise the handle's bound channel.
   - The issuer reads the entry in the same transaction.
   - An entry with `present = false` refuses with `NATIVE_RECOVERY_CHARACTER_CONFLICT`, so the client makes a fresh login. This use only ever denies. A grant never depends on an entry being present.
   - An entry for another World refuses with `NATIVE_RECOVERY_WORLD_STALE`.
   - A `conflicted` entry refuses with `NATIVE_RECOVERY_ROUTE_UNAVAILABLE`.
   - The target must satisfy §7.4 conditions 1–5, otherwise `NATIVE_RECOVERY_ROUTE_UNAVAILABLE`.
   - The issuer never substitutes another channel (FND-04B §23).
6. **Claims.** Header exactly `{"alg":"Ed25519","kid":"<recovery kid>","typ":"oteryn-recovery+jwt"}`. Claims in this order, without whitespace:
   - `iss` = `urn:oteryn:platform:game-recovery`;
   - `aud` = `urn:oteryn:game:recovery`;
   - `iat`, `nbf = iat`, `exp = iat + TTL`, with the TTL rules of §8.2 (U4);
   - `jti`, a fresh 32-byte value;
   - `profile` = `oteryn-reauth-recovery-v1`;
   - `purpose` = `existing_actor_recovery`;
   - `attempt_ref`, `account_id` and `character_id`;
   - `world_id`, the bound World checked in step 4;
   - `account_security_generation`;
   - `protocol_major` = 1 and `transport_profile` = 1;
   - `ruleset_revision`, `content_revision`, `map_revision` and `world_policy_revision`, taken from the target channel's selected report.

   No `channel_id`, instance, node, `route_revision`, `runtime_observation_revision`, `scope_ownership_generation`, `offer_revision` or `GameSessionId` is emitted (recovery profile §7). Size bounds are those of §8.2.
7. **Commit.** Sign with the recovery key (§19.5), insert the attempt row (purpose `recovery`), commit, then respond. §6.2 steps 4–5 and §6.3 apply unchanged, with `RECOVERY_ATTEMPT_RECONCILIATION_REQUIRED` in place of `ADMISSION_ATTEMPT_RECONCILIATION_REQUIRED`.

Success response: `200`, `no-store, private`, exact member set:

```json
{
  "protocol_version": 3,
  "attempt_ref": "<request attempt_ref>",
  "world_id": "<canonical UUIDv7>",
  "channel_id": "<canonical UUIDv7>",
  "endpoint": { "...": "exactly as §3.2" },
  "grant": { "profile": "oteryn-reauth-recovery-v1", "token": "<JWS compact, at most 4096 ASCII bytes>", "valid_for_seconds": 20 }
}
```

- `channel_id` and `endpoint` are routing evidence only. They are not signed, because the profile forbids channel authority. Game resolves current placement before COMMIT (FND-04B §22).
- The handle is not returned or rotated.
- The client presents the grant on the endpoint as `ClientResume`. That resumes the same GameSession while grace lasts (FND-04B §20), or attaches the still-present actor to a new GameSession after grace (FND-04B §21).

### 19.5 Key purposes and custody

| | Fresh admission | Existing-actor recovery |
|---|---|---|
| issuer | `urn:oteryn:platform:game-admission` | `urn:oteryn:platform:game-recovery` |
| audience | `urn:oteryn:game:admission` | `urn:oteryn:game:recovery` |
| `typ` | `oteryn-admission+jwt` | `oteryn-recovery+jwt` |
| profile | `oteryn-pre-admission-v1` | `oteryn-reauth-recovery-v1` |
| `purpose` claim | `fresh_entry` | `existing_actor_recovery` |
| trust key purpose | configured `GAME_AUTH_NATIVE_EVIDENCE_FRESH_KEY_PURPOSE` (deployed value `fresh_admission`) | `existing_actor_recovery`, fixed by Game |
| Game trust read | `ReadFreshSigningTrustV1` | `ReadRecoverySigningTrustV2` |
| Game account-security read | `ReadAccountSecurityV1` | `ReadRecoveryAccountSecurityV2` |
| private key settings (proposed) | `GAME_AUTH_NATIVE_ADMISSION_SIGNING_KEY_FILE`, `..._KEY_ID` | `GAME_AUTH_NATIVE_RECOVERY_SIGNING_KEY_FILE`, `..._KEY_ID` |

- The two scopes use separate key pairs. A `kid` is never published in both scopes.
- Both private keys live inside the §9.1 issuer runtime, in distinct files.
- §9.2 (registry rules and signer self-check) and §9.3 (rotation) apply to each scope on its own.
- A missing or failing recovery scope stops only recovery issuance (`NATIVE_LOGIN_UNAVAILABLE` on `/v1/recover`). Fresh login continues.
- No reconnect key purpose exists. The fast reconnect proof (FND-04B §9) is game-domain material that Platform never issues, sees or stores.

### 19.6 Recovery locator (Game G1, owner decision U22: built now)

Without a locator, Platform's routing hint is the channel bound at the account's last committed fresh or recovery attempt. A Game channel transition, handoff or cross-channel failover can make that hint stale. A stale hint fails closed at Game, for example with `RECOVERY_TARGET_NOT_ELIGIBLE`, and grants nothing. However, it denies recovery to a player whose actor moved. The locator feed closes this gap. It is routing evidence and never authority (FND-04B §22).

#### 19.6.1 Producer and transport

- **Producer:** the Game node that commits a Character placement change, in the same durability transaction as that change. A Character placement change is any of:
  - a fresh admission COMMIT;
  - a `ClientResume` COMMIT within grace or after grace (FND-04B §20, §21);
  - the actor becoming `ABSENT`;
  - a future accepted channel-transition or handoff COMMIT.
- **Transport:** `ReportRecoveryLocatorV1` → `POST /internal/v1/game-auth/native-recovery-locators`. It uses HTTP/1.1 over TLS 1.3 with a client certificate, and follows the transport rules, bounds and capacity rules of the Game `OTERYN_GAME_NATIVE_RUNTIME_STATUS_PRODUCER_V1.md` §3.
- **Identity:**
  - Each node host has its own **locator** client certificate. It is never the runtime-status, native-evidence, projection or ownership-authority certificate.
  - Platform configures, per locator identity, the scopes that node host may serve (`GAME_AUTH_NATIVE_RECOVERY_LOCATOR_IDENTITIES`, same JSON shape as the runtime-status identities).
  - Locator identities are refused on every other route, and other identities are refused on this route (`401`).

#### 19.6.2 Wire

The request has an exact member set: no unknown, duplicate or `null` member. It is at most 1024 bytes.

```json
{
  "contract_version": 1,
  "operation": "ReportRecoveryLocatorV1",
  "source_authority": "<[readiness].source_authority>",
  "assignment_epoch": 1,
  "scope_ownership_generation": 1,
  "world_id": "<canonical UUIDv7>",
  "channel_id": "<canonical UUIDv7>",
  "account_id": "<canonical AccountId>",
  "character_id": "<canonical UUIDv7>",
  "present": true,
  "locator_revision": 1,
  "committed_at": "<RFC 3339 UTC, whole seconds>"
}
```

- `world_id` and `channel_id` are the scope that committed the change. For `present = false` they are the scope the actor left.
- `locator_revision` is a positive per-Character counter. Game advances it by one in the same transaction as each placement change.
- `assignment_epoch` and `scope_ownership_generation` are the assignment the committing node held. They are private evidence, as in §7.2: stored, never logged, never placed in a grant or a public response.

The response is `200` with exactly `{"contract_version":1,"result":"accepted"|"superseded"|"duplicate"}`, at most 256 bytes. Refusals are `400` (malformed), `401` (identity or scope), `409` (assignment mismatch or conflict) and `429`, each with the fixed §11.1-style body and no detail.

#### 19.6.3 Platform acceptance

Platform accepts a report only when every one of these holds:

1. the client identity is a locator identity whose scope list contains `(world_id, channel_id)`;
2. `(assignment_epoch, scope_ownership_generation)` equals the latest accepted assignment for that scope (§7.2), so a deposed owner cannot move a Character;
3. `character_id` is a canonical id, and `account_id` is a canonical AccountId;
4. `committed_at` is not later than Platform `now + clock_uncertainty`.

Each Character has one entry, ordered by `(assignment_epoch, locator_revision)` numerically:

- **Lower:** the report is `superseded`.
- **Equal with identical content:** the report is a `duplicate`.
- **Equal with different content:** the entry is marked `conflicted` until a higher revision arrives.
- **Higher:** the report replaces the entry.
- **Restore reset:** a higher `assignment_epoch` (§7.2) invalidates every entry of lower epochs.

An entry is evidence for §19.4 step 5 only. It never authorizes issuance, never names a channel inside a grant, and never bypasses the §7.4 readiness checks of the target. It has no freshness bound. It stays until a newer one supersedes it, because readiness is checked on the target at issuance.

#### 19.6.4 Delivery

- Game keeps one durable outbox row per Character (latest wins), written in the placement transaction.
- A node reporter sends rows whose revision is above the delivered revision. One request is in flight per node, using bounded backoff.
- `accepted`, `superseded` and `duplicate` mark the row delivered. `409` leaves it for the current owner's next change.
- Delivery never gates admission, resume, logout or shutdown.
- An undelivered row leaves Platform on its fallback hint, which fails closed at Game.

**Release gate:** no Game channel transition may be enabled together with protocol 3 recovery until G1 runs in the same environment and §19.13's locator tests pass.

### 19.7 Handle revocation

`POST /v1/recover/revoke`, exact body `{"protocol_version":3,"recovery_handle":"otr1.<...>"}`, answers `200` with exactly `{"protocol_version":3,"revoked":true}`.

- It is idempotent.
- An unknown or already-dead handle gets the same response, so the route is not an oracle.
- It revokes only that handle and never advances any generation.
- The client calls it on logout and on Character change.

### 19.8 Errors added by protocol 3

§11.1 and §11.2 apply. `SECURITY_TERMINAL` rows collapse to the public code `NATIVE_LOGIN_AUTHENTICATION_REQUIRED`.

| Internal code | HTTP | Category | Progression | Next authority | Public class | Recovery profile counterpart |
|---|---|---|---|---|---|---|
| `NATIVE_LOGIN_CHANNEL_UNAVAILABLE` | 409 | `DEPENDENCY_UNAVAILABLE` | `RETRYABLE` | same ticket, new `attempt_ref`, a listed channel or `null` | `TEMPORARILY_UNAVAILABLE` | issuance-only |
| `NATIVE_RECOVERY_REQUEST_MALFORMED` | 400 | `INVALID_INPUT` | `TERMINAL` | corrected new attempt | `RETRY_LOGIN` | class of `RECOVERY_GRANT_MALFORMED` |
| `NATIVE_RECOVERY_HANDLE_REJECTED` | 401 | `AUTHENTICATION_FAILED` | `SECURITY_TERMINAL` | full sign-in and fresh login | `AUTHENTICATION_REQUIRED` | class of `RECOVERY_GRANT_AUTHENTICATION_FAILED` |
| `NATIVE_RECOVERY_DEVICE_PROOF_REJECTED` | 401 | `AUTHENTICATION_FAILED` | `SECURITY_TERMINAL` | full sign-in and fresh login | `AUTHENTICATION_REQUIRED` | class of `RECOVERY_GRANT_AUTHENTICATION_FAILED` |
| `NATIVE_RECOVERY_ACCOUNT_SECURITY_DENIED` | 401 | `SESSION_REJECTED` | `SECURITY_TERMINAL` | full sign-in after account security permits | `AUTHENTICATION_REQUIRED` | `RECOVERY_GRANT_SECURITY_STATE_REVOKED` |
| `NATIVE_RECOVERY_ATTEMPT_CONFLICT` | 409 | `INVALID_INPUT` | `TERMINAL` | new `attempt_ref` | `RETRY_LOGIN` | issuance-only |
| `NATIVE_RECOVERY_ATTEMPT_PENDING` | 409 | `CONFLICT` | `RETRYABLE` | same request after `Retry-After` | `TEMPORARILY_UNAVAILABLE` | issuance-only (profile §11 retirement) |
| `NATIVE_RECOVERY_CHARACTER_CONFLICT` | 409 | `SESSION_REJECTED` | `TERMINAL` | fresh login | `SESSION_UNAVAILABLE` | class of `RECOVERY_TARGET_NOT_ELIGIBLE` |
| `NATIVE_RECOVERY_WORLD_STALE` | 409 | `STALE_GENERATION` | `TERMINAL` | fresh login for the current World | `RETRY_LOGIN` | `RECOVERY_GRANT_WORLD_STALE` |
| `NATIVE_RECOVERY_ROUTE_UNAVAILABLE` | 503 | `DEPENDENCY_UNAVAILABLE` | `RETRYABLE` | same request with backoff while the handle lives | `TEMPORARILY_UNAVAILABLE` | issuance-only |
| `NATIVE_RECOVERY_GRANT_EXPIRED` | 409 | `SESSION_REJECTED` | `TERMINAL` | same handle, new `attempt_ref` after retirement | `RETRY_LOGIN` | `RECOVERY_GRANT_EXPIRED` |
| `RECOVERY_ATTEMPT_RECONCILIATION_REQUIRED` | 503 | `DEPENDENCY_UNAVAILABLE` | `RETRYABLE` | same `attempt_ref` and same handle only | `TEMPORARILY_UNAVAILABLE` | identical code (profile §11) |

Protocol 3 also reuses these protocol 2 codes with their §11.2 rows: `NATIVE_LOGIN_UNSUPPORTED_VERSION`, `NATIVE_LOGIN_OFFER_UNSUPPORTED`, `NATIVE_LOGIN_RATE_LIMITED` and `NATIVE_LOGIN_UNAVAILABLE`.

### 19.9 Limits

Initial values, tuned as U10:

| Surface | Key | Limit |
|---|---|---|
| Gateway `POST /v1/recover` | source IP | 30 / minute |
| Gateway `POST /v1/recover` | (`attempt_ref`, handle hash) | 6 requests in total |
| Gateway `POST /v1/recover/revoke` | source IP | 30 / minute |
| issuer `native-recoveries` | `AccountId` | 20 committed attempts / hour, at most one unretired (§19.4 step 3) |
| issuer `native-recoveries` and `native-admissions` together | Gateway identity | 120 / minute (§10, shared) |

Body limits: recovery request 2048 bytes, including `device_proof`; recovery response 6144 bytes; protocol 3 login response 8192 bytes.

### 19.10 Security and logging

§13 applies. These are also never logged or traced:

- recovery handles and their hashes;
- recovery grants and signing inputs;
- `native_play_generation`;
- the locator's `assignment_epoch` and `scope_ownership_generation`.

Locator logs carry only the operation, the scope ids, the result class and timings. They never carry `account_id` or `character_id` beyond the §13 rules. `channels` entries and `channel_selection` may be logged.

### 19.11 Rollout and rollback

Steps run in this order. Each code step is `backward-compatible` and default off. Paths for steps DB-1 to DB-9 are in §19.15.

1. **Documentation.**
   - The owner decided U19–U22 on 2026-10-12, and corrected U19 and U21 the same day.
   - The Game architect accepts ADR 0045 and this amendment.
   - Game records G2.
   - Game accepts the G1 producer contract that §19.14 creates.
   - The remembered-device contract amendment DB-1 is written and accepted.
2. **Game G1** (§19.14). It runs in parallel with step 3. Its reports fail harmlessly until step 3's locator route exists.
3. **Platform recovery and channels.** This step adds:
   - play generation, handle storage, recovery attempt rows, the recovery signer and recovery-scope trust publication;
   - the two issuer operations;
   - the locator ingestion route, its identities and its entry store (§19.6.3);
   - the mode 34a widening.
4. **Platform device binding**, DB-2 to DB-7 (§19.15). It may land together with step 3 or right after it. It must be complete before step 8 enables recovery anywhere beyond `testing`.
5. **Gateway.** Protocol 3 parsing and the two routes.
6. **Game client, `client-first-safe`** (DB-8 and protocol 3). This covers:
   - protocol 3 support in `crates/platform-client`;
   - `label` shown for each channel;
   - device-key generation and session or remembered enrollment;
   - `device_proof` on `/v1/recover`.
7. **Step S: staging channels on the NAS preproduction store.** This is an operator-run deploy step.
   - **S1. Deploy support** (Platform paths):
     - `deploy/synology/scripts/lib.sh` publishes a list of channels instead of the current single-channel values;
     - `deploy/synology/native-defaults.env` holds the non-secret list: key, host, port and `tls_server_name` per channel, and each identity's scope list for runtime status, assignment and locator;
     - `deploy/synology/compose.yml` passes the list through;
     - `app/GameAuth/Worlds/PreproductionNativeRouteStore.php` carries the mode 34a widening from step 3.
     - The current single-channel values stay valid for `oteryn-staging-1`.
   - **S2. Topology.** `game-auth:native-topology:issue` in preproduction-store mode issues `antica`, `obscura` and `pythera` on the staging World. It is idempotent, and its receipts are audited under ADR 0044.
   - **S3. Routes.** `game-auth:native-route:publish` runs once per channel. Each channel gets a distinct `(host, port)` on the NAS (Decision M1), its own `tls_server_name`, and login turned on.
     - `OTERYN_STAGING_NATIVE_ROUTE_PUBLISH_ENABLED` stays `true`.
     - Each published `route_revision` is recorded as that channel's node-declared value.
   - **S4. Game side** (listed, not written here):
     - one game node per channel on the NAS;
     - an `oteryn-game-ops` assignment per scope;
     - each node declares its channel's published `route_revision`;
     - runtime-status and locator reporting is turned on.
   - **S5. Flags,** all repository variables or environment, no code:
     - the protocol 3 switch on (preproduction);
     - `game-auth.device_sessions.enabled` on;
     - `game-auth.native_recovery.require_device_binding` on;
     - the locator identities set (`GAME_AUTH_NATIVE_RECOVERY_LOCATOR_IDENTITIES`).
   - **Operator-owned material (never in the repository):**
     - listener TLS certificates and keys for the three `tls_server_name` values;
     - the mTLS client certificate and key per node host, for runtime status and for locator;
     - the ownership-authority certificate;
     - the recovery signing private key and its `kid`;
     - the Gateway service credential.

     The repository holds only certificate subjects, scope lists, ids, hosts and ports.
8. **Joint E2E** on the staging World with `antica`, `obscura` and `pythera`:
   - **Recovery:** lose the connection, call `/v1/recover` with a valid `device_proof`, then `ClientResume` on the channel owner, both within grace and after grace.
   - **Device refusals:** a missing or wrong proof is refused, and a revoked family's handle is refused.
   - **Locator:** a placement change routes recovery to the reported channel, and `present = false` refuses.
   - **Fresh login:** automatic selection, a requested channel, and a refusal that lists an alternative.
9. **Enablement.**
   - **`testing`:** recovery may run without device binding (`require_device_binding` off).
   - **`preproduction`, including staging, and anything beyond:** recovery needs DB-1 to DB-9 complete and `require_device_binding` on. The issuer refuses to start recovery in those environments with binding off.
   - **Production:** needs separate authority, U3 and U8.

Rollback:

- **Protocol 3:** switch it off. Protocol 2 is unaffected. Live handles stop working because `/v1/recover` refuses. Outstanding recovery grants expire within 30 s.
- **G1:** remove the node's `[platform.recovery_locator]` section. Reports stop, the outbox keeps its rows, and Platform falls back to the bound-channel hint.
- **Step S:**
  - turn login off on the three new channels' routes;
  - `oteryn-staging-1` keeps serving;
  - the issued Channel rows stay, because topology issue is not reversed.

### 19.12 UNKNOWN register additions

| ID | Unknown | Blocks | Owner |
|---|---|---|---|
| U19 | **Resolved 2026-10-12 (owner).** The keys are `antica`, `obscura` and `pythera` under the existing Registry grammar, and the label is the capitalised key (§19.2.2). The three channels are added to the staging World (§19.11 step S), and `oteryn-staging-1` keeps its key and route | — | — |
| U20 | **Resolved 2026-10-12 (owner).** H = 12 h, and configuration may only shorten it (§19.3) | — | — |
| U21 | **Resolved 2026-10-12 (owner).** A device-bound handle counts as fresh authentication (§19.3). Device binding is required scope (§19.15) and blocks recovery beyond `testing`. Game records G2 | — | Game records G2 |
| U22 | **Resolved 2026-10-12 (owner).** G1 is built now (§19.6, §19.14) | — | — |
| U23 | Whether a remembered (persistent) device family may also start recovery after a client restart, once the in-memory handle is gone. This would be a new callback purpose under §19.15 | later | Platform + owner |
| U26 | Which OS key stores the client supports for the non-exportable device key, and the fallback when none is available (§19.15 DB-8) | DB-8 | Game client + owner |
| U24 | Per-account recovery attempt limit | §19.9 | Platform, measured |
| U25 | Whether channels ever share an endpoint (Decision M1 forbids it in protocol 3) | later | Game architect |

### 19.13 Required contract tests before activation

- **Protocol 3 shapes:**
  - exact login request, response and channel-refusal schemas;
  - protocol 2 byte-identical with the switch on and off;
  - recover and revoke schemas;
  - `otr1.` handle grammar.
- **Multiple channels:**
  - automatic selection is the lowest candidate;
  - a requested candidate is selected;
  - a requested non-candidate gets the refusal with a listed alternative, and the ticket is still usable with a new `attempt_ref`;
  - no candidate gives the route-unavailable result;
  - a duplicate `(host, port)` routes nowhere;
  - the widened mode 34a publishes per Channel and still refuses a second World.
- **Handle:**
  - hash-only storage;
  - stops working on a newer play generation, from a protocol 2 or 3 commit;
  - stops working on a security-generation change, Identity disable, revoke or absolute expiry;
  - a fresh-attempt replay replaces the handle and leaves one live;
  - a wrong `character_id` is rejected;
  - the handle never authenticates on any other route.
- **Recovery issuance:**
  - the grant is byte-identical on a same-`attempt_ref` retry;
  - a pending refusal comes before retirement, and a new attempt is accepted after it;
  - `attempt_ref` is shared across purposes;
  - ownership before World;
  - a stale feed gives route-unavailable;
  - no substitute channel;
  - claims verified by the pinned Game recovery verifier at an exact Game commit, including each revision dimension mutated on its own and the absence of every forbidden claim.
- **Labels:** each `channels` entry has `label` equal to the capitalised key; `oteryn-staging-1` is unchanged.
- **Device binding (§19.15):**
  - enrollment needs proof of possession of the key;
  - a ticket records its family, both through PKCE enrollment and through `NativeTicketIssue`;
  - the handle binds the family and the thumbprint;
  - a missing, wrong or replayed-input `device_proof` is refused;
  - each of these refuses the handle: a revoked family (logout, owner revoke, replay-terminal), passed family deadlines, and changed generations;
  - recovery never rotates the bearer or touches the idle deadline;
  - binding required with no family gives `"recovery": null`;
  - issuance refuses to run with binding off outside `testing`;
  - session families expire with H.
- **Staging step S:** the publish loop issues and publishes each listed channel and refuses a duplicate `(host, port)`; the single-channel values still publish `oteryn-staging-1`.
- **Locator ingestion:**
  - exact wire and response;
  - a locator identity outside its scope list, or used on another route, is refused;
  - another purpose's identity is refused on the locator route;
  - an assignment mismatch (deposed owner) is refused;
  - ordering: lower is superseded, equal and identical is a duplicate, equal and different is conflicted, and a higher epoch invalidates lower entries;
  - a future `committed_at` is refused;
  - `present = false` refuses recovery, a conflicted entry gives route-unavailable, and an entry on another World gives world-stale;
  - an entry never yields a grant on a target that fails §7.4;
  - logs carry no generation or epoch.
- **Keys:**
  - a recovery `kid` is refused in the fresh scope and the converse;
  - per-scope self-check;
  - a missing recovery key stops only recovery.
- **Logs:** logs and responses are free of every §19.10 secret.

### 19.14 Game implementation packet G1: recovery locator producer (ready to dispatch)

This is a dispatch-ready packet for `Oteryn/Oteryn-Game`. Platform does not write it. The Game lead copies it into a Game task record built from the Game `docs/agents/tasks/TASK_TEMPLATE.md`. Paths were checked against Game `origin/main` at `08e13f7de`; the writer re-checks them at dispatch.

```yaml
task_id: OTV2-20261012-recovery-locator-producer
title: G1 recovery locator producer - the committing node reports each Character placement change to Platform
mode: IMPLEMENT
repository: Oteryn/Oteryn-Game
base_branch: main
execution_policy: continuous_progress
contract: docs/contracts/OTERYN_GAME_RECOVERY_LOCATOR_PRODUCER_V1.md (new, written by this task from Platform section 19.6)
platform_contract: Oteryn/Oteryn-Platform OTERYN_V2_NATIVE_GATEWAY_LOGIN_CONTRACT.md section 19.6 (amendment N4P-4), ADR 0045
owned_paths:
  - docs/contracts/OTERYN_GAME_RECOVERY_LOCATOR_PRODUCER_V1.md
  - docs/contracts/RESOURCE_LIMITS_REGISTRY.json (NRL-* rows only)
  - docs/contracts/CROSS_REPOSITORY_CONTRACT_LOCK.json (recovery locator row only)
  - apps/game-server/migrations/<next free number>_recovery_locator_outbox.sql (0080 at 08e13f7de)
  - apps/game-server/src/durability/recovery_locator_outbox.rs
  - apps/game-server/src/durability/mod.rs (module registration only)
  - apps/game-server/src/durability/fresh_admission.rs (outbox write in the fresh admission COMMIT only)
  - apps/game-server/src/durability/admission_journal.rs (outbox write in the resume, post-grace and ABSENT commits only)
  - apps/game-server/src/native_admission_source/recovery_locator.rs
  - apps/game-server/src/native_admission_source/mod.rs (Operation variant and body cap only)
  - apps/game-server/src/node/** (reporter wiring and the [platform.recovery_locator] section only)
  - apps/game-server/tests/*recovery_locator*
  - docs/agents/tasks/active/OTV2-20261012-recovery-locator-producer.md
  - docs/agents/tasks/archive/OTV2-20261012-recovery-locator-producer.md
public_contracts:
  - ReportRecoveryLocatorV1 (Platform section 19.6.2)
depends_on:
  - OTERYN_GAME_NATIVE_RUNTIME_STATUS_PRODUCER_V1.md section 3 transport and section 6 assignment_epoch
blocks:
  - Platform section 19.11 step 8 joint E2E
  - enabling any Game channel transition together with protocol 3 recovery
```

**Outcome.** These are the deliverables:

1. A new Game contract restating §19.6.1–§19.6.4 from the producer side.
2. A migration that adds `game_durability_recovery_locator_outbox`. It is additive and reversible, and its rollback refuses to drop the table while it holds undelivered rows. It has:
   - one row per `character_id`;
   - the columns `account_id`, `world_id`, `channel_id`, `present`, `locator_revision`, `assignment_epoch`, `scope_ownership_generation`, `committed_at` and `delivered_revision`.
3. An outbox writer, called inside the existing placement transactions:
   - fresh admission COMMIT;
   - `ClientResume` within grace and after grace;
   - actor becoming `ABSENT`.

   It runs under the Character guard those transactions already hold. It advances `locator_revision` by one and records the committing assignment.
4. `recovery_locator.rs`, with:
   - the exact wire and response decoder;
   - a purpose-bound descriptor that refuses a certificate or key another purpose uses;
   - a `deliver` path with its own capacity, never an admission slot.
5. A node reporter. It sends undelivered rows (one request in flight, bounded backoff) and marks a row delivered on `accepted`, `superseded` or `duplicate`. It leaves a row unchanged on `409`. It is off when `[platform.recovery_locator]` is absent.
6. Registry rows `NRL-REPORT-BYTES` (1024), `NRL-RESPONSE-BYTES` (256) and `NRL-INFLIGHT` (1).

**Contract fields.** Exactly the §19.6.2 members. Field sources:

| Field | Source |
|---|---|
| `source_authority` | `[readiness].source_authority` |
| `assignment_epoch` and `scope_ownership_generation` | the assignment held by the committing transaction |
| `world_id` and `channel_id` | the committing scope (for `ABSENT`, the scope left) |
| `account_id` and `character_id` | the admitted actor |
| `present` | `false` only for the `ABSENT` transition |
| `locator_revision` | the outbox counter |
| `committed_at` | the transaction's commit time, never future-dated |

**Authority and fencing.** These rules bind the writer:

- **Evidence only.** The locator never authorizes Game: no admission, resume, handoff or channel selection reads it. FND-04B §22 and §23 stay the authority.
- **Durable atomicity.** The outbox row is written in the same transaction as the placement change, under its existing fences:
  - Character guard;
  - `RuntimeScopeAuthority` generation;
  - `connection_generation` and `ControlLossEpoch`.

  A rolled-back placement leaves no row, and a committed placement always leaves one.
- **No authority from the row.** The reporter reads only committed rows. Neither the reporter nor the row can raise a generation.
- **Deposed owners.** Platform refuses a report whose assignment is not the latest (§19.6.3 rule 2). A deposed owner therefore cannot move a Character's route, even with a valid certificate.
- **Restore.** After a durability restore that could lower revisions, the operator raises `assignment_epoch` as in the runtime-status contract §6. Platform then drops entries of lower epochs.
- **Logging.** The generation and epoch are never logged (§19.10). Logs carry only the operation, scope ids, result class and timings.
- **High-risk qualification.** The packet's high-risk qualification section is `applicable: true`, because the change adds a fenced durable write. Its mutation operators cover, one invariant per negative case:
  - missing assignment;
  - stale generation;
  - wrong scope identity;
  - future `committed_at`;
  - provenance substitution (only committed placements produce rows);
  - replay and concurrency (latest wins per Character, idempotent at Platform);
  - restart, retry and PostgreSQL reload (rows survive and are re-sent).

**Validation** (Game *Local checks by changed path*):

- `cargo fmt --all --check`
- `cargo clippy --locked -p oteryn-game-server --all-targets --quiet -- -D warnings`
- `cargo test --locked -p oteryn-game-server --test native_recovery_locator --quiet`. It covers the wire, the descriptor key separation, response classes, reporter backoff and latest-wins behavior, and off-by-default behavior against a Platform stub.
- `OTERYN_TEST_POSTGRES_ADMIN_URL=... cargo test --locked -p oteryn-game-server --test recovery_locator_postgres --quiet`, on PostgreSQL 17.6. It covers:
  - outbox atomicity with each placement commit;
  - rollback leaves no row;
  - revision monotonicity;
  - restart re-send;
  - migration rollback refused while rows are undelivered.
- `python tools/agents/validate_governance.py` and `python -m unittest discover -s tools/agents/tests` for the task record.
- `git diff --check` for the contract.
- Runtime E2E is Platform §19.11 step 8, performed jointly after the Platform locator route exists.

**Excluded scope:** this packet changes no Platform code, admission or resume decision, channel transition, client code or gameplay. It does not change the FND-04 or FND-04B profiles. G2 is a separate documentation change in the recovery profile.

### 19.15 Device binding (required scope; blocker for recovery beyond `testing`)

Owner correction, 2026-10-12: the recovery handle must be bound to the device, and this is required scope. This section designs the full path.

The path builds on the existing remembered-device family:

- `App\GameAuth\DeviceSessions\NativeRememberedDeviceSessions`;
- the tables `native_device_session_families` and `native_device_session_credentials`;
- `NATIVE_REMEMBERED_DEVICE_SESSION_CONTRACT_CANDIDATE.md`.

#### 19.15.1 What must change in the current remembered-device design

| Current design | Change |
|---|---|
| Disabled by default and refused outside `testing`/`preproduction` | Unchanged as a switch. Wherever protocol 3 recovery runs with binding on, `game-auth.device_sessions.enabled` must be on. The issuer refuses to start otherwise. Production stays separately authorized. |
| Opt-in only: an unchecked "Remember this device" choice, and enrollment requires `explicitConsent: true` | Two persistence classes. A **session** family is enrolled on every protocol 3 native login: its bearer is never written to disk, and its absolute and idle deadlines both equal H. The **remembered** family stays as it is, opt-in and default off. Explicit consent stays required for remembered only, because a session family keeps nothing beyond the process. |
| The credential is a rotating `otd1.` bearer, and a lost response costs a browser sign-in | The bearer keeps rotating for its own purposes (`NativeCharactersRead`, `NativeTicketIssue`). Recovery never presents it. Recovery proves possession of a new **device key** instead. The key does not rotate, so a retry with the same `attempt_ref` is safe. |
| The family has no key, so the bearer is the whole credential | The family records one Ed25519 public key and its thumbprint at enrollment, fixed for the family's life. Enrollment requires a signature by that key (proof of possession). |
| A ticket does not record a family | A ticket gains a nullable `device_family_id`. It is set by PKCE enrollment (the family the same OAuth access token enrolled, found through the unique `enrollment_access_token_id`) or by the `NativeTicketIssue` callback (the rotating family). |
| Revocation ends only the family | Terminal family revocation also revokes that family's recovery handles in the same transaction: logout, owner revoke, replay detection and expiry. |

#### 19.15.2 Wire and cryptography

- **Device key:** Ed25519, generated by the client.
  - It is non-exportable in the OS key store where available (U26).
  - A session family may hold it in process memory only.
  - `device_key_thumbprint` is the unpadded base64url SHA-256 of the 32-byte public key (43 characters).
- **Enrollment proof:** the signature covers the JCS of:

  ```json
  {"purpose":"oteryn-native-device-enroll-v1","public_key":"<base64url>","persistence":"session"|"remembered","client_id":"<oauth client id>","access_token_sha256":"<lowercase hex of SHA-256 of the presented bearer>"}
  ```

  - The request carries the signature alongside the public key.
  - Platform verifies it inside the enrollment transaction, before the family commits.
  - The signature binds the key to the one OAuth access token that enrolls the family.
- **Recovery proof:** the signature covers the JCS of:

  ```json
  {"purpose":"oteryn-native-recovery-pop-v1","attempt_ref":"<attempt_ref>","character_id":"<character_id>","recovery_handle_sha256":"<lowercase hex>","device_key_thumbprint":"<thumbprint>"}
  ```

  - `attempt_ref` is single-use per §19.4 step 1, so a captured proof only reproduces the byte-identical grant of that same attempt.
  - A stolen handle without the private key cannot start a new attempt.
- **Never logged or traced:** device private keys and signatures. Thumbprints may appear in audit records only.

#### 19.15.3 Ordered implementation steps

| Step | Work | Owned paths |
|---|---|---|
| DB-1 | Contract amendment "Device key and session class" (§19.15.1–§19.15.2) in the remembered-device candidate, plus acceptance. This is its own contract section, and it is required before any code. ADR 0045 Decision 2a records the decision, so no new ADR is needed: the change stays inside the ADR 0045 scope and the candidate's own contract. | `docs/contracts/NATIVE_REMEMBERED_DEVICE_SESSION_CONTRACT_CANDIDATE.md` |
| DB-2 | Migration, additive and reversible. Its rollback refuses to drop populated columns. It adds, on families, `persistence_class`, `device_public_key` (32 bytes) and `device_key_thumbprint` (unique). It adds `device_family_id` (nullable, indexed) to `game_login_tickets`. The recovery handle table of step 3 is created with `device_family_id` and `device_key_thumbprint` from the start. | `database/migrations/<new>_add_native_device_binding.php` |
| DB-3 | Enrollment with key proof and the session class. It covers per-class TTL bounds (session = H, remembered within 30 d absolute and 7 d idle) and the exact versioned request shape. | `app/GameAuth/DeviceSessions/NativeRememberedDeviceSessions.php`, `app/GameAuth/DeviceSessions/DeviceSessionFamily.php`, `app/GameAuth/DeviceSessions/DeviceKeyProof.php` (new), `app/Http/Controllers/GameAuth/DeviceSessions/NativeDeviceSessionController.php`, `app/Http/Middleware/GameAuth/GuardNativeDeviceSessionHttp.php`, `config/game-auth.php` |
| DB-4 | Tickets record their family: on the PKCE path from the same access token's enrollment, and in the `NativeTicketIssue` callback. | `app/GameAuth/NativeLogin/NativeGameLoginTickets.php`, `app/GameAuth/Tickets/GameLoginTicket.php`, `app/Http/Controllers/GameAuth/DeviceSessions/NativeDeviceSessionController.php` (ticket callback only), and the OAuth ticket-issuance call site, which the writer names at dispatch |
| DB-5 | The handle binds the family and thumbprint at fresh redemption. §19.4 step 2 adds the family checks and proof verification, plus the `require_device_binding` switch and its refusal outside `testing`. | `app/GameAuth/NativeLogin/NativeAdmissionAttempts.php`, `app/GameAuth/NativeRecovery/**` (new in step 3), `config/game-auth.php` |
| DB-6 | Revocation linkage: every terminal family path revokes the bound handles in the same transaction. | `app/GameAuth/DeviceSessions/NativeRememberedDeviceSessions.php`, `app/GameAuth/NativeRecovery/RecoveryHandles.php` (new) |
| DB-7 | Tests (§19.13 *Device binding*). | `tests/Feature/GameAuth/DeviceSessions/**`, `tests/Feature/GameAuth/NativeRecovery/**` |
| DB-8 | Game client (listed, not written here): key generation per U26, session or remembered enrollment, and `device_proof` on `/v1/recover`. | Oteryn-Game `crates/platform-client/**` |
| DB-9 | Independent security review of DB-1 to DB-8, as the remembered-device candidate requires before any enablement. | review record only |

#### 19.15.4 Blocker

Recovery may run without device binding only in `testing`. Enabling recovery anywhere beyond `testing` is blocked until all of these hold:

- DB-1 to DB-9 are complete;
- `game-auth.native_recovery.require_device_binding` is on;
- the §19.13 device-binding tests pass.

This includes `preproduction` and therefore staging (§19.11 step 9).
