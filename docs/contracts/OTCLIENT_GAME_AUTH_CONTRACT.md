# OTClient Game Authentication Contract

## Status

`TARGET CONTRACT — NOT YET IMPLEMENTED IN OTCLIENT`

This contract defines the first-party Oteryn native-client behavior required by ADR 0009.

It does not authorize writes to `blakinio/otclient`. Implementation requires a separately scoped OTClient task.

## Evidence baseline

Current inspected OTClient `main` at architecture discovery:

```text
a6868920443dc285656bd016acdb2c1ea566e511
```

Current proven legacy behavior includes:

- account and password are read into global login state;
- password material can be encrypted and persisted in settings/ServerList;
- HTTP login receives account/password;
- native `ProtocolLogin` receives account/password.

The target Oteryn flow defined here is additive until cross-repository E2E is proven.

## Goals

For Oteryn-supported modern login:

- OTClient never asks for or stores the user's main Oteryn password;
- authentication occurs in the system browser;
- Authorization Code + PKCE S256 protects the native-client exchange;
- OTClient receives only short-lived bootstrap/game credentials;
- Game Login Ticket is sent to Oteryn Game Gateway;
- Gateway returns character/world routing and Game Session material;
- OTClient connects directly to the selected legacy engine game endpoint.

## Non-goals

This contract does not:

- replace generic/third-party OTClient server compatibility globally;
- define legacy engine gameplay protocol internals;
- implement MFA inside OTClient;
- give OTClient a confidential client secret;
- authorize OTClient to verify Game Login Tickets;
- define passkey UI inside OTClient;
- implement multichannel selection in the first release.

## Client registration

Logical registration:

```text
client_type: public
client_id: fixed first-party Oteryn native client identifier
allowed_grant: authorization_code
pkce: required
pkce_method: S256
redirect_uri: http://127.0.0.1/callback
scope: game:ticket
```

The actual configured redirect may include the fixed product-specific callback path selected during implementation.

Rules:

- OTClient has no confidential `client_secret`;
- dynamic client registration is not part of the first release;
- loopback host is an IP literal, not `localhost`;
- an implementation may additionally register IPv6 loopback `[::1]`, but IPv4 `127.0.0.1` is the required baseline;
- the loopback port is selected at runtime and may vary;
- scheme, host and callback path must match the registered native-client redirect contract.

## Required authorization flow

### Step 1 — create pending login attempt

OTClient generates:

```text
state            = cryptographically random high-entropy value
code_verifier    = PKCE-compliant cryptographically random value
code_challenge   = BASE64URL(SHA256(code_verifier))
callback_port    = OS-selected available ephemeral loopback port
```

Pending values are process-memory/transient state only for the active login attempt.

OTClient must not persist `code_verifier` as a reusable setting.

### Step 2 — bind callback listener

OTClient binds:

```text
127.0.0.1:{ephemeral_port}
```

Requirements:

- loopback interface only;
- no wildcard `0.0.0.0`/public interface bind;
- fixed callback path;
- bounded lifetime;
- one active authorization attempt per logical login flow unless implementation proves safe isolation;
- bind failure aborts the flow before browser launch or restarts with a new port/state/verifier.

### Step 3 — open system browser

Illustrative request:

```http
GET https://account.oteryn.com/oauth/authorize
    ?response_type=code
    &client_id=<public-client-id>
    &redirect_uri=http%3A%2F%2F127.0.0.1%3A<port>%2Fcallback
    &scope=game%3Aticket
    &state=<state>
    &code_challenge=<challenge>
    &code_challenge_method=S256
```

Actual production hostname is deployment configuration and remains outside this contract until production topology is verified.

OTClient must use the operating system's default/external browser.

Embedded credential webviews are not accepted for the first-party Oteryn login flow.

### Step 4 — callback

Expected success callback:

```http
GET /callback?code=<authorization_code>&state=<state>
```

OTClient must:

1. parse only the expected callback path;
2. compare returned `state` with the pending attempt;
3. reject missing/mismatched state;
4. reject duplicate completion of the same pending attempt;
5. never exchange a code received with invalid state;
6. close the listener after terminal success/failure;
7. clear pending verifier/state after terminal success/failure.

