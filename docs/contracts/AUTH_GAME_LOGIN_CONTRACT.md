# Oteryn Web-to-Game Authentication Contract

## Status

`PARTIALLY PROVEN — LEGACY PATHS MAPPED / OTERYN GATEWAY PATH DELIVERED / PRODUCTION CUTOVER UNKNOWN`

This document separates:

1. the **current proven authentication/session behavior** supported by the inspected source;
2. unresolved deployment/product facts that remain `UNKNOWN`;
3. source/documentation disagreements marked `CONFLICT`;
4. a **recommended target contract** that is design direction, not implemented behavior.

Do not implement shared credential migration, global game-login MFA/email enforcement or a new game-login token path until a separately approved implementation task satisfies the rollout gates in this document. Platform-local web Identity controls may be implemented without changing that global-auth boundary, but they must not be represented as Canary/login-server enforcement.

## Current Oteryn path overlay

This section is the current repository-state overlay. It does not erase the legacy-path discovery below.

### Delivered on `main` — PROVEN

```text
OTClient
  -> Oteryn Identity / OAuth + PKCE
  -> short-lived single-use Game Login Ticket
  -> separately deployable Oteryn Game Gateway
  -> private Platform ticket redeem
  -> private Platform login context
       -> Platform World Registry
       -> account-scoped active character projection
  -> Game Session issuer contract version 1
  -> opaque short-lived Game Session credential
  -> Canary world-entry admission
```

The Platform/Gateway producer boundary was delivered through PR #121 and PR #122; PR #122 merged as `8006534108d835474dadd208b0ec934e4a12528b`. The operation-specific world-entry authority is `GAME_SESSION_CANARY_CONTRACT.md`, which records the bounded Canary and OTClient consumer evidence for legacy-compatible Game Session contract version 1.

The delivered path does not send the user's Oteryn password across the Gateway-to-Canary boundary. Gateway authorization is derived from authoritative ticket redeem and the exact ready Platform-to-Canary account binding; client-supplied account or character ownership data cannot establish authority.

### Still retained or unresolved

The legacy native Canary, external login-server, reusable `account_sessions`, old-protocol direct-password and source-level alternate-path findings below remain relevant until exact deployment evidence proves that each unsupported path is disabled, isolated or intentionally retained.

```yaml
repository_gateway_path: PROVEN
legacy_path_source_capability: PROVEN
native_v2_platform_producer: DISABLED_BY_DEFAULT
native_v2_cross_repository_consumer: NOT_PROVEN_BY_THIS_REPOSITORY
production_deployment_identity: UNKNOWN
production_private_ingress: UNKNOWN
production_service_auth_rotation: UNKNOWN
global_legacy_path_retirement: UNKNOWN
PRODUCTION_PROVEN: false
```

No repository document may infer global MFA, email-verification, password-migration or session-revocation enforcement merely from the delivered Gateway path while an alternate deployed password path may remain reachable.

## Evidence baseline

### Oteryn Platform

- Repository: `blakinio/Oteryn-Platform`
- Base revision at discovery start: `f968681732ec3e0688ff29426108b49dce79af16`
- Current state: Platform-owned web Identity is implemented for registration, framework-hashed credentials, login/logout, revocable web sessions, password recovery/change and opt-in web MFA. It is authoritative for the delivered Oteryn Game Gateway path, but it is not yet the sole global game-login credential authority while native Canary, external login-server and other legacy password/session paths may remain reachable.

### Canary

- Repository: `blakinio/canary`
- Pinned revision: `096f6445b29f69a62f03d391a2c02c4dcee74feb`
- Access mode: read-only

### External login-server source reference

- Repository: `opentibiabr/login-server`
- Pinned current-main source revision inspected: `2612930de4d97123a397f8f2cd0d5f784094af40`
- Access mode: read-only

### Deployment-version limitation — UNKNOWN

The Canary Docker quickstart uses `opentibiabr/login-server:latest`, not an immutable image digest or source SHA. Therefore the pinned upstream login-server source is valid compatibility evidence for current upstream source, but **is not proof that any deployed `latest` image or production environment runs exactly that revision**.

No production deployment topology, firewall policy, container digest or live configuration was provided during this task.

## Evidence state legend

