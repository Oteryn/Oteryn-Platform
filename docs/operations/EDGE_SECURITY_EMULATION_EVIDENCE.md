# Edge Security Emulation Evidence

## Purpose

This record describes the repository-owned `Edge Security Emulation` workflow. The workflow exists because no final production DNS zone, certificate, Cloudflare configuration, firewall or origin environment is currently available.

It uses only reserved `.test` hostnames, loopback listeners, isolated SQLite state and ephemeral certificates generated during the workflow. It performs no public DNS, Cloudflare-account, router, DSM, production or external-repository action.

## Classification boundary

A successful exact-SHA workflow run is classified only as `STAGING_PROVEN`.

It can prove that the reviewed emulated topology and assertions work together under controlled CI. It cannot prove:

- ownership or effective configuration of a real DNS zone;
- a publicly trusted production certificate or certificate lifecycle;
- a real Cloudflare proxy mode, WAF managed ruleset, rate-limit policy or Access application;
- production firewall/security-group rules;
- actual origin reachability or direct-origin blocking;
- the final HSTS decision for real hostnames and subdomains;
- completion of Issue #91.

Those production facts remain `UNKNOWN` until directly verified in the actual environment.

## Emulated topology

```text
CoreDNS authoritative .test zone
        |
        v
app.oteryn.test / admin.oteryn.test
        |
        | public TLS 1.2/1.3, HTTP redirect,
        | Cloudflare-style metadata, WAF, rate limit,
        | optional Access assertion admission
        v
loopback edge reverse proxy
        |
        | separately trusted TLS + ephemeral client certificate
        v
loopback-only origin reverse proxy
        |
        v
current-SHA Laravel /health and application auth middleware
```

The Access emulator validates an ephemeral signed assertion only to decide whether a request may reach `/admin`. Oteryn Platform does not consume the asserted identity. Platform authentication, confirmed MFA, exact RBAC and audit remain authoritative.

## Required assertions

The workflow fails closed unless all of these pass:

- authoritative A and CNAME answers plus NXDOMAIN for an absent name;
- DNS-over-UDP and DNS-over-TCP behavior;
- edge certificate hostname verification;
- TLS 1.2 and TLS 1.3 success with TLS 1.0 denial;
- HTTP 308 redirect to the HTTPS edge;
- current-SHA Laravel `/health` success through edge and origin;
- origin listener bound only to loopback;
- authenticated edge-to-origin TLS using a dedicated client certificate;
- direct-origin request without that certificate denied;
- spoofed inbound `CF-Connecting-IP` overwritten by the edge;
- bounded Cloudflare-style `CF-Ray` and `CF-Cache-Status` response metadata;
- traversal, XSS and SQL-injection probes denied;
- unsupported method and oversized request body denied;
- controlled request burst produces HTTP 429;
- missing and invalid Access assertions denied;
- valid Access admission still reaches independent Platform authentication rather than granting administrator access.

## Exact-SHA result

`PENDING` — populate after the first successful workflow run on the final PR head.

## Production boundary

Issue #91 remains the sole Production Go-Live execution tracker. The actual production edge rows in `docs/operations/PRODUCTION_VERIFICATION_EVIDENCE.md` remain `UNKNOWN`; this workflow must never rewrite them to `PRODUCTION_PROVEN`.
