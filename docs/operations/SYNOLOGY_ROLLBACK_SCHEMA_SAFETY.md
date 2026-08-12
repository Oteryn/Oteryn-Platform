# Synology staging rollback schema-safety

## Scope

This contract applies to Synology **staging** deployment and recovery. It is not production rollback proof and must not be represented as such.

## Migration compatibility policy

Synology staging uses `expand-contract`. A release declares a schema compatibility identity and the schema identities its application can safely run against in `deploy/synology/release-contract.env`.

The deployment does **not** trust the workflow checkout as the source of that contract. Before a migration-bearing deployment it reads `/var/www/html/deploy/synology/release-contract.env` from the exact selected Platform image with networking disabled and a read-only container filesystem, and separately proves that the Platform and Gateway OCI `org.opencontainers.image.revision` labels identify the same exact 40-character application commit. This keeps compatibility metadata bound to the immutable application artifact even when a workflow checks out a different repository revision or deploys a historical release.

Destructive contract migrations must be separated from the release that first stops depending on the old schema.

Image rollback is allowed only when all of the following are proven from durable state:

1. the attempted/current release and last-good release have different exact Git SHAs;
2. all runtime images in release metadata are immutable `@sha256:` identities;
3. the database schema state is known rather than ambiguous;
4. the last-good application explicitly accepts the database's actual current schema identity;
5. the last-good release is marked rollback-eligible.

Missing or contradictory metadata fails closed.

## Durable state and first managed deployment

The deployment state directory stores:

- `candidate-release.env`: exact release SHA, immutable runtime image identities and the compatibility contract read from the selected Platform image;
- `current-release.env`: the last deployment that completed all staging health checks;
- `last-good-release.env`: the previous known-good release retained as the image rollback target;
- `schema-state.env`: independent database schema identity; it is written as `unknown` before migration begins and becomes `known` only after the migration command returns success;
- `backups/<old-sha>-before-<candidate-sha>/platform.sql`: pre-migration staging Platform DB backup;
- matching `evidence.env`: old/candidate release identities, the schema identity that actually existed at dump time and SHA-256 of the backup;
- `marketplace.env` when Character Bazaar staging has been configured: the bounded durable Marketplace enablement/escrow/transfer settings used to reconstruct its optional scheduler without losing staging secrets or state.

Before pulling candidate images, `deploy.sh` snapshots any currently running Platform, Gateway and Canary containers by Docker image ID and immediately resolves those images to immutable repository digests. This prevents a later pull of a mutable tag from changing the meaning of the recorded last-good runtime.

On the first deployment that introduces managed release state to an existing staging installation, a complete running-image snapshot is required. Platform and Gateway must expose the same valid OCI application revision. The existing pre-migration database/application pairing is recorded with a synthetic `observed-<release-sha>` schema identity in both application and schema state, then the normal pre-migration backup is created before migration begins. This synthetic identity describes the observed pre-migration state only; it is not presented as an application-authored compatibility contract.

If the Platform database is already non-empty but neither managed release state nor a complete provable running-image baseline exists, deployment fails **before migration**. A fresh empty database may proceed without a prior last-good release.

For an existing release, the deployment reads the actual known identity from `schema-state.env`, not the current application's primary schema ID, and records that identity in backup evidence. This distinction is required after a compatible image-only rollback, where the old application may intentionally be running against a newer schema it accepts.

The deployment then quiesces all known Platform DB consumers **before** the backup: Platform, Gateway and internal proxy are stopped, and a running optional Marketplace scheduler is stopped and verified stopped. Only after consumers are quiesced is the single-transaction pre-migration dump created. The candidate Platform container is not started until that preparation and backup have completed and schema state has been persisted as `unknown`. If the Marketplace scheduler was running before quiesce, it is recreated on the candidate Platform image only after migration succeeds and the new schema identity is known. Scheduler reconstruction loads the effective Character Bazaar control environment when present, otherwise the bounded durable `marketplace.env`, and verifies that the recreated scheduler is actually running the selected Platform image.

The schema state is deliberately independent from application state. A failed deployment after `migrate --force` can therefore never inherit the old release's schema claim.

## Normal compatible image rollback

Run `deploy/synology/scripts/rollback.sh`. It validates the actual schema identity against the last-good application's accepted schema identities before old images are pulled or started. The optional Marketplace scheduler is then reconciled to the selected last-good Platform image and the effective/durable Marketplace enablement state before release-state promotion. On success it updates application release state only.

**Image rollback does not reverse, restore or otherwise change database schema.**

## Migration-bearing failure recovery

If migration starts and its outcome is ambiguous, `schema-state.env` remains `SCHEMA_STATE=unknown`; ordinary image rollback is rejected.

Recovery is explicit and staging-only:

```bash
deploy/synology/scripts/recover-schema.sh /var/lib/oteryn-staging-state/backups/<old-sha>-before-<candidate-sha>/evidence.env
```

The recovery command refuses to proceed unless the managed backup exists, its SHA-256 matches the evidence, source/target release identities match durable last-good/candidate state, the last-good application explicitly accepts the schema identity recorded from the actual database at backup time, and schema state is tied to the failed candidate. Recovery evidence is parsed as an exact allowlisted data format and cannot override operational deployment values such as the configured Platform database name. It persists schema state as unknown before the first destructive database command. It stops the base Platform database consumers and, when present, the optional Marketplace scheduler and verifies that scheduler is no longer running. It then recreates only the configured staging Platform database, restores the verified dump, and records the restored schema identity only after the complete import succeeds.

Recovery does not change runtime images. Run `rollback.sh` separately afterward so image rollback still passes its compatibility gate and health probes.

No deployment or rollback path invokes schema recovery automatically. Laravel migration reversal is not used as a generic recovery mechanism.

## Health probes

Health-check helper containers are mapped at the common Docker invocation boundary to repository-pinned `alpine@sha256:...` and `python@sha256:...` identities. Existing probe behavior and retry bounds remain unchanged. CI contract tests require both immutable pins and coverage of every historical helper alias in `health-check.sh`.

## Required validation

Before merging changes to this contract:

- run `python3 tests/ci/test_synology_rollback_contract.py` through the Synology deployment-package CI gate;
- run shell syntax validation for all modified Synology scripts;
- build the Platform/Gateway/deploy-runner images and validate the Platform image contains the release contract;
- run repository deployment-contract / governance tests applicable to the exact head;
- validate compatibility with the concurrently owned staging workflow without editing that workflow;
- review failure before migration, unprovable legacy baseline, consumer quiesce failure, failure/ambiguity during migration, failure after migration but before health success, compatible rollback, incompatible rollback, verified backup recovery, recovery evidence mismatch, runtime OCI revision mismatch, Marketplace scheduler state preservation and historical-image contract selection;
- obtain a fresh independent review of the exact final head.

The task deliberately performs no protected staging or production deployment. The complete executable integration path is therefore validated with deterministic contract tests plus repository production-like/deep validation; this evidence must never be promoted to a claim of production rollback readiness.