- `PROVEN` — directly supported by pinned source/configuration.
- `DERIVED` — conclusion that follows from proven facts.
- `UNKNOWN` — not established by inspected evidence; must not be guessed.
- `CONFLICT` — authoritative evidence disagrees or supported paths have incompatible semantics.

## Executive current-state summary

### PROVEN

There is **not one single current authentication path** across all repository-supported configurations.

The inspected source supports/coexists with:

1. native Canary `ProtocolLogin` password authentication;
2. external HTTP/gRPC `opentibiabr/login-server` password authentication and DB-backed `account_sessions` issuance;
3. modern Canary `LoginSessionManager` short-lived, single-use in-memory tokens when `authType=session`;
4. DB-backed `account_sessions` fallback when the one-time token is not accepted;
5. modern `authType=password` game login that carries an `accountDescriptor + "\n" + password` session-key string and revalidates the password;
6. old-protocol clients that send account descriptor/password directly on both login and game connections;
7. a separate livestream login path that is not normal account authentication;
8. `cluster_sessions`, which is an online/concurrency lease and **not** an authentication credential.

### DERIVED

A security rule implemented only in Oteryn Platform can currently be bypassed by another exposed credential-validation path unless every alternative path is disabled, network-restricted or changed to consult the same authoritative policy.

This is especially relevant to any future claim of global enforcement for:

- MFA;
- email verification gates;
- password migration;
- session revocation;
- disabled-account policy;
- authentication rate limiting.

## Current supported topology

### Native Canary login path — PROVEN

Canary `Game::start` registers:

- `ProtocolGame` on the modern game port;
- optional legacy game protocol ports;
- `ProtocolLogin` on `loginProtocolPort`;
- `ProtocolStatus` on the status port.

Therefore Canary itself contains and starts a login protocol implementation.

Native character-list flow:

```text
Game client
   |
   v
Canary ProtocolLogin
   |
   +--> account lookup
   +--> password verification
   +--> character list
   +--> session-key / one-time-token field when protocol supports it
```

### Docker quickstart external login path — PROVEN

The current Canary repository Docker quickstart simultaneously starts:

- MariaDB;
- Canary runtime;
- MyAAC website/admin;
- `opentibiabr/login-server` as HTTP/gRPC client login webservice.

The quickstart documentation says MyAAC `login.php` is removed and clients should use the external login-server webservice.

The compose file nevertheless also publishes Canary's native `loginProtocolPort` in addition to the game ports.

External flow:

```text
Modern client HTTP login
   |
   v
opentibiabr/login-server
   |
   +--> same MySQL accounts table
   +--> password verification
   +--> account_sessions INSERT
   +--> character/world response + raw session key
   |
   v
Canary ProtocolGame
   |
   +--> DB session lookup / validation
   +--> character ownership/deletion check
   +--> account-ban/world-entry gates
```

### CONFLICT / bypass risk

The repository-supported quickstart intends the external login-server to be the client login webservice, while the same compose topology publishes the native Canary login port and Canary registers `ProtocolLogin` directly.

Unless production networking prevents direct access, this is an alternate password-validation path.

`config.lua.dist` also documents `loginProtocolEnabled` as a process-local confirmation that a process may act as a login gateway, but the inspected `Game::start` registration point adds `ProtocolLogin` without consulting that flag.

The discovery does not prove whether the flag is enforced elsewhere in a production wrapper/network layer, so production bypass exposure remains `UNKNOWN`; source-level alternate-path capability is `PROVEN`.

## Account descriptor rules

### Native Canary — PROVEN

`AccountRepositoryDB::loadByEmailOrName` uses:

- account `name` for old-protocol compatibility;
- account `email` for modern protocol behavior.

### External login-server — PROVEN

Current upstream login-server authenticates with:

```sql
WHERE (email = ? OR name = ?) AND password = ?
```

Therefore its submitted `Email` field accepts either account email or account name.

### DERIVED

The supported descriptor contract is inconsistent across paths. A future authoritative Identity contract must explicitly define whether canonical login uses email, account name, or both, and normalize ambiguity before rollout.

## Password verification and hash compatibility

### Native Canary accepted verification paths — PROVEN

`Account::authenticatePassword`:

1. tries Canary's custom Argon2id verifier;
2. if that fails, compares `SHA1(plaintext password)` with the stored `accounts.password` value.

The default distributed config includes:

