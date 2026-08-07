# Game Gateway to Oteryn Identity Contract

## Status

`IMPLEMENTED REPOSITORY CONTRACT — DEPLOYMENT AND PRODUCTION ACTIVATION NOT PROVEN`

`PRODUCTION_PROVEN=false`.

## Delivered repository boundary

The bounded contract producer is delivered on `main`:

- Oteryn Platform PR #121 delivered Game Login Ticket issuance and the private atomic redeem endpoint;
- Oteryn Platform PR #122 delivered the separately buildable Game Gateway, authoritative login-context orchestration, World Registry use, account-scoped character projection and the Game Session issuer boundary, merged as `8006534108d835474dadd208b0ec934e4a12528b`;
- `GAME_SESSION_CANARY_CONTRACT.md` is the current operation-specific authority for the legacy-compatible Game Session contract version 1 and records its bounded cross-repository E2E evidence.

Repository delivery proves the application and service contract, not a production deployment. Exact public/private ingress, TLS termination, service identity, credential rotation, deployed revisions, global legacy-path retirement and effective network isolation remain `UNKNOWN` until verified against one exact environment.

The selected native Game Session contract version 2 remains disabled by default and separately governed. The producer package associated with PR #542 is not, by itself, evidence that a native consumer, cross-repository cutover or production activation is complete.

This contract defines the security boundary between the separately deployable Oteryn Game Gateway and Oteryn Platform Identity.

It covers Game Login Ticket issuance semantics insofar as they determine redeem behavior, and the private Gateway-to-Identity redeem API.

## Ownership

### Oteryn Identity owns

- user authentication;
- password/MFA/passkey/recovery policy;
- Identity disabled/security state;
- OAuth native-client authorization;
- exact ready Identity -> Canary account binding;
- Game Login Ticket issuance;
- Game Login Ticket authoritative storage/consume state;
- `game_auth_generation` or equivalent revocation generation.

### Game Gateway owns

- accepting a ticket from OTClient;
- presenting it to the private redeem API;
- consuming the successful redeem result as the sole account authorization for the login attempt;
- world/character/session orchestration after successful redeem.

Gateway does not independently verify user credentials or infer ownership from client input.

## Contract version

Initial private API version:

```text
/internal/v1/game-auth/tickets/redeem
```

Breaking semantic changes require a new version.

## Ticket semantic contract

A Game Login Ticket is:

- opaque random bearer material;
- at least 256 bits of CSPRNG entropy before transport encoding;
- single-use;
- short-lived, default 60 seconds;
- audience-bound to `oteryn-game-gateway`;
- bound to exact `identity_id` internally;
- bound to exact ready `canary_account_id` internally;
- bound to the Identity game-auth security generation current at issuance;
- persisted only as a cryptographic hash/HMAC-derived lookup representation;
- never logged in plaintext.

The ticket is not a JWT requirement and contains no client-visible authoritative claims.

## Public ticket issuance precondition

Although issuance is not called by Gateway, redeem correctness depends on this invariant.

Identity may issue a ticket only when:

```text
OAuth bearer is valid
AND OAuth client is the allowed first-party native client
AND scope includes game:ticket
AND Identity is enabled
AND current security policy is satisfied
AND IdentityCanaryAccount binding is READY
AND canary_account_id is non-null and exact
```

Client-supplied `identity_id` or `canary_account_id` cannot establish ownership.

## Private redeem request

Target request:

```http
POST /internal/v1/game-auth/tickets/redeem
Authorization: <service authentication mechanism>
Accept: application/json
Content-Type: application/json
```

Semantic body:

```json
{
  "protocol_version": 1,
  "ticket": "<opaque-game-login-ticket>",
  "audience": "oteryn-game-gateway"
}
```

The final transport authentication header may be mTLS workload identity, a dedicated bearer service credential, or another reviewed mechanism. It must not be confused with the user's ticket.

## Service authentication

Minimum requirements:

- only Game Gateway service identity is authorized to redeem;
- authentication is enforced by Identity application logic, not private-network location alone;
- TLS required;
- production ingress should be private/restricted;
- service credential/certificate is unique to Gateway capability;
- credential is externally injected, rotatable and never committed/logged;
- service authorization grants ticket redeem only, not generic Identity administration.

