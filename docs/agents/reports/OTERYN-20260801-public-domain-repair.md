# Public-domain repair report

Task: `OTERYN-20260801-public-domain-repair`  
Source validation: PR `#387` at `c8ca2fc995fbbc4a0f3c7268872d3843db950af8`  
Repository repairs: PRs `#388`, `#392`, `#396`  
Exact deployed staging source: `3eb109b505f7d1c8718cffb823de6d9d5166717c`

## Status

Repository-owned repair and Marketplace-aware Synology staging deployment are complete.

- `STAGING_PROVEN`: **true** for exact source/image SHA `3eb109b505f7d1c8718cffb823de6d9d5166717c`.
- `PRODUCTION_PROVEN`: **false**.
- `PUBLIC_DOMAIN_LAUNCH_READY`: **false** until the separately controlled Cloudflare/DNS edge repair and public acceptance probes pass.

No production, Cloudflare, DNS, Canary-source, OTClient or PR #387 evidence mutation occurred.

## Proven source failure

PR #387 established `gateway-public-tls-handshake-failure` as the first relevant public failure:

- `login.oteryn.molehill.cloud` failed TLS 1.2 and TLS 1.3 negotiation before HTTP;
- observed SANs `molehill.cloud` and `*.molehill.cloud` do not cover the deeper hostname;
- representative WWW requests received Cloudflare `403` interstitials;
- HTTP did not redirect to HTTPS;
- WWW returned HSTS with `max-age=0`;
- historical staging generated user-visible URLs from loopback `APP_URL`.

The endpoint contract remains:

```text
https://oteryn.molehill.cloud       -> http://127.0.0.1:8000
https://login.oteryn.molehill.cloud -> http://127.0.0.1:8080
```

## Repository repair

The merged repair:

1. enforces `https://oteryn.molehill.cloud` for browser-facing guarded and Marketplace-aware staging;
2. preserves Platform and Gateway loopback-only origin bindings;
3. enables Secure cookies for public staging;
4. verifies requestless login, password-reset and signed URLs use the canonical HTTPS origin;
5. verifies exact Gateway service/version identity, bounded malformed login JSON, private no-store/no-cache headers and negative cross-routing;
6. distinguishes the partial durable Marketplace state file from the complete deployment `.env`;
7. runs host-loopback protocol probes from an ephemeral `--network host` container, matching the containerized Synology runner boundary;
8. retains bounded readiness retries and fail-closed behavior.

No broad proxy trust, authentication bypass, cache relaxation or public origin exposure was introduced.

## Repair and deployment chronology

### PR #388

Implemented canonical URL, Secure-cookie, requestless URL and bounded Gateway/cross-routing checks, including the actual Marketplace Compose layer.

Exact PR head `a2ecd6fa1981283e4436e498e26fc8c4cf1345c5` passed all nine required workflow families before squash merge as `82abef518f91d72d392db4420bb335773087c3e1`.

First staging run `30693873142` / #5 stopped before Docker because the shared loader applied complete-environment validation to partial `marketplace.env` state.

### PR #392

Scoped canonical migration to the complete file named exactly `.env` and added an executable Bash regression test.

Exact PR head `770a4921d1de20fd1685bea5edbb95940cbcfb32` passed every path-applicable workflow before squash merge as `b249e5e9cb864ba01376efb273be323b90bcd500`.

Second staging run `30694481769` / #6 resolved exact images, rendered the environment, recreated services and verified bindings. It then failed because direct Python `127.0.0.1` requests used the containerized runner namespace rather than the NAS host namespace. Final state persistence and evidence upload were skipped.

### PR #396

Moved protocol probes into an ephemeral `python:3.12-alpine` container with host networking and bounded retries. Added `SynologyStagingNetworkBoundaryTest`.

Exact PR head `b61cfc1ac2f5900d3ad9e78e2433bede8f7eec88` passed:

- CI #4018;
- Agent Governance #3809;
- Phase 7 Production-Like Validation #3053;
- Build Synology Staging Images #1582;
- Edge Security Emulation #1474;
- Platform DB Outage Validation #2980;
- Game Auth Ticket Concurrency #2551.

It was squash merged as `3eb109b505f7d1c8718cffb823de6d9d5166717c`.

## Exact staging proof

Character Bazaar Staging Control run `30695167157` / #7 completed successfully on runner `oteryn-synology-staging` using:

```text
ghcr.io/blakinio/oteryn-platform:sha-3eb109b505f7d1c8718cffb823de6d9d5166717c
ghcr.io/blakinio/oteryn-game-gateway:sha-3eb109b505f7d1c8718cffb823de6d9d5166717c
```

The run proved:

