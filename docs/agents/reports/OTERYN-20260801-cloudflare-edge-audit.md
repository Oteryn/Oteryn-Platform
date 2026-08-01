# Cloudflare remaining-edge audit

Task: `OTERYN-20260801-cloudflare-edge-audit`

## Current classification

```text
CLOUDFLARE_INTEGRATION_AVAILABLE: true
TUNNEL_DNS_AUTOMATION_AVAILABLE: true
DEDICATED_ZONE_READ_TOKEN_ACTIVE: true
REMAINING_EDGE_API_READABLE: true
CANONICAL_GATEWAY_HOST: gateway.molehill.cloud
ADVANCED_CERTIFICATE_MANAGER_REQUIRED: false
WAF_BOT_WRITE_SCOPE_AVAILABLE: false
PUBLIC_DOMAIN_LAUNCH_READY: false
PRODUCTION_PROVEN: false
```

## Trusted read-only evidence

```text
permission_complete_edge_run: 30708559130
permission_complete_edge_job: 91391822768
permission_complete_edge_artifact: 8821103628
permission_complete_edge_digest: sha256:95fe01f1ebeec45aabad5c0e5c71e7cea866224b6e1f9648674949b508321128
apply_preflight_run: 30709108382
apply_preflight_job: 91393282575
apply_preflight_artifact: 8821278907
apply_preflight_digest: sha256:520bdbf591388ff30bba4cce232be413bab671ff040b6fb619e2c933d4553559
mutation: none
```

Raw rule expressions, country literals, authorization headers and token values were not emitted.

## Certificate finding and superseding decision

The live audit proved that Universal certificate packs did not cover the earlier two-label hostname:

```text
login.oteryn.molehill.cloud
```

The Advanced Certificate Manager quota was:

```text
advanced.allocated: 0
advanced.used: 0
```

The repository owner subsequently approved the single-level canonical Gateway hostname:

```text
gateway.molehill.cloud
```

ADR 0020 records this decision. A normal Universal certificate entry `*.molehill.cloud` covers exactly one label such as `gateway.molehill.cloud`, while it does not cover `login.oteryn.molehill.cloud`. Therefore:

```text
Advanced Certificate Manager purchase: not required
SSL and Certificates Edit for certificate ordering: not required
```

The trusted audit must still re-run after Tunnel/DNS migration and public TLS must be validated independently. Certificate eligibility is not the same as proof of the currently presented certificate.

## Custom WAF finding

Zone custom firewall ruleset:

```text
ruleset_id: 67ca2e19272a4c7d97c2a53681d0eb2f
phase: http_request_firewall_custom
rule_count: 3
```

Exactly one enabled terminating rule broadly affects Oteryn:

```text
rule_id: e0f91939eb494d4490d975498a9a9724
action: block
enabled: true
host_scope: broad_no_host_predicate
expression_sha256: 3f5a9e27f91d9cfe4fb6f77ede8c1e91997ef32a91a443cd1e6b61211ff13c45
field: ip.geoip.country
operator: ne
```

The rule has no host, path or method predicate. It blocks traffic outside one configured country and explains Cloudflare `403` observations from external runners. The repair must preserve the country restriction for unrelated services and exempt only the two canonical Oteryn hosts.

## Bot, browser, HTTPS and Access state

```text
bot_fight_mode: true
javascript_detections: true
browser_check: on
security_level: high
always_use_https: on
minimum_tls_version: 1.3
hsts_enabled: true
hsts_max_age: 0
hsts_include_subdomains: true
hsts_preload: true
access_applications_matching_canonical_hosts: 0
```

Bot Fight Mode is zone-wide and can interfere with the machine/native-client Gateway. Browser Check and `security_level=high` remain unchanged until post-WAF probes show whether they are still relevant. HSTS remains deliberately non-persistent while public Gateway TLS is unproven.

## Guarded hostname migration

The fixed-scope endpoint automation now targets:

```text
oteryn.molehill.cloud  -> http://127.0.0.1:8000
gateway.molehill.cloud -> http://127.0.0.1:8080
```

It treats `login.oteryn.molehill.cloud` as retired and may delete its DNS record only when it is exactly one `CNAME` pointing to the same managed Oteryn tunnel. It preserves unrelated Tunnel ingress and DNS state and verifies the new canonical route before legacy DNS retirement.

## Remaining minimal repair design

1. Merge and run the endpoint audit from trusted `main`.
2. Apply the guarded hostname migration through the protected environment and exact confirmation phrase.
3. Re-run the GET-only edge audit to prove active Universal wildcard coverage for `gateway.molehill.cloud`.
4. Run independent DNS/TLS/HTTP acceptance for WWW and Gateway.
5. Add only these zone-scoped permissions to the dedicated edge token when ready for the separate policy repair:

```text
Zone WAF Edit
Bot Management Edit
```

6. Insert an exact-host exception for `oteryn.molehill.cloud` and `gateway.molehill.cloud` before the broad country block.
7. Disable Bot Fight Mode for native Gateway compatibility.
8. Preserve Browser Check, security level, Always Use HTTPS and HSTS max-age until bounded post-apply evidence justifies any further change.
9. Record exact changed resource IDs and same-run rollback evidence.

`Zone Settings Edit`, `SSL and Certificates Edit`, account-wide write permissions and Advanced Certificate Manager are not prerequisites for this migration.

## Security boundary

No WAF, Bot, certificate, Access, HSTS, application, database, Canary or OTClient mutation is included in the hostname implementation PR. Live Tunnel/DNS mutation remains available only through trusted `main`, the protected `production-cloudflare` environment and the exact apply confirmation.
