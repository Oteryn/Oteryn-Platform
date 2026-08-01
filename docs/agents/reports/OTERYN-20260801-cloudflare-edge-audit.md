# Cloudflare remaining-edge and token-capability audit

Task: `OTERYN-20260801-cloudflare-edge-audit`

## Trusted implementation

```text
edge_audit_pr: 406
edge_audit_merge: 5ea883c26dead9d58d363df1fb7909e3c399e206
token_capability_pr: 411
token_capability_merge: 63771e2565dd0d691c8229d97090c0d0fcceb9c3
```

Both collectors are GET-only, run live only from trusted `main` code behind the protected `production-cloudflare` environment and emit sanitized evidence.

## First remaining-edge audit

Marker PR `#408` was closed without merge after evidence review.

```text
workflow_run: 30702383389
job: 91375538793
observed_at_utc: 2026-08-01T13:46:29.646815+00:00
artifact_id: 8819238641
artifact_digest: sha256:fce53d0651b496e42e56654bfdcad491afe2e01e80fea79e7e5b8630e38215ae
mutation: none
```

The account-owned token verified as active, but every remaining edge family returned `permission_denied`:

- certificate packs;
- zone Rulesets;
- Bot Management;
- Access applications;
- Always Use HTTPS;
- minimum TLS version;
- security level;
- browser check;
- security header / HSTS.

## Account-token capability audit

Marker PR `#413` was closed without merge after evidence review.

```text
workflow_run: 30702827344
job: 91376706288
edge_observed_at_utc: 2026-08-01T13:59:13.709937+00:00
token_observed_at_utc: 2026-08-01T13:59:14.398267+00:00
artifact_id: 8819368872
artifact_digest: sha256:36797349c8b0b0250bfeea88cd92c77b730d7efb7c62b4137223ef8b938ec329
mutation: none
```

The token cannot inspect or extend its own policy:

- its own token-details endpoint returned `permission_denied`;
- the account permission-group catalog returned `permission_denied`;
- `Account API Tokens Read` is not proven;
- `Account API Tokens Write` is not proven;
- all remaining edge reads remain `permission_denied`.

## Classification

```text
TRUSTED_AUDIT_IMPLEMENTATION: PROVEN
TOKEN_ACTIVE: PROVEN
TOKEN_SELF_INSPECTION: BLOCKED
TOKEN_SELF_EXPANSION: BLOCKED
EDGE_CONFIGURATION_STATE: UNKNOWN
CLOUDFLARE_MUTATION: NONE
PUBLIC_DOMAIN_LAUNCH_READY: false
PRODUCTION_PROVEN: false
```

The permission-denied results are not evidence that certificates, rules, Access applications or settings are absent. The current values remain unknown.

## Required external action

An authorized Cloudflare administrator must create or edit an account/zone-bounded API token outside this repository and replace the protected GitHub environment secret `CLOUDFLARE_API_TOKEN`.

The first replacement should contain read permissions only:

- `SSL and Certificates Read`;
- `Zone Settings Read`;
- `Bot Management Read`;
- `Access: Apps and Policies Read`;
- the applicable Rulesets read permissions for WAF, redirects, transforms and configuration rules;
- `Page Rules Read` when the separate zone-edge collector is retained.

Do not add write permissions until a permission-complete read-only audit identifies the exact controls that require changes.

After secret replacement, run exactly one marker-only trusted-main audit, review the sanitized artifact, and design only the smallest evidence-supported apply path.
