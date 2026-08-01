# Public edge revalidation after staging repair

Task: `OTERYN-20260801-public-domain-repair`  
Observation type: public, read-only, GitHub-hosted runner  
Observation time: `2026-08-01T11:05:03.497039+00:00`  
Evidence PR: `#399` (closed without merge)  
Workflow run: `30696983913` / Public Edge Revalidation `#1`  
Runner region: West US

## Evidence identity

```text
artifact_id: 8817569426
artifact_name: public-edge-revalidation-2d9c19b5f1686fa5af25fad728314c36c9fc2949
artifact_digest: sha256:a090c5562ac2ed529f214fc5dd2d1f765b27facbb63a56f3838a46a4ba66c4a1
observation_head: f9658480c26671fc408113d37e945e45f4d85305
pull_request_merge_ref: 2d9c19b5f1686fa5af25fad728314c36c9fc2949
```

The artifact contains bounded public DNS, TLS, certificate, HTTP status, redirect, response-header and sampled-body metadata. It contains no credentials, cookies, valid login, password-reset token, private endpoint, Cloudflare token or production secret.

## Result

**PUBLIC EDGE: BLOCKED / UNCHANGED FROM PR #387.**

The exact staging deployment `3eb109b505f7d1c8718cffb823de6d9d5166717c` is separately `STAGING_PROVEN`, but the public Cloudflare/DNS layer remains unrepaired.

## DNS

Both canonical names resolved to the same Cloudflare anycast addresses:

```text
A     104.21.2.166
A     172.67.186.250
AAAA  2606:4700:3031::6815:2a6
AAAA  2606:4700:3033::ac43:bafa
```

Observed names:

```text
oteryn.molehill.cloud
login.oteryn.molehill.cloud
```

## WWW TLS and certificate

Target: `oteryn.molehill.cloud`

- TLS 1.2 hostname verification: **FAIL** with protocol-version alert.
- TLS 1.3 hostname verification: **PASS**.
- certificate extraction: **PASS**.
- subject: `CN = molehill.cloud`.
- issuer: Google Trust Services `WE1`.
- SANs: `molehill.cloud`, `*.molehill.cloud`.
- validity: `2026-06-28 02:01:55 UTC` through `2026-09-26 02:59:39 UTC`.
- SHA-256 fingerprint: `5F:72:D6:27:54:66:07:D0:59:B7:73:7C:85:2F:9B:1A:1B:B4:59:D7:F5:85:2B:D1:57:66:90:3D:A8:1A:18:3F`.

The certificate covers `oteryn.molehill.cloud` but does not cover the deeper hostname `login.oteryn.molehill.cloud`.

## Gateway TLS

Target: `login.oteryn.molehill.cloud`

- TLS 1.2 hostname verification: **FAIL** before certificate delivery.
- TLS 1.3 hostname verification: **FAIL** before certificate delivery.
- certificate extraction: **FAIL**.
- OpenSSL and curl received `sslv3 alert handshake failure`.

Therefore the following contracted endpoints remain externally unreachable over canonical HTTPS:

```text
GET  /health
GET  /ready
GET  /version
POST /v1/login
```

The bounded invalid login request used `{}` only and received no HTTP response because TLS failed first.

## WWW HTTPS behavior

Every representative route returned Cloudflare `403` with the HTML title `Just a moment...`:

```text
/
/login?locale=en
/register
/forgot-password
/health
/news
/highscores
/version
```

The effective edge header remained:

```text
Strict-Transport-Security: max-age=0; includeSubDomains; preload
```

This explicitly removes persisted HSTS despite the decorative `includeSubDomains` and `preload` tokens.

## Plain HTTP behavior

These requests returned Cloudflare `403`, not an HTTPS redirect:

```text
http://oteryn.molehill.cloud/
http://login.oteryn.molehill.cloud/health
```

The block responses included private no-store/no-cache directives and the title `Attention Required! | Cloudflare`.

## Comparison with source validation

The public findings are materially unchanged from PR #387:

1. Gateway canonical HTTPS still fails before HTTP.
2. WWW anonymous automated access remains blocked by Cloudflare.
3. HTTP is not redirected to HTTPS before edge blocking.
4. WWW HSTS remains `max-age=0`.
5. Public password recovery remains untestable because `/forgot-password` is intercepted.

The repository and staging fixes did not claim to mutate the separately controlled Cloudflare/DNS layer. This revalidation directly confirms that boundary.

## Current classification

```text
STAGING_PROVEN: true
PUBLIC_DOMAIN_LAUNCH_READY: false
PRODUCTION_PROVEN: false
```

## Remaining operator action

Usable Cloudflare/DNS operator access is required to execute the reversible plan in `docs/agents/reports/OTERYN-20260801-public-domain-repair.md`:

1. capture current certificate, DNS, Tunnel, WAF/Bot/Access, redirect and HSTS state;
2. provision certificate coverage for `login.oteryn.molehill.cloud`;
3. preserve exact Gateway and WWW loopback origin mappings;
4. remove browser-only challenges from contracted Gateway machine paths;
5. permit intended WWW anonymous access without globally weakening controls;
6. redirect HTTP to HTTPS before challenge processing;
7. apply a reviewed positive HSTS policy only after HTTPS coverage is proven;
8. rerun the exact public acceptance sequence;
9. execute controlled redacted password-recovery delivery only with an authorized identity and mailbox.

No external mutation or rollback was performed during this revalidation.
