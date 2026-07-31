# Character Bazaar Synology Staging Enablement

## Status and boundary

Character Bazaar application behavior was delivered by PR #270. This runbook covers only the missing Synology staging activation boundary.

The repository package is fail-closed:

- `config/marketplace.php` enables Marketplace by default only in the isolated `testing` and `acceptance` harnesses;
- normal local, staging and production runtimes require explicit `MARKETPLACE_ENABLED=true`;
- `deploy/synology/compose.marketplace.yml` is a separate overlay, so the ordinary Synology deployment remains disabled;
- the overlay adds no published port;
- live staging secret changes and staging data mutation require a separate explicit user authorization;
- production is excluded.

## Required exact runtime

Before any Marketplace control action, deploy the exact trusted-main Platform and Gateway images through `.github/workflows/deploy-synology-staging.yml` and verify the standard Synology health gate.

Do not run the Marketplace staging control against an older Platform image. The current image must contain:

- `App\Marketplace\Actions\ReconcileCharacterAuctions`;
- the `marketplace:reconcile` command and one-minute schedule;
- the Character Bazaar migrations;
- the character-transfer privilege verifier;
- the reviewed transfer SQL template.

## Staging-only secret

The Synology staging Environment requires one new dedicated value:

```text
OTERYN_STAGING_CANARY_CHARACTER_TRANSFER_DB_PASSWORD
```

Requirements:

- randomly generated staging-only hex/alphanumeric value;
- not reused from root, Canary, read-only, provisioning or character-create credentials;
- never committed, printed, attached to artifacts or copied into task/PR text;
- no production value may be used.

Adding or changing this secret is a human approval gate. Repository validation does not create it.

## Escrow identity

The reviewed staging identity is:

```text
name: oteryn_bazaar_escrow
creation marker: 1785456000
```

The enable action must fail if the name already exists with another creation marker. On an explicitly authorized first enablement it creates the row with:

- empty email;
- a generated one-way sink password hash whose plaintext is never retained;
- no Platform Identity binding.

Every authorized enablement rotates the sink hash. The account identifier is resolved from Canary and injected into both the Platform web process and Marketplace scheduler. The identifier is not accepted from browser input.

## Compose boundary

Always combine the normal Synology manifest with the Marketplace overlay:

```bash
docker compose \
  --env-file deploy/synology/.env \
  -f deploy/synology/compose.yml \
  -f deploy/synology/compose.marketplace.yml \
  config --quiet
```

The overlay provides:

- explicit Platform Marketplace/escrow/transfer environment;
- one `marketplace-scheduler` process using the exact Platform image;
- `php artisan schedule:work --no-interaction`;
- shared Platform storage for the scheduler mutex and logs;
- private-network-only database access;
- no host bind.

## Guarded actions

`deploy/synology/scripts/marketplace-staging.sh` supports exactly three actions.

### `enable`

This action mutates staging and must be separately authorized. It:

1. verifies the current Platform image contains the Marketplace implementation;
2. renders the operation-specific transfer grants without logging the password;
3. creates or validates the dedicated non-login escrow account;
4. initially recreates Platform with Marketplace disabled;
5. runs migrations and verifies effective transfer grants;
6. proves the escrow account is not bound to a Platform Identity;
7. runs a bounded reconciliation pass;
8. recreates Platform with Marketplace enabled;
9. starts the persistent scheduler;
10. refreshes the internal proxy after the Platform container IP changes;
11. verifies the public EN Bazaar route, scheduler state, grants and escrow configuration.

### `verify`

This is read-only with respect to application and Canary data. It resolves the reviewed escrow identity and verifies:

- Platform and scheduler are running with Marketplace enabled;
- both processes use a positive matching escrow identifier;
- effective character-transfer grants remain exact;
- the escrow account remains unbound;
- `/en/bazaar` is reachable inside the Platform network namespace.

### `prepare-rollback`

This action changes staging runtime state and must be separately authorized. It:

1. reconciles up to 1000 operations;
2. rejects rollback while any active, pending or recovery-required auction remains;
3. recreates Platform with Marketplace disabled;
4. stops/removes the Marketplace scheduler;
5. refreshes the internal proxy;
6. verifies Marketplace routes are absent.

Only after this action passes may the standard Synology image rollback workflow be used. Marketplace database migrations and ledger history are not rolled back automatically.

## Isolated validation

`.github/workflows/character-bazaar-staging-validation.yml` verifies on the exact PR head:

- shell syntax;
- merged base/overlay Compose validity;
- fail-closed Platform and scheduler defaults;
- no scheduler host port;
- exact Platform image reuse;
- exact scheduler command and restart policy;
- absence of checkout bind mounts;
- fail-closed non-test runtime configuration.

This validation does not establish a deployed staging state and never establishes `PRODUCTION_PROVEN`.

## Evidence and stop conditions

Record only sanitized evidence:

- exact trusted-main SHA and exact deployed image tags;
- workflow/run/job identifiers;
- grant verifier PASS/FAIL;
- escrow existence/unbound PASS/FAIL without account name beyond the reviewed constant or account ID if not needed;
- scheduler and route PASS/FAIL;
- first failure and one next action.

Stop immediately on:

- missing or reused transfer credential;
- current images not matching trusted main;
- escrow name/creation-marker conflict;
- escrow Platform binding;
- privilege-verifier failure;
- reconciliation recovery-required result;
- any non-terminal auction before rollback;
- scheduler or route health failure.