Preferred production direction: mTLS/workload identity with managed rotation.

Exact production mechanism remains a deployment decision and must be directly verified.

## Redeem algorithm

The authoritative operation must be equivalent to one atomic/transactionally safe decision:

1. authenticate Gateway service;
2. validate request/protocol version and expected audience;
3. derive/hash the presented ticket before lookup;
4. find exact matching ticket state;
5. verify `used_at IS NULL` or equivalent unused state;
6. verify `expires_at > authoritative_server_now`;
7. verify stored audience equals `oteryn-game-gateway`;
8. verify current Identity exists and is enabled;
9. verify stored ticket generation equals current Identity `game_auth_generation`;
10. verify Identity's current Canary binding remains ready and still resolves to the exact ticket-bound `canary_account_id`;
11. atomically mark ticket used/consumed;
12. return the minimum successful authorization result.

Steps affecting replay/revocation correctness must run in one transaction/atomic store operation or an equivalent design that cannot produce two successful redeems.

Exactly one concurrent redeem may succeed.

## Ordering note

The implementation must choose a transaction ordering that prevents a race where:

- security generation changes;
- Identity becomes disabled;
- binding becomes invalid;

while a stale ticket is being redeemed.

The safe invariant is:

> A redeem may succeed only if the authoritative security/binding state validated for that redeem is still current at the atomic commit point.

Exact locking/CAS mechanism is storage-specific and belongs to implementation tests.

## Successful response

Semantic response:

```json
{
  "protocol_version": 1,
  "authorization": {
    "canary_account_id": 12345,
    "security_generation": 7,
    "redeemed_at": "2026-07-21T21:45:00Z"
  }
}
```

`canary_account_id` is authoritative for this login attempt.

Gateway must not need:

- password/hash;
- email;
- MFA state/secret;
- recovery codes;
- OAuth authorization code;
- OAuth access/refresh token;
- Platform web session ID.

An opaque Identity subject may be added only if a proven Gateway audit/correlation requirement cannot be met without it. It is omitted by default.

## Error response policy

Internal service responses may use bounded machine-readable codes while never returning ticket or credential material.

Recommended semantic categories:

```text
invalid_request
unsupported_protocol_version
unauthorized_service
invalid_ticket
expired_ticket
already_used
revoked_ticket
identity_disabled
game_account_unavailable
temporarily_unavailable
```

Security rule:

- public OTClient-facing Gateway responses may collapse several internal categories into a generic invalid/expired sign-in response;
- internal codes are for service orchestration/metrics, not detailed user enumeration;
- no error contains raw ticket, ticket hash, service credential or user password data.

HTTP status mapping is finalized during API implementation. Suggested direction:

- malformed request: 400;
- service authentication failure: 401/403;
- invalid/expired/reused/revoked ticket: 401;
- unsupported version: 400/409 depending final API convention;
- authoritative dependency failure: 503.

## Replay semantics

### First request

```text
UNUSED -> USED
result: success
```

### Any later request

```text
USED -> no transition
result: already_used/invalid_ticket
```

### Concurrent requests

Exactly one transition to `USED`.

No successful redeem result may be cached/replayed by Gateway as authorization for a separate new client login attempt unless a later explicit idempotency protocol binds the cache to the same original request and preserves single authorization semantics.

## Ticket expiry

Default:

```text
TTL = 60 seconds
```

Server-side expiry is authoritative.

Client clocks do not determine validity.

Expired tickets are never revived.

Cleanup may physically delete expired/used rows after an appropriate retention interval, but deletion strategy must preserve the needed replay/audit semantics without retaining plaintext credentials.

## Revocation generation

Ticket record stores the generation current at issuance.

Redeem compares against current Identity generation.

At minimum the generation advances for:

- password change;
- password reset;
- high-risk recovery;
- Identity disablement;
- administrator compromise response;
- MFA reset/disable when policy requires pending game authorization invalidation.

A generation mismatch fails redeem.

Explicit deletion of outstanding ticket rows may supplement generation invalidation but is not required for correctness if the generation comparison is atomic/current.

## Binding semantics

Ticket issuance uses the exact current ready immutable binding.

Redeem revalidates that the Identity still has a ready exact binding to the ticket-bound account.

Current product model does not support normal self-service rebind/transfer.

If binding is:

