# Public-domain repair report

Task: `OTERYN-20260801-public-domain-repair`  
Phase: `implementation_and_staging_verification`  
Source validation: PR `#387` at `c8ca2fc995fbbc4a0f3c7268872d3843db950af8`  
Repair branch: `fix/OTERYN-20260801-public-domain-repair`  
Repair PR: `#388`

## Status

Repository-owned repair is implemented. External edge mutation and staging deployment were not executed because this session has no usable Cloudflare, DNS or Synology operator access and no explicit staging deployment authorization.

`PRODUCTION_PROVEN`: **false**.  
`PUBLIC_DOMAIN_LAUNCH_READY`: **false until the edge plan is applied and independently revalidated**.

## Proven source failure

PR #387 established the first relevant failure as `gateway-public-tls-handshake-failure`:

- `login.oteryn.molehill.cloud` failed TLS 1.2 and TLS 1.3 negotiation before HTTP;
- the observed certificate SANs `molehill.cloud` and `*.molehill.cloud` do not cover the deeper two-label hostname;
- representative WWW requests received Cloudflare `403` interstitials;
- HTTP did not redirect to HTTPS;
- WWW returned HSTS with `max-age=0`;
- exact staging used `APP_URL=http://127.0.0.1:8000` while the canonical public root is `https://oteryn.molehill.cloud`.

The repository contract remains:

```text
https://oteryn.molehill.cloud       -> http://127.0.0.1:8000
https://login.oteryn.molehill.cloud -> http://127.0.0.1:8080
```

## Repository repair

The repair candidate:

1. makes `https://oteryn.molehill.cloud` the guarded Synology public-staging `APP_URL`;
2. fails deployment configuration validation when `OTERYN_STAGING_APP_URL` differs from that exact origin;
3. keeps Platform and Gateway bindings on Synology loopback and does not alter origin exposure;
4. sets Secure session cookies for the guarded public staging workflow;
5. adds regression coverage for requestless login, password-reset and signed-route URL generation;
6. extends the bounded Synology health check to verify:
   - exact Gateway `/version` service identity and version;
   - malformed `/v1/login` returns bounded JSON `400`;
   - sensitive Gateway login responses retain private no-store/no-cache headers;
   - Platform and Gateway ports do not cross-route;
   - deployed requestless login, reset and signed URLs use the canonical HTTPS origin;
   - the existing trusted-proxy forwarded HTTPS login form remains canonical.

No broad proxy trust, public origin binding, authentication bypass, cache relaxation, Canary source change or production mutation was introduced.

## External operator change plan

This plan is intentionally not marked applied.

### 1. Capture current state and rollback material

Before mutation, record sanitized exports or screenshots of only the two canonical hostnames and their exact origin/rule dependencies:

- DNS records and proxy state;
- Cloudflare Tunnel public-hostname entries and target origins;
- edge certificate/custom-hostname coverage and certificate status;
- minimum TLS policy affecting each hostname;
- WAF, Bot, Access, rate-limit and challenge rules that match either hostname;
- HTTP redirect rules;
- HSTS settings and effective response-header transforms;
- Synology reverse-proxy or tunnel-origin mappings, if present.

Do not record tokens, private keys, cookies, credentials or private environment contents. Assign stable identifiers to every changed rule so each can be restored exactly.

### 2. Repair Gateway certificate coverage first

Provision or attach an edge certificate whose SAN directly covers:

```text
login.oteryn.molehill.cloud
```

Do not rely on `*.molehill.cloud` for this hostname. Preserve the canonical hostname. Verify the exact certificate chain, SAN and hostname with standards-compliant clients before changing later policies.

The minimum TLS version must follow the exact native-client/Gateway compatibility contract. The current evidence does not prove whether TLS 1.3-only is an intentional compatible policy, so do not guess or lower/raise it solely to satisfy the audit. Record the existing value, test the exact supported client, and retain the strongest compatible setting.

### 3. Repair exact Gateway routing and machine-API policy

Set the exact public-hostname route:

```text
login.oteryn.molehill.cloud -> http://127.0.0.1:8080
```

Reject or remove any mapping to Platform `8000`, Canary legacy login `7171`, game protocol `7172`, or unrelated ports.

