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
3. after pulling the last-good images, Platform and Gateway OCI revision labels still resolve to the persisted last-good `RELEASE_SHA`;
4. the database schema state is known rather than ambiguous;
5. the last-good application explicitly accepts the database's actual current schema identity;
6. the last-good release is marked rollback-eligible.

Missing or contradictory metadata fails closed.

## Durable state and first managed deployment

The deployment state directory stores:

- `candidate-release.env`: exact release SHA, immutable runtime image identities and the compatibility contract read from the selected Platform image;
- `current-release.env`: the last deployment that completed all staging health checks;
- `last-good-release.env`: the previous distinct known-good release retained as the image rollback target;
- `schema-state.env`: independent database schema identity; it is written as `unknown` before a migration-bearing transition and becomes `known` only after the migration command returns success or an explicit verified recovery completes;
- `backups/<old-sha>-before-<candidate-sha>/platform.sql`: pre-migration backup for an existing managed/observed release;
- `backups/fresh-empty-before-<candidate-sha>/platform.sql`: verified empty Platform DB baseline for the first managed migration when no prior application release exists;
- matching `evidence.env`: release/baseline kind, source/target identity as applicable, the schema identity that actually existed at dump time, the exact Compose project and Platform database target, and SHA-256 of the backup;
- `marketplace.env` when Character Bazaar staging has been configured: the bounded durable Marketplace enablement/escrow/transfer settings used to reconstruct its Platform/scheduler runtime without losing staging secrets or state.

Before pulling candidate images, `deploy.sh` snapshots any currently running Platform, Gateway and Canary containers by Docker image ID and immediately resolves those images to immutable repository digests. This prevents a later pull of a mutable tag from changing the meaning of the recorded last-good runtime.

On the first deployment that introduces managed release state to an existing staging installation, a complete running-image snapshot is required. Platform and Gateway must expose the same valid OCI application revision. The existing pre-migration database/application pairing is recorded with a synthetic `observed-<release-sha>` schema identity in both application and schema state, then the normal pre-migration backup is created before migration begins. This synthetic identity describes the observed pre-migration state only; it is not presented as an application-authored compatibility contract.

If there is no managed or legacy running application baseline, `prepare-fresh-schema-baseline.sh` runs immediately before the candidate Platform can start. It requires the Platform database to contain zero tables and rejects any existing Platform/Gateway/internal-proxy or Marketplace scheduler consumer. It then takes a real SQL dump of that empty database, re-proves that the table count is still zero after the dump, records `BACKUP_BASELINE_KIND=fresh-empty` plus the exact candidate/Compose/database/digest identities, and records independent `fresh-empty` schema state. Only then may the normal migration preparation mark schema state unknown and start the candidate. A non-empty Platform database without a managed or provable legacy baseline fails closed before migration.

For a migration-bearing release change, the deployment reads the actual known identity from `schema-state.env`, not the current application's primary schema ID, and records that identity in backup evidence. This distinction is required after a compatible image-only rollback, where the old application may intentionally be running against a newer schema it accepts.

The deployment then quiesces all known Platform DB consumers **before** the backup: Platform, Gateway and internal proxy are stopped, and a running optional Marketplace scheduler is stopped and verified stopped. Only after consumers are quiesced is the single-transaction pre-migration dump created. The candidate Platform container is not started until preparation and backup complete and schema state is persisted as `unknown`.

After a successful migration, Marketplace reconciliation is based on the effective Character Bazaar control environment when it explicitly contains Marketplace state, otherwise the bounded durable `marketplace.env`. If Marketplace is enabled, both the browser-facing Platform container and the scheduler are force-recreated with the selected Platform image and verified enabled/running. A standard deploy therefore cannot leave the Platform service disabled while only the scheduler is enabled.

A redeploy of the exact already-current application SHA is not treated as a new migration boundary. The candidate must still accept the known schema, migration execution is skipped, the existing distinct `last-good-release.env` is preserved, and Marketplace runtime is reconciled before final health validation. This prevents a same-release repair/recreate from silently destroying the previous rollback target.

The schema state is deliberately independent from application state. A failed migration-bearing deployment can therefore never inherit the old release's schema claim.

## Normal compatible image rollback

