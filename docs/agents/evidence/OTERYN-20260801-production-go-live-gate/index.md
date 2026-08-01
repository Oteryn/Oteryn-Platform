# Production Go-Live Gate evidence — 2026-08-01

## Verdict

```text
BLOCKED — PENDING PRODUCTION VERIFICATION
PRODUCTION_PROVEN=false
```

Issue #91 remains open. Direct evidence proves that the Synology target is a healthy staging runtime, not an effective production runtime, and that the canonical public edge does not deliver the expected applications.

## Evaluated repository state

- task branch base: `de949075d14ebecc57423237b9330d865da28645`;
- task branch: `agent/production-go-live-gate`;
- draft PR: #405;
- no Cloudflare, DNS, Synology runtime, database, Redis, secret, deployment, rollback, restore or application-data mutation was performed.

The task branch SHA is not the deployed application identity.

## PROVEN

### Exact Synology runtime identity and topology

Sanitized read-only inventory:

- observer run: `30701773214`;
- dispatched inventory run: `30701775782`;
- live inventory job: `91373911925`;
- observed at: `2026-08-01T13:38:45.781133+00:00`;
- observer source SHA: `0c435dd02d2afcc7f0e8d963a79b5441b29a6cb7`;
- artifact: `8819161257`;
- artifact digest: `sha256:67b1d16eb67f90e1534a9071644aeaf42da97adc527643964a14df120c37db9c`;
- production mutation: `NONE`;
- restore drill: `NOT_RUN`.

Exact deployed application images:

| Service | Source/tag | Repository digest | Local image ID |
|---|---|---|---|
| Platform | `3eb109b505f7d1c8718cffb823de6d9d5166717c` | `sha256:ac0e88a1627a8ab78b4bca87dbce16035c29da8f3ab01c152fe2bf0946651b7a` | `sha256:6e6ceedeb900761089e449136fc272c9916f3396237e80793f4bf60417111872` |
| Game Gateway | `3eb109b505f7d1c8718cffb823de6d9d5166717c` | `sha256:9a731ca6528faec0ae70106898b757a22a6e561d99efc639d1fe71e5ece687ed` | `sha256:e55cd036d283c21ad845ddccb883a7496354c7f590dace19454a98faae353ee8` |
| Canary | immutable digest reference | `sha256:784e5dbdcc64e311c48c51cd94aa206e2efa1e5eefb2f4ef40170d5aac55031f` | `sha256:adbb592a96e3ea4a2c44f09462330f73e150fd0b515b175186aea79856f9299b` |

Platform and Gateway use the same exact source SHA. The Compose project is `oteryn-staging`, all six expected service containers are singletons and running, and all use network `oteryn-staging_private`.

| Service | Container | Start time UTC | Restart policy/count | Health |
|---|---|---|---|---|
| MariaDB | `oteryn-staging-mariadb-1` | `2026-07-31T10:16:30.084801205Z` | `always` / 0 | `healthy` |
| Redis | `oteryn-staging-redis-1` | `2026-07-31T10:16:27.575408438Z` | `always` / 0 | `healthy` |
| Canary | `oteryn-staging-canary-1` | `2026-08-01T06:05:35.182280883Z` | `always` / 1 | no Docker healthcheck |
| Platform | `oteryn-staging-platform-1` | `2026-08-01T10:16:38.114260294Z` | `always` / 0 | no Docker healthcheck |
| Internal proxy | `oteryn-staging-internal-proxy-1` | `2026-08-01T10:17:37.107468550Z` | `always` / 0 | no Docker healthcheck |
| Gateway | `oteryn-staging-gateway-1` | `2026-08-01T10:14:18.642439716Z` | `always` / 0 | no Docker healthcheck |

All bounded Platform/Gateway container-namespace and Synology host-loopback health, ready and version probes passed. No critical/fatal/panic/exception/error marker was counted in the last 30 minutes for any expected service.

Published bindings:

- Platform: loopback-only `8000/tcp`;
- Game Gateway: loopback-only `8080/tcp`;
- Canary legacy login: loopback-only `7171/tcp`;
- Canary game protocol: one private-IP `7172/tcp` binding;
- MariaDB, Redis and internal proxy: no published host ports.

Effective application profile:

- `APP_ENV=staging`;
- `APP_ENV=production`: false;
- debug disabled;
- file sessions;
- file cache;
- synchronous queue;
- array/non-delivery mail;
- `production:verify-configuration` exit code: `1`;
- runtime classification: `STAGING_TARGET`;
- production environment proven: `false`.