Expected OAuth error callback may include:

```text
error=access_denied
state=<state>
```

The client validates state before accepting the error as belonging to the pending flow.

### Step 5 — authorization code exchange

Illustrative token request:

```http
POST /oauth/token
Content-Type: application/x-www-form-urlencoded

grant_type=authorization_code
client_id=<public-client-id>
redirect_uri=http://127.0.0.1:<port>/callback
code=<authorization_code>
code_verifier=<code_verifier>
```

OTClient must not send a confidential client secret.

Expected successful result contains a short-lived OAuth access credential authorized for `game:ticket`.

First-release OTClient behavior:

- do not persist the OAuth access token beyond the login bootstrap lifecycle;
- do not persist or use an OAuth refresh token;
- do not send OAuth tokens to Game Gateway or legacy engine;
- clear bootstrap OAuth material after Game Login Ticket issuance or terminal failure.

## Game Login Ticket issuance

Illustrative request:

```http
POST /api/v1/game-auth/tickets
Authorization: Bearer <short-lived OAuth bootstrap credential>
Accept: application/json
Content-Type: application/json

{
  "protocol_version": 1
}
```

The client must not send:

- password;
- `identity_id` as ownership evidence;
- Legacy engine `account_id` as ownership evidence;
- MFA code/secret;
- recovery code;
- refresh token.

Illustrative success response:

```json
{
  "protocol_version": 1,
  "ticket": "<opaque-one-time-ticket>",
  "expires_in": 60
}
```

The exact JSON envelope is finalized by the Platform API implementation; semantic requirements are fixed by this contract.

Client requirements:

- ticket is bearer material and sensitive;
- keep only in process memory for the immediate Gateway request;
- never persist to settings/ServerList;
- never print/log/trace it;
- do not retry indefinitely after expiry;
- after a terminal ticket failure, restart the authorization flow.

## Gateway login request

Target endpoint:

```http
POST https://<game-gateway>/v1/login
Accept: application/json
Content-Type: application/json
```

Illustrative request:

```json
{
  "protocol_version": 1,
  "game_login_ticket": "<opaque-one-time-ticket>",
  "client": {
    "version": "<otclient-build-or-protocol-version>"
  }
}
```

Rules:

- no password;
- no OAuth access/refresh token;
- no client-selected authoritative `account_id`;
- no client-selected authoritative character ownership;
- ticket sent in request body, not URL/query string;
- TLS required outside local test environments.

## Gateway login response

Semantic response:

```json
{
  "protocol_version": 1,
  "session": {
    "credential": "<game-session-secret>",
    "expires_at": "<RFC3339 timestamp>"
  },
  "worlds": [
    {
      "id": 1,
      "slug": "oteryn-eu",
      "name": "Oteryn",
      "region": "EU",
      "status": "online",
      "host": "game-eu.oteryn.com",
      "port": 7172
    }
  ],
  "characters": [
    {
      "name": "Invalid Monk",
      "world_id": 1,
      "world_slug": "oteryn-eu"
    }
  ]
}
```

Fields shown are illustrative. The implementation may include additional non-sensitive character presentation fields after the contract explicitly allows them.

Required semantics:

- every character belongs to the exact redeemed legacy engine account;
- every returned world is login-enabled and authorized for that account;
- each character references a returned/known world;
- routing comes from World Registry, not user input;
- Game Session credential is sensitive bearer material;
- OTClient does not need to know Platform Identity internal IDs.

## Character/world selection

First release supports at least one world but the client data model must not hard-code singleton-world behavior.

After response:

1. user selects a character;
2. client resolves the character's `world_id` against returned world data;
3. client uses the authoritative `host`/`port` returned for that world;
4. client connects to legacy engine with the Game Session credential using the exact game protocol mapping defined by the retired legacy-engine contracts (see the ADR index).

Client must not rewrite a session for another world merely by changing host/port.

## Credential lifecycle in OTClient

