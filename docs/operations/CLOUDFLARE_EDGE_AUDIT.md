# Cloudflare Oteryn edge audit

## Purpose

This audit reads the Cloudflare configuration that remains relevant after the canonical Tunnel and DNS reconciliation completed successfully.

It is intentionally separate from `.github/workflows/cloudflare-oteryn-endpoints.yml`. The endpoint workflow owns only the two canonical proxied DNS records and the two canonical Cloudflare Tunnel ingress rules.

The audit inspects, but does not mutate:

- edge certificate packs and hostname coverage for `oteryn.molehill.cloud` and `login.oteryn.molehill.cloud`;
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

The Python implementation issues `GET` requests only. Its deterministic test rejects mutating method literals and validates sanitized output against a mock API.

## Required API permissions

Use a least-privilege account/zone-bounded token. Do not use a Global API Key and do not add edit/write permission merely for inspection.

The audit requires read access to the API families it queries:

- `SSL and Certificates Read` for certificate packs and SSL verification;
- `Zone Settings Read` for Always Use HTTPS, minimum TLS, browser check, security level and HSTS/security headers;
- `Bot Management Read`;
- `Access: Apps and Policies Read`;
- the relevant Rulesets read groups for configured WAF, redirect, transform and configuration rule families, such as `Account WAF Read`, `Account Rulesets Read`, `Mass URL Redirects Read`, `Transform Rules Read` and `Select Configuration Read` where applicable.

A `401` or `403` is retained only as `permission_denied` with bounded Cloudflare error metadata. A denied endpoint remains `UNKNOWN`; it is not evidence that a feature is absent or disabled.

## Sanitized output

The artifact contains:

```text
cloudflare-edge-audit/evidence.json
cloudflare-edge-audit/summary.md
```

It records selected non-secret values, canonical hostname certificate coverage, Oteryn-matching rule IDs/actions, matching Access application IDs/domains, bounded Bot settings and `mutation: none`.

It does not store the token, authorization header, raw account-wide responses, unrelated hostnames or full unrelated rule expressions.

## Trigger procedure

1. Ensure the protected `production-cloudflare` token has the required read capabilities for the intended account and zone.
2. Update only `ops/triggers/cloudflare-edge-audit.md` on a new branch.
3. Open a pull request to `main`.
4. Inspect the live sanitized audit artifact.
5. Close the trigger PR without merge.

No certificate, WAF, redirect, Bot, Access or HSTS mutation should be added until the audit proves the current state and permissions.

## First live execution

The first trusted-main live audit was triggered through PR `#408`:

```text
trusted_implementation: 5ea883c26dead9d58d363df1fb7909e3c399e206
workflow_run: 30702383389
job: 91375538793
artifact_id: 8819238641
artifact_digest: sha256:fce53d0651b496e42e56654bfdcad491afe2e01e80fea79e7e5b8630e38215ae
mutation: none
```

The token verified as active, but all remaining edge families returned `permission_denied`. The next execution must occur only after the protected token receives the documented read permissions.