No cloudflared container was visible through Docker. Whether cloudflared runs as a host process remains `UNKNOWN`; no effective host-process/network-path proof was collected.

### Cloudflare fixed scope

- audit run `30699270139`: PASS;
- apply run `30700054602`: PASS;
- managed Tunnel/DNS state after apply: `current`.

This proves only the guarded Tunnel ingress and canonical DNS records managed by that workflow.

### Public edge after Cloudflare apply

Independent observation:

- run: `30701140509`;
- job: `91372237869`;
- observed at: `2026-08-01T13:09:16.214513+00:00`;
- artifact: `8818850803`;
- artifact digest: `sha256:787ea72c616812ade431eb1cc396e921a6c8b04e459c89557221cbf6caebe656`;
- runner region: West US.

Direct observations:

- both canonical hostnames resolved through the same Cloudflare IPv4/IPv6 anycast set;
- `oteryn.molehill.cloud` verified only with TLS 1.3 and presented the expected wildcard-domain certificate;
- all representative HTTPS WWW routes returned Cloudflare `403` challenge content instead of Platform;
- `login.oteryn.molehill.cloud` failed TLS 1.2 and TLS 1.3 before HTTP and exposed no certificate;
- plain HTTP on both canonical names returned Cloudflare `403` rather than redirecting to HTTPS;
- WWW returned `Strict-Transport-Security: max-age=0; includeSubDomains; preload`.

### Repository validation

PR #405 head `0c435dd02d2afcc7f0e8d963a79b5441b29a6cb7` passed:

- Agent Governance `30701773251`;
- CI `30701773237`;
- Synology Production Target Preflight static validation `30701773212`;
- Edge Security Emulation `30701773227`;
- Game Auth Ticket Concurrency `30701773203`;
- Platform DB Outage Validation `30701773233`;
- Phase 7 Production-Like Validation `30701773198`;
- Build Synology Staging Images `30701773188`.

These are repository/staging-support results, not production proof.

## DERIVED

- The exact running target is a staging deployment and cannot be classified as `PRODUCTION_PROVEN`.
- File sessions/cache and array mail are incompatible with the unproven multi-instance/delivery requirements until a launch topology explicitly justifies them; array mail directly blocks real password-recovery delivery.
- Cloudflare Tunnel/DNS convergence did not resolve the separately controlled certificate, WAF/Bot/Access, redirect or HSTS failures.
- Local loopback health does not substitute for canonical public application delivery.
- Mutation smoke is unsafe and prohibited while production runtime, rollback, dated restore, controlled identities/data and public application reachability remain unproven.
- The final result cannot be `PRODUCTION_PROVEN`.

## UNKNOWN

- cloudflared host-process status, network mode and effective path to both loopback origins;
- production DB topology, credential ownership, least privilege, backup policy and dated restore evidence;
- effective production Canary SQL grants;
- production Redis ACL/TLS/freshness monitoring;
- selected production session/cache/queue topology;
- production mail provider, sender-domain readiness and bounce monitoring;
- centralized logs, metrics, alerts and on-call ownership;
- actual production deployment, migration and emergency rollback mechanism;
- launch-scope decisions and controlled smoke identities/data;
- all mutation-authorized critical production smoke results.

## CONFLICT

- archived Synology preflight requires restart policy `unless-stopped`;
- the current live runtime directly proves `always` for all six expected services;
- open PR #335 also proposes `always`, but remains unmerged and therefore does not yet reconcile the authoritative repository check.

## Blockers and ownership

| Blocker | Responsible operator |
|---|---|
| Synology runtime is `STAGING_TARGET`; production verifier exits 1 | Production/Synology runtime operator |
| WWW returns Cloudflare challenge instead of Platform | Cloudflare zone/security-policy operator |
| Game Gateway hostname has no usable TLS | Cloudflare zone/certificate operator |
| HTTP does not redirect to HTTPS and HSTS is disabled | Cloudflare zone/TLS operator |
| cloudflared host topology remains unproven | Synology/Cloudflare Tunnel operator |
| Production backup/restore, rollback, mail, observability and smoke prerequisites are absent | Production owner/operator |
| Launch-scope and controlled mutation-smoke inputs are unresolved | Repository owner |

## Single next action

Obtain explicit owner authorization for a separately guarded Cloudflare zone-edge audit/apply task covering certificate issuance for `login.oteryn.molehill.cloud`, the WWW challenge policy, HTTP-to-HTTPS redirect and HSTS. The task must first audit read-only, present the exact diff, risk and rollback, and must not change Tunnel ingress or DNS that are already current.
