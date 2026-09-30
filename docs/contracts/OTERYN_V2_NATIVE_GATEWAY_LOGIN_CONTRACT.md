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
- what stays Canary-only.

Out of scope: game admission itself (Game FND-04A), reconnect/recovery grants (`oteryn-reauth-recovery-v1`), channel switch, public status pages, capacity-aware balancing, production PKI, deployment.

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

No step falls back to password login, OAuth-token authentication at the game server, the Canary Game Session issuer or a Canary route.

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

- `protocol_version: 2` selects the native branch. Version 1 keeps its current Canary-compatible meaning (§12). Any other value: `NATIVE_LOGIN_UNSUPPORTED_VERSION`.
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

- audience `oteryn-native-game-gateway` (distinct from the Canary audience `oteryn-game-gateway`);
- binding to the persisted Identity, its canonical `identities.account_id` (lowercase UUIDv7, non-null) and its `native_security_generation` at issuance;
- the existing properties: at least 256 bits of CSPRNG entropy, hash-at-rest, single use, TTL at most 60 seconds (default 60), never logged.

Issuance is the existing `POST /v1/game-auth/tickets` under the existing OAuth checks (valid bearer, allowed first-party client, scope `game:ticket`, current `game_auth_generation`, Identity enabled and not terminated), with these native differences:

- the ticket kind is selected by the authenticated OAuth client, never by a request parameter: the native Rust client receives native tickets only (U1: whether this is a separate Passport client or the existing native public client);
- native issuance requires a canonical non-null `account_id` and does **not** require an `IdentityCanaryAccount` binding;
- `canary_account_id` is not stored on a native ticket.

A Canary ticket never redeems on the native branch and a native ticket never redeems on the Canary branch (audience mismatch → `NATIVE_LOGIN_TICKET_REJECTED`).

### 4.2 Native redemption

Inside the issuer transaction (§6), under row locks, equivalent to:

1. derive the ticket hash and lock the ticket row;
2. require audience exactly `oteryn-native-game-gateway`, `used_at` null, `expires_at > server_now`;
3. lock the Identity; require it exists, is enabled and not terminated;
4. require `identities.account_id` equals the ticket-bound `account_id` and is canonical lowercase UUIDv7;
5. require the current `native_security_generation` equals the ticket-bound generation (a revocation in between advances it and fails redemption);
6. mark the ticket used and record `attempt_ref` on it;
7. yield `AccountId` and `account_security_generation` (decimal of `native_security_generation`).

`AccountId` is Platform-owned (Platform ADR 0028) and is never derived from Canary ids or integer Identity ids. Any failure in steps 1–5 is `NATIVE_LOGIN_TICKET_REJECTED` or `NATIVE_LOGIN_ACCOUNT_SECURITY_DENIED` and rolls the transaction back.

## 5. Character selection (Q17a)

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

Empty list when the account has no snapshot. `503` when the account's read model is `invalid`. Never another account's data; never public. Rate limit §10.

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
2. Platform native login policy for (W, C) allows login (not maintenance, not disabled). Its representation is `UNKNOWN` (U7); Canary `game_worlds.status`/`login_enabled` alone is not native readiness;
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

The response `endpoint` is the same Registry route record the `route_revision` digest covers. Every string value must already satisfy the FND-04 §5 grammar; a report value that does not is rejected at ingestion, and the issuer re-checks before signing (fail closed with `NATIVE_LOGIN_UNAVAILABLE`). No `compatibility_revision`, no `schema_revision`, no Canary id, no claim not listed. Token at most 4096 bytes, header at most 512 and payload at most 3072 bytes decoded.

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

## 12. What stays Canary-only

Unchanged and never used by the native branch:

- `POST /v1/login` with `protocol_version: 1`, the Canary `gameplay_offer`, `canary_account_id`, integer world/channel ids;
- `/internal/v1/game-auth/tickets/redeem` and its Canary binding requirement, audience `oteryn-game-gateway`;
- `/internal/v1/game-auth/accounts/{canaryAccountId}/login-context` and `GameLoginContextProvider`;
- the Game Session v1 issuer and the historical Game Session v2 producer (`GAME_SESSION_CANARY_CONTRACT.md`, `OTERYN_NATIVE_GAMEPLAY_PROTOCOL_CONTRACT.md`);
- `game_worlds.status`/`login_enabled` as compatibility state;
- the OTClient OAuth client and OTClient tickets.

A native failure never falls back to any of these, and a Canary request never reaches the native issuer.

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

Rollback: switch the native branch off; outstanding grants expire within 30 s; Canary paths are untouched. A new claim, header member, profile or algorithm needs a new Game profile revision first; Platform never emits a claim the pinned Game verifier does not accept.

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
- native vs Canary ticket audience isolation in both directions;
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

Selection follows §7.4 over these records, using the runtime-status read model under the shared epoch lock. The issuer takes that lock before redeeming the ticket and before any non-locking read, so its snapshot sees every committed epoch raise (lock order: attempt row, epoch, ticket, Identity; ingestion: epoch, assignment, report). A report that is not `fresh` (stale, invalid, superseded or not ownership-bound), not ready, or whose `route_revision` differs from the record routes nowhere (`NATIVE_LOGIN_ROUTE_UNAVAILABLE`). The response `endpoint` is the record whose `route_revision` is bound in the grant. If that record changed after issuance, the retry answers `NATIVE_LOGIN_ROUTE_UNAVAILABLE` until `exp`, then `NATIVE_LOGIN_GRANT_EXPIRED`.

Issuer limits in this amendment: every InnoDB lock wait inside the issuer transaction is bounded by `GAME_AUTH_NATIVE_ADMISSION_LOCK_WAIT_TIMEOUT_SECONDS` (default 3 s, 1..10), and a timeout rolls back as `NATIVE_LOGIN_UNAVAILABLE`. The per-Gateway-credential limit is 120/minute (§10). The `native-scope-assignments` ingestion default is now 120/minute, matching §10. The Go Gateway forwarding of the native branch is a separate later change.

## 18. Non-authorization

This candidate authorizes no code, migration, route, configuration, secret, key, certificate, deployment, production change or Game repository change. Each #1419 delivery item needs its own task packet and PR after acceptance.
