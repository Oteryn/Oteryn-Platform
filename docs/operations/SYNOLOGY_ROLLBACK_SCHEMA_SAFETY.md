# Synology staging rollback schema-safety

## Scope

This contract applies to Synology **staging** deployment and recovery. It is not production rollback proof and must not be represented as such.

## Migration compatibility policy

Synology staging uses `expand-contract` by default. A release must declare a schema compatibility identity and the schema identities its application can safely run against. Destructive contract migrations must be separated from the release that first stops depending on the old schema.

Image rollback is allowed only when all of the following are proven from durable state:

1. the attempted/current release and last-good release have different exact Git SHAs;
2. all runtime images in release metadata are immutable `@sha256:` identities;
3. the database schema state is known rather than ambiguous;
4. the last-good application explicitly accepts the database's actual current schema identity;
5. the last-good release is marked rollback-eligible.

Missing or contradictory metadata fails closed.

## Durable state

The deployment state directory stores:

- `candidate-release.env`: exact release SHA, immutable runtime image identities and compatibility contract for the deployment currently being attempted;
- `current-release.env`: the last deployment that completed all staging health checks;
- `last-good-release.env`: the previous known-good release retained as the image rollback target;
- `schema-state.env`: independent database schema identity; it is written as `unknown` before migration begins and becomes `known` only after the migration command returns success;
- `backups/<old-sha>-before-<candidate-sha>/platform.sql`: pre-migration staging Platform DB backup;
- matching `evidence.env`: old/candidate release identities, schema identity and SHA-256 of the backup.

The schema state is deliberately independent from application state. A failed deployment after `migrate --force` can therefore never inherit the old release's schema claim.

## Normal compatible image rollback

Run `deploy/synology/scripts/rollback.sh`. It validates the actual schema identity against the last-good application's accepted schema identities before old images are pulled or started. On success it updates application release state only.

**Image rollback does not reverse, restore or otherwise change database schema.**

## Migration-bearing failure recovery

If migration starts and its outcome is ambiguous, `schema-state.env` remains `SCHEMA_STATE=unknown`; ordinary image rollback is rejected.

Recovery is explicit and staging-only:

```bash
deploy/synology/scripts/recover-schema.sh /var/lib/oteryn-staging-state/backups/<old-sha>-before-<candidate-sha>/evidence.env
```

The recovery command refuses to proceed unless the managed backup exists, its SHA-256 matches the evidence, source/target release identities match durable last-good/candidate state, the backup schema identity matches the last-good release, and schema state is tied to the failed candidate. It stops application-facing services, recreates only the staging Platform database, restores the verified dump, and records the restored schema identity. It does not change runtime images. Run `rollback.sh` separately afterward so image rollback still passes its compatibility gate and health probes.

No deployment or rollback path invokes schema recovery automatically. Laravel migration reversal is not used as a generic recovery mechanism.

## Health probes

Health-check helper containers are mapped at the common Docker invocation boundary to repository-pinned `alpine@sha256:...` and `python@sha256:...` identities. Existing probe behavior and retry bounds remain unchanged. CI contract tests require both immutable pins and coverage of every historical helper alias in `health-check.sh`.

## Required validation

Before merging changes to this contract:

- run `python -m pytest tests/ci/test_synology_rollback_contract.py`;
- run shell syntax validation for all modified Synology scripts;
- run repository deployment-contract / governance tests applicable to the exact head;
- validate the deployment workflow contract without changing a concurrently owned workflow;
- review failure before migration, failure/ambiguity during migration, failure after migration but before health success, compatible rollback, incompatible rollback, verified backup recovery, and recovery evidence mismatch;
- obtain a fresh independent review.

Staging evidence must never be promoted to a claim of production rollback readiness.