For only this hostname and these contracted paths:

```text
/health
/ready
/version
/v1/login
```

remove browser-only JavaScript/managed challenges that prevent machine clients from receiving the Gateway response. Retain hostname/path-scoped rate limiting, request-size controls and abuse protection. Do not cache `/v1/login`; preserve the application's private no-store/no-cache response headers. Do not expose the loopback origin directly.

### 4. Repair WWW public policy without global weakening

Inspect the exact rules matching `oteryn.molehill.cloud`. Permit intended normal anonymous browser access to public routes and bounded health monitoring. Do not disable WAF, Bot or Access globally.

Use hostname/path-scoped rules. Keep appropriate stronger controls for login, registration, password recovery, MFA and administrator paths, but those controls must return usable browser/application behavior rather than an unintended blanket interstitial on every public route.

Keep the exact route:

```text
oteryn.molehill.cloud -> http://127.0.0.1:8000
```

and do not route it to Gateway or Canary ports.

### 5. Redirect HTTP before challenge processing

For both exact hostnames, configure plain HTTP to redirect to the exact HTTPS equivalent, preserving path and query where supported:

```text
http://oteryn.molehill.cloud/*       -> https://oteryn.molehill.cloud/*
http://login.oteryn.molehill.cloud/* -> https://login.oteryn.molehill.cloud/*
```

Place the redirect ahead of challenge/block behavior and verify there is no loop. Confirm the Gateway client contract accepts the redirect policy for accidental HTTP use; canonical clients must use HTTPS directly.

### 6. HSTS decision

Do not enable preload or `includeSubDomains` merely to satisfy validation. First prove valid HTTPS for every hostname included by the selected scope.

Until the Gateway certificate and included-subdomain posture are proven, keep HSTS deliberately disabled and remove contradictory decorative tokens where the edge permits. After proof, select one positive reviewed `max-age`, decide whether the policy is exact-host only or includes subdomains, and emit one non-contradictory header without `max-age=0`. Preload remains a separate owner decision with irreversible-operational implications.

### 7. Deploy and verify the repository candidate only with authorization

After exact-head CI passes and explicit staging authorization is recorded:

1. build/publish the exact approved Platform/Gateway revision through the established workflow;
2. deploy with `OTERYN_STAGING_APP_URL=https://oteryn.molehill.cloud`;
3. record exact Platform image/SHA, Gateway image/version and Canary image digest;
4. run the extended Synology health check;
5. run sanitized public DNS, TLS, redirect and route probes;
6. execute one controlled redacted password-recovery flow only when an authorized controlled identity and mailbox exist;
7. record host, scheme and path only—never the token.

Staging evidence remains `STAGING_PROVEN`, not production proof.

## External acceptance probes

After the operator changes, verify in this order:

1. certificate SAN and hostname verification for `login.oteryn.molehill.cloud`;
2. supported TLS versions using the exact client contract;
3. Gateway `/health`, `/ready` and `/version` bounded JSON;
4. malformed/invalid `/v1/login` bounded client error and private no-store headers;
5. no HTML challenge on Gateway machine paths;
6. anonymous WWW public-route reachability;
7. HTTP-to-HTTPS redirect chains for both hostnames;
8. explicit effective HSTS state;
9. negative cross-routing probes in both directions;
10. canonical requestless reset/verification/signed URLs;
11. controlled redacted password-recovery delivery when authorized.

## Rollback

Repository rollback before merge is a PR revert or branch reset to the recorded baseline. The runtime deployment procedure already snapshots Platform/Gateway/Canary image references and supports runtime-image rollback; it intentionally does not reverse database migrations automatically.

External rollback must restore the exact pre-change certificate attachment, tunnel routes, WAF/Bot/Access/rate-limit rules, redirect rules and HSTS configuration captured in step 1. If Gateway verification fails after any edge change, restore its previous exact hostname rule set without changing WWW or unrelated subdomains. No external rollback was required in this session because no external mutation occurred.

## Remaining blocker

Usable Cloudflare, DNS and Synology operator access plus explicit staging deployment authorization are absent. Therefore Gateway TLS, edge routing, WAF/challenge behavior, redirects, HSTS and public password-recovery delivery remain unproven after the repository repair.
