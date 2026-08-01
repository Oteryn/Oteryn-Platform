# Public-domain validation evidence

Task: `OTERYN-20260801-public-domain-validation`  
Phase: `complete`  
Repository baseline: `7dac56d3f3f4606be958c875f278edbe410e6b54`  
Evidence branch: `audit/OTERYN-20260801-public-domain-validation`  
Draft PR: `#387`

## Verdict

**PUBLIC DOMAIN LAUNCH: BLOCKED / FAIL.**

Two independent GitHub-hosted observations from different Azure regions directly reached the public Cloudflare edge on 2026-08-01:

- run `30690877286`, job `91345253565`, East US / IAD edge;
- run `30690957415`, job `91345468758`, West US / SJC edge.

The observations prove:

1. both canonical names resolve through Cloudflare to the same IPv4 and IPv6 anycast addresses;
2. `oteryn.molehill.cloud` presents a valid certificate for `molehill.cloud` / `*.molehill.cloud` over TLS 1.3, but anonymous HTTP and HTTPS requests to every representative WWW route receive a Cloudflare `403` interstitial;
3. `login.oteryn.molehill.cloud` fails the TLS handshake before HTTP for both TLS 1.2 and TLS 1.3, with both Python/OpenSSL and curl clients;
4. plain HTTP requests are answered by Cloudflare with `403`, not redirected to HTTPS;
5. the WWW edge sends `Strict-Transport-Security: max-age=0; includeSubDomains; preload`, which disables persisted HSTS rather than enforcing it.

The exact currently deployed Platform and Canary identities remain unknown. The Gateway `/version` endpoint cannot be reached because TLS fails. No `PRODUCTION_PROVEN` claim is made.

## Canonical roles and last staging identity

The repository contract defines:

- `https://oteryn.molehill.cloud` → Oteryn Platform WWW → `127.0.0.1:8000`;
- `https://login.oteryn.molehill.cloud` → Oteryn Game Gateway → `127.0.0.1:8080`.

The last exact Synology staging deployment directly observed before this audit was workflow `30669701842`:

- Platform image/source: `sha-6bfbc5f351758392d144baf0d2877a290ec69535`;
- Gateway image/source: `sha-6bfbc5f351758392d144baf0d2877a290ec69535`;
- Canary image: `sha256:784e5dbdcc64e311c48c51cd94aa206e2efa1e5eefb2f4ef40170d5aac55031f`;
- Platform binding: `127.0.0.1:8000`;
- Gateway binding: `127.0.0.1:8080`;
- Platform/Gateway/Canary health: PASS;
- forwarded canonical HTTPS login-form action: PASS.

That staging evidence remains valid only for its exact revision and host-loopback boundary. It does not override the current public-edge failures.

## Evidence identity

| Evidence | Classification | Identity |
|---|---|---|
| Repository baseline | `PROVEN` | `7dac56d3f3f4606be958c875f278edbe410e6b54` |
| Public DNS/TLS/HTTP observation | `PROVEN` | run `30690877286`; head `19e62011f5920c89d22aa70738b3ea66ab61ef20`; artifact `8815612315`; ZIP digest `sha256:174ff9dd5c1a098a49277926aca100b41f7a3761e7e67595f98b92097c7ea909` |
| Independent corroboration | `PROVEN` | run `30690957415`; head `b66012b086f03b2cf70f1c194cb4c72593bc2426`; artifact `8815638539`; ZIP digest `sha256:b5b3effb61e350c4a5fd59ff2949c9f38f265b42f3c81787bf894745d738a1d8` |
| Last exact staging deployment | `STAGING_PROVEN` | run `30669701842`; artifact `8808580115`; ZIP digest `sha256:f5ea1efb02b8508d3b54765c2e7d15551dfab9d44c6a6c80ea3a299b970c0d44` |
| Current external Platform image/SHA | `UNKNOWN` | no non-secret version marker is exposed by the reachable WWW edge |
| Current external Gateway image/SHA | `UNKNOWN` | `/version` is unreachable because TLS negotiation fails |
| Current Canary image/SHA | `UNKNOWN` | no safe public version marker is available |
| Production environment | `UNKNOWN` | Issue `#91` remains the authoritative production-verification gate |

All evidence is sanitized. No secret, credential, token, cookie value, form submission or valid Game Login Ticket was used.

## DNS evidence

