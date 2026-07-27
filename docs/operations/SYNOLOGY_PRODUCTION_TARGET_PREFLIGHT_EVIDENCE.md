# Synology Production Target Preflight Evidence

## Purpose

This record describes the repository-owned live preflight for the existing local Synology staging target. It verifies the parts of a future production target that can be proven safely on the current NAS without exposing the service to the Internet.

The preflight does not configure DSM reverse proxy, router/NAT, public DNS, public certificates, Cloudflare, a real mail provider or an external monitoring service.

## Classification boundary

A successful run is classified only as `STAGING_PROVEN`.

The evidence artifact contains:

- exact trusted workflow source SHA;
- exact currently deployed Platform/Gateway release SHA;
- immutable Canary digest;
- status values for container singleton/running state, restart policies, network and binding restrictions;
- status values for named volumes, runtime health, Canary effective-grant verifiers, Redis AOF/ACL and rollback snapshot readiness;
- isolated restore-drill status, duration and aggregate base-table counts;
- `production_environment_proven: false`;
- a bounded list of remaining public-production gaps.

It contains no secret, environment variable, private key, database row, dump byte, user data, private endpoint inventory or copied `.env` content.

## Live target assertions

The workflow fails closed unless all applicable assertions pass:

1. exactly one running `mariadb`, `redis`, `canary`, `platform`, `internal-proxy` and `gateway` container belongs to the expected Compose project;
2. all long-running services use `unless-stopped`, while MariaDB and Redis report healthy;
3. all services use the expected private bridge network;
4. Platform `8000`, Gateway `8080` and Canary legacy login `7171` bind only to `127.0.0.1`;
5. Canary game TCP `7172` binds only to loopback or one exact private IPv4 address;
6. no wildcard binding exists and MariaDB, Redis and the internal TLS proxy publish no host ports;
7. Platform and Gateway use the same exact `sha-<40 hex>` release tag and Canary uses an immutable digest;
8. MariaDB, Redis, Platform storage, Canary data and internal TLS use named local volumes;
9. the persistent runner state directory and permission-restricted last-good image snapshot exist;
10. Platform/Gateway health and readiness, Canary TCP and all three Canary database privilege verifiers pass;
11. Redis AOF is enabled, the dedicated runtime ACL user can `PING`, and a write attempt is denied;
12. the restore drill streams each live staging database directly into a temporary isolated database, compares deterministic dump digests plus aggregate base-table counts, and drops temporary databases on every exit path.

## Restore-drill safety

The drill creates no retained dump file. `mariadb-dump` output is read in bounded chunks, hashed in memory and written directly to an isolated temporary restore database. The restored database is dumped through the same deterministic options and its in-memory digest is compared with the streamed source digest.

Only the final PASS status, duration and aggregate base-table counts enter the sanitized evidence artifact. No table name, row, dump byte or comparison file is retained or uploaded.

The drill does not change either source database. Temporary restore database names are generated from the workflow run identity and are removed in a Python `finally` block whether the workflow passes or fails.

## Exact live result

Validation date: `2026-07-27T14:44:23Z`

Workflow: `Synology Production Target Preflight`

Workflow source SHA: `50d917acd7fde333f0e74757ec1ced70e30c53de`

Live workflow run: `30275482522`

Evidence artifact: `synology-production-target-preflight-evidence-30275482522`

Evidence artifact digest: `sha256:b54ec5fc619201685fe792328dd9682e958b07f41ab6b5c2f9d6f255b1e2a704`

Inspected Compose project: `oteryn-staging`

Deployed Platform/Gateway release SHA: `415aa3febd04c8d9c61082d4a7451352bf084013`

Platform image: `ghcr.io/blakinio/oteryn-platform:sha-415aa3febd04c8d9c61082d4a7451352bf084013`

Gateway image: `ghcr.io/blakinio/oteryn-game-gateway:sha-415aa3febd04c8d9c61082d4a7451352bf084013`

Canary image digest: `sha256:784e5dbdcc64e311c48c51cd94aa206e2efa1e5eefb2f4ef40170d5aac55031f`

Result: `PASS`

Classification: `STAGING_PROVEN`

Production environment proven: `false`

### Sanitized result matrix

| Boundary | Result |
|---|---|
| Container singleton and running state | `PASS` |
| Restart policies | `PASS` |
| Private network membership | `PASS` |
| Host bindings fail closed | `PASS` |
| MariaDB and Redis unpublished | `PASS` |
| Immutable runtime image identities | `PASS` |
| Named persistent volumes | `PASS` |
| Runner state and last-good rollback snapshot | `PASS` |
| Platform and Gateway health/readiness | `PASS` |
| Canary game TCP | `PASS` |
| Canary effective-grant verifiers | `PASS` |
| Redis AOF and ACL boundary | `PASS` |
| Platform and Canary streaming restore drill | `PASS` |

Restore drill duration: `717610 ms` for this local staging dataset. This is controlled staging evidence and is not a production RTO or RPO.

Platform base-table count: `34`

Canary base-table count: `59`

The inspected local runtime uses the bounded single-instance profile `file` sessions, `file` cache and synchronous queue execution. The current local mail profile is Laravel `array` non-delivery. Those values are recorded as the current local-target facts, not as proof of a future multi-instance or real-mail production design.

## Remaining environment-only gaps

The following remain `UNKNOWN` for production:

- public DNS, real TLS certificate lifecycle, Cloudflare proxy/WAF/Access and direct-origin exposure;
- real production mail transport, sender-domain readiness and delivery/bounce monitoring;
- external centralized logging, metrics, alerting, retention/access policy and on-call routing;
- DSM/Hyper Backup schedule, retention, encryption, off-device copy and restore ownership;
- final public smoke against an exact production deployment;
- authoritative game-login bridge if Platform-originated game login is required for launch.

Issue #91 remains the sole real Production Go-Live execution tracker.