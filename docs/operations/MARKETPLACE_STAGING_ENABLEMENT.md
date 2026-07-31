# Character Bazaar Synology Staging Enablement

## Status and boundary

Character Bazaar application behavior was delivered by PR #270. This runbook covers only Synology staging activation and recovery.

The package is fail-closed:

- normal local, staging and production runtimes require explicit `MARKETPLACE_ENABLED=true`;
- `deploy/synology/compose.marketplace.yml` adds the private scheduler and transfer configuration without publishing a port;
- staging mutations run only through `.github/workflows/character-bazaar-staging-control.yml`;
- production is excluded.

## Guarded control workflow

The workflow accepts these actions:

- `deploy-enable` — deploy an exact Platform/Gateway tag through the existing reviewed deploy script, then provision and enable Character Bazaar;
- `enable` — provision or rotate the staging-only transfer principal and escrow sink, migrate, reconcile, enable routes and start the scheduler;
- `verify` — verify the exact grants, unbound escrow identity, enabled Platform/scheduler environment and `/en/bazaar` route;
- `prepare-rollback` — reconcile, reject non-terminal work, disable routes and remove the scheduler;
- `rollback` — run `prepare-rollback` first and only then invoke the standard image rollback script.

The first trusted-main activation is marker-gated by `[character-bazaar-staging]`. Manual executions use `workflow_dispatch` and the `synology-staging` Environment. The workflow shares the `synology-staging-deployment` concurrency group with the standard deployment workflow.

Once Character Bazaar is active, operators must use this Marketplace-aware workflow for Marketplace deploy, verification and rollback actions. Direct standard rollback is not an approved Marketplace recovery path because it does not perform the drain gate.

## Staging-only transfer secret

The transfer password is generated with a cryptographically secure random source on the dedicated staging runner. It is never supplied through a browser, committed, printed or uploaded.

The runner stores only these Marketplace control values in:

```text
${OTERYN_STAGING_STATE_DIR:-/var/lib/oteryn-staging-state}/marketplace.env
```

The file must remain mode `0600` inside the existing mode `0700` staging state directory. It contains the dedicated transfer password and reviewed Marketplace runtime identifiers. It must not contain unrelated Platform, Canary, root, service-token or user credentials.

A prepared disabled state is written before the first database mutation. This preserves the generated credential for deterministic retry if a later enablement step fails. The final state is replaced atomically after the guarded action completes.

## Escrow identity

The reviewed staging identity is:

```text
name: oteryn_bazaar_escrow
creation marker: 1785456000
```

The enable action fails if the name exists with another creation marker. On first authorized enablement it creates the row with:

- empty email;
- a generated one-way sink password hash whose plaintext is never retained;
- no Platform Identity binding.

Every authorized enablement rotates the sink hash. The Canary account identifier is resolved by the control script and injected into Platform and scheduler configuration; it is not accepted from browser input or recorded in shared evidence.

## Transfer privilege boundary

The workflow renders `database/provisioning/canary-character-transfer.sql.template` only on staging and only with the dedicated operation-specific principal. The effective verifier must prove:

- `accounts.id` SELECT only;
- approved `players` snapshot columns SELECT;
- `players.account_id` UPDATE only;
- approved `cluster_sessions` columns SELECT only;
- no schema-wide privilege, wildcard write or grant option.

The principal may not reuse root, Canary game-process, read-only, provisioning or character-create credentials.

## Runtime sequence

The `enable` path performs this ordered sequence:

1. verify the running Platform image contains the Marketplace implementation and recovery command;
2. provision exact transfer grants;
3. create or validate the reviewed escrow identity and rotate its sink hash;
4. recreate Platform with Marketplace disabled;
5. run migrations and verify effective grants;
6. prove the escrow identity is unbound from Platform Identity records;
7. run bounded reconciliation;
8. recreate Platform with Marketplace enabled;
9. start one `php artisan schedule:work --no-interaction` scheduler using the exact Platform image;
10. refresh the private internal proxy after Platform recreation;
11. verify scheduler state, grants, escrow configuration and `/en/bazaar`.

The scheduler shares Platform storage for mutex and logs, uses only the private Compose network and has no host port.

## Rollback

`prepare-rollback` is mandatory before image rollback. It:

1. runs bounded reconciliation;
2. rejects rollback while any `escrow_pending`, `active`, `settlement_pending`, `cancel_pending` or `recovery_required` auction exists;
3. disables Marketplace routes;
4. stops and removes the scheduler;
5. refreshes the internal proxy;
6. verifies the route is absent.

Marketplace migrations, wallet ledger and auction history remain preserved. Never delete Marketplace rows or edit Canary ownership manually to force rollback.

## Evidence

The control workflow uploads sanitized evidence containing only:

- action;
- exact source SHA and workflow run ID;
- exact Platform image reference;
- enabled/disabled state;
- scheduler count;
- verifier and unbound-escrow PASS markers;
- `STAGING_PROVEN` or `STAGING_CONTROLLED` classification;
- `production_environment_proven: false`.

Do not include the transfer password, escrow account ID, user emails, Platform Identity IDs, Canary account IDs or raw database errors.

## Stop conditions

Stop immediately on:

- missing or unreadable runner state;
- current image without Marketplace support;
- escrow name/creation-marker conflict;
- escrow Platform binding;
- excessive or missing transfer privileges;
- reconciliation exit `1` or `2`;
- any non-terminal auction before rollback;
- scheduler or route health failure.

Repository CI and isolated staging-package validation do not prove live staging or production. Only a successful trusted-main control run may establish `STAGING_PROVEN`; it never establishes `PRODUCTION_PROVEN`.