Both names resolve without CNAME records to the same Cloudflare addresses:

```text
A     104.21.2.166
A     172.67.186.250
AAAA  2606:4700:3031::6815:2a6
AAAA  2606:4700:3033::ac43:bafa
```

Classification: `PROVEN` for the observation timestamps. The origin addresses remain undisclosed and were not probed.

## WWW hostname

Target: `https://oteryn.molehill.cloud`

### TLS

- TLS 1.3: PASS.
- TLS 1.2: rejected with `protocol version` alert in the corroboration run.
- certificate subject: `molehill.cloud`;
- SANs: `molehill.cloud`, `*.molehill.cloud`;
- issuer: Google Trust Services `WE1`;
- valid from `2026-06-28 02:01:55 UTC` to `2026-09-26 02:59:39 UTC`;
- SHA-256 fingerprint: `5f72d627546607d059b7737c852f9b1a1bb459d7f5852bd15766903da81a183f`.

The certificate directly covers `oteryn.molehill.cloud`.

### HTTP behavior

The following HTTPS routes all returned Cloudflare `403` before the Platform response could be observed:

- `/`;
- `/login?locale=en`;
- `/register`;
- `/forgot-password`;
- `/health`;
- `/news`;
- `/highscores`;
- `/version` cross-routing probe.

The first run returned `Attention Required! | Cloudflare`. The independent run returned `Just a moment...` for HTTPS. The same result occurred with both a bounded validator User-Agent and a current Chrome-like User-Agent. Because the probe did not execute a JavaScript challenge, this proves automated anonymous reachability failure; it does not prove that every interactive human browser is blocked.

Plain `http://oteryn.molehill.cloud/` returned Cloudflare `403` without an HTTP-to-HTTPS redirect.

### Edge headers

Observed HTTPS challenge response included:

```text
Server: cloudflare
Strict-Transport-Security: max-age=0; includeSubDomains; preload
Content-Type: text/html; charset=UTF-8
```

The first observation also recorded no-store/no-cache response directives on the Cloudflare block page. Application CSP, session cookies, CSRF form behavior and generated action URLs could not be observed externally because the edge interstitial stopped the request before Platform content.

### Source and staging facts retained

Repository and exact-staging evidence still prove:

- explicit trusted-proxy configuration without wildcard trust;
- request-bound forwarded HTTPS login action generation on staging;
- session-cookie source configuration using `Secure`, `HttpOnly`, `SameSite=Lax`, path `/` and no configured domain in the Marketplace staging overlay;
- Platform security-header middleware;
- no-store policy on sensitive authentication APIs.

Those facts are not promoted to current public-edge behavior while Cloudflare prevents direct observation.

## Gateway hostname

Target: `https://login.oteryn.molehill.cloud`

### Contracted public surface

```text
GET  /health
GET  /ready
GET  /version
POST /v1/login
```

The source contract returns bounded JSON and applies `no-store, no-cache, must-revalidate, private`, `Pragma: no-cache` and `Expires: 0` to `/v1/login` responses.

### Current TLS behavior

Both independent observations failed before any HTTP response:

```text
TLS 1.2: sslv3 alert handshake failure
TLS 1.3: sslv3 alert handshake failure
curl: (35) OpenSSL SSL routines: sslv3 alert handshake failure
HTTP status: 000
```

The failure was reproduced with:

- Python `ssl` default verification;
- forced TLS 1.2;
- forced TLS 1.3;
- curl/OpenSSL;
- validator and Chrome-like User-Agents;
- both Cloudflare IPv4 addresses selected across the observations.

Therefore `/health`, `/ready`, `/version` and `/v1/login` are not externally usable over the canonical HTTPS hostname from standards-compliant clients.

The observed WWW certificate SAN `*.molehill.cloud` covers one label such as `oteryn.molehill.cloud`; it does not cover the two-label name `login.oteryn.molehill.cloud`. The absence of a successful Gateway handshake is consistent with no edge certificate covering the exact deeper hostname. This cause is `DERIVED` with high confidence; the exact Cloudflare certificate configuration was not read directly.

Plain `http://login.oteryn.molehill.cloud/health` returned Cloudflare `403` and did not redirect to HTTPS.

## Password recovery

Source inspection proves:

- `/forgot-password` GET/POST and `/reset-password/{token}` routes;
- request and source throttling;
- standard Laravel password broker use;
- rejection of the `log` mail transport for reset links.

