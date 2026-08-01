# Oteryn public edge validation

## Purpose

`.github/workflows/oteryn-public-edge-validation.yml` combines two read-only observations after Cloudflare Tunnel/DNS migration:

1. sanitized Cloudflare edge configuration for the canonical Oteryn hosts;
2. independent public DNS, TLS and HTTP behavior from a GitHub-hosted runner.

Canonical hosts:

```text
oteryn.molehill.cloud
gateway.molehill.cloud
```

The retired `login.oteryn.molehill.cloud` hostname is inspected only as historical/negative certificate state by the Cloudflare collector.

## Trust boundary

Pull requests run deterministic offline tests and receive no Cloudflare secret. Live observation executes only after an exact audit marker is merged to `main`:

```text
# Oteryn public edge validation trigger

mode: audit
```

The push job checks out the exact trusted `main` SHA and enters the protected `production-cloudflare` environment.

The Cloudflare collector uses:

- `CLOUDFLARE_EDGE_AUDIT_TOKEN` for zone certificate, settings, Rulesets and Bot reads;
- `CLOUDFLARE_API_TOKEN` only as the separate Access applications read token;
- `CLOUDFLARE_ACCOUNT_ID` and `CLOUDFLARE_ZONE_ID` environment variables.

Both collectors are read-only. No Cloudflare, DNS, Tunnel, Synology, application, database, Canary, client or secret mutation is available in this workflow.

## Public probes

The validator checks:

- DNS resolution for WWW and Gateway;
- TLS 1.2 and TLS 1.3 hostname verification;
- bounded certificate metadata fingerprints;
- representative browser WWW routes;
- Gateway `/health`, `/ready` and `/version`;
- bounded invalid `POST /v1/login` response and private no-store headers;
- negative cross-routing between Platform and Gateway;
- plain-HTTP redirect behavior for both hosts;
- current WWW HSTS max-age as a non-blocking observation.

Response bodies are not retained. The evidence contains only up to 8 KiB-derived hashes and boolean content signals. `Set-Cookie` and other non-allowlisted headers are discarded.

## Required acceptance

The combined public verdict is `PASS` only when:

- both hostnames resolve;
- both hostnames complete a verified TLS handshake;
- representative WWW routes avoid Cloudflare interstitials and return non-error public responses;
- Gateway identity and health routes succeed;
- invalid Gateway login fails with the expected bounded private no-store response;
- Platform and Gateway do not cross-route;
- both HTTP endpoints redirect to their canonical HTTPS hosts.

Positive HSTS is observed but intentionally not required while public rollout is still being stabilized.

## Evidence

The workflow uploads a 14-day sanitized artifact containing:

```text
cloudflare-edge-audit/evidence.json
cloudflare-edge-audit/summary.md
public-edge-validation/evidence.json
public-edge-validation/summary.md
public-edge-result.txt
```

It also posts a fixed allowlist result to Issue #91. Raw API responses, authorization headers, tokens, WAF expressions, country literals, response bodies and cookies are not published.

A failed product acceptance does not imply the collector malfunctioned. The published fields distinguish collector execution from public `PASS`/`FAIL` and list the exact failed required checks.