| Credential | May enter OTClient? | Persistence | Destination |
|---|---|---|---|
| Oteryn password | No in new Oteryn flow | Never | Browser -> Identity only |
| MFA secret | No | Never | Identity only |
| MFA one-time code | No in new Oteryn client UI | Never | Browser -> Identity only |
| PKCE verifier | Yes | Memory only, pending flow | OAuth token endpoint |
| Authorization code | Yes | Memory only, single exchange | OAuth token endpoint |
| OAuth access token | Yes | Memory only, bootstrap | Platform ticket endpoint only |
| OAuth refresh token | Must not be used in first release | Never | None |
| Game Login Ticket | Yes | Memory only, immediate | Game Gateway only |
| Game Session credential | Yes | Memory for game connection; any optional bounded reconnect behavior requires explicit contract | legacy engine only |

## UI behavior

Target primary action:

```text
Sign in with Oteryn
```

New-flow UI must not show Oteryn email/password fields inside OTClient.

States:

- signed out;
- opening browser;
- waiting for browser sign-in;
- completing sign-in;
- loading characters/worlds;
- ready to select character;
- connecting to world;
- recoverable error;
- cancelled.

Error messages should be user-safe and not expose whether an internal ticket was expired/reused/revoked in a way that aids probing beyond what the user needs to retry.

## Cancellation and timeout

User cancellation or timeout must:

- close loopback listener;
- discard state/verifier/code/token/ticket material held by the active attempt;
- cancel outstanding requests where practical;
- return to signed-out UI;
- not silently invoke legacy password login.

## Retry policy

### Safe automatic retry

May be considered for transient idempotent GET/health operations only.

### Do not blindly retry

- authorization code exchange after ambiguous success;
- Game Login Ticket issuance after ambiguous response unless server idempotency is explicitly implemented;
- Game Login Ticket redeem after ambiguous response without the Gateway protocol defining deterministic recovery;
- Game Session creation without idempotency/recovery semantics.

On an expired/reused ticket, restart the browser authorization/ticket flow.

## Legacy compatibility

Current legacy behavior remains available only during migration and for explicitly supported non-Oteryn/custom-server compatibility.

The implementation must have a clear server/profile distinction between:

```text
Oteryn native auth
legacy password/HTTP/ProtocolLogin auth
```

Security decisions must be server-side/configuration-controlled; a user changing a local checkbox must not create an unintended public bypass on Oteryn production infrastructure.

After authoritative Oteryn cutover:

- official Oteryn profile defaults to native OAuth/Game Gateway flow;
- password fields/storage are not used for the Oteryn profile;
- legacy Oteryn password paths are disabled/fenced according to rollout policy.

## Required client-side security tests

- state generation has sufficient entropy;
- PKCE verifier/challenge uses S256 correctly;
- callback with wrong/missing state is rejected without token exchange;
- callback wrong path is rejected;
- callback listener binds loopback only;
- cancellation clears pending auth state;
- token/ticket/verifier not written to settings/logs;
- Oteryn new flow does not call password-based HTTP login;
- Oteryn new flow does not call password-based `ProtocolLogin`;
- no persisted Oteryn password is required for autologin/new flow;
- Gateway response with unknown world reference fails closed;
- client does not accept client-supplied/locally edited account ownership as authority;
- unsupported protocol version fails safely;
- expired ticket returns to reauthentication path rather than password fallback.

## Required integration tests

With exact Platform/Gateway versions:

- successful browser login without MFA;
- successful browser login with MFA;
- cancelled browser login;
- invalid state;
- invalid PKCE;
- expired code;
- ticket issuance failure;
- Gateway unavailable;
- expired/reused ticket;
- successful character/world list;
- multiworld-shaped response even when only one world is configured;
- successful legacy engine connection with selected Game Session contract.

## Version compatibility

Initial contract version:

```text
protocol_version = 1
```

Breaking changes require a new version.

During migration, the compatibility matrix must identify at least:

```text
OTClient build
Platform auth API version
Gateway API version
Legacy engine version
Game Session adapter version
```

## Handoff for OTClient implementation task

The future OTClient task must inspect current repository modules/services before selecting exact C++/Lua ownership.

It must not assume this contract dictates an internal module layout.

It must update OTClient's own cross-repository contract/task records and validate the exact legacy engine/Platform versions used in integration tests.
