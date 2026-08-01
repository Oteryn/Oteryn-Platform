# Cloudflare remaining-edge and token-capability audit

Task: `OTERYN-20260801-cloudflare-edge-audit`  
Tunnel/DNS apply: run `30700054602`  
Protected edge-audit implementation: PR `#406`, merge `5ea883c26dead9d58d363df1fb7909e3c399e206`  
Protected token-capability implementation: PR `#411`, merge `63771e2565dd0d691c8229d97090c0d0fcceb9c3`

## Status

```text
CLOUDFLARE_INTEGRATION_AVAILABLE: true
TUNNEL_DNS_CONVERGED: true
REMAINING_EDGE_API_READABLE: false
TOKEN_SELF_MANAGEMENT_AVAILABLE: false
PUBLIC_DOMAIN_LAUNCH_READY: false
PRODUCTION_PROVEN: false
```

The existing Cloudflare integration is functional and already reconciled the two canonical Tunnel ingress entries. Both canonical proxied DNS records were current. The remaining blocker is not missing integration access; it is the permission boundary of the protected account-owned token.

## Live remaining-edge audit

Trusted-main GET-only audit:

```text
workflow_run: 30702383389
job: 91375538793
trusted_sha: 5ea883c26dead9d58d363df1fb7909e3c399e206
artifact: 8819238641
artifact_digest: sha256:fce53d0651b496e42e56654bfdcad491afe2e01e80fea79e7e5b8630e38215ae
observation_time_utc: 2026-08-01T13:46:29.646815+00:00
mutation: none
```

The token authenticated successfully, but Cloudflare returned `permission_denied` for every API family required to inspect the remaining public failures:

- certificate packs and exact coverage for `login.oteryn.molehill.cloud`;
- zone Rulesets;
- Bot Management;
- Access applications;
- `always_use_https`;
- `min_tls_version`;
- `security_level`;
- `browser_check`;
- `security_header` / HSTS.

Therefore the audit could not identify the certificate product, challenge owner, redirect rule, HSTS owner or exact policy IDs.

## Live account-token capability audit

Trusted-main GET-only capability audit:

```text
workflow_run: 30702827344
job: 91376706288
trusted_sha: 63771e2565dd0d691c8229d97090c0d0fcceb9c3
artifact: 8819368872
artifact_digest: sha256:36797349c8b0b0250bfeea88cd92c77b730d7efb7c62b4137223ef8b938ec329
observation_time_utc: 2026-08-01T13:59:14.398267+00:00
mutation: none
```

Direct result:

```text
self_details: permission_denied
permission_group_catalog: permission_denied
Account API Tokens Read proven: false
Account API Tokens Write proven: false
```

The current credential cannot read its own policies or the account permission-group catalog. It therefore cannot safely inspect, update or replace its own permission policy through the existing automation.

## Exact external prerequisite

An account administrator must rotate or replace the `production-cloudflare` environment secret `CLOUDFLARE_API_TOKEN`. Do not paste the token into chat, repository files, issue comments or workflow inputs.

The replacement token should initially receive only the read permissions needed to complete the existing audit:

- zone SSL/certificate read access;
- zone settings read access;
- read access for the Rulesets products used by the zone, including redirects, configuration/transform rules and WAF as applicable;
- Bot Management read access;
- Access applications and policies read access.

`Account API Tokens Write` is not required for the edge audit and should not be added merely to let an operational token self-elevate. Token administration should remain an external administrator action.

After the read audit succeeds, add only the corresponding write permissions for the exact products and resources proven to require repair. Do not grant broad account-wide write access in advance.

## Automatic continuation after token replacement

No repository change is required to resume. The existing trusted-main workflow can be retriggered through a marker-only pull request that modifies only:

```text
ops/triggers/cloudflare-edge-audit.md
```

Required sequence:

1. replace the protected environment token;
2. rerun the GET-only remaining-edge audit;
3. capture exact certificate, Ruleset, Bot, Access, redirect and HSTS state;
4. design the smallest fixed-scope apply automation;
5. run deterministic mock and exact-head validation;
6. perform an explicitly confirmed live apply;
7. rerun public TLS/HTTP acceptance;
8. execute controlled redacted password-recovery delivery only after WWW public access is usable.

## Current public state

Tunnel/DNS convergence did not repair the separately controlled edge policies. Public revalidation after apply still proved:

- Gateway TLS fails before HTTP;
- representative WWW routes return Cloudflare `403` interstitials;
- plain HTTP does not redirect to HTTPS;
- WWW HSTS remains `max-age=0`.

No production application, Canary source or OTClient mutation is required to address these specific remaining findings.

## Rollback and security boundary

Both live audits used trusted code from `main`, marker-only trigger pull requests and GET requests only. No Cloudflare mutation occurred. The existing Tunnel/DNS apply remains independently proven and does not need rollback.

The next mutation path must preserve unrelated Cloudflare configuration, record exact changed resource IDs, verify after each bounded change and restore only those exact items if public acceptance fails.
