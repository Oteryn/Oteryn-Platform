# Cloudflare remaining-edge audit

Task: `OTERYN-20260801-cloudflare-edge-audit`

## Current classification

```text
CLOUDFLARE_INTEGRATION_AVAILABLE: true
TUNNEL_DNS_CONVERGED: true
DEDICATED_ZONE_READ_TOKEN_ACTIVE: true
REMAINING_EDGE_API_READABLE: true
ADVANCED_CERTIFICATE_QUOTA: 0
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
apply_preflight_pr: 425
apply_preflight_merge: ee38558a8420c8c32a8cfa92b69e60910e1695c5
```

The zone collector uses the protected `CLOUDFLARE_EDGE_AUDIT_TOKEN`. The existing account-owned `CLOUDFLARE_API_TOKEN` remains isolated to Tunnel/DNS and account-scoped reads. Live collectors execute from trusted `main`, issue GET requests only and emit sanitized artifacts.

## Permission-complete edge evidence

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

## Fixed-scope apply preflight

Marker PR `#426` was closed without merge.

```text
workflow_run: 30709108382
job: 91393282575
trusted_sha: ee38558a8420c8c32a8cfa92b69e60910e1695c5
artifact: 8821278907
artifact_digest: sha256:520bdbf591388ff30bba4cce232be413bab671ff040b6fb619e2c933d4553559
observation_time_utc: 2026-08-01T16:53:07.607034+00:00
mutation: none
```

Raw rule expressions, country literals, authorization headers and token values were not emitted.

## Exact findings

### Gateway certificate coverage

Two Universal certificate packs exist. Neither covers the canonical multi-level hostname:

```text
login.oteryn.molehill.cloud
```

The advanced certificate quota endpoint returned:

```text
advanced.allocated: 0
advanced.used: 0
```

Advanced Certificate Manager is not enabled for the zone. The canonical Gateway hostname cannot obtain the required multi-level edge certificate through the current Universal SSL coverage. The hostname contract must not be changed merely to fit the existing certificate.

### Custom WAF

Zone custom firewall ruleset:

```text
ruleset_id: 67ca2e19272a4c7d97c2a53681d0eb2f
phase: http_request_firewall_custom
rule_count: 3
```

Exactly one enabled terminating rule broadly affects Oteryn:

```text
rule_id: e0f91939eb494d4490d975498a9a9724
ref: e0f91939eb494d4490d975498a9a9724
action: block
enabled: true
host_scope: broad_no_host_predicate
expression_sha256: 3f5a9e27f91d9cfe4fb6f77ede8c1e91997ef32a91a443cd1e6b61211ff13c45
fingerprint_matches_preflight: true
```

Safe expression classification:

```text
field: ip.geoip.country
operator: ne
host predicate: absent
path predicate: absent
method predicate: absent
quoted literal count: 1
```

The rule blocks traffic outside one configured country and has no hostname boundary. This directly explains the Cloudflare `403` observations from GitHub-hosted runners in other regions. A future apply must preserve the country restriction for unrelated services and add an exact Oteryn hostname exception before this block rather than deleting or disabling the broad rule globally.

Other custom rules:

```text
616428125f9b4f9bbaee3e12ad671341: enabled skip, zone-domain-scoped
e011766c505043a59d4c38499de3b558: disabled broad skip
```

### Bot and browser protections

```text
bot_fight_mode: true
javascript_detections: true
browser_check: on
security_level: high
```

Bot Fight Mode is zone-wide and can interfere with API/native-client traffic. It cannot be bypassed by a normal custom WAF exception when only the zone-wide Bot Fight Mode product is available.

### HTTPS and HSTS

```text
always_use_https: on
minimum_tls_version: 1.3
hsts_enabled: true
hsts_max_age: 0
hsts_include_subdomains: true
hsts_preload: true
```

The HTTP redirect setting itself is on. Earlier public `403` responses occurred before the redirect became observable. HSTS remains non-persistent because `max_age=0`; positive include-subdomains/preload HSTS must not be enabled until valid TLS is proven for every included hostname.

### Cloudflare Access

Eight Access applications are readable. None targets either canonical Oteryn hostname. Access is not the owner of the observed public block.

## Minimal repair design

The next implementation must be separately validated, fixed-scope and reversible:

1. Enable Advanced Certificate Manager for `molehill.cloud` through the Cloudflare dashboard.
2. Add only the required zone-scoped write permissions to the dedicated token.
3. Verify the WAF ruleset/rule IDs and expression fingerprint immediately before mutation.
4. Insert an exact-host `skip` rule for `oteryn.molehill.cloud` and `login.oteryn.molehill.cloud` before the country block, preserving the block for unrelated hostnames.
5. Disable Bot Fight Mode because the canonical Gateway is a machine/native-client API; preserve unrelated settings unless directly proven harmful.
6. Leave Browser Check and `security_level=high` unchanged initially; change them only if post-WAF public probes still show Cloudflare interception.
7. Preserve `always_use_https=on`.
8. Keep HSTS non-persistent until the Gateway certificate is active and both canonical HTTPS paths pass.
9. Order an Advanced certificate containing the zone apex and `login.oteryn.molehill.cloud` after entitlement and quota become available.
10. Record every created or changed resource ID and implement same-run rollback for each mutation.
11. Repeat public TLS, redirect, WWW, Gateway and cache-control acceptance from an independent network.

## Exact external prerequisites

An authorized administrator must:

1. Enable the paid Advanced Certificate Manager add-on for `molehill.cloud`.
2. Add only these zone-scoped permissions to `Oteryn Edge Audit`:

```text
Zone WAF Edit
Bot Management Edit
Zone Settings Edit
SSL and Certificates Edit
```

Do not add account-wide write permissions. Do not replace or weaken the existing Tunnel/DNS token.

## Security boundary

No Cloudflare mutation occurred in runs `30708157965`, `30708559130` or `30709108382`. No DNS, Tunnel, Synology, application, database, Canary or OTClient change occurred. Tunnel/DNS convergence from run `30700054602` remains independently proven.
