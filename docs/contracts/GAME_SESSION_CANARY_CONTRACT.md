# Game Session to legacy engine Contract

> **LEGACY / COMPATIBILITY** — legacy engine compatibility path. The native target is split by owner: Platform owns `AccountId`, Game Login Ticket issuance/redemption, routing policy and Gateway pre-admission (`docs/architecture/OTERYN_V2_INTEGRATION_ARCHITECTURE.md`, `docs/contracts/OTERYN_V2_ACCOUNT_IDENTITY_CONTRACT.md`, `docs/contracts/OTERYN_V2_NATIVE_GATEWAY_LOGIN_CONTRACT.md`); the Game contracts (LCFA, native login, character authority boundary) own final admission and session authority.

## Status

`CANDIDATE B SELECTED — IMPLEMENTED, BOUNDED E2E PROVEN, PRODUCTION ACTIVATION GATED`

This document is the authoritative current contract between Oteryn Identity/Game Gateway and legacy engine world-entry authentication.

It remains authoritative for the legacy Canary-compatible Game Session contract version 1. The disabled-by-default producer for selected Game Session contract version 2 is governed by `OTERYN_NATIVE_GAMEPLAY_PROTOCOL_CONTRACT.md` and `docs/operations/OTERYN_NATIVE_PROTOCOL_PRODUCER.md`; no Otheryn native v2 consumer is claimed by this Platform package.

It supersedes the earlier pre-selection Candidate A/Candidate B analysis in this file. That exploratory history remains available in Git history; it is no longer the active implementation contract.

## Selected architecture

The selected compatibility path is a direct legacy engine Game Session issuer built on legacy engine's process-local `LoginSessionManager`.

The supported flow is:

```text
OTClient
  -> Oteryn Identity / OAuth + PKCE
  -> one-time Game Login Ticket
  -> Oteryn Game Gateway
  -> authoritative ticket redeem
  -> authoritative legacy engine account + world context
  -> POST exact legacy engine process /internal/v1/game-sessions
  -> opaque short-lived Game Session credential
  -> OTClient
  -> existing GameSessionKey world-login field
  -> legacy engine ProtocolGame / IOLoginData admission checks
```

The supported native-auth path does not send or validate the user's Oteryn password at the Gateway -> legacy engine boundary.

## Delivered components

### Oteryn Platform / Game Gateway

Gateway protocol v1 is delivered by Oteryn Platform PR #122, merged as:

```text
8006534108d835474dadd208b0ec934e4a12528b
```

Production hardening is delivered by Oteryn Platform PR #124. The exact repository-hardening head validated before final documentation is:

```text
2e664c440379af45b6413a26c9c0ee968275d049
```

That hardening adds:

- private ticket-redeem source throttling before Gateway service-credential authentication;
- one required current and one optional previous Gateway service-credential SHA-256 hash for bounded overlap rotation;
- `no-store` / `no-cache` response policy for sensitive ticket issue/redeem and Gateway native-login responses, including bounded error paths;
- fail-closed Gateway configuration that requires HTTPS for non-loopback Platform and Game Session service dependencies;
- explicit credential-rotation sequencing without committing plaintext secrets.

### legacy engine

The disabled-by-default legacy engine Game Session issuer is delivered by legacy engine PR #722, merged as:

```text
b8a88f073b2609b444fa15370aae30ac9f80b908
```

Gateway -> legacy engine bounded overlap credential rotation is implemented by legacy engine PR #807. The exact current validated implementation/checkpoint head before final documentation is:

```text
c8503b7d35fe15015e89b9d0067a8614e7a9d7a9
```

### OTClient

The maintained OTClient native-auth consumer is delivered by PR #17, merged as:

```text
bb87346f6c516a19d19497d82bb01fb389334ff5
```

## Gateway -> legacy engine protocol v1

### Endpoint

```text
POST /internal/v1/game-sessions
Authorization: Bearer <Gateway -> legacy engine service credential>
Content-Type: application/json
```

### Request

```json
{
  "protocol_version": 1,
  "canary_account_id": 101,
  "world_id": 1,
  "login_attempt_id": "00112233445566778899aabbccddeeff"
}
```

Requirements:

