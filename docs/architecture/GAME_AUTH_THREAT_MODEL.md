# Oteryn Game Authentication Threat Model

## Status

Architecture-foundation threat model — 2026-07-21

This document defines the security threats and required controls for the target Oteryn game-authentication flow described by ADR 0009.

It is a design/security contract, not proof that the controls are already implemented or deployed.

## Scope

In scope:

```text
System Browser
  -> Oteryn Identity / Authorization Server
  -> OTClient loopback callback
  -> OAuth Authorization Code + PKCE
  -> short-lived OAuth bootstrap credential
  -> Game Login Ticket issuance
  -> OTClient
  -> Oteryn Game Gateway
  -> private ticket redeem at Identity
  -> World/Character resolution
  -> Game Session creation
  -> Legacy engine world entry
```

Out of scope for the first implementation:

- multichannel gameplay-state synchronization;
- gameplay protocol confidentiality beyond the existing legacy engine protocol contract;
- production DDoS provider selection;
- passkey implementation details;
- existing-account import/claim;
- production deployment proof.

## Assets

Highest-value assets:

1. reusable Oteryn user credentials;
2. MFA/passkey/recovery material;
3. OAuth authorization codes and bootstrap access credentials;
4. Game Login Tickets;
5. Game Session secrets;
6. exact Identity -> legacy engine account binding;
7. character ownership and world authorization;
8. Identity security/revocation generation;
9. service-to-service credentials between Gateway and Identity;
10. private session/ticket storage.

## Trust boundaries

### Boundary A — Browser / public Identity ingress

Untrusted inputs:

- authorization request parameters;
- login form inputs;
- MFA/passkey inputs;
- redirect URI attempts;
- browser cookies and request metadata.

Authoritative component:

- Oteryn Identity.

### Boundary B — Identity -> OTClient loopback redirect

Untrusted environment:

- local desktop processes;
- local browser state;
- loopback port race/interception attempts;
- malicious application claiming the redirect.

Required protections:

- Authorization Code + PKCE S256;
- cryptographically random `state`;
- loopback IP literal;
- fixed callback path;
- ephemeral port;
- bounded authorization-code lifetime;
- one-time code redemption.

### Boundary C — OTClient -> Oteryn Platform ticket issuance

OTClient is an untrusted public client.

It has no client secret and must never be trusted merely because it identifies itself as the official client.

Authorization comes from the validated OAuth grant and current Identity policy, not from client-supplied account identifiers.

### Boundary D — OTClient -> Game Gateway

The public Gateway login endpoint receives attacker-controlled network traffic.

The presented Game Login Ticket is a bearer secret until consumed.

Gateway must not infer Identity or legacy engine account ownership from any client-supplied `identity_id`, `account_id`, character ownership field or world claim.

### Boundary E — Gateway -> Identity internal redeem

This is a privileged service-to-service boundary.

Only authenticated Gateway instances may call ticket redeem.

Private networking is defense in depth, not authentication by itself.

### Boundary F — Gateway -> legacy engine/read/session integration

Game-server-owned data and session persistence are separate trust domains.

Gateway must use narrowly scoped capabilities and must not receive generic legacy engine database privileges.

### Boundary G — OTClient -> legacy engine game endpoint

The client presents only the Game Session credential required by the selected compatibility contract plus the selected character/world context.

Legacy engine remains the final owner of world-entry checks such as account/character ownership, deletion state, ban/name-lock and runtime admission rules supported by the exact deployed version.

## Adversaries

Assume:

- an unauthenticated remote attacker can call public Identity and Gateway endpoints;
- a malicious user controls their own OTClient process and can alter requests;
- another local process may race for or observe loopback traffic;
- logs, metrics and error telemetry may be accessible to operators and therefore must not contain bearer credentials;
- one Gateway instance may fail while another continues;
- concurrent replay requests can arrive at different instances;
- databases/Redis/service dependencies can be unavailable or partially fail;
- legacy engine/login-server password endpoints may remain reachable during migration unless explicitly fenced;
- a service credential may eventually need rotation or revocation.

Do not assume:

- the official OTClient binary is untampered;
- source IP proves user identity;
- private network placement alone authenticates a service;
- TLS termination prevents application-level credential leakage;
- a successful Identity login remains valid forever after a security-sensitive account change.

## Threat matrix

