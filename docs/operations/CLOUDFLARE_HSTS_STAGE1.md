# Cloudflare Oteryn HSTS stage 1

## Purpose

After two independent public-edge passes, this operation promotes the zone from disabled browser persistence (`max-age=0`) to a conservative one-month HSTS policy.

Stage-1 target:

```yaml
enabled: true
max_age: 2592000
include_subdomains: false
preload: false
nosniff: true
```

The target deliberately avoids `includeSubDomains` and preload. It limits the initial browser commitment while still protecting repeat visits against HTTP downgrade. A later task may consider a longer duration only after extended production stability and an inventory of every affected hostname.

## Cloudflare requirements

The operation uses:

```text
PATCH /zones/{zone_id}/settings/security_header
```

The API requires zone-bounded `Zone Settings Write`, displayed as `Zone Settings → Edit` in the Cloudflare token UI. Read-only audit works with `Zone Settings → Read`.

Cloudflare requires HTTPS to remain available throughout the HSTS lifetime. Disabling Cloudflare, SSL, proxied DNS or HTTPS before the original max-age expires can make the site inaccessible to browsers that cached the policy.

Authoritative references:

- `https://developers.cloudflare.com/api/resources/zones/subresources/settings/methods/edit/`
- `https://developers.cloudflare.com/ssl/edge-certificates/additional-options/http-strict-transport-security/`

## Exact baseline and fail-closed behavior

The accepted pre-stage baseline is exact:

```yaml
enabled: true
max_age: 0
include_subdomains: true
preload: true
nosniff: true
```

`apply` refuses unknown drift. It accepts only the exact baseline or the exact staged target. `rollback` accepts only the exact target or baseline.

If Cloudflare accepts a PATCH but response validation or post-write verification fails, the script re-reads the setting. It restores the exact baseline only when the observed state is the exact staged target. Unexpected drift is never overwritten automatically.

## Operational modes

```text
audit
apply
rollback
```

`audit` performs only token verification and GET requests.

`apply` requires:

```text
APPLY-OTERYN-HSTS-STAGE1
```

It patches the exact staged target, verifies it, then runs public DNS/TLS/HTTP validation. Acceptance requires the existing public suite to remain `PASS` and the WWW response to contain a positive HSTS max-age.

`rollback` requires:

```text
ROLLBACK-OTERYN-HSTS-STAGE1
```

It restores the exact baseline and verifies public acceptance remains `PASS` while the observed HSTS max-age returns to zero.

## Trusted execution

Pull requests execute deterministic mock tests without Cloudflare credentials. Live operations run only from an exact marker merged to `main`, inside the protected `production-cloudflare` GitHub environment.

The workflow uses `CLOUDFLARE_EDGE_AUDIT_TOKEN`. Only `Zone Settings → Edit` must be added to its existing specific-zone policy for `molehill.cloud`; no account-wide permission, DNS write, certificate write or IP allowlist is needed.

Artifacts and Issue #91 comments contain only bounded setting values and public acceptance booleans. Tokens, raw API responses, cookies and response bodies are excluded.