- `protocol_version` is exactly `1`;
- `canary_account_id` is the numeric Canary account obtained from authoritative Platform ticket redeem/context;
- `world_id` is the Platform `game_worlds.id` configured for the exact target legacy engine issuer process;
- `login_attempt_id` is a server-generated 32-hex-character idempotency/replay-guard identifier, not a bearer credential.

Platform `game_worlds.id` is not legacy engine `ChannelContext::channel_id`.

### Response

```json
{
  "protocol_version": 1,
  "session": {
    "credential": "<opaque-single-use-secret>",
    "expires_at": "2026-07-23T12:00:00Z"
  }
}
```

The raw Game Session credential is returned only to the client path that needs it and must never be logged.

## legacy engine authorization semantics

The legacy engine issuer:

- is disabled by default;
- is bound to one exact process/listener configuration;
- loads the authoritative legacy engine account by numeric account ID without user-password authentication;
- loads the account's allowed character-name set;
- binds issued sessions to `ProtocolProfileId::Current`;
- uses process-local `LoginSessionManager` storage;
- stores only the SHA-256 representation used by the existing session manager;
- uses the existing 60-second `LoginSessionManager` TTL;
- consumes a matching Game Session atomically once;
- burns the credential on wrong character/native protocol version consumption attempts according to existing manager semantics;
- rejects duplicate successful issuance for the same `login_attempt_id` within the bounded TTL;
- releases the `login_attempt_id` reservation when issuance itself fails;
- invalidates all unconsumed process-local credentials on process restart.

After Game Session authentication, existing `ProtocolGame` and `IOLoginData` ownership, deletion, ban and runtime admission checks remain authoritative.

## Capability boundary

Protocol v1 is intentionally limited to:

- one configured Platform world mapped to one exact legacy engine process;
- legacy engine `ProtocolProfileId::Current`;
- one process-local Game Session store per issuer process.

Not claimed by protocol v1:

- multi-world issuer selection;
- same-world horizontal issuer replicas without exact sticky/process routing;
- shared Game Session storage across legacy engine processes;
- immediate `security_generation`-based revocation of already-issued unconsumed Game Sessions;
- active-player disconnection on Identity security events.

Those require separate explicit contracts before they are claimed.

## Replay and retry semantics

A Game Login Ticket and a legacy engine Game Session are separate credentials.

```text
Game Login Ticket
  owner: Oteryn Identity
  TTL: ~60 seconds by current policy
  consume: atomic single-use

Game Session
  owner: legacy engine issuer process
  TTL: 60 seconds
  consume: atomic single-use
```

One successful Game Session issuance is permitted per `login_attempt_id` per issuer process/TTL.

If Gateway successfully creates a Game Session but loses the HTTP response, retrying the same `login_attempt_id` does not mint a second credential. The orphan credential expires and the client starts a fresh native-login attempt.

## Service credential rotation

Gateway -> Platform and Gateway -> legacy engine use separate secrets. Never reuse one credential across both boundaries.

### Gateway -> Platform

Platform runtime configuration:

```text
GAME_AUTH_GATEWAY_SERVICE_TOKEN_SHA256=<new/current hash>
GAME_AUTH_GATEWAY_PREVIOUS_SERVICE_TOKEN_SHA256=<retiring hash, optional>
```

Safe rotation:

1. generate a new high-entropy credential in the approved secret manager;
2. configure the new hash as current and the retiring hash as previous;
3. deploy/reload Platform and verify both credentials during the overlap window;
4. roll Gateway instances to the new plaintext runtime secret;
5. verify all Gateway instances use the new credential;
6. remove the previous hash and redeploy/reload Platform.

### Gateway -> legacy engine

legacy engine runtime configuration:

```text
CANARY_GAME_SESSION_SERVICE_TOKEN_SHA256=<new/current hash>
CANARY_GAME_SESSION_PREVIOUS_SERVICE_TOKEN_SHA256=<retiring hash, optional>
```

Use the same bounded overlap order:

1. configure legacy engine current=new and previous=retiring;
2. restart/reload the exact issuer process using the approved deployment mechanism;
3. roll Gateway `GAME_SESSION_SERVICE_TOKEN` to the new plaintext secret;
4. verify the new credential through health/readiness/native-auth evidence;
5. remove the previous hash from legacy engine and restart/reload the issuer process.