| ID | Threat | Attack / failure | Required controls | Residual / status |
|---|---|---|---|---|
| GA-001 | Password exposure to game components | Client submits Oteryn password to Gateway/legacy engine or password is copied into legacy-compatible storage | Password entry only on Oteryn Identity browser surface; no password field in Gateway/legacy engine contracts; random sink credential never exposed or accepted for login | Target control; legacy paths remain migration risk |
| GA-002 | Authorization request CSRF / login swap | Attacker causes victim client to accept another authorization response | High-entropy `state`, client-side exact comparison, one pending flow per attempt, discard callback on mismatch | Implementation required |
| GA-003 | Authorization-code interception | Local process intercepts loopback authorization code | PKCE S256 with high-entropy verifier; one-time short-lived code; verifier never leaves client except token exchange | Implementation required |
| GA-004 | Loopback listener hijack | Malicious local app binds callback port first or races callback | OS-selected ephemeral port, IP-literal loopback, PKCE, state, bounded listener lifetime, fail closed on bind failure | PKCE prevents intercepted code redemption without verifier; local DoS remains possible |
| GA-005 | Redirect URI abuse | Attacker redirects authorization response to remote or attacker-controlled URI | Pre-registered loopback callback path; exact scheme/host/path validation; only loopback port variance; no wildcard remote redirects | Implementation required |
| GA-006 | Embedded webview credential theft | Game client embeds a webview capable of intercepting credentials | Use system/default browser; do not collect Identity password/MFA in OTClient UI | Implementation required |
| GA-007 | Public-client secret illusion | Static client secret is shipped and treated as authenticating OTClient | Register OTClient as public client; no confidential secret; authorize user/session, not binary possession | Architecture decision |
| GA-008 | Excessive OAuth token lifetime | Stolen bootstrap access/refresh token provides durable account access | Narrow `game:ticket` scope; short access-token TTL; no long-lived refresh-token use; revoke family after ticket issuance if library cannot suppress refresh issuance safely | Exact Passport implementation mechanism remains to be proven |
| GA-009 | OAuth token sent to game layer | Access/refresh token leaks to Gateway or legacy engine and expands trust | Gateway contract accepts only Game Login Ticket; legacy engine contract accepts only Game Session material | Contract-enforced target |
| GA-010 | Ticket prediction | Attacker guesses a valid Game Login Ticket | At least 256 bits CSPRNG entropy; opaque token | Implementation required |
| GA-011 | Ticket database disclosure | Read access to ticket store yields redeemable plaintext tickets | Store only cryptographic hash/HMAC-derived lookup value; plaintext returned once; never persist plaintext | Implementation required |
| GA-012 | Ticket replay | Stolen ticket is redeemed twice | Atomic single-use consume in shared authoritative store; exactly one concurrent request succeeds | Critical regression test required |
| GA-013 | Cross-instance replay race | Two Gateway instances redeem same ticket concurrently | Shared atomic storage/transaction/conditional update; no process-local authoritative consume | Critical regression test required |
| GA-014 | Expired ticket use | Delayed or stolen ticket used after intended window | Default 60s TTL; authoritative server clock; expiry checked atomically during redeem | Implementation required |
| GA-015 | Confused deputy / wrong audience | Ticket minted for another consumer is accepted by Gateway/redeem API | Explicit audience `oteryn-game-gateway`; redeem verifies audience; no generic bearer-token endpoint | Implementation required |
| GA-016 | Client-controlled account substitution | Client supplies another `account_id` with its ticket | Ticket binds exact ready immutable `legacy_account_id`; redeem result is authoritative; ignore/reject client account identifiers | Existing binding supports target |
| GA-017 | Binding not ready / conflict | Identity has pending/conflicting legacy engine mapping but receives game access | Ticket issuance and redeem fail closed unless immutable binding is ready with non-null exact account ID | Implementation required |
| GA-018 | Stale authorization after password reset/change | Ticket remains usable after security-sensitive account change | Monotonic `game_auth_generation`; ticket captures generation; redeem compares current generation | New Platform state required |
| GA-019 | Disabled Identity still enters game | Identity disabled after ticket issuance | Check disabled state at issuance and again at redeem; advance generation/revoke pending tickets | Implementation required |
| GA-020 | MFA bypass | User with enrolled/required MFA obtains ticket via a password-only alternate path | OAuth authorization uses normal Identity login/MFA flow; all external legacy password paths must be fenced before global-authority claim | Legacy bypass remains blocker until migration closure |
| GA-021 | Recovery/MFA reset stale credentials | Pending game credentials survive high-risk recovery event | Advance game-auth generation; revoke OAuth/bootstrap credential family as defined; explicit Game Session policy | Pending-ticket behavior required; active-session policy still explicit decision |
| GA-022 | Gateway impersonation | Attacker calls internal ticket redeem directly | Service authentication, TLS, least-privilege credential, rotation/revocation; mTLS preferred where operationally supported | Deployment mechanism not yet proven |
| GA-023 | Internal endpoint public exposure | Redeem API reachable from public internet | Private ingress/network policy plus application service authentication; fail closed even if network boundary fails | Production topology UNKNOWN |
| GA-024 | Service credential leakage | Gateway service secret appears in repo/log/error | External secret management; no secrets in Git/logs; rotation procedure; separate credential from other services | Production secret manager UNKNOWN |
| GA-025 | Ticket leakage in logs | Request body/header/URL containing ticket is logged | Ticket in request body, never URL; redact credential fields; bounded request logging; avoid reverse-proxy body logging; regression tests | App request-completion logger already excludes bodies/headers; full deployment logging UNKNOWN |
| GA-026 | Session-secret leakage | Game Session secret logged by Gateway/legacy engine/proxy | Never log raw session credential; store hash where compatibility permits; sanitize errors/metrics | legacy engine exact behavior must be revalidated for chosen adapter |
| GA-027 | Session replay | DB-backed Game Session remains replayable until expiry | Bound TTL; revocation/delete semantics; single active intended use where feasible; final contract must document unavoidable replay characteristics | Existing `account_sessions` replayability is proven risk; adapter not yet approved |
| GA-028 | Session fixation/substitution | Client chooses or reuses session ID tied to another account | Server-generated high-entropy secret; authoritative account/world binding; no client-selected session ownership fields | Implementation required |
| GA-029 | Wrong-world session use | Session created for EU world is accepted by another world | Bind Game Session to `world_id`; route/adapter verifies world audience where protocol permits; per-world credentials if necessary | Exact legacy engine mechanism UNKNOWN |
| GA-030 | Wrong-character use | Session intended for one account used to enter character belonging to another | Gateway list from authoritative account; legacy engine final ownership/deletion checks; never trust client ownership assertion | Current legacy engine final ownership gate is proven baseline |
| GA-031 | Character-list data leak | Gateway lists deleted/foreign characters | Narrow allowlisted query with `account_id` and active/listable-state filtering proven by contract; do not reuse inconsistent upstream login-server query blindly | New Gateway read contract required |
| GA-032 | World-routing injection | Client supplies arbitrary game host/port | Gateway returns routing from authoritative World Registry; client cannot create registry entries | Implementation required |
| GA-033 | Unauthorized world access | Client requests test/tournament world without entitlement | Gateway filters by account/world policy and `login_enabled`; Game Session world-scoped | First release may have one world |
| GA-034 | Registry tampering | Compromised admin/config redirects clients to hostile server | Restrict registry writes; audit privileged changes; validate endpoint shape; deployment/config review | Registry administration policy deferred if registry is static/config-backed initially |
| GA-035 | Gateway broad DB compromise | Gateway credential can read password hashes or mutate unrelated legacy engine data | Separate least-privilege credentials/capabilities for character read and session persistence; no generic Platform/legacy engine DB access | New operation-specific contract required |
| GA-036 | Identity DB coupling | Gateway directly queries Platform Identity tables and bypasses policy | Redeem through versioned private Identity API; no direct Identity DB credential | Architecture decision |
| GA-037 | Fail-open on Identity outage | Gateway accepts ticket without successful authoritative redeem | Redeem dependency failure returns login failure; no cached success fallback | Required outage test |
| GA-038 | Fail-open on ticket store outage | Gateway/Identity guesses ticket validity | No session creation without atomic consume success | Required outage test |
| GA-039 | Partial commit: ticket consumed, session not created | User loses ticket after downstream failure | Ticket remains consumed; client must restart authorization/ticket flow or receive a narrowly designed retry token only if separately specified; never un-consume a ticket | Expected safe failure; UX retry behavior required |
| GA-040 | Partial commit: session created, response lost | Client retries and creates multiple sessions | Session-create adapter requires idempotency key or deterministic bounded recovery tied to redeem transaction/result; exact mechanism contract required | UNKNOWN until Game Session adapter design |
| GA-041 | Legacy native legacy engine bypass | Attacker skips Identity and uses exposed native login port/password flow | Fence/disable public native login after new E2E proof; verify process registration and network exposure | Current source-level alternate path capability proven; production exposure UNKNOWN |
| GA-042 | Upstream login-server password bypass | Attacker uses SHA-1 password login endpoint instead of Identity | Replace/fence endpoint for Oteryn clients; no sink-password fallback | Migration blocker |
| GA-043 | Old-protocol direct-password bypass | Legacy client cannot carry target opaque credentials | Disable by default for production secure surface or document separately accepted lower-security tier | Product rollout decision required before production claim |
| GA-044 | Silent fallback to replayable credential | Failed one-time auth falls back to long-lived `account_sessions` or password | Authoritative target path has no fallback; compatibility adapter only after successful ticket redeem | Required contract/test |
| GA-045 | legacy engine stored-hash logging | Failed native password auth logs stored credential material | Separately authorized legacy engine security fix before production security-ready claim | Existing contract marks high priority; external fix not in this task |
| GA-046 | Replay after process restart | Process-local ticket state disappears or loses consume history | Shared durable/controlled state with atomic semantics; restart must not resurrect consumed tickets | Implementation/storage choice required |
| GA-047 | Clock skew | Different services disagree on ticket/session expiry | Server-authoritative timestamps, bounded synchronized clocks, validation against authoritative store; no client clock trust | Deployment monitoring required |
| GA-048 | Rate-limit bypass / brute force | Attackers flood authorization, ticket issuance or Gateway redeem | Application-level rate limits per identity/source/endpoint; edge controls as supplement; avoid account enumeration | Concrete limits to be set from runtime evidence |
| GA-049 | Error oracle | Different public errors reveal identity/binding/security state | Public errors are coarse and non-enumerating; internal structured codes remain bounded and credential-free | Contract required |
| GA-050 | Sensitive observability | Metrics/traces capture tickets/tokens/account PII | Structured allowlist logging; no bodies/headers; hash/pseudonymous IDs only where necessary; trace attribute denylist | Production observability sink UNKNOWN |
| GA-051 | Dependency SSRF / arbitrary routing | Gateway uses client-supplied Identity/legacy engine endpoint | Endpoints come only from trusted configuration/World Registry; no user URL fetching | Implementation required |
| GA-052 | Protocol downgrade | Client forces legacy password mode when modern flow should be mandatory | Server-side supported-version policy and explicit migration feature flag; no client-only security decision | Rollout control required |
| GA-053 | Version confusion | Old/new client interprets credential fields differently | Explicit API/protocol version, compatibility matrix, fail closed on unsupported version | Contracts required |
| GA-054 | Unauthorized ticket mint via stolen web session | Attacker with ordinary browser session mints game ticket without intended native authorization context | Ticket issuance requires OAuth bearer with `game:ticket` scope/client context, not generic Laravel web session alone | Implementation required |
| GA-055 | Open redirect after login | `redirect()->intended()` is manipulated to external target | OAuth authorization endpoint and Laravel intended-redirect handling must constrain return target to safe same-origin authorization flow; add focused tests | Must be verified during Passport integration |
| GA-056 | Consent/phishing confusion | User cannot tell which native client is requesting game access | First-party fixed public client registration, recognizable Oteryn authorization screen, bounded scope; no arbitrary dynamic client registration | Implementation required |