- `passwordType = "sha1"`;
- Argon memory/time/parallelism parameters;
- a comment that the Argon settings must match between website and server.

### Canary custom Argon2 representation — PROVEN

The pinned Canary Argon verifier does **not** delegate stored-string parsing to the standard Argon2 PHC verifier.

It:

- extracts Base64-looking salt/hash components from the stored string with a custom regular expression;
- decodes them;
- calls `argon2id_hash_raw` using configured memory/time/parallelism;
- compares the computed raw bytes with the decoded stored hash.

The inspected Canary code provides verification behavior; this discovery did not prove a corresponding canonical password-hash writer/encoder used by the current Oteryn website.

### External login-server password verification — PROVEN

Current upstream `opentibiabr/login-server` computes SHA-1 of the submitted plaintext password and queries `accounts.password` for an exact SHA-1 match.

It does not use Canary's custom Argon2 verifier in the inspected source.

### Compatibility conclusion — CONFLICT

Current native Canary can verify SHA-1 and its custom Argon2 representation, while current upstream external login-server verifies SHA-1 only.

Therefore an account converted only to the Canary custom Argon2 representation would not be authenticated by the inspected external login-server source.

### Laravel Argon2id compatibility — UNKNOWN / BLOCKED

A standard Laravel/PHP Argon2id PHC string has **not** been proven compatible with Canary's custom parser/verification format.

Oteryn Platform must not write Laravel's default Argon2id representation directly into the shared `accounts.password` field until a deterministic compatibility test or a deliberate authentication-authority redesign removes Canary/login-server direct password verification.

### Password migration gate

Credential migration is blocked until all of the following are true:

1. exact deployed login paths are inventoried;
2. direct legacy/native password paths are disabled, isolated or updated;
3. one authoritative password verifier is selected;
4. legacy SHA-1 verification and upgrade behavior are specified;
5. stored-format compatibility is tested with exact components/versions;
6. session/token revocation on credential change is implemented and tested;
7. rollback order is defined.

## Security finding: credential hash logging

### PROVEN

On failed native Canary password authentication, the pinned `Account::authenticatePassword` implementation logs the stored credential value returned by `getPassword()`.

The stored value may be a password hash and is therefore credential-sensitive data.

### Security requirement

This must be removed/redacted in a separately authorized Canary security fix before a production security-ready claim.

No real credential value was copied into this contract.

## Character-list authentication paths

### Native Canary `ProtocolLogin` — PROVEN

The native login connection:

1. resolves protocol profile/layout;
2. RSA-decrypts the first message;
3. enables XTEA;
4. checks server startup/maintenance state;
5. checks IP ban;
6. reads account descriptor/password;
7. loads account;
8. verifies password;
9. returns character list and login/session material.

### External login-server — PROVEN

The current upstream external login-server:

1. accepts HTTP/gRPC login request;
2. authenticates by SHA-1 against `(email OR name)`;
3. loads players for the account;
4. creates a DB-backed `account_sessions` entry;
5. returns character/world data and the raw session key.

### Character-list deletion inconsistency — CONFLICT

Native Canary account player loading excludes players with `deletion != 0`.

The inspected external login-server `LoadPlayers` query selects players by `account_id` without a `deletion` filter.

Canary game-world authentication still rejects deleted/unavailable characters before world entry, so this does not bypass the final ownership/deletion gate, but the external character list can expose entries that native Canary would omit.

## Game-world authentication

### Common ownership/deletion gate — PROVEN

Regardless of whether the account was authenticated by:

- password;
- DB-backed session;
- redeemed in-memory one-time token;

Canary game-world authentication still verifies:

- selected character belongs to the authenticated account;
- account loads successfully;
- character is not in the excluded/deleted state.

A pre-authenticated one-time token does not bypass these checks.

## Mode A — modern `authType=password`

### PROVEN

Default distributed `authType` is `password`.

For modern layouts that carry a session-key string, native Canary character-list response uses the ad-hoc value:

```text
accountDescriptor + "\n" + password
```

On game connection, Canary splits the value and revalidates the real password.

### Security consequence — DERIVED

The password remains the game-login credential and is sent through both authentication stages in an encoded/encrypted protocol context rather than being replaced by a server-side session authorization.

This mode cannot support a Platform-only MFA/email-verification policy unless Canary password login is also gated by the authoritative policy.

