# Oteryn Game Gateway

Standalone Go runtime for the Oteryn game-login orchestration boundary defined by ADR 0009.

## Current producer scope

Implemented public surface:

```text
GET  /health
GET  /ready
GET  /version
POST /v1/login
```

`POST /v1/login` retains Gateway API `protocol_version: 1` and one opaque Game Login Ticket. It also accepts the optional bounded `gameplay_offer` defined by the native gameplay contract.

The Gateway:

1. redeems the ticket through the Oteryn Platform private Identity API;
2. receives the exact authorized Canary account ID;
3. obtains the single-world-ready World Registry/character login context through a separate narrow Platform private API;
4. preserves the existing Game Session v1 issuer for legacy requests;
5. for an extended request, selects the first exact authoritative World Registry candidate, verifies its v2 readiness identity and invokes exactly one Game Session v2 issuance;
6. returns the Game Session plus sanitized world, character and immutable selection data.

The Gateway has no Platform or Canary database credentials.

`POST /v1/login` with `protocol_version: 2` is the native login branch of `docs/contracts/OTERYN_V2_NATIVE_GATEWAY_LOGIN_CONTRACT.md`. It is off by default; see [Native login branch](#native-login-branch).

Native candidate persistence is empty and disabled by default. Producer rollout and rollback are documented in `docs/operations/OTERYN_NATIVE_PROTOCOL_PRODUCER.md`. Otheryn and Rust native consumers remain required before activation.

## Environment

Required:

```text
OTERYN_PLATFORM_BASE_URL
OTERYN_PLATFORM_SERVICE_TOKEN
GAME_SESSION_SERVICE_BASE_URL
GAME_SESSION_SERVICE_TOKEN
```

Optional:

```text
GATEWAY_LISTEN_ADDR=:8080
GATEWAY_REQUEST_TIMEOUT=5s
GATEWAY_VERSION=dev
GATEWAY_NATIVE_LOGIN_ENABLED=false
GATEWAY_NATIVE_ADMISSION_TIMEOUT=5s
```

Service credentials are injected runtime secrets. Do not commit them or place them in URLs.

Dependency URL policy is fail-closed:

- non-loopback Platform and Game Session service URLs must use `https://`;
- plain `http://` is accepted only for `localhost`, `127.0.0.0/8` and `::1` loopback development/test dependencies;
- standard Go HTTPS certificate and hostname verification remains enabled because the Gateway does not install an insecure TLS transport.

A private network is defense in depth, not a replacement for TLS or service authentication. Production routing should expose the Platform private API and Canary Game Session issuer only to the Gateway through explicit internal ingress/firewall rules.

## Service credential rotation

### Gateway -> Platform

Platform accepts one current and one optional previous Gateway service credential SHA-256 hash:

```text
GAME_AUTH_GATEWAY_SERVICE_TOKEN_SHA256
GAME_AUTH_GATEWAY_PREVIOUS_SERVICE_TOKEN_SHA256
```

The Gateway receives the corresponding plaintext bearer value only through `OTERYN_PLATFORM_SERVICE_TOKEN` at runtime.

Safe rotation order:

1. generate a new high-entropy service credential in the approved secret manager;
2. configure Platform `PREVIOUS` with the retiring hash and `SERVICE_TOKEN_SHA256` with the new hash;
3. deploy/reload Platform and verify both credentials are temporarily accepted;
4. roll Gateway instances to the new plaintext `OTERYN_PLATFORM_SERVICE_TOKEN`;
5. verify all Gateway instances use the new credential;
6. clear the Platform `PREVIOUS` hash and redeploy/reload Platform.

Never reuse the same service credential for Gateway -> Platform and Gateway -> Canary.

### Gateway -> Canary Game Session issuer

`GAME_SESSION_SERVICE_TOKEN` is a separate runtime secret. The Canary issuer must be configured with the matching SHA-256 service-credential hash set before the Gateway token is rotated. Use the same overlap sequence: add the new hash while retaining the old hash, roll Gateway, verify, then remove the old hash.

## Dependency contracts

### Platform ticket redeem

```text
POST /internal/v1/game-auth/tickets/redeem
Authorization: Bearer <platform service credential>
```

### Platform login context

```text
GET /internal/v1/game-auth/accounts/{canaryAccountId}/login-context
Authorization: Bearer <platform service credential>
```

The Phase 4 Platform endpoint is intentionally single-world-ready and fails closed when zero or more than one eligible world exists because persistent character-to-world ownership is not yet defined for true multiworld.

### Game Session issuer

```text
POST /internal/v1/game-sessions
Authorization: Bearer <session service credential>
```

Request semantics:

```json
{
  "protocol_version": 1,
  "canary_account_id": 1001,
  "world_id": 1,
  "login_attempt_id": "server-generated-id"
}
```

Response semantics:

```json
{
  "protocol_version": 1,
  "session": {
    "credential": "<opaque-game-session-secret>",
    "expires_at": "2026-07-22T08:30:00Z"
  }
}
```

The concrete Canary-compatible Session Issuer is delivered by Canary PR #722. Its bounded OTClient -> Gateway -> Canary E2E is proven, but production activation still requires exact private/TLS routing, injected credential rotation and production-like re-verification against the deployed revisions.

### Native login branch

With `GATEWAY_NATIVE_LOGIN_ENABLED=true` (exactly `true` or `false`; default off), a request whose top-level `protocol_version` is the integer `2` is served by the native branch (`internal/nativelogin`). Every other request keeps the Canary-compatible path above unchanged, and a native request never reaches the ticket redeem, login-context or Game Session issuer calls. With the switch off, `protocol_version: 2` keeps its previous `400 {"error":"invalid_request"}` answer.

Native-only mode: with the switch on and both `GAME_SESSION_SERVICE_BASE_URL` and `GAME_SESSION_SERVICE_TOKEN` unset, the Gateway builds no Game Session client. Legacy requests then fail closed with `503 {"error":"login_unavailable"}` before the ticket is redeemed, and `/ready` depends only on the Platform. Setting exactly one of the two variables is a configuration error in every mode. With the switch off, both remain required.

The native branch:

1. applies the contract §10 Gateway limits in process: 30 requests per minute per connection source address (no forwarding header is trusted) and 6 requests in total per (`attempt_ref`, ticket hash), retained for 2 minutes (longer than a ticket's 60 s plus a grant's 30 s); the limiter refuses new keys when its bounded table is full;
2. validates the exact §3.1 request with the issuer's rules (2048 bytes, exact members, canonical UUIDv7s, bounded ASCII, at least one supported transport);
3. forwards the re-encoded request once to the private issuer:

```text
POST /internal/v1/game-auth/native-admissions
Authorization: Bearer <platform service credential>
```

4. returns the issuer's §3.2 success only after validating it exactly (same `attempt_ref`, requested channel if any, Registry endpoint, `oteryn-pre-admission-v1` JWS of at most 4096 bytes, validity 1..30 s), or a §11.1 error body.

The issuer call uses the Gateway -> Platform service credential, is bounded by `GATEWAY_NATIVE_ADMISSION_TIMEOUT` (at most 8 s) and never follows redirects. Error mapping for an issuer that gives no valid answer: a request that never left the Gateway is `NATIVE_LOGIN_UNAVAILABLE`; once written, a timeout, transport failure or invalid success body is `ADMISSION_ATTEMPT_RECONCILIATION_REQUIRED` (the client retries the same ticket and `attempt_ref`, and the issuer returns the committed grant); an issuer error body outside the §11.2 mapping is `NATIVE_LOGIN_UNAVAILABLE`. Issuer error codes pass through; `SECURITY_TERMINAL` codes are only ever public as `NATIVE_LOGIN_AUTHENTICATION_REQUIRED`.

The Gateway holds no database or signing credential and no grant state. Enablement is limited to `testing`/`preproduction` by the issuer; production enablement needs separate authority.

## Response caching

`POST /v1/login` returns opaque Game Session material and therefore sets `Cache-Control: no-store, no-cache, must-revalidate, private`, `Pragma: no-cache` and `Expires: 0` on success and bounded failure responses.

## Logging

Structured logs contain bounded request metadata only:

- request ID;
- HTTP method;
- path;
- status;
- duration.

The native branch adds one `native_login_result` line with the canonical `attempt_ref` (or empty) and the result code.

The service does not log request/response bodies, headers, Game Login Tickets, grants, service credentials or Game Session secrets.

## Local validation

```bash
gofmt -w .
go test ./...
go vet ./...
go build ./cmd/game-gateway
```

The repository workflow `Game Gateway CI` performs formatting, tests, vet and build independently from the Laravel CI.
