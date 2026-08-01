# Public-domain validation evidence

Task: `OTERYN-20260801-public-domain-validation`  
Phase: `discovery_and_evidence`  
Repository baseline: `7dac56d3f3f4606be958c875f278edbe410e6b54`  
Evidence branch: `audit/OTERYN-20260801-public-domain-validation`  
Draft PR: `#387`

## Result

The repository contract and the last exact Synology staging deployment establish the intended split:

- `https://oteryn.molehill.cloud` is the Oteryn Platform web application, with loopback origin `127.0.0.1:8000`;
- `https://login.oteryn.molehill.cloud` is the Oteryn Game Gateway, with loopback origin `127.0.0.1:8080`.

The exact staging deployment observed by workflow run `30669701842` used Platform and Gateway source/image revision `6bfbc5f351758392d144baf0d2877a290ec69535` and Canary image digest `sha256:784e5dbdcc64e311c48c51cd94aa206e2efa1e5eefb2f4ef40170d5aac55031f`. The run directly proved loopback bindings, Platform/Gateway/Canary health, Gateway `/health`, `/ready` and `/version`, and preservation of the canonical WWW HTTPS login form action through forwarded proxy headers.

This package does **not** establish the current external deployment identity or current Internet-facing DNS, certificate, redirect, header, cookie, caching or routing behavior. Direct fetches and DNS resolution for both public hostnames were unavailable from the execution environment. The exact public-edge state therefore remains `UNKNOWN/BLOCKED`, and no `PRODUCTION_PROVEN` claim is made.

## Evidence identity

| Item | Classification | Evidence |
|---|---|---|
| Repository `main` at task start | `PROVEN` | `7dac56d3f3f4606be958c875f278edbe410e6b54` from repository preflight |
| Canonical hostname roles | `PROVEN` | `docs/contracts/PUBLIC_ENDPOINTS_CONTRACT.md`; `deploy/synology/PUBLIC_ENDPOINTS.md` |
| Last exact observed staging Platform source/image | `PROVEN` | workflow `30669701842`; `ghcr.io/blakinio/oteryn-platform:sha-6bfbc5f351758392d144baf0d2877a290ec69535` |
| Last exact observed staging Gateway source/image | `PROVEN` | workflow `30669701842`; `ghcr.io/blakinio/oteryn-game-gateway:sha-6bfbc5f351758392d144baf0d2877a290ec69535` |
| Last exact observed staging Canary image | `PROVEN` | workflow `30669701842`; digest `sha256:784e5dbdcc64e311c48c51cd94aa206e2efa1e5eefb2f4ef40170d5aac55031f` |
| Sanitized deployment artifact | `PROVEN` | artifact `8808580115`, ZIP digest `sha256:f5ea1efb02b8508d3b54765c2e7d15551dfab9d44c6a6c80ea3a299b970c0d44`, payload digest `sha256:2b94d392f97d2afa179ce32ec618f11b61c0bb38829a4ca8637efb6e6b84ab6d` |
| Current externally deployed Platform/Gateway/Canary identity | `UNKNOWN` | no current public response or deployment marker was directly observable |
| Production environment | `UNKNOWN` | artifact explicitly records `production_environment_proven: false`; Issue `#91` remains a production-verification gate |

## Scope reuse

PR `#381` already inventories 27 portal surfaces and 228 named-route assignments. This task reuses that inventory and does not recreate a general completeness audit. Domain-dependent representative WWW surfaces include:

- home and SEO resources;
- public news and managed pages;
- public game data;
- registration, login and logout;
- password lifecycle;
- MFA lifecycle;
- account and administration surfaces;
- signed Wiki article and media previews.

The Game Gateway is not a portal surface. Its current public HTTP contract is exactly:

```text
GET  /health
GET  /ready
GET  /version
POST /v1/login
```

A Gateway root-path `404` is therefore expected and is not a finding.

## WWW hostname evidence

Target: `https://oteryn.molehill.cloud`