## Critical invariants

The following are non-negotiable:

1. **No user password beyond Identity.** The target Gateway and legacy engine contracts contain no reusable Oteryn password.
2. **No client-secret trust for OTClient.** OTClient is a public native client.
3. **PKCE S256 + state are mandatory.** Missing or invalid values fail closed.
4. **Ticket consume is atomic and shared.** Exactly one concurrent redeem succeeds.
5. **Ownership comes from Platform trusted state.** Client input never establishes `legacy_account_id` ownership.
6. **Ticket and Game Session are different credentials.** A ticket is one-time bootstrap authorization; a Game Session is the legacy-engine-entry credential.
7. **Security-state changes invalidate pending authorization.** A stale generation cannot redeem.
8. **No credential logging.** Passwords, OAuth codes/tokens, tickets, session secrets and service credentials are never logged.
9. **No fail-open dependency fallback.** Identity/ticket/session-store failure prevents new login.
10. **No silent legacy fallback.** Failed target authentication cannot fall back to password or a longer-lived DB session.
11. **Global-authority claims require bypass closure.** Reachable native/login-server/legacy password paths must be fenced, removed or explicitly governed by the same policy.

## Atomic ticket lifecycle

Logical states:

```text
ISSUED
  |
  +-- expires --------------------> EXPIRED
  |
  +-- security generation changes -> REVOKED / INVALID
  |
  +-- atomic redeem --------------> USED
```

