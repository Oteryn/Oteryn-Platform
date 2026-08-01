# Cloudflare Oteryn zone-edge audit

## Purpose

The repository provides a separately protected, read-only audit for Cloudflare controls that are outside the fixed Tunnel/DNS automation:

- edge certificate coverage for `oteryn.molehill.cloud` and `login.oteryn.molehill.cloud`;
- Universal SSL and certificate verification state;
- zone SSL mode, minimum TLS, TLS 1.3, Always Use HTTPS and HSTS;
- zone and account rulesets relevant to WAF challenge/block, redirects and response headers;
- Bot Management / Bot Fight Mode;
- Cloudflare Access applications matching either canonical hostname;
- legacy Page Rules that match either canonical hostname.

The audit never changes Tunnel ingress or DNS and never issues a Cloudflare request other than `GET`.

## Protected execution boundary

Pull requests run only deterministic offline tests. Live API access is restricted to `main` and the protected GitHub environment `production-cloudflare`.

The workflow consumes:

- environment secret `CLOUDFLARE_API_TOKEN`;
- environment variables `CLOUDFLARE_ACCOUNT_ID` and `CLOUDFLARE_ZONE_ID`.

The first live execution runs automatically when the workflow itself reaches `main`. Later audits use manual workflow dispatch from `main`.

## Required read capabilities

Use a zone/account-bounded API token, never a Global API Key. The audit attempts the following capability groups and records a sanitized HTTP status plus the expected capability when one is unavailable:

- SSL and Certificates Read;
- Zone Settings Read;
- Zone/Account WAF or Rulesets Read;
- Bot Management Read;
- Access Apps and Policies Read;
- Page Rules Read.

The audit does not infer a passing state from a denied endpoint. Missing access remains `UNKNOWN`, and `audit_complete` is false.

## Sanitized evidence

The uploaded JSON contains only:

- API availability and HTTP status;
- booleans, bounded counts and selected non-secret setting values;
- whether the two canonical hostnames are covered by an active certificate;
- action counts for relevant enabled rules;
- matching Access/Page Rule counts;
- `mutation: none` and `secrets_emitted: false`.

It excludes token values, account/zone/rule/certificate IDs, full rule expressions, unrelated hostnames, private origins and complete API responses.

## Interpretation limits

Configuration inspection can identify active controls capable of producing challenge/block/redirect behavior. It cannot execute Cloudflare Rules Trace because that API requires a non-GET request, and this task is explicitly GET-only. Direct public probes remain required to confirm effective request behavior.

An API certificate-coverage result is configuration evidence. The canonical hostname must still complete a real TLS handshake before public readiness can pass.

## Current API references

- `GET /zones/{zone_id}/ssl/certificate_packs`: Cloudflare Certificate Packs API.
- `GET /zones/{zone_id}/ssl/universal/settings`: Universal SSL Settings API.
- `GET /zones/{zone_id}/ssl/verification`: SSL Verification API.
- `GET /zones/{zone_id}/settings`: Zone Settings API.
- `GET /{accounts_or_zones}/{id}/rulesets`: Rulesets API.
- `GET /zones/{zone_id}/bot_management`: Bot Management API.
- `GET /{accounts_or_zones}/{id}/access/apps`: Access Applications API.
- `GET /zones/{zone_id}/pagerules`: Page Rules API.

## Mutation and rollback

There is no apply mode. The audit cannot mutate Cloudflare state, so rollback is not applicable.

Any remediation must be proposed separately with:

1. exact current and desired values;
2. minimal API/dashboard change;
3. affected hostnames and request classes;
4. risk and blast radius;
5. value-level rollback;
6. independent TLS/HTTP/HSTS acceptance after apply;
7. new explicit owner authorization before execution.