Current public validation result:

- GET `/forgot-password`: Cloudflare `403` in the external observation;
- sender identity: `UNKNOWN`;
- delivered link host/scheme: `UNKNOWN`;
- reset completion: `NOT_RUN`;
- controlled identity/mailbox: unavailable;
- no token or credential was generated or recorded.

## Generated URL configuration

The exact staging deployment used:

```text
APP_URL=http://127.0.0.1:8000
```

while the canonical public root is:

```text
https://oteryn.molehill.cloud
```

Request-bound form generation passed on staging because trusted forwarded headers supplied the public origin. Requestless console/scheduler/mail URL generation may still use the loopback/plain-HTTP application root. No delivered user-visible link was inspected, so actual impact remains unproven, but the configuration conflict is direct.

## Findings

### OTERYN-PUBLIC-DOMAIN-001 — Gateway canonical HTTPS hostname has no usable TLS service

- evidence_class: `PROVEN`
- outcome: `FAIL`
- severity: `HIGH`
- confidence: `HIGH`
- affected surface: `login.oteryn.molehill.cloud`; all Gateway endpoints and native-client login
- evidence: runs `30690877286` and `30690957415`; TLS 1.2 and 1.3 handshake failure with independent clients and regions
- impact: the canonical Game Gateway cannot be used by a standards-compliant HTTPS client; native login through that hostname is blocked before application processing
- likely cause: `DERIVED` certificate coverage gap for the two-label hostname; observed SAN `*.molehill.cloud` does not cover `login.oteryn.molehill.cloud`
- ownership: Cloudflare edge certificate/hostname configuration and public endpoint contract
- recommendation: provision an edge certificate explicitly covering `login.oteryn.molehill.cloud`, or move Gateway to a hostname covered by the chosen certificate hierarchy; then prove `/health`, `/ready`, `/version` and a bounded invalid `/v1/login` response externally

### OTERYN-PUBLIC-DOMAIN-002 — WWW anonymous automated reachability is blocked by Cloudflare

- evidence_class: `PROVEN`
- outcome: `FAIL` for anonymous automated reachability; interactive-browser result remains `UNKNOWN`
- severity: `MEDIUM`
- confidence: `HIGH`
- affected surface: all representative Platform public routes, including `/health`, login, registration and password recovery
- evidence: every tested route returned Cloudflare `403` in two regions with validator and Chrome-like User-Agents
- impact: external health monitoring, API-style validation and non-JavaScript clients cannot reach Platform; route correctness, cookies and application headers cannot be continuously observed
- ownership: Cloudflare WAF/Bot/Access policy
- recommendation: document the intended public access policy; allow bounded health and required non-browser traffic without a JavaScript challenge, and use explicit rate limits/service authentication where appropriate

### OTERYN-PUBLIC-DOMAIN-003 — HSTS is explicitly disabled at the WWW edge

- evidence_class: `PROVEN`
- outcome: `FAIL`
- severity: `MEDIUM`
- confidence: `HIGH`
- affected surface: `oteryn.molehill.cloud` and declared subdomains
- evidence: `Strict-Transport-Security: max-age=0; includeSubDomains; preload`
- impact: supporting browsers are instructed to remove the HSTS policy; the `includeSubDomains` and `preload` tokens do not compensate for zero max-age
- ownership: Cloudflare edge TLS/HTTP policy
- recommendation: after HTTPS coverage is corrected for all included subdomains, set a positive reviewed HSTS max-age and validate the effective response

### OTERYN-PUBLIC-DOMAIN-004 — HTTP does not redirect to HTTPS before edge blocking

- evidence_class: `PROVEN`
- outcome: `FAIL`
- severity: `MEDIUM`
- confidence: `HIGH`
- affected surface: both canonical hostnames
- evidence: HTTP root/health probes returned Cloudflare `403` with no redirect chain
- impact: clients are not consistently upgraded to HTTPS; users can receive an edge block page over plaintext HTTP
- ownership: Cloudflare redirect and WAF rule ordering
- recommendation: apply an unconditional HTTP-to-HTTPS redirect before browser challenge/block processing and revalidate both hostnames

### OTERYN-PUBLIC-DOMAIN-005 — Canonical public URL conflicts with deployed staging `APP_URL`

