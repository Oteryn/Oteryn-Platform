# Oteryn Game Authentication Rollout Plan

## Status

Architecture-foundation rollout plan — 2026-07-21

This plan moves Oteryn from the current parallel password/session authentication landscape to the target Identity-authoritative flow without removing legacy access before the replacement is proven end to end.

Production execution remains a separate explicitly authorized verification/deployment gate.

## Current baseline

### PROVEN

- Oteryn Platform owns supported greenfield user Identity and its reusable credentials.
- Platform web Identity supports password login, revocable web sessions, password recovery/change and TOTP MFA.
- A ready immutable Platform-owned binding maps each supported Identity to exactly one Canary `accounts.id`.
- Current OTClient login flow still reads and submits reusable account/password credentials and can persist encrypted password material.
- Repository-supported Canary/login-server configurations contain parallel authentication paths.
- Current upstream login-server authenticates reusable SHA-1 credentials and creates DB-backed `account_sessions`.
- Canary has native password authentication capability and a short-lived process-local one-time session-token mechanism in one mode.
- The authoritative Platform-originated game-login bridge is not implemented.

### UNKNOWN

- Exact production exposure/fencing of native Canary login ports and upstream login-server endpoints.
- Exact deployed login-server image digest/version.
- Final Game Session compatibility adapter and active-session revocation behavior.
- Production service identity, secret management, shared ticket storage and network topology.

## Rollout principles

1. **Add before remove.** Build and prove the new path before disabling legacy login.
2. **No false authority claim.** Identity is not globally authoritative while an independently validating password path remains externally reachable.
3. **No password migration dependency.** The new flow does not require Oteryn passwords to be copied into Canary-compatible password storage.
4. **No silent fallback.** A failed new-flow ticket/session operation never falls back automatically to a legacy password or long-lived session path.
5. **Cross-repository contract first.** OTClient and Canary/login-server work starts only from versioned contracts and separate authorized tasks.
6. **Feature-gated exposure.** Candidate flow is enabled in controlled environments/accounts before default rollout.
7. **Rollback preserves old path until cutover.** Before final legacy fencing, rollback means disabling the candidate path, not restoring a partially removed password path.
8. **After final cutover, rollback is architectural.** Re-enabling an independent password bypass requires an explicit security decision, not an automatic emergency toggle.
9. **Every phase is fail-closed.** Missing dependency/state prevents new login rather than weakening authentication.
10. **Evidence is exact-version.** E2E claims pin Platform, OTClient, Gateway, Canary and any login-server/adapter versions plus relevant configuration/network exposure.

## Phase 0 — Discovery and architecture foundation

### Goal

Establish durable current-state evidence, target architecture, threat model and cross-component contracts.

### Deliverables

- ADR 0009;
- threat model;
- sequence diagrams;
- this rollout plan;
- OTClient game-auth contract;
- Gateway↔Identity contract;
- Game Session↔Canary contract;
- World Registry contract.

### Writes

Only `blakinio/Oteryn-Platform` documentation/contracts.

### Exit gate

- current auth paths classified as PROVEN/DERIVED/UNKNOWN/CONFLICT;
- target trust boundaries decided;
- no unresolved contradiction inside the architecture documents;
- all external runtime changes remain contract-only;
- repository validation/CI green for the docs-only PR.

## Phase 1 — Platform architecture foundation

### Goal

Create Platform-owned domain/state required by later auth phases without exposing a usable new login path yet.

### Planned scope

- `game_auth_generation` or equivalent monotonic revocation generation on Identity-owned state;
- Game Login Ticket persistence model/lifecycle abstraction;
- World Registry persistence/model with one seeded/configured Oteryn world;
- Game Session domain interface, without yet claiming Canary compatibility;
- Gateway/Identity configuration contracts;
- protocol/API version constants;
- audit event taxonomy that records events without bearer secrets;
- cleanup/expiry job boundaries;
- migration rollback strategy.

### Security requirements