Run `deploy/synology/scripts/rollback.sh`. It validates the actual schema identity against the last-good application's accepted schema identities before old images are started. After pulling the immutable images it also re-inspects Platform/Gateway OCI revision labels and rejects any mismatch with the persisted last-good release SHA. Optional Marketplace state is reconciled for **both** Platform and scheduler to the selected last-good image before health checks and release-state promotion. On success it updates application release state only.

**Image rollback does not reverse, restore or otherwise change database schema.**

## Migration-bearing failure recovery

If a migration-bearing transition is ambiguous or fails, ordinary image rollback is rejected whenever schema identity cannot be proven compatible.

The supported post-failure entry point is the manual `Recover Synology Staging Schema` GitHub Actions workflow (`.github/workflows/recover-synology-staging-schema.yml`). The deployment workflow intentionally removes its secret-bearing ephemeral `.env` even on failure, so recovery reconstructs a new ephemeral staging environment from the protected `synology-staging` Environment. It accepts only a managed relative evidence path of one of these forms:

```text
<old-40-char-sha>-before-<candidate-40-char-sha>/evidence.env
fresh-empty-before-<candidate-40-char-sha>/evidence.env
```

The workflow serializes with the same `synology-staging-deployment` concurrency group, checks out the exact dispatch SHA, and calls `recover-schema.sh` with the managed evidence path. It is a repository-delivered operator path; Issue #1007 does not authorize dispatching it against protected staging.

For a normal release baseline, recovery refuses to proceed unless the managed backup exists, its SHA-256 matches evidence, source/target release identities match durable last-good/candidate state, the last-good application explicitly accepts the schema identity recorded from the actual database at backup time, schema state is tied to the failed candidate, and the recorded Compose project plus Platform database name exactly match the reconstructed operational target.

For a first-deploy `fresh-empty` baseline, recovery additionally requires that no `current-release.env`, `last-good-release.env`, or legacy `last-good.env` exists, that the managed directory is exactly `fresh-empty-before-<candidate-sha>`, that the evidence claims no source application release, and that the restored database still contains zero tables before `fresh-empty` schema state is marked known. There is deliberately no image rollback target in this case; after recovery the operator explicitly retries the candidate deployment.

Recovery evidence is parsed as an exact allowlisted data format and cannot override operational values. All target/evidence checks complete **before** schema state is marked unknown or any destructive database command is issued. Recovery stops Platform DB consumers, recreates only the verified staging Platform database, restores the verified dump, and records schema identity only after complete import and baseline-specific verification.

No deployment or rollback path invokes schema recovery automatically. Laravel migration reversal is not used as a generic recovery mechanism.

## Health probes

Health-check helper containers are mapped at the common Docker invocation boundary to repository-pinned full `alpine@sha256:<64-hex>` and `python@sha256:<64-hex>` identities. Existing probe behavior and retry bounds remain unchanged. CI contract tests require both exact immutable digest syntax and coverage of every historical helper alias in `health-check.sh`.

## Required validation

Before merging changes to this contract:

- run `python3 tests/ci/test_synology_rollback_contract.py`;
- run `python3 tests/ci/test_synology_rollback_recovery_contract.py`;
- run `python3 deploy/synology/tests/test_fresh_baseline_contract.py`;
- run shell syntax validation for all modified/new Synology scripts;
- run the dedicated `Synology Rollback Contract` workflow on the exact head;
- build the Platform/Gateway/deploy-runner images and validate the Platform image contains the release contract;
- run repository deployment-contract / governance tests applicable to the exact head;
- validate compatibility with the concurrently owned staging deployment workflow without editing that workflow;
- review failure before migration, existing non-empty DB without baseline, first empty DB partial-migration recovery, consumer quiesce failure, failure/ambiguity during migration, failure after migration but before health success, compatible rollback, incompatible rollback, stale/incorrect last-good identity, verified backup recovery, target/evidence mismatch, runtime OCI revision mismatch, Marketplace Platform/scheduler state preservation and historical-image contract selection;
- obtain a fresh independent review of the exact final head.

The task deliberately performs no protected staging or production deployment. Repository CI and production-like/deep validation prove the implementation contract only; they must never be promoted to a claim of production rollback readiness.