| Check | Outcome | Classification | Evidence |
|---|---|---|---|
| Intended role and origin | PASS | `PROVEN` | public-endpoint contract maps WWW to Platform `127.0.0.1:8000` |
| Exact staging binding | PASS | `PROVEN` | workflow `30669701842`: Platform `8000/tcp -> 127.0.0.1:8000` |
| Forwarded HTTPS host/scheme handling | PASS for exact staging revision | `PROVEN` | health check sent `Host`/`X-Forwarded-Host: oteryn.molehill.cloud`, `X-Forwarded-Proto: https`, `X-Forwarded-Port: 443`; resulting login form action was `https://oteryn.molehill.cloud/login?locale=en` |
| Trusted proxy policy | PASS as source contract | `PROVEN` | explicit IP/CIDR-only trust; wildcard proxy trust is rejected; forwarded host/port/proto headers are consumed |
| Home and representative public-route reachability at public edge | BLOCKED | `UNKNOWN` | no direct current Internet response |
| TLS certificate and hostname validation | BLOCKED | `UNKNOWN` | no direct current TLS handshake |
| HTTP-to-HTTPS redirects and redirect chains | BLOCKED | `UNKNOWN` | no direct current HTTP response |
| Login, registration and password-recovery pages at public edge | BLOCKED | `UNKNOWN` | source routes exist; current external responses unavailable |
| Controlled login/logout | BLOCKED | `UNKNOWN` | no controlled public test identity was available |
| CSRF behavior at public edge | BLOCKED | `UNKNOWN` | source routes use Laravel web middleware; no current public form submission evidence |
| Security headers at public edge | BLOCKED | `UNKNOWN` | source middleware defines CSP, `nosniff`, frame denial, referrer and permissions policies; effective edge response unavailable |
| HSTS at public edge | BLOCKED | `UNKNOWN` | application middleware does not set HSTS; Cloudflare/edge policy was not directly observable |
| User-visible internal host/port leakage | BLOCKED | `UNKNOWN` | login form host/scheme passed staging proxy emulation; current rendered public pages were not retrievable |

### Session-cookie classification

The deployed source contract configures:

- path `/`;
- `HttpOnly=true`;
- `SameSite=Lax`;
- no explicit cookie domain, yielding a host-only cookie under normal Laravel behavior;
- `Secure=true` in the staging Marketplace overlay, overriding the historical base environment value.

These settings are `PROVEN` as repository and exact-staging configuration. The actual public `Set-Cookie` header, including effective attributes and absence of cross-host sharing with `login.oteryn.molehill.cloud`, is `UNKNOWN` because no current public response was observable.

### Generated URLs

- Browser forms and redirects use request-aware route generation. Exact staging evidence proves the login form preserved the forwarded canonical HTTPS origin.
- Password recovery invokes Laravel's standard password broker. The application does not install a custom reset-URL callback. Laravel 13 generates the reset route through the active URL generator.
- Email-change verification and recovery notifications call named routes directly.
- Wiki preview and preview-media URLs are temporary signed routes generated during authenticated browser requests.

The exact staging environment nevertheless recorded `APP_URL=http://127.0.0.1:8000`. Request-bound generation can still be correct when trusted forwarded headers are present, as the login-form proof demonstrates. Console, scheduler or other requestless absolute URL generation would use the configured application root and may therefore emit a loopback/plain-HTTP URL. No current user-visible occurrence was directly observed.

## Password-recovery evidence

Target flow: `POST /forgot-password` followed by delivery of a reset link to a controlled mailbox.

| Check | Outcome | Classification | Evidence |
|---|---|---|---|
| Route and throttling contract | PASS | `PROVEN` | `/forgot-password` GET/POST and `/reset-password/{token}` exist; request and source rate limits are configured |
| Non-logging mail safety guard | PASS | `PROVEN` | reset sender rejects the `log` mail transport before dispatch |
| Reset link generation source | PASS as source contract | `PROVEN` | standard Laravel password broker and `password.reset` route are used |
| Sender identity | BLOCKED | `UNKNOWN` | no controlled mailbox delivery evidence |
| Link hostname and HTTPS scheme | BLOCKED | `UNKNOWN` | no delivered message was available for inspection |
| Token completion | BLOCKED | `UNKNOWN` | no controlled identity/mailbox and no authorized public mutation path |

