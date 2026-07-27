# Synology Production Target Preflight Evidence

## Purpose

This record describes the repository-owned live preflight for the existing local Synology staging target. It verifies the parts of a future production target that can be proven safely on the current NAS without exposing the service to the Internet.

The preflight does not configure DSM reverse proxy, router/NAT, public DNS, public certificates, Cloudflare, a real mail provider or an external monitoring service.

## Classification boundary

A successful run is classified only as `STAGING_PROVEN`.

The evidence artifact must contain:

- exact trusted workflow source SHA;
- exact currently deployed Platform/Gateway release SHA;
- immutable Canary digest;
- status values for container singleton/running state, restart policies, network and binding restrictions;
- status values for named volumes, runtime health, Canary effective-grant verifiers, Redis AOF/ACL and rollback snapshot readiness;
- isolated restore-drill status, duration and aggregate base-table counts;
- `production_environment_proven: false`;
- a bounded list of remaining public-production gaps.

It must not contain secrets, environment variables, private keys, database rows, dump bytes, user data, private endpoint inventories or copied `.env` content.

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
12. the restore drill streams each live staging database directly into a temporary isolated database, compares schema and per-table row-count manifests, and drops temporary databases on every exit path.

## Restore-drill safety

The drill does not create a retained dump file. `mariadb-dump` output is streamed directly into the isolated restore database through a pipe. Comparison files contain only table names and aggregate row counts, remain in a permission-restricted temporary directory and are removed by the exit trap.

The drill does not change either source database. Temporary restore database names are generated from the workflow run identity and are removed whether the workflow passes or fails.

## Exact live result

`PENDING` — populate after the first successful trusted-main live run and sanitized artifact inspection.

## Remaining environment-only gaps

Even after a successful local Synology preflight, the following remain `UNKNOWN` for production:

- public DNS, real TLS certificate lifecycle, Cloudflare proxy/WAF/Access and direct-origin exposure;
- real production mail transport, sender-domain readiness and delivery/bounce monitoring;
- external centralized logging, metrics, alerting, retention/access policy and on-call routing;
- DSM/Hyper Backup schedule, retention, encryption, off-device copy and restore ownership;
- final public smoke against an exact production deployment;
- authoritative game-login bridge if Platform-originated game login is required for launch.

Issue #91 remains the sole real Production Go-Live execution tracker.