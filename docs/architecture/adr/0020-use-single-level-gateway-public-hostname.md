# ADR 0020: Use a single-level public Game Gateway hostname

## Status

Accepted

## Context

Oteryn exposes two distinct HTTPS services through the same Cloudflare Tunnel:

- the Platform website at `oteryn.molehill.cloud`;
- the native-client Game Gateway at a separate public hostname.

The earlier Gateway hostname, `login.oteryn.molehill.cloud`, is a two-label hostname below the zone apex. The live Cloudflare audit proved that the zone's Universal certificate packs do not cover it and that Advanced Certificate Manager is not enabled. Enabling the paid add-on only to retain this hostname would add recurring cost without providing a product requirement.

Cloudflare Universal SSL already covers one-label hostnames under `molehill.cloud`, including `gateway.molehill.cloud`. A dedicated hostname remains preferable to path multiplexing under the website because it preserves a clear service boundary, requires no URL-prefix rewriting, and lets native clients use the Gateway's existing `/v1/*`, `/health`, `/ready`, and `/version` paths unchanged.

The repository owner approved the single-level hostname on 2026-08-01.

## Decision

Use the following canonical public endpoint mapping:

```text
https://oteryn.molehill.cloud
  -> Oteryn Platform
  -> http://127.0.0.1:8000

https://gateway.molehill.cloud
  -> Oteryn Game Gateway / native client login API
  -> http://127.0.0.1:8080
```

`login.oteryn.molehill.cloud` is a retired legacy hostname. The guarded Cloudflare migration may remove its Tunnel ingress rule and DNS record only when the live state exactly matches the repository-owned tunnel target and contains no conflicting record. Ambiguous legacy state must fail closed.

## Consequences

- Advanced Certificate Manager is not required for the canonical Gateway hostname.
- The existing Universal `*.molehill.cloud` edge certificate may cover `gateway.molehill.cloud`; live TLS acceptance still must prove the presented certificate after migration.
- Tunnel/DNS automation, edge auditing, public endpoint contracts, operational documentation, deployment validation, and native-client configuration must use the new hostname.
- The Gateway remains a separate service on Synology loopback port `8080`; this decision does not merge it with the Platform or place it under a website path.
- The broad country-based WAF block and Bot Fight Mode remain independent controls. The hostname migration does not weaken or silently change them.
- HSTS remains non-persistent until both canonical HTTPS endpoints pass public acceptance.

## Migration and rollback

Migration order:

1. validate the exact current Tunnel/DNS state;
2. add the new Gateway ingress and proxied CNAME;
3. verify the new hostname state;
4. retire only the exact safe legacy ingress and CNAME;
5. run independent DNS/TLS/HTTP acceptance.

The automation preserves unrelated Tunnel ingress rules and DNS records. A conflicting or ambiguous legacy record blocks mutation.

Rollback may restore the legacy ingress and proxied CNAME to the same tunnel target, but the legacy hostname is not expected to provide valid TLS without a separately enabled certificate product. Rollback therefore restores routing state only; it does not supersede the certificate evidence that motivated this decision.
