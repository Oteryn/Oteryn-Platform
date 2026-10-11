# ADR 0009 — Oteryn game authentication architecture

## Status

Accepted — 2026-07-21

## Context

Oteryn Platform already owns the supported product Identity lifecycle, including framework-hashed credentials, revocable web sessions, password recovery/change, TOTP MFA and the immutable greenfield mapping:

```text
1 Platform Identity <-> 1 legacy engine accounts.id
```

The authoritative game-login bridge is not implemented.

The current repository-supported game authentication landscape contains parallel password/session authorities:

- native legacy engine `ProtocolLogin` can validate reusable credentials;
- upstream `opentibiabr/login-server` validates SHA-1 credentials and creates replayable DB-backed `account_sessions`;
- Legacy engine can issue process-local short-lived `LoginSessionManager` tokens in `authType=session` mode;
- legacy protocol layouts can carry account/password directly;
- current OTClient still reads account/password into global login state, can persist encrypted password material, sends the password through HTTP login, and passes it to `ProtocolLogin`.

Fresh Phase 0 evidence pins:

- Oteryn Platform `main`: `09450e6f96638ae2dbf6d18c9585e7ccc5f6d24f`;
- OTClient `main`: `a6868920443dc285656bd016acdb2c1ea566e511`;
- Legacy engine `main`: `d760ce44c55b9aa6f01e80d2d407f6833938bdce`;
- upstream login-server `main`: `2612930de4d97123a397f8f2cd0d5f784094af40`.

The upstream login-server pin is unchanged from the existing game-auth discovery contract. Legacy engine is 122 commits ahead of that contract's original legacy engine pin; the compare delta does not list authentication/runtime source files, so the existing mapped auth behavior remains the current evidence baseline for this decision.

Oteryn requires a target architecture in which:

- Oteryn Identity is the only reusable user-credential authority;
- OTClient does not submit the user's main password to Game Gateway or legacy engine;
- browser-based password/MFA/passkey policy can evolve without legacy engine changes;
- login is ready for multiworld and multiregion;
- the login layer can scale independently;
- legacy password paths remain available only during controlled migration and cannot be described as globally authoritative once the new policy is claimed.

## Decision

### 1. Oteryn Identity is the sole reusable credential authority

Only Oteryn Platform Identity may authenticate reusable user credentials and apply reusable-credential policy:

- password verification and hashing;
- MFA;
- future passkeys;
- recovery;
- disabled-account policy;
- Identity security state and revocation generation.

Game Gateway and legacy engine must not become independent password verifiers in the target architecture.

The existing random legacy engine sink credential used for greenfield account compatibility is not an authentication credential and must never become a login fallback.

### 2. Native-client authorization uses standards-based Authorization Code + PKCE

OTClient is registered as a **public native client** without a confidential client secret.

The target flow uses:

```text
OTClient
  -> system browser
  -> Oteryn Authorization Server
  -> Authorization Code + PKCE
  -> OTClient loopback callback
```

The Authorization Server implementation will use Laravel Passport rather than a custom OAuth authorization protocol.

The native client must:

- generate a high-entropy PKCE `code_verifier`;
- use `S256` `code_challenge`;
- generate and validate high-entropy `state`;
- bind only to loopback interfaces;
- use an IP literal loopback redirect, preferably `127.0.0.1` and optionally `[::1]`;
- use a fixed registered callback path and an OS-selected ephemeral port;
- open the system browser rather than an embedded credential webview;
- close the loopback listener after the authorization response completes or fails.

The registered redirect is exact except for the loopback port, as permitted by RFC 8252 and supported by the selected OAuth server stack.

### 3. OAuth authorization is a short-lived bootstrap, not the game credential

Authorization Code exchange yields a narrowly scoped short-lived OAuth access credential for the native client.

The only required initial scope is:

```text
game:ticket
```

OTClient uses that credential only to request a Game Login Ticket from Oteryn Platform.

The OAuth access credential:

- is never sent to Game Gateway;
- is never sent to legacy engine;
- is never used as a game session credential;
- must have a bounded short lifetime suitable for bootstrap use;
- must not provide long-lived launcher login semantics in the first release.