- reversible/backward-compatible migrations;
- ticket plaintext never persisted;
- unique/atomic lifecycle constraints designed before endpoint exposure;
- no generic Canary write grants;
- no new public auth behavior.

### Exit gate

- focused unit/integration tests green;
- migrations prove rollback in isolated test environment;
- revocation generation increments on selected security events or is explicitly deferred to the phase that wires those events before ticket issuance is enabled;
- no production-facing ticket issuance endpoint enabled.

## Phase 2 — Native OAuth Authorization Code + PKCE

### Goal

Make Oteryn Platform a standards-based authorization server for the first-party native OTClient without yet requiring Gateway/Canary integration.

### Planned scope

- add Laravel Passport compatible with current Laravel version;
- generate/manage OAuth signing keys outside source control for deployed environments;
- register fixed first-party OTClient as a public client;
- Authorization Code grant with PKCE S256;
- loopback redirect support with fixed path and ephemeral port;
- `state` handled by OTClient contract, with server-side normal OAuth protections;
- narrow `game:ticket` scope;
- bounded authorization-code/access-token lifetime;
- explicitly prevent long-lived refresh-token semantics for first-release game login;
- authorization UI integrated with existing Identity password/MFA flow;
- safe same-origin return/intended URL behavior.

### Compatibility requirement

This phase is additive. Legacy OTClient login continues unchanged.

### Exit gate

Tests prove:

- public client has no required client secret;
- valid PKCE succeeds;
- missing/wrong verifier fails;
- invalid redirect fails;
- loopback dynamic port with registered path succeeds;
- non-loopback/wrong-path redirect fails;
- expired/reused code fails;
- MFA-enabled Identity completes authorization through browser;
- disabled Identity cannot complete authorization;
- OAuth credentials are not logged;
- refresh-token policy matches ADR 0009.

## Phase 3 — Game Login Ticket

### Goal

Issue and atomically redeem a game-specific one-time credential independent of OAuth/web sessions.

### Planned public endpoint

```text
POST /api/v1/game-auth/tickets
Authorization: Bearer <short-lived OAuth credential with game:ticket>
```

### Planned private endpoint

```text
POST /internal/v1/game-auth/tickets/redeem
```

### Planned controls

- >=256-bit random opaque ticket;
- 60-second default TTL;
- hash/HMAC-derived server-side representation;
- exact Identity + ready Canary binding + audience + generation;
- atomic consume;
- service-authenticated redeem;
- rate limits;
- bounded audit events without raw ticket;
- cleanup/expiry.

### Exit gate

- no generic web session can mint a ticket without the required OAuth scope/client context;
- exactly one of concurrent redeems succeeds against the actual selected shared store;
- expiry/reuse/wrong audience fails;
- password/reset/selected security event invalidates pending ticket through generation;
- disabled/pending/conflict binding fails closed;
- ticket never appears in logs/test snapshots;
- outage of authoritative ticket storage fails closed.

## Phase 4 — Game Gateway MVP

### Goal

Create separately deployable `services/game-gateway` runtime.

### Planned MVP

```text
GET  /health
GET  /ready
GET  /version
POST /v1/login
```

`POST /v1/login`:

1. accepts one Game Login Ticket;
2. calls private Identity redeem;
3. receives exact authorized Canary account ID;
4. resolves authorized/login-enabled worlds;
5. loads listable characters through a narrow adapter;
6. creates Game Session through the selected compatibility adapter;
7. returns versioned character/world/session/routing response.

### Deployment properties

- standalone Go service/container;
- no direct access to Identity password data;
- no generic Platform database credentials;
- no generic Canary database credentials;
- horizontal process design;
- structured bounded logs;
- health/readiness separated;
- configuration from environment/secret manager, safe `.env.example` placeholders only where repository conventions allow.

### Exit gate

- service builds/tests independently;
- contract tests against Identity redeem fixtures pass;
- dependency outages fail closed;
- no secret/token logging;
- one-world registry path works;
- no claim of full game E2E until Game Session adapter and OTClient are implemented.

## Phase 5 — OTClient implementation

