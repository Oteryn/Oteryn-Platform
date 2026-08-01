# Cloudflare edge-policy audit

## Purpose

`Cloudflare Oteryn Edge Audit` is a fixed-scope, read-only workflow for the two canonical public endpoints:

```text
oteryn.molehill.cloud
login.oteryn.molehill.cloud
```

It exists because the canonical Tunnel and DNS reconciliation can be correct while certificate issuance or hostname-scoped edge policy still prevents public traffic.

The audit does not change Cloudflare state. It has no `apply` input and the collector issues only HTTP `GET` requests.

## Execution boundary

Pull requests execute deterministic validation only. A live audit can run only through a manual dispatch from `main` and uses the protected GitHub environment `production-cloudflare`.

Required protected configuration:

```text
secret: CLOUDFLARE_API_TOKEN
variable: CLOUDFLARE_ACCOUNT_ID
variable: CLOUDFLARE_ZONE_ID
```

The existing token may return permission-denied results for some optional surfaces. Those results are preserved as sanitized evidence rather than treated as proof that the configuration is absent.

## Audited surfaces

The collector verifies the token and zone identity, then reads the smallest relevant Cloudflare surfaces available to the token:

- edge certificate packs and hostname coverage;
- SSL verification state;
- Total TLS state;
- Bot Management state;
- relevant zone Rulesets and rule details;
- Access applications matching the two exact canonical hosts;
- Always Use HTTPS;
- HSTS/security-header settings;
- browser integrity, security level and minimum TLS settings.

Relevant Ruleset phases are limited to:

```text
http_request_firewall_custom
http_request_sbfm
http_request_dynamic_redirect
http_request_redirect
http_response_headers_transform
http_config_settings
```

## Sanitization

The artifact contains only the two canonical hostnames in clear text.

- unrelated rule expressions, descriptions, names, identifiers and action parameters are omitted or represented by SHA-256 digests;
- unrelated Access application domains are omitted;
- Cloudflare resource identifiers are hashed;
- the API token is never serialized;
- API errors are bounded to status, code and a short message;
- no raw account-wide configuration export is uploaded.

Generated artifact files:

```text
cloudflare-edge-audit/audit.json
cloudflare-edge-audit/summary.md
```

## Interpretation

The audit distinguishes evidence from remediation:

- certificate coverage reports whether an active certificate name covers each exact hostname according to normal single-label wildcard matching;
- challenge candidates report canonical rules separately from broad or unrelated rules;
- Access reports only applications whose domain is exactly one canonical host or a path below it;
- positive HSTS requires an enabled policy with `max_age > 0`;
- missing API permissions remain explicit and must not be interpreted as a clean configuration.

A successful workflow means the audit executed and produced sanitized evidence. It does not mean public-domain acceptance passed.

## Follow-up policy

Review the live audit artifact before authorizing any Cloudflare mutation. A later apply-capable task must:

1. own only the proven defective control;
2. capture exact current state and rollback material;
3. preserve unrelated rules and applications;
4. require an explicit confirmation phrase;
5. verify post-write state;
6. repeat the independent public DNS/TLS/HTTP acceptance sequence.

Certificate ordering, Total TLS changes, WAF/Bot/Access changes, redirects and HSTS changes are intentionally outside this audit task.