Implementation may use a single atomic conditional transition rather than a visible `REDEEMING` state.

Required atomic predicate is equivalent to:

```text
hash matches
AND audience matches
AND used_at IS NULL
AND expires_at > now
AND stored_generation == current_identity_generation
AND identity is enabled
AND binding remains ready and exact
```

Exactly one request may transition to `USED`.

A failed downstream Game Session creation does **not** roll the ticket back to `ISSUED`.

## Revocation model

### Pending Game Login Tickets

Must be invalidated by at least:

- password reset;
- password change;
- high-risk account recovery;
- Identity disablement;
- administrator compromise-response action;
- MFA reset/disable when game-login policy treats the event as security-sensitive.

Recommended implementation: monotonic `game_auth_generation` checked at redeem, optionally combined with explicit ticket deletion/expiry cleanup.

### OAuth bootstrap credentials

Must have narrow scope and short lifetime.

Password/recovery/MFA security events must revoke or invalidate the relevant OAuth authorization/token family according to the Passport implementation contract.

### Game Sessions

Required first-release policy must explicitly define behavior for:

- password change;
- password reset;
- MFA reset/recovery;
- Identity disablement;
- Legacy engine account ban;
- normal logout;
- Gateway/legacy engine restart.

Until the Game Session compatibility adapter is proven, these effects are `UNKNOWN` and must not be represented as implemented.