### Repository

`blakinio/otclient`

### Authorization

Requires a separately scoped OTClient task. Oteryn Platform task must not silently write this repository.

### Goal

Replace reusable password custody in the Oteryn-specific modern login path with system-browser native authorization.

### Planned scope

- `Sign in with Oteryn` UI;
- PKCE S256 verifier/challenge generation;
- high-entropy `state`;
- loopback listener bound to loopback only;
- system browser launch;
- authorization code callback handling;
- token exchange;
- Game Login Ticket request;
- Gateway login;
- character/world response parsing;
- routing to selected Canary endpoint;
- cancellation/timeout/retry UX;
- Oteryn-specific flow must not persist password or collect it in the client.

### Migration behavior

Legacy login implementation remains available behind an explicit compatibility/configuration path until cross-repository E2E is proven.

Do not silently convert all third-party/custom-server OTClient use cases to Oteryn OAuth.

### Exit gate

- valid browser login reaches Gateway response in controlled integration environment;
- state mismatch rejected locally;
- PKCE verifier never logged/persisted beyond required transient state;
- main Oteryn password never enters OTClient process in the new flow;
- cancel/timeout leaves no reusable pending state;
- compatibility matrix documents new vs legacy server combinations.

## Phase 6 — Game Session / Canary compatibility

### Goal

Select and prove the smallest safe bridge from Gateway authorization to Canary world entry.

### Candidate A — bounded `account_sessions` adapter

Preferred only if exact current Canary evidence proves it can satisfy the accepted first-release session contract without a Canary code change.

Required proof:

- exact table/session hash semantics;
- exact TTL semantics;
- dedicated least-privilege session persistence credential;
- account binding from redeemed ticket only;
- revocation/delete behavior;
- cleanup behavior;
- retry/idempotency after ambiguous commit;
- no password/sink-password dependency;
- exact game-world lookup behavior;
- acceptable replay window and documented risk.

### Candidate B — Canary code/protocol change

Required if Candidate A cannot meet replay/revocation/world-scoping/security requirements.

Any Canary write requires a separately authorized `blakinio/canary` task.

Possible direction:

- shared/authoritative one-time or bounded session credential recognized directly by Canary;
- explicit world/audience binding;
- deterministic consume/revocation semantics.

### Upstream login-server role

The target Oteryn architecture does not require upstream login-server to remain the reusable password authority.

It may be:

- bypassed by Oteryn Game Gateway;
- replaced by Oteryn-specific compatibility behavior;
- retained only for unrelated/legacy compatibility behind a fenced boundary.

No writes to `opentibiabr/login-server` are permitted from Oteryn Platform work.

### Exit gate

- exact adapter contract proven against current Canary SHA;
- Gateway can create a usable Game Session without user/sink password;
- wrong account/character denied;
- expiry/revocation behavior measured;
- ambiguous commit/retry behavior deterministic;
- no unapproved DB privileges;
- any required Canary PR is separately reviewed/green/merged in correct rollout order.

## Phase 7 — Cross-repository staging E2E

### Goal

Prove the new path across exact versions before legacy removal.

### Required pinned evidence

- Oteryn Platform SHA;
- Game Gateway SHA/image digest;
- OTClient SHA/build identifier;
- Canary SHA/image digest;
- login-server SHA/image digest if still in any path;
- database schema migration state;
- ticket/session storage version/config;
- network exposure/port configuration relevant to bypass testing.

### Required success path

```text
OTClient
-> Browser
-> Oteryn Identity
-> Authorization Code + PKCE
-> OTClient
-> Game Login Ticket
-> Game Gateway
-> Atomic redeem
-> exact Canary account binding
-> World Registry
-> Character List
-> Game Session
-> Canary
-> Player enters game
```

### Required negative matrix

At minimum:

