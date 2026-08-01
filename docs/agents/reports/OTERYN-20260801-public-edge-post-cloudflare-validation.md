# Public edge validation after Cloudflare endpoint apply

Task: `OTERYN-20260801-public-domain-repair`  
Observation type: public, read-only, GitHub-hosted runner  
Cloudflare endpoint apply reference: run `30700054602`  
Validation PR: `#407`  
Validation workflow run: `30701999967` / `Public Edge Post-Cloudflare Validation` #1  
Job: `91374523427`  
Runner region: West US  
Observation time: `2026-08-01T13:35:12.890573+00:00`

## Evidence identity

```text
observation_branch: audit/OTERYN-20260801-public-domain-post-cloudflare-validation
observation_head: cbc8937a34218347cae79b2f44caefb8961e80f6
pull_request_merge_ref: fc3b075637876f27a56dbe3f6451fe24bfb178af
artifact_id: 8819120004
artifact_name: public-edge-post-cloudflare-fc3b075637876f27a56dbe3f6451fe24bfb178af
artifact_digest: sha256:0d776dca5fd73d5faf05c971aaa51cdbeb8aa498883b08873c12c6d07e843579
artifact_expires_at: 2026-08-15T13:35:14Z
```

The artifact contains bounded DNS, TLS, certificate, HTTP status, redirect, response-header and sampled-body metadata. It contains no credentials, cookies, password-reset token, valid Game Login Ticket, private endpoint, Cloudflare token or production secret.

## Verdict

**PUBLIC DOMAIN ACCEPTANCE: FAIL.**

The authorized Cloudflare endpoint reconciliation successfully made the remote Tunnel and both DNS records current, but it did not repair the remaining certificate and edge-policy failures.

## Proven results

### DNS

Both canonical names resolved to the expected Cloudflare anycast addresses:

```text
A     104.21.2.166
A     172.67.186.250
AAAA  2606:4700:3031::6815:2a6
AAAA  2606:4700:3033::ac43:bafa
```

### WWW hostname

Target: `oteryn.molehill.cloud`

- TLS 1.3 hostname verification: PASS.
- TLS 1.2 hostname verification: FAIL.
- certificate extraction: PASS.
- every representative route returned Cloudflare `403` with `Just a moment...` for both a bounded machine user agent and a current Chrome-like user agent:
  - `/`;
  - `/login?locale=en`;
  - `/register`;
  - `/forgot-password`;
  - `/news`;
  - `/highscores`.
- effective HSTS remained `Strict-Transport-Security: max-age=0; includeSubDomains; preload`.
- plain HTTP returned Cloudflare `403` rather than an HTTPS redirect.

### Gateway hostname

Target: `login.oteryn.molehill.cloud`

- TLS 1.2 hostname verification: FAIL before HTTP.
- TLS 1.3 hostname verification: FAIL before HTTP.
- certificate extraction: FAIL.
- `/health`, `/ready`, `/version`, negative `/login` and bounded invalid `POST /v1/login` received no HTTP response because TLS failed first.
- plain HTTP `/health` returned Cloudflare `403` rather than an HTTPS redirect.

### Routing boundary

The WWW `/version` observation did not expose the Gateway service identity. The Gateway-side negative cross-routing check could not execute because Gateway TLS failed.

## Acceptance matrix

```text
dns_www: PASS
dns_gateway: PASS
tls_www: PASS
tls_gateway: FAIL
www_browser_public: FAIL
gateway_health: FAIL
gateway_ready: FAIL
gateway_version: FAIL
gateway_invalid_login: FAIL
gateway_no_www_cross_route: BLOCKED_BY_TLS
www_no_gateway_cross_route: PASS
http_redirect_www: FAIL
http_redirect_gateway: FAIL
positive_hsts_www: FAIL
```

## Boundary conclusion

The following are separately proven complete:

- repository-owned canonical `APP_URL` and requestless URL repair;
- exact Synology staging deployment `3eb109b505f7d1c8718cffb823de6d9d5166717c`;
- remote Cloudflare Tunnel mapping and canonical DNS reconciliation from apply run `30700054602`.

The remaining failures are not caused by the Synology images, host bindings, canonical URL generation, Tunnel hostname-to-origin entries or DNS records. They remain in Cloudflare certificate coverage and edge policy layers that the endpoint reconciler intentionally does not manage:

- certificate coverage for `login.oteryn.molehill.cloud`;
- WWW challenge/WAF/Bot/Access behavior;
- HTTP-to-HTTPS redirect ordering;
- HSTS configuration;
- controlled password-recovery acceptance after public access is restored.

## Classification

```text
STAGING_PROVEN: true
CLOUDFLARE_TUNNEL_AND_DNS_CURRENT: true
PUBLIC_DOMAIN_LAUNCH_READY: false
PRODUCTION_PROVEN: false
```

## Required next action

Create a separate fail-closed Cloudflare edge-policy audit/apply path that can inspect and narrowly manage certificate coverage, hostname-scoped challenge policy, HTTP redirects and HSTS for only the two canonical Oteryn hosts. Run it in audit mode first, review exact drift, then apply only explicitly authorized changes and repeat this public acceptance sequence.

No external mutation or rollback was performed by this validation.