No reset token, signature or credential was generated or recorded.

## Gateway hostname evidence

Target: `https://login.oteryn.molehill.cloud`

| Check | Outcome | Classification | Evidence |
|---|---|---|---|
| Intended role and origin | PASS | `PROVEN` | public-endpoint contract maps hostname to Gateway `127.0.0.1:8080` |
| Exact staging binding | PASS | `PROVEN` | workflow `30669701842`: Gateway `8080/tcp -> 127.0.0.1:8080` |
| Actual public endpoints | PASS | `PROVEN` | source registers only `/health`, `/ready`, `/version`, `/v1/login` |
| Staging health/readiness/version | PASS for exact staging revision | `PROVEN` | health-check workflow probed all three endpoints successfully |
| Current public routing to Gateway rather than Platform | BLOCKED | `UNKNOWN` | no direct current public response |
| TLS certificate and hostname validation | BLOCKED | `UNKNOWN` | no direct current TLS handshake |
| `/version` current deployment identity | BLOCKED | `UNKNOWN` | external endpoint unavailable; last staging version was `sha-6bfbc5f351758392d144baf0d2877a290ec69535` |
| JSON content type and bounded errors | PASS as source contract | `PROVEN` | handlers emit JSON; malformed request `400 invalid_request`, invalid ticket `401 invalid_login`, dependency failure `503 login_unavailable` |
| Sensitive login response caching | PASS as source contract | `PROVEN` | every `/v1/login` response sets `no-store, no-cache, must-revalidate, private`, `Pragma: no-cache`, `Expires: 0` |
| Request-size and protocol bounds | PASS as source contract | `PROVEN` | body limited to 4096 bytes; unknown fields and trailing JSON rejected; protocol must equal 1; ticket length bounded |
| Current edge cache and server/origin leakage | BLOCKED | `UNKNOWN` | no direct public response |
| Effective public rate limiting | BLOCKED | `UNKNOWN` | Gateway source has no application-layer limiter; Cloudflare/edge policy unavailable |
| Native-client end-to-end success | NOT CLAIMED | `UNKNOWN` | no current direct native-client evidence tied to the observed public deployment |

## Edge behavior

For both hostnames, current DNS resolution, Cloudflare proxy path, certificate chain, HTTP-to-HTTPS redirect behavior, cache status, server/origin header leakage, representative errors and cross-routing are `UNKNOWN/BLOCKED`. Repository documentation and historical staging probes cannot substitute for a current external observation.

## Findings

### OTERYN-PUBLIC-DOMAIN-001 — Current public-edge state and deployment identity are unproven

- evidence_class: `UNKNOWN`
- outcome: `BLOCKED`
- severity: `MEDIUM`
- confidence: `HIGH`
- affected hostname and route: both canonical hostnames; all public routes
- evidence: direct web fetches failed; sandbox resolver could not resolve either name; direct public DNS/TLS/HTTP evidence was unavailable; the newest durable evidence is a host-loopback staging validation for revision `6bfbc5f351758392d144baf0d2877a290ec69535`
- impact: launch readiness cannot establish current certificate, redirect, routing, header, cookie, caching or deployment-revision correctness; `PRODUCTION_PROVEN` would be unsupported
- ownership boundary: external edge/DNS/Cloudflare and deployment observation; no mutation is authorized in this task
- recommendation: run one read-only public-domain probe from an Internet-capable trusted runner and bind every result to current Platform, Gateway and Canary identities

### OTERYN-PUBLIC-DOMAIN-002 — Canonical public URL conflicts with exact staging `APP_URL`