- cancelled login;
- wrong state;
- invalid PKCE;
- expired/reused authorization code;
- OAuth token without scope;
- expired/reused/wrong-audience ticket;
- concurrent ticket replay;
- disabled Identity;
- generation changed after ticket issue;
- missing/pending/conflict Canary binding;
- missing/foreign/deleted character;
- unauthorized/disabled world;
- Gateway unavailable;
- Identity unavailable;
- ticket store unavailable;
- session store unavailable;
- Canary unavailable;
- sensitive log regression;
- direct native Canary login attempt;
- upstream login-server password attempt;
- legacy protocol attempt;
- session expiry/revocation behavior.

### Exit gate

All required E2E is green on exact staging versions with zero unresolved security blocker.

This proves only staging.

## Phase 8 — Legacy migration and cutover

### Stage 8A — Candidate opt-in

- new Oteryn auth enabled for controlled testers;
- legacy remains available;
- measure success/failure/latency without logging credentials;
- no global-authority claim.

Rollback: disable candidate feature flag/routing and use unchanged legacy path.

### Stage 8B — New flow default for official Oteryn client

- official client defaults to `Sign in with Oteryn`;
- legacy path requires explicit compatibility configuration;
- monitor candidate path;
- still no global-authority claim while alternate public password path exists.

Rollback: return official client routing/config to legacy while investigating. No database credential migration is required.

### Stage 8C — Fence direct native Canary password login

Actions depend on proven production topology:

- stop publishing native login port publicly where not required;
- make process-level login protocol enablement behavior unambiguous if used;
- firewall/restrict internal-only paths;
- verify from an external test point that bypass is denied.

Rollback must not automatically expose a weaker public password path. Emergency reversal requires explicit security authorization.

### Stage 8D — Fence/replace upstream login-server password authentication

For Oteryn-supported modern clients:

- remove the endpoint from the public official-client path;
- or restrict it to explicitly accepted legacy/internal compatibility;
- ensure the random Canary sink credential cannot authenticate a user.

### Stage 8E — Legacy protocol decision

Choose explicitly:

1. disabled for production Oteryn secure surface; or
2. retained as a separately documented lower-security compatibility tier with isolated exposure and no claim that MFA/security policy applies globally to it.

Recommended default: disable unsupported direct-password legacy protocol for the production Oteryn secure surface.

### Stage 8F — Authoritative cutover declaration

Only after external bypass tests prove all reachable auth paths are governed/fenced:

```text
Oteryn Identity = authoritative reusable credential authority for supported game login
```

Update:

- `AUTH_GAME_LOGIN_CONTRACT.md` current-state section;
- security/readiness documentation;
- production verification checklist;
- compatibility documentation.

## Phase 9 — Multiworld

After single-world stability:

- add multiple World Registry records;
- account/world access policy;
- maintenance/login-enabled state;
- region;
- world-scoped sessions;
- dynamic host/port routing;
- world-specific E2E.

Do not introduce channel allocation in this phase unless separately designed.

## Phase 10 — Scale and multiregion

Potential work:

- multiple Gateway instances;
- load balancer;
- shared atomic ticket/session state;
- multi-region login ingress;
- region-aware routing;
- queues/capacity;
- tournament/preview worlds;
- future Channel Allocator.

Every scale step must preserve atomic ticket consume and deterministic session ownership without sticky-process correctness assumptions unless sticky routing is an explicit proven contract.

## Rollout dependency graph

```text
Phase 0 Architecture/Contracts
        |
        v
Phase 1 Platform Domain Foundation
        |
        +-------------------+
        v                   v
Phase 2 OAuth/PKCE      World Registry foundation
        |
        v
Phase 3 Game Ticket
        |
        v
Phase 4 Game Gateway
        |
        +------------------------+
        v                        v
Phase 5 OTClient            Phase 6 Canary Session Adapter
        \                        /
         \                      /
          v                    v
             Phase 7 Staging E2E
                    |
                    v
             Phase 8 Legacy Cutover
                    |
                    v
             Production Verification
                    |
                    v
             Phase 9 Multiworld
                    |
                    v
             Phase 10 Scale
```

Phases 5 and 6 may proceed in parallel only after their shared contracts are frozen enough to avoid incompatible implementations.