- exact images resolved before deployment;
- Platform, Gateway and Canary host bindings matched the contract;
- Gateway identity/version, malformed-login response and private cache controls passed;
- Platform/Gateway negative cross-routing passed;
- QR-first MFA renderer and protected route checks passed;
- public forwarded-HTTPS login form action remained canonical;
- requestless login, password-reset and signed URLs used `https://oteryn.molehill.cloud`;
- Canary LAN endpoint `192.168.1.2:7172` was reachable;
- Character Bazaar enablement and transfer privilege boundary passed;
- final staging state was persisted.

Sanitized evidence:

```text
artifact_id: 8817085021
artifact_name: character-bazaar-staging-3eb109b505f7d1c8718cffb823de6d9d5166717c-deploy-enable
artifact_digest: sha256:5523ee4c0a49a156e23a894e808915a9a1f5b424b961168eb732774e6056efbb
classification: STAGING_PROVEN
action: deploy-enable
exact_source_sha: 3eb109b505f7d1c8718cffb823de6d9d5166717c
marketplace_enabled: true
scheduler_running_count: 1
transfer_privileges: verified-by-guarded-action
escrow_identity: reviewed-unbound-non-login-account-verified
production_environment_proven: false
```

The artifact expires on `2026-08-15`; this report preserves its non-secret digest and exact fields.

## External operator change plan

The following plan is **not applied**.

### 1. Capture current state and rollback material

Record sanitized current state for only the two canonical hostnames:

- DNS records and proxy state;
- Cloudflare Tunnel hostname-to-origin entries;
- certificate coverage and status;
- minimum TLS policy;
- WAF, Bot, Access, challenge, rate-limit and redirect rules;
- HSTS/header transforms;
- any matching Synology reverse-proxy mappings.

Do not record credentials, tokens, private keys, cookies or private environment contents. Assign stable identifiers to every changed rule.

### 2. Repair Gateway certificate coverage first

Provision or attach a certificate whose SAN directly covers:

```text
login.oteryn.molehill.cloud
```

Do not rely on `*.molehill.cloud`. Verify hostname, chain and exact native-client TLS compatibility before changing minimum TLS policy. The supported minimum TLS version remains unknown and must not be guessed.

### 3. Preserve exact routing

Use only:

```text
login.oteryn.molehill.cloud -> http://127.0.0.1:8080
oteryn.molehill.cloud       -> http://127.0.0.1:8000
```

Do not route public traffic to Canary `7171`/`7172` or expose loopback origins directly.

### 4. Repair machine and browser policy narrowly

For Gateway paths `/health`, `/ready`, `/version` and `/v1/login`, remove browser-only JavaScript/managed challenges that block machine clients. Keep hostname/path-scoped abuse controls and preserve application private no-store headers.

For WWW, permit intended anonymous public pages without globally disabling WAF, Bot or Access. Keep appropriate stronger controls for identity and administrator surfaces while returning usable application behavior.

### 5. Redirect HTTP before challenge processing

Configure exact redirects preserving path/query:

```text
http://oteryn.molehill.cloud/*       -> https://oteryn.molehill.cloud/*
http://login.oteryn.molehill.cloud/* -> https://login.oteryn.molehill.cloud/*
```

Verify no loop and keep canonical clients on HTTPS directly.

### 6. HSTS only after HTTPS coverage is proven

Do not enable preload or `includeSubDomains` merely to satisfy validation. After certificate and included-host coverage are proven, choose one reviewed positive `max-age` and emit one non-contradictory header. Preload remains a separate owner decision.

## Required public acceptance probes

After authorized edge changes, verify in this order:

1. Gateway certificate SAN, chain and hostname;
2. exact supported TLS versions using the native-client contract;
3. Gateway `/health`, `/ready`, `/version` JSON;
4. malformed `/v1/login` bounded error and private no-store headers;
5. no HTML challenge on Gateway machine paths;
6. anonymous WWW public-route reachability;
7. HTTP-to-HTTPS redirects for both hostnames;
8. effective HSTS state;
9. negative cross-routing in both directions;
10. canonical requestless reset/verification/signed URLs;
11. controlled redacted password-recovery delivery using an authorized identity and mailbox.

## Rollback

Repository rollback is a revert of merged repair commits. Synology deployment retains prior image snapshots for explicit guarded runtime rollback; database migrations are intentionally not reversed automatically.

External rollback must restore the exact captured certificate attachment, tunnel routes, WAF/Bot/Access/rate-limit rules, redirects and HSTS/header transforms. No external rollback was required because no edge mutation occurred.

## Remaining blocker

The repository and staging portions are complete. Public launch remains blocked only by unavailable Cloudflare/DNS operator access and the unexecuted external acceptance plan. Gateway public TLS, edge routing/challenge behavior, redirects, HSTS and controlled password-recovery delivery remain externally unproven.