## Mode B — modern `authType=session` + LoginSessionManager token

### Token issuance — PROVEN

For a modern non-old protocol profile with a session-key field and `authType=session`, native Canary `ProtocolLogin` may replace the ad-hoc password string with a `LoginSessionManager` token.

Token properties:

- 32 random bytes / 256 bits generated by CSPRNG;
- hex encoded on the wire;
- only SHA-256 hash stored in memory;
- bound to account ID;
- bound to the complete allowed character-name set at issuance;
- bound to protocol profile ID;
- default TTL 60 seconds;
- default maximum 4096 active entries;
- oldest entries evicted when capacity is exceeded;
- not bound to client IP.

### Consumption — PROVEN

On game connection:

- token hash lookup uses constant-time comparison;
- the matched entry is removed **before** character/profile validation;
- wrong character/profile therefore burns the token;
- concurrent redemption cannot both succeed;
- successful redemption yields the pre-authenticated account ID.

### Revocation characteristics — PROVEN / DERIVED

The manager interface exposes issuance, consumption, expiry cleanup and active count; no account-wide explicit revocation operation is proven.

A pending token can cease to work through:

- successful or failed matching consumption;
- TTL expiry;
- capacity eviction;
- process restart/loss of in-memory state.

Password change/reset does not inherently invalidate a still-pending token in the inspected implementation.

### Multi-process limitation — DERIVED

The token store is process-local in-memory state. A token issued by one Canary process cannot be assumed redeemable by another process without sticky routing/shared state. The current multichannel target therefore requires explicit routing or a shared atomic token store if this primitive becomes the authoritative cross-process game-login token.

## Mode C — DB-backed `account_sessions`

### External login-server issuance — PROVEN

Current upstream login-server:

- generates 32 random bytes;
- hex-encodes the raw session key returned to the client;
- stores SHA-256(session key) in `account_sessions.id`;
- stores account ID;
- sets expiry to current time + 24 hours;
- fails the login response if session persistence fails.

### Canary lookup — PROVEN

Canary hashes the presented session key with:

- SHA-256;
- legacy SHA-1 fallback;

and joins `account_sessions.account_id` to `accounts.id`.

Account session authentication then checks only whether the stored expiry has passed.

### Replay semantics — PROVEN

The DB session row is not consumed on successful authentication.

Therefore the raw session key is replayable until:

- expiry; or
- external deletion/revocation of the DB row.

### Revocation — UNKNOWN / INCOMPLETE

Current upstream login-server source inspection proves session creation but did not find an account-session deletion/revocation flow.

Canary loads and validates DB sessions but the inspected game authentication path does not delete them after use.

`resetSessionsOnStartup` is present and loaded from configuration, but this discovery did not prove its runtime effect at the pinned revision.

No password-change/reset coupling to `account_sessions` was proven.

### Security consequence — DERIVED

A password reset/change cannot be claimed to revoke game login unless the implementation explicitly deletes or invalidates all relevant DB sessions and pending game-login tokens.

## Mode D — old-protocol direct password

### PROVEN

Old `AccountPassword` protocol layouts have no opaque session-key field.

They send account descriptor/password directly on:

- login connection;
- game connection.

The distributed config defaults `allowOldProtocol=true` in the pinned Oteryn Canary fork.

### Target-policy consequence — DERIVED

A legacy direct-password path cannot transparently carry the same opaque game-login token semantics as modern clients.

If MFA/email verification is intended to gate all game login, production must choose one of:

- disable unsupported legacy protocols;
- put legacy login behind a separately controlled trusted gateway that enforces the same policy;
- accept and explicitly document a weaker legacy security tier.

A production security-ready claim must not silently leave the legacy direct-password path as a bypass.

## ProtocolSessionHintStore

### PROVEN scope

Canary also stores reusable protocol/session hints associated with IP/profile/session-key/character data to resolve protocol compatibility across connections.

### DERIVED

These hints assist protocol-layout selection and are not sufficient account authentication proof. They must not be treated as an Identity session or a substitute for password/token verification.

## Account bans, name locks and disabled state

### Account bans — PROVEN

Account ban enforcement occurs during `ProtocolGame::login` before world placement.

Therefore:

- a banned account may pass password/session authentication;
- a character list/session may be issued;
- final world entry is still rejected by Canary's account-ban gate.