If the selected OAuth library emits a refresh token for this grant, the implementation must prevent long-lived refresh semantics for the native game-login client. The exact mechanism is an implementation gate for Phase 2: disable issuance when safely supported, or use a short lifetime and revoke the OAuth token family immediately after successful Game Login Ticket issuance. OTClient must not persist or use a refresh token in the first release.

### 4. Game Login Ticket is opaque, one-time and Identity-owned

After successful native authorization, Oteryn Identity issues an opaque Game Login Ticket.

Required properties:

- generated from at least 256 bits of CSPRNG entropy;
- opaque to OTClient;
- plaintext returned exactly once to the client;
- only a cryptographic hash/HMAC-derived lookup key stored server-side;
- default TTL: 60 seconds;
- single-use;
- atomically consumed;
- bound to exact `identity_id`;
- bound to exact ready immutable `canary_account_id`;
- bound to audience `oteryn-game-gateway`;
- bound to the current `game_auth_generation` or equivalent monotonic revocation generation;
- optionally constrained by client/protocol version policy;
- never written to application, access, audit or request-completion logs.

The ticket is not:

- an OAuth access token;
- an OAuth refresh token;
- a Laravel web session;
- a legacy engine password;
- a Game Session.

### 5. Ticket redeem is a private server-to-server Identity operation

Game Gateway receives the plaintext ticket from OTClient and calls a private Identity endpoint:

```text
POST /internal/v1/game-auth/tickets/redeem
```

The endpoint must:

- authenticate the Gateway service;
- require audience `oteryn-game-gateway`;
- hash/derive the presented ticket before lookup;
- atomically consume exactly one unused unexpired ticket;
- re-check current Identity disabled/security generation state;
- fail closed on missing, expired, reused, revoked, malformed or ambiguous state;
- return only the minimum canonical authorization result required by Gateway.

The first deployment may use private-network TLS plus a rotated service credential. mTLS is the preferred production service-authentication direction when the deployment platform supports reliable certificate issuance/rotation.

Gateway receives no password hash, password, MFA secret, recovery code, OAuth authorization code or OAuth refresh token.

### 6. Game Gateway is a separate Go runtime in the Oteryn Platform repository

Game Gateway will live under:

```text
services/game-gateway/
```

The selected implementation language is **Go**.

Reasoning:

- the Gateway is intentionally a separately deployable network/service boundary rather than Laravel application code;
- it should remain stateless at the process layer and scale horizontally;
- a small Go service provides a narrow dependency footprint and straightforward standalone container/runtime semantics;
- integration with Laravel/Identity occurs through versioned contracts rather than shared in-process classes.

Consequences:

- the repository gains a Go toolchain and CI surface;
- no Laravel application class is imported into Gateway;
- cross-runtime behavior is contract-tested;
- service configuration, health/readiness and observability are independent.

### 7. Gateway responsibilities are deliberately narrow

Gateway owns orchestration for:

1. accepting Game Login Ticket login requests;
2. redeeming tickets through Identity;
3. resolving the authorized legacy engine account ID returned by redeem;
4. loading worlds available to that account;
5. loading the account's listable characters through an approved narrow read boundary;
6. creating a Game Session through an approved compatibility adapter;
7. returning character/world routing to OTClient.

Gateway does not own:

- password authentication;
- MFA/passkey/recovery policy;
- registration;
- CMS/admin;
- general Platform database access;
- generic legacy engine database access.

### 8. Game Session is a distinct credential lifecycle

After ticket redeem, Gateway creates a Game Session credential.

Logical minimum state:

```text
session_id
session_secret_hash
canary_account_id
world_id
created_at
expires_at
revoked_at
```

A Game Session is not the Game Login Ticket and not an OAuth credential.

The first legacy engine compatibility adapter may use legacy engine's existing `account_sessions` mechanism **only if** a separate operation-specific contract proves:

- exact schema and hash semantics;
- exact legacy engine version compatibility;
- bounded session TTL;
- dedicated least-privilege INSERT/SELECT/DELETE privileges as required;
- deterministic revocation/cleanup behavior;
- no fallback to the legacy engine sink password;
- no privilege expansion into generic account/player writes.