## Data migration policy

The target auth rollout intentionally avoids migrating user passwords into Canary.

Required data changes are Platform/game-auth metadata only, such as:

- OAuth server/client state managed by the selected framework;
- Identity game-auth revocation generation;
- hashed Game Login Ticket state;
- World Registry;
- selected Game Session compatibility state.

Any migration that mutates existing Canary `accounts.password` is outside this rollout plan unless a new explicit contract supersedes the no-password-to-Canary target.

## Feature flags and configuration

Recommended independent controls:

```text
GAME_AUTH_OAUTH_ENABLED
GAME_AUTH_TICKET_ISSUANCE_ENABLED
GAME_GATEWAY_LOGIN_ENABLED
OTERYN_NATIVE_AUTH_ENABLED        # client/build-side equivalent
LEGACY_LOGIN_COMPATIBILITY        # migration only; not a permanent safety fallback
```

Names are illustrative until implementation follows repository configuration conventions.

Rules:

- flags default fail closed for incomplete new components;
- enabling ticket issuance before atomic redeem is production-ready is forbidden;
- disabling a broken new path before final cutover may route users back to legacy only while legacy is still intentionally supported;
- after authoritative cutover, `LEGACY_LOGIN_COMPATIBILITY` must not become an unreviewed emergency password bypass.

## Observability rollout

Measure without credentials:

- authorization attempts/success/failure category;
- ticket issue success/failure;
- redeem success/expired/reused/revoked category;
- session-create success/failure category;
- Gateway dependency availability;
- end-to-end login duration using request/correlation IDs that are not bearer credentials;
- world routing/availability category.

Never record:

- password;
- authorization code;
- PKCE verifier;
- OAuth access/refresh token;
- Game Login Ticket;
- Game Session secret;
- MFA secret/recovery code;
- service credential.

## Rollback matrix

| Rollout point | Safe rollback |
|---|---|
| Phase 1 migrations before public use | Revert reversible Platform migrations after confirming no dependent runtime is enabled |
| OAuth candidate | Disable native OAuth candidate; legacy remains unchanged |
| Ticket candidate | Disable ticket issuance/redeem candidate; no user password migration to undo |
| Gateway candidate | Remove Gateway from official client routing; legacy remains unchanged |
| OTClient candidate | Ship/configure previous official legacy path while candidate remains non-authoritative |
| Game Session adapter before cutover | Disable candidate adapter/Gateway path; legacy remains unchanged |
| After native login port fencing | Do not auto-reopen port; investigate/fix new path or use explicitly approved controlled rollback |
| After authoritative cutover | Roll forward preferred; any password-path restoration is a security architecture change requiring explicit approval |

## Production go-live gate

The new game-auth architecture may be `STAGING_PROVEN` before production, but production remains `UNKNOWN` until directly verified.

Required final production evidence includes:

- exact deployed component SHAs/image digests;
- OAuth keys/secret injection mechanism without exposing secret values;
- TLS and service-authentication topology;
- externally observed legacy/native bypass fencing;
- database/Redis/shared-store permissions and availability;
- centralized log redaction behavior;
- live health/readiness/monitoring;
- critical successful login smoke;
- critical replay/revocation/bypass negative smoke where safe;
- rollback procedure and owner/on-call readiness.

No repository or staging result substitutes for this production verification gate.

## Program completion definition

The base game-authentication program is complete when the supported official Oteryn flow proves:

```text
OTClient
-> Oteryn Identity
-> Authorization Code + PKCE
-> Game Login Ticket
-> Oteryn Game Gateway
-> Game Session
-> Character List / World routing
-> Canary
-> Player enters game
```

and:

- the user's main password never reaches Game Gateway or Canary;
- Game Login Ticket concurrent replay has exactly one winner;
- security-state revocation is deterministic;
- Gateway is separately deployable;
- single-world uses World Registry;
- contracts are ready for multiworld;
- all supported public password bypasses are fenced before the global-authority claim;
- staging E2E is green;
- production verification is separately completed.