### IP bans — PROVEN

IP bans are checked on:

- native Canary `ProtocolLogin`;
- Canary `ProtocolGame` connection.

The external login-server source inspected in this task did not prove equivalent IP/account-ban enforcement before session issuance.

### Name locks — PROVEN

Character namelocks are checked during Canary world login and block world entry.

### Disabled-account model — UNKNOWN

No separate universally enforced account-disabled field/policy was proven by the inspected current auth paths.

## MFA and email verification

### Current path enforcement — NOT PROVEN

No MFA/TOTP or email-verification gate is present in the inspected:

- native Canary `ProtocolLogin` password flow;
- Canary `ProtocolGame` password/session flow;
- current upstream external login-server authentication flow.

Oteryn Platform now has a Platform-owned web Identity implementation with password recovery/change, revocable web sessions and opt-in MFA. These controls govern Platform web authentication only and do not change the inspected Canary or external login-server paths listed above.

### Platform policy status — PARTIALLY PROVEN / GLOBAL GAME POLICY UNKNOWN

Oteryn Platform web Identity now implements opt-in MFA for identities that enroll. Current Phase 3 product policy does not require email verification, so no Platform email-verification gate is enabled. No evidence proves that Platform MFA or any future email-verification policy gates every native Canary or external login-server authentication path.

### Security consequence — DERIVED

Platform-only MFA or email verification would not become a global game-login requirement while native Canary password/session paths or an external login-server can authenticate without consulting that state.

## Password change and password reset

### Current global end-to-end behavior — PARTIALLY PROVEN / GAME REVOCATION UNKNOWN

Oteryn Platform web Identity implements password change and password reset and revokes Platform web sessions according to its `web_session_generation` policy. The inspected Canary and upstream login-server paths remain separate reusable-credential/session authorities.

No evidence proves that Platform password change/reset currently:

- deletes Canary/login-server `account_sessions`;
- invalidates pending Canary LoginSessionManager tokens;
- disconnects active game sessions;
- enforces Platform email-verification or MFA policy across every game-login path.

### proven non-coupling

The LoginSessionManager token contains account/character/profile authorization state and does not re-check the password after successful token redemption.

DB `account_sessions` authenticate by session-key hash and expiry, not by re-checking the current account password.

### DERIVED

Without explicit revocation, changing the password alone is insufficient to invalidate already issued game-login credentials.

## Logout

### Current authentication-credential revocation — NOT PROVEN

The inspected Canary `ProtocolGame::release` clears reusable protocol session hints and disconnects/releases the game protocol/player association.

No evidence in the inspected paths proves that normal game logout deletes DB `account_sessions`.

A LoginSessionManager token is already single-use by the time the game session starts, so there is normally no consumed token left to revoke; other pending tokens for the same account are not proven to be invalidated.

## Active game sessions

### PROVEN

After successful game authentication, Canary uses a separate cluster-session/lease mechanism in multichannel mode to prevent split-brain/concurrent character presence.

This is not the login credential itself.

### Revocation — UNKNOWN

Password change/reset is not proven to disconnect an already active player session.

Ban checks are proven at login entry; automatic immediate disconnection of an already-online player solely because a new ban row appears is not established by this contract.

## Outage and failure behavior

### Canary database unavailable — PROVEN

Canary startup fails if it cannot connect to its database.

### External login-server database unavailable — PROVEN

Current upstream login-server classifies DB connectivity failures and does not issue a successful login response.

### External session persistence unavailable — PROVEN

If `account_sessions` persistence fails, login-server fails session creation rather than returning a usable session key.

### Native one-time token CSPRNG failure — PROVEN

LoginSessionManager token issuance fails closed if CSPRNG seeding/generation fails.

### Multichannel cluster dependency failure — PROVEN for world entry

Canary cluster runtime can fail closed for new sessions when it cannot safely verify cluster session state.

### Platform outage — PATH-DEPENDENT

For the delivered Oteryn Gateway path, Oteryn Platform Identity is in the game-authentication critical path: ticket issuance, private ticket redeem and private login-context dependencies fail closed when Platform is unavailable.

Native Canary, external login-server and other retained legacy password/session paths may still authenticate independently when deployed and reachable. Therefore a Platform outage is not yet a proven global game-login shutdown condition, and exact production exposure or isolation of those alternate paths remains `UNKNOWN`.