## Service authentication model

Minimum accepted first release:

```text
Gateway
  -> TLS
  -> private/restricted Identity internal ingress
  -> dedicated rotated service credential
  -> explicit service authorization for game-ticket redeem only
```

Preferred production direction:

```text
Gateway
  -> mTLS
  -> Identity internal ingress
  -> workload identity / certificate rotation
```

A bearer service credential, if used initially, must:

- be injected outside Git;
- be unique to Game Gateway;
- have no user identity privileges;
- authorize only the required internal API;
- be rotatable without downtime where practical;
- never appear in URLs/logs.

## Data minimization

Ticket redeem response should contain only fields required by Gateway, for example:

```text
identity_subject        # opaque/pseudonymous identifier only if Gateway truly needs it
legacy_account_id
security_generation
issued_at
redeemed_at
```

Gateway does not need:

- email;
- password/hash;
- MFA enrollment state or secret;
- recovery codes;
- OAuth authorization code;
- OAuth refresh token;
- Platform web-session identifier.

The final contract may omit `identity_subject` entirely if account resolution and audit correlation do not require it.

## Failure policy

| Failure | Required result |
|---|---|
| Browser login cancelled | No code/ticket/session; client returns to signed-out state |
| `state` mismatch | Reject locally; do not exchange code |
| Invalid PKCE | Token exchange denied |
| Expired/reused auth code | Denied; restart authorization |
| Ticket issuance dependency failure | No ticket |
| Expired/reused/revoked ticket | Gateway login denied |
| Concurrent ticket replay | Exactly one winner |
| Identity unavailable at redeem | Deny new login |
| Ticket store unavailable | Deny new login |
| Binding missing/pending/conflict | Deny ticket issuance/redeem |
| World disabled/unavailable | No session for that world |
| Character missing/foreign/deleted | Deny selection/entry |
| Session-store persistence fails | No successful login response |
| Legacy engine unavailable | Do not claim successful world entry; session cleanup/retry follows explicit adapter policy |

## Security test requirements

Minimum focused regression tests before production-auth readiness:

- authorization request with missing/invalid PKCE;
- state mismatch;
- unregistered redirect and malicious non-loopback redirect;
- callback wrong path/port semantics;
- expired and reused authorization code;
- OAuth token lacking `game:ticket` scope;
- generic web session attempting direct ticket issuance;
- expired Game Login Ticket;
- wrong audience;
- ticket hash/storage does not expose plaintext;
- two concurrent redeems with exactly one success;
- identity disabled between issue and redeem;
- generation advanced between issue and redeem;
- missing/pending/conflict legacy engine binding;
- client-supplied wrong `account_id` ignored/rejected;
- wrong/foreign/deleted character;
- unauthorized/disabled world;
- Identity/ticket-store/session-store outage;
- sensitive-token redaction/log assertions;
- direct native legacy engine password bypass attempt after fencing phase;
- upstream login-server password bypass attempt after fencing phase;
- unsupported legacy protocol downgrade attempt.

## Remaining architecture unknowns

1. Exact production network topology and whether native legacy engine/login-server password endpoints are externally reachable.
2. Exact production secret-management and service-identity mechanism.
3. Exact shared storage selected for Game Login Ticket atomic consume.
4. Exact Game Session compatibility adapter and its replay/revocation/idempotency semantics.
5. Whether current legacy engine can enforce world-scoped session audience without a code change.
6. Exact active-game-session revocation/disconnect policy for each Identity security event.
7. Exact Passport configuration needed to prevent long-lived refresh semantics for the first-party native client while preserving standards-compliant PKCE behavior.
8. Exact production rate limits based on measured legitimate login behavior.

These unknowns are implementation/deployment gates. None may be silently converted into assumptions.
