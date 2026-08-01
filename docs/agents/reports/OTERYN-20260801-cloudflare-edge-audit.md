# Cloudflare remaining-edge and token-capability audit

Task: `OTERYN-20260801-cloudflare-edge-audit`  
Tunnel/DNS apply: run `30700054602`  
Protected edge-audit implementation: PR `#406`, merge `5ea883c26dead9d58d363df1fb7909e3c399e206`  
Protected token-capability implementation: PR `#411`, merge `63771e2565dd0d691c8229d97090c0d0fcceb9c3`

## Current status

```text
CLOUDFLARE_INTEGRATION_AVAILABLE: true
TUNNEL_DNS_CONVERGED: true
ACCESS_APPLICATIONS_API_READABLE: true
CERTIFICATE_API_READABLE: false
RULESETS_API_READABLE: false
BOT_MANAGEMENT_API_READABLE: false
ZONE_SETTINGS_API_READABLE: false
TOKEN_SELF_MANAGEMENT_AVAILABLE: false
PUBLIC_DOMAIN_LAUNCH_READY: false
PRODUCTION_PROVEN: false
```

The protected token was rechecked after an external permission change. The change was only partially successful: Cloudflare Access applications are now readable, but all other API families required to diagnose the public failures remain permission-denied.

## Partial-scope recheck

Marker PR `#418` was closed without merge after evidence review.

```text
workflow_run: 30704310678
job: 91380665868
trusted_sha: 064643e01e56607739425f6936d24497cc450821
edge_observed_at_utc: 2026-08-01T14:42:02.055194+00:00
token_observed_at_utc: 2026-08-01T14:42:03.531877+00:00
artifact: 8819823874
artifact_digest: sha256:1b6ec2a8314b620737fc6e428db31b169c0289d23de7418b986b3078cbef2b52
mutation: none
```

Direct result:

- Access applications: `readable`;
- certificate packs: `permission_denied`;
- zone Rulesets: `permission_denied`;
- Bot Management: `permission_denied`;
- `always_use_https`: `permission_denied`;
- `min_tls_version`: `permission_denied`;
- `security_level`: `permission_denied`;
- `browser_check`: `permission_denied`;
- `security_header` / HSTS: `permission_denied`;
- token self-details: `permission_denied`;
- permission-group catalog: `permission_denied`.

The token therefore has usable Access read capability but still cannot inspect certificate coverage, WAF/Rulesets, Bot controls, redirect/TLS settings or HSTS. It also cannot inspect or expand its own policy.

## Earlier live evidence

The first trusted-main remaining-edge audit used run `30702383389`, job `91375538793`, artifact `8819238641`, digest `sha256:fce53d0651b496e42e56654bfdcad491afe2e01e80fea79e7e5b8630e38215ae`. At that time every remaining edge family, including Access, was permission-denied.

The account-token capability audit used run `30702827344`, job `91376706288`, artifact `8819368872`, digest `sha256:36797349c8b0b0250bfeea88cd92c77b730d7efb7c62b4137223ef8b938ec329`. It proved that the token cannot read its own policies or the account permission-group catalog.

## Remaining least-privilege prerequisite

An authorized Cloudflare administrator must edit or replace the protected GitHub environment secret `CLOUDFLARE_API_TOKEN`. Do not paste the token into chat, repository files, issue comments or workflow inputs.

Access read capability is already proven and does not need to be added again. The remaining token should receive only the read permissions needed for:

- SSL and certificate packs for the configured zone;
- Zone Settings;
- Rulesets products used by the zone, including WAF, redirects, transforms and configuration rules as applicable;
- Bot Management.

`Account API Tokens Write` is not required for the edge audit. Do not add broad write permissions before a permission-complete read-only audit identifies the exact resources requiring repair.

## Continuation

After the protected secret is corrected, rerun exactly one marker-only trusted-main audit by changing only:

```text
ops/triggers/cloudflare-edge-audit.md
```

Then:

1. capture exact certificate, Ruleset, Bot, redirect and HSTS state;
2. design the smallest fixed-scope apply automation;
3. run deterministic mock and exact-head validation;
4. execute an explicitly confirmed bounded apply;
5. repeat public TLS/HTTP acceptance;
6. execute controlled redacted password-recovery delivery only after WWW public access works.

## Current public state

No edge mutation occurred during the recheck. The latest direct public evidence therefore remains the previously proven failure state: Gateway TLS failed before HTTP, representative WWW routes returned Cloudflare `403`, plain HTTP did not redirect to HTTPS and WWW HSTS used `max-age=0`. A fresh public acceptance run is required after an actual edge configuration change.

## Security boundary

All live audits used trusted code from `main`, marker-only trigger pull requests and GET requests only. No Cloudflare configuration, DNS, Synology runtime, application code, database, Canary source, OTClient or secret value was changed or exposed.
