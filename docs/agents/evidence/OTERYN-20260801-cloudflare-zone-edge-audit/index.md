# Cloudflare zone-edge read-only audit evidence — 2026-08-01

## Verdict

```text
BLOCKED — READ PERMISSIONS REQUIRED
AUDIT_COMPLETE=false
MUTATION=none
```

The protected live audit executed successfully as an evidence collector, but the active Cloudflare token cannot read any requested zone-edge control. No certificate, zone-setting, WAF/Bot, Access or Page Rule state is classified as passing.

## Direct evidence

- merged audit implementation: PR #409;
- exact main SHA: `cff0ee1b8ecfd1d795e2636d488be6d1d1d0b4ea`;
- workflow run: `30702827936`;
- live audit job: `91376722540`;
- observed at: `2026-08-01T13:59:24Z`;
- artifact: `8819370547`;
- artifact digest: `sha256:d0e303b88b5ecc39a80c7020d8da9741e05f31b70caaca2ce47fa80d13a56a67`;
- token status: `active`;
- audit complete: `false`;
- mutation: `none`;
- secrets emitted: `false`.

The workflow validation and protected live audit jobs both passed. The audit script dynamically records only `GET` requests and exposes no apply mode.

## API availability

| Read surface | Result | Required capability recorded by the audit |
|---|---:|---|
| Certificate packs | HTTP 403 | SSL and Certificates Read |
| Universal SSL settings | HTTP 403 | SSL and Certificates Read |
| SSL verification | HTTP 403 | SSL and Certificates Read |
| Zone settings | HTTP 403 | Zone Settings Read |
| Zone rulesets | HTTP 403 | Zone WAF Read or Account Rulesets Read |
| Account rulesets | HTTP 403 | Account WAF Read or Account Rulesets Read |
| Bot Management | HTTP 403 | Bot Management Read |
| Access applications | HTTP 403 | Access Apps and Policies Read |
| Page Rules | HTTP 403 | Page Rules Read |

## PROVEN

- The active account-owned token authenticates successfully.
- The protected main-only audit path works and performs no Cloudflare mutation.
- Every requested zone-edge read is denied with HTTP 403.
- The existing token is sufficient for the separately managed Tunnel/DNS workflow but is not sufficient for this zone-edge audit.
- No secret, Cloudflare resource ID, rule expression, unrelated hostname or complete API response appears in the evidence artifact.

## UNKNOWN

- Active certificate packs and exact coverage for `login.oteryn.molehill.cloud`.
- Universal SSL and certificate verification state.
- Zone SSL mode, minimum TLS version, Always Use HTTPS and HSTS configuration.
- Effective security level, Browser Integrity Check and challenge TTL.
- Zone/account WAF and Rulesets configuration.
- Bot Management / Bot Fight Mode.
- Matching Cloudflare Access applications.
- Matching legacy Page Rules.

The earlier public observations remain valid: WWW returns Cloudflare 403 challenge content, the Gateway hostname fails TLS before HTTP, HTTP does not redirect to HTTPS, and WWW emits HSTS `max-age=0`. Those observations do not identify the underlying Cloudflare configuration without the denied API reads.

## Exact next plan — not authorized for execution

1. Keep the existing `CLOUDFLARE_API_TOKEN` unchanged for the fixed Tunnel/DNS workflow.
2. Create a separate account-owned, read-only Cloudflare token restricted to the intended account and the `molehill.cloud` zone.
3. Grant only the read capabilities required by the denied endpoints:
   - SSL and Certificates Read;
   - Zone Settings Read;
   - Bot Management Read;
   - Access Apps and Policies Read;
   - Page Rules Read;
   - the least-privilege Rulesets/WAF read capability accepted for both the zone and account requests.
4. Add it only as protected environment secret `CLOUDFLARE_ZONE_AUDIT_TOKEN`.
5. Change only the audit workflow to consume that new secret; do not alter the Tunnel/DNS workflow.
6. Rerun the GET-only audit and review the sanitized artifact.
7. Present the exact certificate, challenge-policy, redirect and HSTS remediation diff, risk and value-level rollback.
8. Request a new explicit owner authorization before any Cloudflare apply.

## Risk and rollback for the proposed permission step

- Risk: the new token can disclose zone-edge configuration within its bounded read scopes; it cannot mutate Cloudflare state.
- Blast radius: the intended Cloudflare account and `molehill.cloud` zone only.
- Secret handling: GitHub protected environment only; never repository variables, files, logs, artifacts or comments.
- Rollback: delete/revoke the new token and remove `CLOUDFLARE_ZONE_AUDIT_TOKEN`; existing Tunnel/DNS automation remains unchanged.