- missing;
- pending;
- conflict;
- null account ID;
- unexpectedly changed;

redeem fails closed.

Gateway cannot override this by passing an account ID.

## Idempotency

Ticket redeem itself is intentionally **not idempotent as a reusable success operation**: a ticket is single-use.

A duplicate request after a completed consume is denied.

If transport response is lost after Identity commits consume, Gateway must not redeem the ticket again and expect a second success.

Therefore the Gateway login orchestration must handle ambiguous redeem outcomes conservatively.

Possible future enhancement:

- Gateway-generated `login_attempt_id` recorded with consume;
- exact same authenticated Gateway attempt may retrieve a bounded already-committed redeem result without authorizing a different attempt.

This enhancement is not part of v1 unless implementation proves it necessary and updates this contract/threat model.

## Failure semantics

| Condition | Identity behavior | Gateway behavior |
|---|---|---|
| Ticket store unavailable | No consume; fail closed | Return temporary login failure; create no Game Session |
| Identity DB unavailable | No authorization | Return temporary login failure |
| Service auth invalid | Deny | Treat as internal configuration/security failure |
| Ticket unknown | Deny | Generic sign-in invalid/expired response |
| Ticket expired | Deny | Restart sign-in required |
| Ticket already used | Deny | Restart sign-in required |
| Generation stale | Deny | Restart sign-in required |
| Identity disabled | Deny | Do not create Game Session |
| Binding not ready/exact | Deny | Do not create Game Session |

No failure condition triggers password fallback.

## Rate limiting and abuse controls

Identity should rate-limit private redeem defensively even though Gateway is authenticated.

Controls may include:

- per-service-instance/global request budget;
- malformed/invalid ticket abuse counters;
- circuit-breaking only where it fails closed;
- alerting on abnormal replay volume.

Do not rate-limit in a way that changes exactly-once semantics or permits fallback.

Concrete thresholds require measured runtime evidence.

## Logging and audit

May record:

- event type;
- server-generated request/correlation ID;
- success/failure category;
- bounded service identity identifier;
- pseudonymous/authorized account reference where policy permits and is operationally necessary;
- timestamps/duration.

Must not record:

- raw ticket;
- ticket hash if it can become a stable credential-correlator without necessity;
- OAuth token/code;
- service credential;
- user password/hash;
- MFA/recovery material.

Security audit event examples:

```text
game_ticket_issued
game_ticket_redeemed
game_ticket_redeem_rejected
game_ticket_revoked_by_generation
```

High-volume invalid-ticket probing should be operational security telemetry, not a detailed user audit trail containing attacker-controlled credential material.

## Ticket storage requirements

The authoritative store must support:

- atomic conditional consume;
- TTL/expiry query;
- concurrency across multiple Identity/Gateway processes;
- restart-safe consumed state for the ticket lifetime;
- no plaintext ticket storage;
- deterministic tests for concurrent redeem.

Possible implementations include a transactional database row-lock/conditional update or Redis atomic primitive/script with appropriate durability/availability semantics.

The architecture does not select one in Phase 0.

Process-local memory is rejected as the authoritative multi-instance ticket store.

## Gateway retry requirements

Gateway must not blindly retry redeem after an ambiguous transport failure.

Safe choices for v1:

1. treat ambiguous redeem as terminal and require a fresh user login/ticket; or
2. implement an explicit bounded idempotent attempt protocol in a later contract revision.

Automatic retry is allowed only when the client can prove the request was not delivered/committed, which is generally not safely knowable after a connection failure.

## Required contract tests

- valid service + valid ticket succeeds;
- invalid service denied;
- wrong audience denied;
- unknown ticket denied;
- expired ticket denied;
- reused ticket denied;
- two concurrent redeems produce exactly one success;
- generation changed before redeem denied;
- Identity disabled before redeem denied;
- binding pending/conflict/missing denied;
- client/Gateway cannot select a different account ID;
- ticket store outage fails closed;
- response/logs contain no raw ticket;
- unsupported protocol version denied.

## Deployment verification

Repository tests do not prove:

- internal endpoint is not publicly reachable;
- TLS termination is correct;
- mTLS/service secret rotation works;
- production store is shared/atomic;
- logs/proxies do not capture request bodies.

These remain production verification requirements.
