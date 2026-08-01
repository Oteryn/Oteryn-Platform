# Cloudflare Oteryn edge audit

## Purpose

This audit reads the Cloudflare configuration that remains relevant after the canonical Tunnel and DNS reconciliation completed successfully.

It is intentionally separate from `.github/workflows/cloudflare-oteryn-endpoints.yml`. The existing endpoint workflow owns only the two canonical proxied DNS records and the two canonical Cloudflare Tunnel ingress rules.

This audit inspects, but does not mutate:

- edge certificate packs and exact coverage for `login.oteryn.molehill.cloud`;
- selected zone TLS and security settings;
- zone Rulesets for redirects, custom WAF, response-header transforms and configuration rules;
- Bot Management/Bot Fight Mode state;
- Cloudflare Access applications matching the two canonical hosts.

## Trust boundary

A live audit executes only from trusted code already present on `main`.

A trigger pull request may change only:

```text
ops/triggers/cloudflare-edge-audit.md
```

The `pull_request_target` job checks out the base SHA, not the trigger branch. The trigger therefore cannot alter code receiving the protected `production-cloudflare` token.

The Python implementation issues `GET` requests only. Its deterministic test rejects mutating method literals and validates the sanitized output against a mock API.

## Likely API permissions

- certificate packs: `SSL and Certificates Read`;
- zone settings: `Zone Settings Read`;
- Rulesets: relevant product Rulesets read permissions;
- Bot Management: `Bot Management Read`;
- Access applications: `Access: Apps and Policies Read`.

A `401` or `403` is retained only as `permission_denied` with bounded Cloudflare error metadata. No token, authorization header or raw API response is stored.

## Output

The artifact contains:

```text
cloudflare-edge-audit/evidence.json
cloudflare-edge-audit/summary.md
```

It records selected non-secret values, exact certificate coverage, Oteryn-matching rule IDs/actions, matching Access application IDs/domains, bounded Bot settings, and `mutation: none`.

## Trigger procedure

After this implementation is merged:

1. update only `ops/triggers/cloudflare-edge-audit.md` on a new branch;
2. open a pull request to `main`;
3. inspect the live sanitized audit;
4. close the trigger PR without merge.

No certificate, WAF, redirect, Bot, Access or HSTS mutation should be added until this audit proves current state and permissions.
