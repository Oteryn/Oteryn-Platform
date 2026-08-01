# Cloudflare Oteryn edge audit

## Purpose

This audit reads the Cloudflare configuration that remains relevant after canonical Tunnel and DNS reconciliation.

It is intentionally separate from `.github/workflows/cloudflare-oteryn-endpoints.yml`. The endpoint workflow owns only the two canonical proxied DNS records, the two canonical Cloudflare Tunnel ingress rules, and the guarded retirement of the exact legacy Gateway route/record.

Canonical hosts:

```text
oteryn.molehill.cloud
gateway.molehill.cloud
```

Retired host:

```text
login.oteryn.molehill.cloud
```

The audit inspects, but does not mutate:

- edge certificate packs and coverage for both canonical hosts;
- whether the retired multi-level hostname is still covered;
- selected zone TLS and security settings;
- zone Rulesets for redirects, custom WAF, response-header transforms and configuration rules;
- Bot Management/Bot Fight Mode state;
- Cloudflare Access applications matching canonical or retired Oteryn hosts.

## Certificate matching

The collector models certificate wildcard scope explicitly:

- an exact certificate hostname covers only that exact hostname;
- `*.molehill.cloud` covers exactly one label, including `oteryn.molehill.cloud` and `gateway.molehill.cloud`;
- that wildcard does not cover `login.oteryn.molehill.cloud`.

This classification explains why ADR 0020 can avoid Advanced Certificate Manager while retaining a separate Gateway hostname. A readable certificate pack still does not replace independent public TLS acceptance after DNS/Tunnel migration.

## Trust boundary

A live audit executes only from trusted code already present on `main`.

A trigger pull request may change only:

```text
ops/triggers/cloudflare-edge-audit.md
```

The `pull_request_target` job checks out the base SHA, not the trigger branch. The trigger therefore cannot alter code receiving the protected `production-cloudflare` token.

The Python implementation issues `GET` requests only. Its deterministic test rejects mutating method literals and validates sanitized output against a mock API.

## API permissions

- certificate packs: `SSL and Certificates Read`;
- zone settings: `Zone Settings Read`;
- Rulesets: relevant product Rulesets read permissions;
- Bot Management: `Bot Management Read`;
- Access applications: `Access: Apps and Policies Read`.

A `401` or `403` is retained only as `permission_denied` with bounded Cloudflare error metadata. No token, authorization header, raw rule expression or country literal is stored.

## Output

The artifact contains:

```text
cloudflare-edge-audit/evidence.json
cloudflare-edge-audit/summary.md
cloudflare-edge-audit/apply-preflight.json
cloudflare-edge-audit/apply-preflight-summary.md
cloudflare-edge-audit/token-capability.json
```

It records selected non-secret values, canonical and retired certificate coverage, sanitized rule IDs/actions/scope, matching Access application metadata, bounded Bot settings, and `mutation: none`.

## Trigger procedure

1. update only `ops/triggers/cloudflare-edge-audit.md` on a new branch;
2. open a pull request to `main`;
3. inspect the live sanitized audit;
4. close the trigger PR without merge.

The hostname migration is performed by the separate guarded endpoint workflow. WAF/Bot mutation must remain independently authorized, fixed-scope and reversible.
