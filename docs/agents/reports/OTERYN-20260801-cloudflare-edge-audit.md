# Cloudflare remaining-edge audit

Task: `OTERYN-20260801-cloudflare-edge-audit`

## Current classification

```text
CLOUDFLARE_INTEGRATION_AVAILABLE: true
TUNNEL_DNS_CONVERGED: true
DEDICATED_ZONE_READ_TOKEN_ACTIVE: true
REMAINING_EDGE_API_READABLE: true
EXTERNAL_WRITE_SCOPE_AVAILABLE: false
PUBLIC_DOMAIN_LAUNCH_READY: false
PRODUCTION_PROVEN: false
```

## Trusted implementation

```text
initial_edge_audit_pr: 406
initial_edge_audit_merge: 5ea883c26dead9d58d363df1fb7909e3c399e206
account_token_capability_pr: 411
account_token_capability_merge: 63771e2565dd0d691c8229d97090c0d0fcceb9c3
dedicated_user_token_wiring_pr: 420
dedicated_user_token_wiring_merge: d4a3c0c56673ac1ff918f5be94d0b3be0bfe7ec3
sanitized_rule_scope_pr: 422
sanitized_rule_scope_merge: 4dec2825a9375040dcee01a5dde5426d102ffe35
```

The zone collector uses the protected `CLOUDFLARE_EDGE_AUDIT_TOKEN`. The existing account-owned `CLOUDFLARE_API_TOKEN` remains isolated to Tunnel/DNS and account-scoped reads. Live collectors execute from trusted `main`, issue GET requests only and emit sanitized artifacts.

## Permission-complete live evidence

Marker PR `#423` was closed without merge.

```text
workflow_run: 30708559130
job: 91391822768
trusted_sha: 4dec2825a9375040dcee01a5dde5426d102ffe35
artifact: 8821103628
artifact_digest: sha256:95fe01f1ebeec45aabad5c0e5c71e7cea866224b6e1f9648674949b508321128
observation_time_utc: 2026-08-01T16:37:46.776730+00:00
mutation: none
```

Raw rule expressions, authorization headers and token values were not emitted.

## Exact findings

### Certificate coverage

Two active or issued Universal certificate packs exist. Neither covers the canonical multi-level hostname:

```text
login.oteryn.molehill.cloud
```

Universal `*.molehill.cloud` coverage is insufficient for this two-label hostname. The canonical hostname contract must not be changed merely to fit the existing certificate.

### Custom WAF

Zone custom firewall ruleset:

```text
ruleset_id: 67ca2e19272a4c7d97c2a53681d0eb2f
phase: http_request_firewall_custom
rule_count: 3
```

Exactly one enabled terminating candidate can broadly affect Oteryn:

```text
rule_id: e0f91939eb494d4490d975498a9a9724
ref: e0f91939eb494d4490d975498a9a9724
action: block
enabled: true
host_scope: broad_no_host_predicate
expression_sha256: 3f5a9e27f91d9cfe4fb6f77ede8c1e91997ef32a91a443cd1e6b61211ff13c45
```

Other rules:

```text
616428125f9b4f9bbaee3e12ad671341: enabled skip, zone-domain-scoped
e011766c505043a59d4c38499de3b558: disabled broad skip
```

The broad active block is the first evidence-supported owner of the public WWW `403` response. A future apply must preserve the block for unrelated traffic and add or verify an exact-host exception rather than deleting the entire ruleset.

### Bot and browser protections

```text
bot_fight_mode: true
javascript_detections: true
browser_check: on
security_level: high
```

Bot Fight Mode is zone-wide and can interfere with API/native-client traffic. It cannot be scoped away with a normal WAF custom-rule exception on plans where only Bot Fight Mode is available.

### HTTPS and HSTS

```text
always_use_https: on
minimum_tls_version: 1.3
hsts_enabled: true
hsts_max_age: 0
hsts_include_subdomains: true
hsts_preload: true
```

The HTTP redirect setting itself is on. Previous public `403` responses occurred before the redirect could be observed. HSTS remains deliberately non-persistent because `max_age=0`; positive include-subdomains/preload HSTS must not be enabled until valid TLS is proven for every included hostname.

### Cloudflare Access

Eight Access applications are readable. None targets either canonical Oteryn hostname. Access is therefore not the owner of the observed public block.

## Minimal repair design

The next implementation must be a separately validated, fixed-scope and reversible apply path:

1. Verify the exact ruleset and blocking-rule IDs and the stored expression fingerprint immediately before mutation.
2. Add an exact-host skip/exemption before the broad block, preserving unrelated WAF behavior.
3. Disable Bot Fight Mode for the zone when native Gateway/API compatibility requires it; preserve unrelated bot settings unless explicitly proven harmful.
4. Disable Browser Check only if bounded post-WAF validation still shows Cloudflare interception.
5. Preserve `always_use_https=on`.
6. Keep HSTS non-persistent until Gateway certificate coverage and both public HTTPS paths pass.
7. Order an Advanced Certificate containing the zone apex and `login.oteryn.molehill.cloud`, but only after Advanced Certificate Manager entitlement and certificate quota are proven.
8. Record every created or changed resource ID and implement same-run rollback for each mutation.
9. Repeat public TLS, redirect, WWW, Gateway and cache-control acceptance from an independent network.

## Exact external prerequisite

The dedicated user token currently has read permissions only. Before apply, an authorized administrator must add only these zone-scoped permissions for `molehill.cloud`:

```text
Zone WAF Edit
Bot Management Edit
Zone Settings Edit
SSL and Certificates Edit
```

Do not add account-wide write permissions. `SSL and Certificates Edit` alone does not purchase or enable Advanced Certificate Manager; the zone must separately have that paid entitlement before an advanced multi-level certificate can be ordered.

## Security boundary

No Cloudflare mutation occurred in runs `30708157965` or `30708559130`. No DNS, Tunnel, Synology, application, database, Canary or OTClient change occurred. Tunnel/DNS convergence from run `30700054602` remains independently proven.