Using `account_sessions` does not make the Game Login Ticket replayable; ticket redeem remains one-time. The known replayability of an issued `account_sessions` credential until expiry/revocation must be treated as a separate Game Session risk and bounded explicitly.

If these requirements cannot be satisfied without unacceptable replay or revocation behavior, a separately authorized legacy engine protocol/session change is required.

### 9. World Registry is a first-class, versioned domain from day one

The initial release may contain one world, but API/storage contracts must not assume a singleton.

Minimum world record:

```text
world_id
slug
name
region
status
login_enabled
game_host
game_port
```

The World Registry is logically owned by Oteryn Platform and consumed by Gateway through a narrow repository/configuration boundary.

Gateway returns only worlds authorized and currently login-enabled for the redeemed account.

Game Sessions are world-scoped.

Future additions may include:

- maintenance state;
- queues/capacity;
- preview/test/tournament access policy;
- region-aware endpoints;
- channel allocation.

Multichannel gameplay synchronization is explicitly not part of the first game-auth implementation.

### 10. Game-auth revocation uses an explicit monotonic generation

The Platform introduces `game_auth_generation` or an equivalent monotonic Identity security generation dedicated to game authorization.

Ticket issuance captures the current generation. Ticket redeem requires it to remain current.

At minimum, the generation must advance for:

- password reset;
- password change;
- high-risk recovery action;
- MFA reset/disable where game-login policy requires invalidation;
- Identity disablement;
- administrator compromise-response action.

The implementation must define whether already issued Game Sessions are also revoked/disconnected for each event. That active-session policy is separate from pending-ticket revocation and must not be guessed.

### 11. Legacy password paths are migration-only

The new flow is introduced additively.

Until full cross-repository E2E is proven:

```text
legacy password login = compatibility path
new Oteryn login      = candidate authoritative path
```

After E2E proof and rollout readiness:

1. direct public native legacy engine login is disabled or network-fenced;
2. upstream login-server password authentication is removed/replaced/fenced for Oteryn clients;
3. unsupported legacy protocol direct-password login is disabled by default or explicitly documented as a lower-security compatibility tier;
4. long-lived DB-session fallback is not silently accepted when one-time authorization fails.

Only after every reachable bypass is removed, fenced or governed by the same authoritative policy may Oteryn claim Identity is globally authoritative for game login.

### 12. Trust boundaries and storage

Public:

```text
Browser
OTClient
Gateway public game-login endpoint
Legacy engine game endpoint
```

Private:

```text
Gateway -> Identity ticket redeem
Gateway -> narrow character/account read boundary
Gateway -> Game Session compatibility persistence
World Registry persistence
shared ticket/session state
administrative endpoints
```

Gateway must use separate least-privilege service/database credentials per capability. A single credential with broad Platform + legacy engine database access is rejected.

Ticket/session shared state must support horizontal Gateway/Identity deployment without process-local correctness assumptions.

### 13. Protocol versioning

All new external/internal contracts are versioned from the first release:

- public Gateway API: `/v1/...`;
- private Identity redeem API: `/internal/v1/...`;
- explicit `protocol_version` in native client requests where payload evolution requires it.

Breaking changes require a new major contract version or an explicitly backward-compatible rollout window.

## Required first-release flow

```text
OTClient
  -> system browser
  -> Oteryn Identity / OAuth Authorization Server
  -> Authorization Code + PKCE
  -> short-lived OAuth bootstrap credential
  -> Game Login Ticket issuance
  -> Oteryn Game Gateway
  -> private atomic ticket redeem at Identity
  -> World Registry + Character List
  -> Game Session creation
  -> selected legacy engine endpoint
  -> player enters game
```

The user's main password never reaches Game Gateway or legacy engine in this flow.

## Consequences

### Positive

- password/MFA/passkey policy remains centralized in Identity;
- Legacy engine can evolve independently from reusable user credential policy;
- OTClient becomes a public native OAuth client rather than a password custodian;
- one-time ticket replay is deterministically blocked;
- Gateway can scale independently and route multiworld/multiregion sessions;
- world routing becomes data-driven;
- cross-repository coupling is explicit and versioned.

### Costs