### DERIVED sole-authority implication

If Platform Identity becomes the sole global credential authority, its availability becomes part of every supported login critical path unless the architecture uses narrowly scoped pre-issued authorizations with explicit expiry/failure semantics.

## Direct alternate/bypass paths

### PROVEN path inventory

| Path | Password checked by | Game credential | Current bypass concern |
|---|---|---|---|
| Native modern Canary, `authType=password` | Canary | embedded descriptor/password string | Bypasses Platform-only policy |
| Native modern Canary, `authType=session` | Canary at character-list stage | 60s single-use token, then DB-session fallback | DB fallback remains alternate credential |
| External HTTP/gRPC login-server | login-server SHA-1 verifier | 24h DB `account_sessions` token | Bypasses Platform unless integrated |
| Old protocol | Canary | password on both connections | Cannot carry modern opaque-token policy |
| Livestream path | special non-account flow | special account/password marker | Separate capability; must remain isolated from normal account auth |

### Production-readiness rule

A future policy cannot be called globally enforced until every externally reachable row in this table is either:

- removed/disabled;
- network-restricted to trusted internal callers;
- changed to validate the same authoritative Identity policy;
- explicitly accepted as a documented lower-security compatibility exception.

## Current security findings

### Finding A — credential hash logging

`PROVEN / HIGH PRIORITY`

Failed native Canary password authentication logs the stored credential value. Remove/redact it before production security readiness.

### Finding B — parallel authentication paths

`PROVEN / HIGH PRIORITY ARCHITECTURAL RISK`

Native Canary login and external login-server can coexist in repository-supported topology. Stronger policy added to only one path is bypassable through another reachable path.

### Finding C — SHA-1 compatibility dependency

`PROVEN / MIGRATION BLOCKER`

Current upstream external login-server authenticates SHA-1 only. Migrating shared credentials without updating/removing that path would break login.

### Finding D — stale LoginSessionManager documentation

`CONFLICT`

`docs/systems/login-session-manager.md` says the manager is built/tested but not wired. Current pinned `ProtocolLogin` and `ProtocolGame` source do wire issuance and consumption.

Source behavior is authoritative for this contract.

### Finding E — external character-list deletion filtering

`CONFLICT`

Native Canary excludes deleted players from account player lists; current upstream external login-server does not filter `players.deletion` in its list query.

## Current revocation matrix

| Event | Platform web sessions | Pending 60s Canary one-time tokens | DB `account_sessions` | Active game session | Current evidence |
|---|---|---|---|---|---|
| Password change | Revoked by Platform `web_session_generation` policy | Not proven revoked | Not proven revoked | Not proven disconnected | PROVEN for Platform web / game revocation UNKNOWN |
| Password reset | Revoked by Platform `web_session_generation` policy | Not proven revoked | Not proven revoked | Not proven disconnected | PROVEN for Platform web / game revocation UNKNOWN |
| MFA reset | Revoked by Platform `web_session_generation` policy | No current game MFA binding | No current game MFA binding | No current game MFA binding | PROVEN for Platform web / DERIVED for game non-binding |
| Account banned | N/A current Platform | Not proven revoked | Not proven revoked | New world entry rejected; existing-session effect unknown | PARTIALLY PROVEN |
| Email changed | NOT_IMPLEMENTED/UNKNOWN | Token binds account ID, not email | Session binds account ID | No proven effect | PARTIALLY PROVEN |
| Normal game logout | N/A | Consumed login token already gone; other tokens unaffected | Not proven deleted | Connection/player session ends | PARTIALLY PROVEN |
| Canary process restart | N/A | In-memory tokens lost | DB sessions persist unless separate reset behavior occurs | Process sessions end | PARTIALLY PROVEN |

## Recommended target contract

Everything in this section is `DERIVED DESIGN DIRECTION`, not current implementation.

### Principle 1 — one authoritative Identity policy

Oteryn Platform Identity should become the only component that verifies reusable account credentials and applies:

- password policy;
- legacy-hash migration;
- email verification policy;
- MFA policy;
- disabled-account policy;
- recovery/reset policy;
- web-session revocation.

Canary and login-server should not independently verify reusable passwords once migration completes.