- evidence_class: `CONFLICT`
- outcome: `FAIL` for configuration consistency
- severity: `MEDIUM`
- confidence: `HIGH` for mismatch; `MEDIUM` for user-visible impact
- affected surface: requestless absolute URL generation, including mail and scheduler contexts
- evidence: canonical root is HTTPS public hostname; exact staging `APP_URL` is loopback HTTP
- impact: requestless absolute URLs may contain an internal host or plaintext scheme
- ownership: Synology deployment configuration and URL-generation policy
- recommendation: configure the deployed application root as the canonical HTTPS URL or explicitly override every requestless link generator, then validate a redacted delivered link

### OTERYN-PUBLIC-DOMAIN-006 — Password-recovery delivery remains unproven

- evidence_class: `UNKNOWN`
- outcome: `BLOCKED`
- severity: `MEDIUM`
- confidence: `HIGH`
- affected surface: `/forgot-password` and delivered reset URL
- evidence: public GET is intercepted by Cloudflare; no controlled identity/mailbox was available
- impact: a critical account-recovery path may have an inaccessible entry page, delivery failure or incorrect URL origin without detection
- ownership: Identity mail delivery plus Cloudflare public access policy
- recommendation: after WWW access and `APP_URL` are corrected, execute one controlled recovery flow, inspect only a redacted host/scheme/path, and complete reset under explicit mutation authorization

### OTERYN-PUBLIC-DOMAIN-007 — WWW accepts TLS 1.3 only in the observed configuration

- evidence_class: `PROVEN`
- outcome: `RISK`
- severity: `LOW`
- confidence: `HIGH`
- affected surface: `oteryn.molehill.cloud`
- evidence: forced TLS 1.2 received `protocol version`; TLS 1.3 succeeded
- impact: clients limited to TLS 1.2 cannot connect
- ownership: Cloudflare minimum TLS policy
- recommendation: confirm TLS 1.3-only is an explicit compatibility decision; otherwise permit TLS 1.2 while retaining modern cipher policy

## Non-findings retained

- Gateway root `404` would be expected if TLS and routing worked because no root endpoint is contracted.
- Platform and Gateway hostnames are intentionally distinct.
- Synology origins remained loopback-bound in the last exact staging deployment.
- Gateway source does not expose service credentials or database credentials.
- Sensitive Gateway login responses have source-enforced no-store headers.
- No evidence supports current production correctness or exact current runtime identity.

## Required remediation and revalidation gate

Public-domain launch remains blocked until all launch-applicable items are directly proven:

1. working certificate and TLS negotiation for the exact Gateway hostname;
2. correct Gateway routing and JSON responses for `/health`, `/ready`, `/version` and bounded invalid login;
3. documented WWW Cloudflare policy that permits intended anonymous/browser/monitoring traffic;
4. HTTP-to-HTTPS redirect for both names;
5. positive reviewed HSTS after every included subdomain has valid HTTPS;
6. canonical HTTPS `APP_URL` or equivalent requestless URL-generation control;
7. controlled redacted password-recovery delivery proof;
8. exact current Platform, Gateway and Canary deployment identities;
9. Issue `#91` production go-live evidence if this is the production target.

## Validation record

- Repository/task/PR/ownership preflight: PASS.
- PR `#381` portal inventory reuse: PASS.
- Platform proxy/URL/cookie/cache/security source inspection: PASS.
- Gateway route/error/cache/transport source inspection: PASS.
- Last exact staging workflow/artifact inspection: PASS.
- Public DNS/TLS/HTTP run `30690877286`: PASS as evidence collection; findings detected.
- Independent corroboration run `30690957415`: PASS as evidence collection; findings reproduced.
- Public Gateway TLS: FAIL.
- Public WWW anonymous automated access: FAIL / Cloudflare `403`.
- Public HTTP-to-HTTPS redirect: FAIL.
- Public HSTS enforcement: FAIL.
- Controlled password recovery: NOT_RUN / exact blocker recorded.
- Secrets and credentials used: none.
- External infrastructure mutation: none.

## Acceptance disposition

The audit and evidence-collection task is complete. It produced direct public-edge evidence, independently corroborated the launch blockers, retained exact staging boundaries and recorded all remaining unknowns without unsupported inference.

`PRODUCTION_PROVEN`: **false**.  
`PUBLIC_DOMAIN_LAUNCH_READY`: **false**.