Malformed configured hashes fail closed.

## Transport boundary

Repository-level Gateway configuration enforces:

- `https://` for non-loopback `OTERYN_PLATFORM_BASE_URL`;
- `https://` for non-loopback `GAME_SESSION_SERVICE_BASE_URL`;
- plain HTTP only for loopback development/test dependencies;
- standard Go TLS certificate and hostname verification;
- no credentials in dependency URLs;
- bounded request timeout.

A private network is defense in depth, not a replacement for TLS or service authentication.

Production activation additionally requires direct evidence that:

- Platform private endpoints are reachable only through the intended internal ingress/firewall boundary;
- the legacy engine issuer listener is reachable only from the intended Gateway/deployment network;
- deployed TLS certificates and hostnames validate from the Gateway runtime;
- plaintext service credentials are injected through the approved secret manager and are not stored in Git/logs;
- the deployed revisions match the approved release evidence.

Repository tests cannot by themselves prove those external deployment facts.

## HTTP caching boundary

Sensitive native-auth responses must not be cacheable.

Platform ticket issue/redeem and Gateway native-login responses use:

```text
Cache-Control: no-store, no-cache, must-revalidate, private
Pragma: no-cache
Expires: 0
```

The legacy engine issuer returns `Cache-Control: no-store` and `Pragma: no-cache` on its HTTP responses.

## Proven E2E baseline

The pre-hardening bounded native-auth path is proven by Universal Agent E2E:

- behavior run `29988893301`;
- final evidence run `29992417296`;
- scenario `login/oteryn-native-auth`;
- legacy engine adapter revision `285dec6a034aa3620ae5ca12549fb9e8e1b35631`;
- OTClient revision `bb87346f6c516a19d19497d82bb01fb389334ff5`;
- Gateway revision `8006534108d835474dadd208b0ec934e4a12528b`.

The evidence proves:

- a maintained OTClient submits a fresh Game Login Ticket to the real Gateway;
- Gateway obtains a legacy engine Game Session;
- `Knight 1` enters the world exactly once;
- logout completes;
- replay of the same Game Session is rejected;
- `successful_world_entries=1`.

This baseline predates PR #124 / PR #807 hardening and therefore is not, by itself, proof of the hardened production boundary.

## Hardened E2E gate

Before production activation, rerun the same `login/oteryn-native-auth` scenario with exact merged hardened revisions for:

- Oteryn Platform PR #124;
- legacy engine PR #807;
- maintained OTClient `bb87346f6c516a19d19497d82bb01fb389334ff5`.

Required behavior remains:

- one successful world entry;
- fail-closed replay;
- exactly one successful server login/world entry;
- no credential leakage in retained logs/evidence.

## Rollout ordering

1. Merge/deploy Platform hardening with native-auth activation unchanged.
2. Merge/deploy legacy engine credential-rotation support with the issuer still disabled unless already deliberately enabled in a controlled environment.
3. Provision private/TLS Platform and legacy engine service routes.
4. Provision separate Gateway -> Platform and Gateway -> legacy engine service credentials using overlap-capable configuration.
5. Deploy the compatible OTClient build.
6. Re-prove the hardened exact-revision native-auth E2E in the production-like deployment boundary.
7. Verify deployed revision, TLS, ingress/firewall and secret-manager evidence.
8. Enable the legacy engine issuer and Gateway native-auth path in the controlled production rollout.
9. Observe health/readiness/authentication/replay metrics and retain rollback capability.
10. Only after successful production validation consider fencing/removing legacy password authentication.

## Production activation gate

Repository merge is deploy-first-safe and does **not** authorize production native-auth activation.

Production activation remains blocked while any of these are unproven:

- exact private-network/ingress/firewall boundary;
- exact TLS certificate/hostname validation from deployed Gateway;
- exact secret-manager injection and credential-rotation state;
- exact deployed Platform/Gateway/legacy engine/OTClient revisions;
- hardened exact-revision native-auth E2E;
- rollback/legacy-auth availability during controlled cutover.

Legacy password authentication remains available until a separately evidenced cutover decision.