### Principle 2 — separate reusable credentials from game-login authorization

Recommended flow for modern supported clients:

```text
Browser / game client login request
        |
        v
Oteryn Platform Identity
(password + verification + MFA + account policy)
        |
        v
short-lived single-use game-login authorization
        |
        v
login compatibility endpoint / adapter
        |
        v
Canary ProtocolGame
(atomic consume + account/character/ban validation)
        |
        v
active game / cluster session
```

The game-login authorization is not a web session and not a reusable password substitute.

### Principle 3 — target game token properties

A production game-login token should be:

- generated with CSPRNG;
- opaque to the client;
- stored only as a cryptographic hash where persistent/shared storage is required;
- short lived, recommended order of 60–120 seconds;
- single-use with atomic consume;
- bound to account ID;
- bound to selected character or an explicit allowed-character set;
- bound to protocol/audience;
- optionally bound to intended channel/world when routing requires it;
- revocable through account security state;
- safe across multi-process login/game routing through shared atomic storage or guaranteed sticky routing;
- never logged in plaintext.

### Principle 4 — remove long-lived DB session fallback from authoritative game login

The current 24-hour replayable `account_sessions` credential is too broad to serve as the preferred one-time game-login authorization.

A target architecture should either:

- repurpose it only for a clearly defined launcher/web session role with explicit revocation; or
- replace it for game entry with the short-lived single-use authorization described above.

A failed one-time token should not silently fall back to a longer-lived credential in the production-authoritative path.

### Principle 5 — direct Canary login must not bypass Identity

Before enforcing MFA/email verification globally:

- native Canary `ProtocolLogin` must be disabled for public modern auth, or restricted behind trusted network boundaries and integrated with authoritative Identity;
- `loginProtocolEnabled`/service registration behavior must be made unambiguous if used as the control;
- published firewall/container ports must match the intended trust boundary;
- direct password mode must be disabled for production modern clients once token migration is complete.

### Principle 6 — legacy protocol decision is explicit

Old clients that cannot carry an opaque session token require an explicit product decision.

Recommended secure default: do not include legacy direct-password login in the production security-ready surface.

If retained, document it as a separate compatibility boundary and ensure it cannot silently bypass account security policy.

### Principle 7 — credential migration rollout order

Recommended order:

1. inventory and pin deployed login components/images;
2. implement one authoritative Identity verifier that can validate legacy SHA-1 during migration;
3. integrate all supported login entry points with authoritative Identity;
4. implement short-lived single-use game-login authorization and atomic shared consumption;
5. remove/fence direct password and long-lived fallback paths;
6. implement password/reset/MFA/ban revocation across web sessions, DB sessions and pending game tokens;
7. prove end-to-end tests on exact deployed versions;
8. only then upgrade stored passwords to Laravel/PHP modern hashing via controlled rehash-on-login or forced reset strategy;
9. remove legacy SHA-1 verification after the compatibility window and rollback gate expire.

### Principle 8 — explicit security version / revocation generation

Recommended account security state includes a monotonically changing credential/session generation or equivalent revocation mechanism.

Game-login authorizations should carry/reference that generation so that:

- password change;
- password reset;
- account security recovery;
- MFA reset where policy requires;
- administrator credential compromise response;

can invalidate pending authorizations deterministically.

### Principle 9 — ban/disabled state checked twice

Recommended defense in depth:

- check account disabled/ban state before issuing game authorization;
- check it again atomically at Canary game-world entry.

This prevents unnecessary credential/session issuance while retaining Canary as a final safety gate.

### Principle 10 — MFA/email verification cannot be website-only when intended to gate game login

If product policy says MFA or verified email is required for game login, the authoritative Identity authorization decision must be represented in the game-login authorization itself or validated through an equivalent trusted server-to-server decision.

A UI-only or website-session-only check is insufficient.

## Target revocation matrix

