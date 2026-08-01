# Cloudflare edge audit result

Task: `OTERYN-20260801-cloudflare-edge-audit`  
Trusted implementation: `5ea883c26dead9d58d363df1fb7909e3c399e206`  
Implementation PR: `#406`  
Marker-only trigger PR: `#408` — closed without merge

## Live evidence

```text
workflow_run: 30702383389
job: 91375538793
observed_at_utc: 2026-08-01T13:46:29.646815+00:00
artifact_id: 8819238641
artifact_digest: sha256:fce53d0651b496e42e56654bfdcad491afe2e01e80fea79e7e5b8630e38215ae
mutation: none
trusted_code_source: main@5ea883c26dead9d58d363df1fb7909e3c399e206
```

## Result

The protected read-only execution boundary passed:

- the implementation was loaded from trusted `main`;
- the trigger branch changed only the inert marker;
- the Cloudflare token verified as active;
- the audit issued GET requests only;
- no token or secret value was emitted;
- no Cloudflare mutation occurred.

The configured account-owned token does not have read access to any remaining edge-policy family. Every relevant API family returned `permission_denied`:

- certificate packs;
- zone Rulesets;
- Bot Management;
- Access applications;
- `always_use_https`;
- `min_tls_version`;
- `security_level`;
- `browser_check`;
- `security_header` / HSTS.

## Classification

```text
AUDIT_IMPLEMENTATION: PROVEN
TRUSTED_EXECUTION_BOUNDARY: PROVEN
TOKEN_ACTIVE: PROVEN
EDGE_CONFIGURATION_STATE: UNKNOWN
EXTERNAL_MUTATION: NONE
PUBLIC_DOMAIN_LAUNCH_READY: false
PRODUCTION_PROVEN: false
```

The denied endpoints are not evidence that the corresponding features are absent or disabled. Certificate coverage, the source of the WWW challenge, redirect ownership and effective HSTS configuration remain unknown.

## Least-privilege read scope required

The protected `production-cloudflare` token needs read access for the configured account and the `molehill.cloud` zone to the families queried by the audit:

- `SSL and Certificates Read`;
- `Zone Settings Read`;
- `Bot Management Read`;
- `Access: Apps and Policies Read`;
- the applicable Rulesets read groups needed for the enabled rule families, including WAF, redirects, transform and configuration rules.

Do not add edit/write permission merely to complete this audit. Keep the token restricted to the intended account and zone.

## Next action

Update or replace the protected `CLOUDFLARE_API_TOKEN` with the required least-privilege read capabilities, then create a new marker-only trigger PR and review the sanitized artifact before authorizing any edge mutation.