- evidence_class: `CONFLICT`
- outcome: `FAIL` for canonical configuration consistency; no direct broken user flow proven
- severity: `MEDIUM`
- confidence: `HIGH` for the configuration mismatch; `MEDIUM` for user impact
- affected hostname and route: `oteryn.molehill.cloud`; requestless generated absolute URLs
- evidence: canonical contract requires `https://oteryn.molehill.cloud`, while workflow `30669701842` rendered `APP_URL=http://127.0.0.1:8000`; request-bound login-form generation still passed through trusted forwarded headers
- impact: console, scheduler or other requestless absolute URL generation can emit loopback/plain-HTTP URLs even though request-bound forms are correct
- ownership boundary: Synology deployment configuration and application URL-generation policy; implementation/configuration changes are outside this validation task
- recommendation: make the deployed application root canonical HTTPS or provide explicit safe per-flow URL generation, then prove request-bound and requestless links independently

### OTERYN-PUBLIC-DOMAIN-003 — Password-recovery delivery and link origin are not directly proven

- evidence_class: `UNKNOWN`
- outcome: `BLOCKED`
- severity: `MEDIUM`
- confidence: `HIGH`
- affected hostname and route: `oteryn.molehill.cloud`; `/forgot-password`, delivered `/reset-password/{token}` link
- evidence: route, broker and notification source are present, but no controlled mailbox or delivered message was available; no token was exposed or used
- impact: a launch-relevant account-recovery path may be broken or may expose an incorrect host/scheme without detection
- ownership boundary: Identity mail delivery and controlled validation identity/mailbox; no production mutation is authorized here
- recommendation: submit one bounded recovery request for a controlled identity, inspect a redacted delivered URL, and complete the reset only under explicit mutation authorization

### OTERYN-PUBLIC-DOMAIN-004 — Effective public Gateway rate limiting is unresolved

- evidence_class: `UNKNOWN`
- outcome: `BLOCKED`
- severity: `MEDIUM`
- confidence: `HIGH`
- affected hostname and route: `login.oteryn.molehill.cloud`; `POST /v1/login`
- evidence: Gateway source bounds request size and errors but registers no application-layer rate limiter; effective Cloudflare or reverse-proxy limiting was not observable
- impact: repeated invalid or dependency-triggering requests may consume Gateway, Platform or Canary capacity if no edge policy exists
- ownership boundary: Game Gateway implementation or public edge policy; both are outside documentation-only ownership
- recommendation: directly verify a documented low-volume public rate-limit policy, or add an implementation-owned limiter in a separately authorized task

## Non-findings retained

- Gateway root `404` is expected because no root endpoint is contracted.
- Platform and Gateway hostnames are not interchangeable.
- Exact staging proxy handling corrected the WWW login form action for forwarded HTTPS.
- Exact staging Platform and Gateway services remained loopback-bound.
- Sensitive Gateway login responses have a source-enforced no-store policy.
- No evidence supports a claim of current production exposure, current native-client success or current password-reset delivery.

## Validation record

- Repository/branch/task/PR preflight: PASS.
- Required durable context and overlap inspection: PASS.
- PR `#381` route/surface inventory reuse: PASS.
- Platform proxy, URL, cookie, security-header and cache source inspection: PASS.
- Gateway route, error, cache and transport-bound source inspection: PASS.
- Workflow `30669701842` job log review: PASS.
- Artifact `8808580115` download, ZIP digest and JSON payload digest verification: PASS.
- Exact staging Platform/Gateway/Canary identity extraction: PASS.
- Direct current DNS/TLS/HTTP probes for both public domains: BLOCKED.
- Controlled login/logout and password-recovery mailbox validation: BLOCKED.
- Repository mutation boundary: PASS; only the task and report paths were changed.

## Acceptance disposition

The discovery-and-evidence phase is complete. Both hostname roles, the source contracts and the last exact staging candidate are durably recorded. Every missing current external observation is explicitly classified. The package remains evidence-only and does not authorize implementation, deployment, Cloudflare, DNS, Synology, credential, mailbox or production changes.

`PRODUCTION_PROVEN`: **false**.