| Event | Web sessions | Pending game tokens | Launcher/DB sessions | Active game sessions | Target policy |
|---|---|---|---|---|---|
| Password change | Rotate/revoke according to policy | Revoke all | Revoke all reusable login sessions | Decide explicitly; recommended revoke/re-auth for high-risk change | REQUIRED |
| Password reset | Revoke all | Revoke all | Revoke all | Revoke/disconnect or require re-auth | REQUIRED |
| MFA reset/recovery | Revoke sensitive sessions | Revoke all | Revoke all | Product policy; recommended revoke for admin/high-risk accounts | REQUIRED |
| Account banned/disabled | Revoke/deny | Revoke all | Revoke/deny | Disconnect or enforce immediate denial according to ban policy | REQUIRED |
| Email changed | Rotate verification-sensitive sessions if required | Invalidate if verification state changes | Invalidate if policy requires | Explicit product policy | REQUIRED |
| Normal logout | Revoke selected web session | Cancel user-initiated pending tokens where practical | Revoke selected launcher session if logout means global logout | End current game session | EXPLICIT |

## Required implementation tasks before production auth

The discovery task does not implement these. Separate approved tasks are required for:

1. **Canary security fix** — stop logging stored credential hashes on failed password authentication.
2. **Deployment/auth topology decision ADR** — choose authoritative Identity/login path and legacy-client policy.
3. **Credential compatibility test task** — deterministic fixtures for SHA-1, Canary custom Argon2 and proposed Laravel hash handling.
4. **Identity implementation** — web login/logout, secure hashing, verification, MFA, reset/recovery and server-side authorization.
5. **Game-login token implementation** — shared atomic one-time token issuance/consumption and exact routing contract.
6. **Revocation implementation** — password/reset/MFA/ban events revoke all intended session classes.
7. **Direct-path hardening** — close/restrict native login ports and remove password/DB-session fallbacks according to ADR.
8. **End-to-end auth matrix** — exact deployed components and versions.

## Production readiness blockers

Authentication must not be called production-ready while any of these remain unresolved:

- stored credential hash logging in Canary;
- unpinned/unknown deployed login-server image/version;
- public alternate native password login path not governed by authoritative Identity;
- SHA-1-only external login-server compatibility requirement;
- unproven Laravel/Canary password-hash compatibility;
- no global session/token revocation on password reset/change;
- no proven MFA/email-verification enforcement across all game-login paths when those policies are required;
- legacy direct-password path retained without an explicit accepted security policy;
- no end-to-end tests for expired/replayed/revoked/banned/direct-bypass cases.

## Required end-to-end verification matrix

Before merging any future production auth migration, test exact deployed versions for:

- valid modern login;
- wrong password;
- legacy SHA-1 account during migration;
- migrated modern-hash account;
- character ownership denial;
- deleted character denial;
- account ban before token issuance and at world entry;
- IP ban;
- expired game token;
- replayed single-use token;
- token used for wrong character;
- token used through wrong protocol/audience;
- password change revocation;
- password reset revocation;
- MFA reset/recovery revocation;
- email-verification gate if required;
- native Canary direct-login attempt;
- legacy client direct-login attempt;
- login-server outage;
- Platform Identity outage;
- DB session-store outage;
- shared token-store outage;
- Canary restart;
- multi-process/channel token routing;
- normal logout;
- active-session ban/disable behavior.

Evidence must include exact component SHAs/image digests/configuration and network exposure.

## Platform-native OAuth revocation generation — PROVEN

For the repository-owned Oteryn native OAuth `game:ticket` producer, authorization codes capture the current `Identity.game_auth_generation`; authorization-code and refresh-token access-token descendants inherit that same generation.

Game-ticket bootstrap locks the current Identity and access token, fails closed when the generation is missing or stale, and preserves the existing revoked, expired, ownership, scope, native-client, disabled and terminated gates before issuing a single-use Game Login Ticket. A successful bootstrap revokes the presenting access token and its refresh descendants atomically.

`RevokeIdentityGameAuthorizations` increments the Identity generation and revokes outstanding native `game:ticket` authorization codes, access tokens and refresh descendants in the same database transaction. Consequently, OAuth material issued before password change/reset or another caller of that revocation action cannot mint a fresh Game Login Ticket after the generation changes; a newly authorized token family receives the new generation.

The Passport generation columns are nullable for additive-schema compatibility. For `game:ticket` bootstrap, a missing generation is intentionally invalid, so pre-binding or stale game-ticket access tokens fail closed rather than being grandfathered.

This proof is limited to the Platform-owned native OAuth producer/bootstrap path. It does not prove revocation of retained Canary `LoginSessionManager` tokens, external `account_sessions`, legacy direct-password paths, active game sessions, or production deployment/cutover.