- Laravel Passport and OAuth key lifecycle become new Platform security dependencies;
- a second runtime/toolchain (Go) is introduced in the repository;
- Gateway availability and Identity redeem availability join the login critical path;
- Game Session compatibility with current legacy engine still requires a dedicated operation contract and may require a separately authorized legacy engine change;
- legacy bypass closure requires coordinated deployment/network changes after E2E proof.

## Rejected alternatives

### OTClient sends the Oteryn password directly to Gateway

Rejected. It duplicates reusable credential handling outside Identity and prevents clean MFA/passkey evolution.

### Gateway validates Platform password hashes

Rejected. It breaks the single authoritative Identity boundary and expands credential exposure.

### Legacy engine validates the Oteryn password directly

Rejected as the target architecture. It preserves the current distributed password authority and makes Identity-only MFA/revocation bypassable.

### Use JWT Game Login Tickets

Rejected for the first release. Opaque random tickets make single-use consume, immediate revocation and minimal client-visible data simpler.

### Reuse Laravel web sessions as game credentials

Rejected. Browser session lifecycle, CSRF/cookie semantics and game-entry credential lifecycle are different trust domains.

### Send OAuth access/refresh tokens to Gateway or legacy engine

Rejected. OAuth credentials terminate at Oteryn Identity/Platform APIs; game components receive only game-specific credentials.

### Use current 24-hour `account_sessions` as the Game Login Ticket

Rejected. The current DB session is replayable and has a different lifecycle from the required short-lived one-time ticket.

### Put Game Gateway inside the Laravel web process

Rejected. The Gateway must remain independently deployable/scalable and isolated from web/CMS/admin runtime responsibilities.

### Implement multichannel in the first release

Rejected. Authentication/routing readiness for a future `channel_id` is required, but cross-instance gameplay-state synchronization is a separate architecture problem.

## Implementation order

1. Phase 0 — architecture, threat model, contracts, sequence diagrams and rollout plan.
2. Phase 1 — Platform domain foundation: world model, ticket/session lifecycle abstractions, configuration and protocol versioning.
3. Phase 2 — Laravel Passport native-client Authorization Code + PKCE integration.
4. Phase 3 — Game Login Ticket issuance, atomic redeem and revocation generation.
5. Phase 4 — Go Game Gateway MVP.
6. Phase 5 — OTClient implementation under a separately scoped OTClient task.
7. Phase 6 — legacy engine/Game Session compatibility implementation after exact contract proof and separate authorization if legacy engine changes are needed.
8. Phase 7 — cross-repository staging E2E.
9. Phase 8 — legacy path fencing/deprecation/removal.
10. Phase 9 — multiworld expansion.
11. Phase 10 — horizontal/multiregion scale.

## Production gates

The architecture is not production-proven until all of the following are demonstrated against exact deployed versions/configuration:

- Authorization Code + PKCE success and failure cases;
- `state` mismatch denial;
- expired/reused authorization code denial;
- Game Login Ticket expiry and atomic concurrent replay denial;
- disabled/revoked Identity denial;
- ready immutable legacy engine binding enforcement;
- wrong-account/wrong-character denial;
- Game Session expiry/revocation behavior;
- Gateway/Identity/session-store/legacy engine outage fail-closed behavior;
- direct legacy/native bypass attempts;
- no secrets/credentials in logs;
- exact deployed network fencing and service authentication;
- full browser -> client -> ticket -> gateway -> session -> legacy engine -> player entry E2E.

Production deployment remains a separate manual verification gate.

## Related contracts

- `docs/contracts/AUTH_GAME_LOGIN_CONTRACT.md` — current-state/bypass evidence baseline;
- `docs/contracts/OTCLIENT_GAME_AUTH_CONTRACT.md`;
- `docs/contracts/GAME_GATEWAY_IDENTITY_CONTRACT.md`;
- `docs/contracts/WORLD_REGISTRY_CONTRACT.md`;
- `docs/architecture/GAME_AUTH_THREAT_MODEL.md`;
- `docs/architecture/GAME_AUTH_SEQUENCE_DIAGRAMS.md`;
- `docs/architecture/GAME_AUTH_ROLLOUT_PLAN.md`.
