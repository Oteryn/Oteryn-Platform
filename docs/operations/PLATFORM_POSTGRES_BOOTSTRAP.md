# Platform clean-slate bootstrap on PostgreSQL

Status: P3 runbook for [ADR 0046](../architecture/adr/0046-platform-persistence-moves-to-postgresql.md) and [the migration plan](PLATFORM_POSTGRES_MIGRATION_PLAN.md) (packet P3). The cutover itself (P6) calls this procedure; do not run it against the live Synology stack before P4 and P6 are merged and the P6 preconditions hold.

`deploy/synology/scripts/bootstrap-postgres.sh` takes an empty PostgreSQL database and an empty native evidence witness directory to a working Platform in one scripted procedure. No data is carried over from MariaDB (owner answer Q1). The script never drops, empties or deletes a database or a witness directory and never edits the MariaDB-era witness: it only moves that directory aside, unedited, to a dated name.

## The one rule

The first witnessed write on PostgreSQL is the native admission trust publication (P6 step 4, run through P6a: the provenance-only init and a durable attempt record, then `publish-trust`; this script does none of that). From that write on there is **no rollback and no re-run** of this bootstrap: the PostgreSQL witness directory is never emptied, deleted or lowered, and the database is never emptied under it. Recovery reconciles forward only ([P6a](PLATFORM_POSTGRES_TRUST_SAFEGUARDS_DESIGN.md)). Before that write, the only way to retry is a clean-slate re-run: an emptied database and an emptied new witness directory.

The script enforces this:

- `run` (operational, Compose mode) never publishes native admission trust and refuses both `--with-native-admission` and native admission being on: the P6a safeguards (provenance-only witness-store init, durable attempt record outside the database and the witness directory) are not part of this script, and cannot be run externally first because preflight rejects an existing binding. `--with-native-admission` exists only with `--local`, so CI exercises the publication mechanics on a throwaway witness; it is not an operational mode.
- `run` refuses a target database that has any relation, any witness-store binding (`native_game_evidence_witness_stores`), any signing trust row (`native_game_signing_trust_*`) or any native evidence row (`native_game_evidence_*`). A re-run on a bootstrapped database is therefore refused before any change, never partially applied.
- The Compose one-off containers that run PHP set `OTERYN_PLATFORM_FASTCGI_ENABLED=false`: the image entrypoint is PID 1 there, and with the pool enabled it would start PHP-FPM beside the command and map its normal exit to status 9 (failing step 2 after the migrations ran). `GAME_WORLD_HOST` is checked by the same PHP filters as `game-auth:world:ensure`, inside the preflight probe. The operational run additionally applies the `deploy.sh` policy (loopback or an exact RFC1918 IPv4 address, `validate-ipv4.sh private-or-loopback`) before any change, so a DNS name or IPv6 host that the command accepts cannot pass the bootstrap and then fail the next deploy; `--local` keeps only the command's rule.
- Two locks are taken before any change: `run.lock` in the state directory and a target-wide lock directory (`OTERYN_PG_BOOTSTRAP_TARGET_LOCK`, default `/var/lock/oteryn-postgres-bootstrap.lock`; `<witness directory>.bootstrap-lock` with `--local`), so two runs with different state directories cannot race on the same database and witness directory. The target-wide lock is taken first, before the state directory can be created, and a state directory created by a run that then refuses is removed again, so an exit-1 refusal leaves nothing behind. Each lock is released only by the run that took it. An existing state directory must be owned by the caller and not group- or world-writable, checked before any record in it is read. A refusal before the first change (record write or witness move) restores the state directory's original mode; after it the exit is 2. The target must be PostgreSQL 17 (16 or 17 with --local) and, operationally, the default connection must be pgsql, both checked before Step 1. The witness directory must be owned by the bootstrap user and its parent writable, both refused in the inspection before any change (the same predicates the later verify-aside applies). A stale lock of either kind refuses the run until the operator removes it with no bootstrap running.
- Every refusal check (services, witness directory, database probe, records) runs once before any change and again with `run.lock` held, so a concurrent run that migrated the database or advanced the witness in between is refused, not continued on a stale probe. The state directory mode is restricted only after the second pass. The witness directory is enumerated by glob, never by parsing `ls`, so a file name of newlines or a leading dash is seen and refused (`unsupported_name`).
- `run` refuses a witness directory that is not empty and is not exactly the MariaDB-era directory recorded at the freeze (same `witness-store.id`, same files, same contents). This covers a PostgreSQL store id, the missing-profile state (a floor left by a failed trust publication, with or without a new store id) and a recorded directory that advanced after the freeze.
- After moving the MariaDB-era directory aside, `run` creates the new directory (mode `0700`, owned by the Platform user, not a link) and stops before the schema step if it is not empty.

## Exit status

| Status | Meaning |
|---|---|
| `0` | Bootstrapped. The non-secret summary follows (below). |
| `1` | Refused. Every reason is printed and nothing was changed. |
| `2` | Failed after a change began. Before the first witnessed write: clean-slate re-run only (empty the database and the new witness directory, keep the moved-aside directory as it is). After it: forward reconciliation only. |

## Before you start

- P4 is merged: the `postgres` service, the runtime role and the migration owner role exist, and the environment file carries `DB_MIGRATIONS_USERNAME` and `DB_MIGRATIONS_PASSWORD` for the owner role. The script refuses without them (Compose mode).
- The target PostgreSQL database is empty, and the runtime (`pgsql`) and owner (`pgsql_migrations`) connections point at the same database. The script checks both.
- `GAME_WORLD_ID`, `GAME_WORLD_SLUG`, `GAME_WORLD_NAME`, `GAME_WORLD_REGION`, `GAME_WORLD_HOST` and `GAME_WORLD_PORT` are set for the World Registry row. With `OTERYN_NATIVE_ROUTE_PUBLISH_ENABLED=true`, `OTERYN_NATIVE_CHANNEL_KEY` is set too.
- Native admission is off for the bootstrap. Trust is published in P6 step 4 through P6a (which owns the provenance-only init before the attempt), after this bootstrap completed.

## Procedure

1. **Freeze** (P6 step 1). Stop `platform` and `marketplace-scheduler` (the witness writers), and the ingress: `internal-proxy`, `gateway` and `internal-mtls`. `record-witness` checks the writers; `run` checks all five and starts only `platform`, so ingress returns only with the deploy that follows, behind its trust gate. Then record the MariaDB-era witness:

   ```bash
   deploy/synology/scripts/bootstrap-postgres.sh record-witness
   ```

   It writes `witness-freeze.env` (directory, state, store id and a SHA-256 manifest of every file) to the bootstrap state directory, `${OTERYN_STATE_DIR:-/var/lib/oteryn-staging-state}/postgres-bootstrap` unless `OTERYN_PG_BOOTSTRAP_STATE_DIR` is set. It writes nothing in the witness directory. A second call with the same directory is a no-op; a call that sees a different directory is refused. A non-empty directory without `witness-store.id` cannot be recorded.

2. **Bootstrap** (P6 step 3 with native admission off):

   ```bash
   deploy/synology/scripts/bootstrap-postgres.sh run
   ```

   All refusal checks run first. Then the plan steps:

   | Step | Action |
   |---|---|
   | 1 | Move the MariaDB-era witness directory aside to `<dir>.mariadb-era-<UTC time>` (or prepare the empty directory when the freeze recorded none), create the new `0700` directory and verify it is empty. |
   | 2 | `php artisan migrate --database=pgsql_migrations` in a one-off container with the owner credentials. This seeds the built-in admin roles and permissions. |
   | 3 | Start `platform` alone (ingress stays stopped), keep existing Passport keys or run `passport:keys`. |
   | 4 | No trust publication, no witnessed write (native admission is off; the script refuses otherwise). Only `--local` (tests) can publish trust here, then runs the signing self-check. |
   | 5 | `game-auth:oauth-client:ensure` and `--rust`. |
   | 6 | `game-auth:world:ensure`; with route publication on, `game-auth:native-topology:issue` and print the new WorldId. The route itself is published by the next deploy. |
   | 7 | Printed only: the administrator and wiki steps need a registered identity (below). |
   | 8 | `production:verify-configuration` when `APP_ENV=production`; the self-check when native admission is on; a read-only check that no migration is pending, the roles, OAuth clients and World row exist, and that with native admission off no trust row, binding or witness file was written. |

   It records `bootstrap-complete.env` in the state directory; a later `run` with that state directory is refused before any probe.

3. **Native admission** (P6 step 4). Not part of this script. Once P6a is merged, its provenance-only init and attempt record run first and then `publish-trust` with the next deploy. From that write on the rule above applies.

### Summary output

The last lines are non-secret and safe to paste into the cutover record:

```text
bootstrap_result=complete
oauth_client_id=<id>
oauth_rust_client_id=<id>
native_world_id=<UUIDv7 or not-issued>
witness_moved_aside_to=<path or none>
```

## Operator follow-ups

- **OAuth client ids.** The ids are new. Update `OTERYN_OAUTH_CLIENT_ID` (and the Rust client's consumers) as in [NATIVE_LOGIN_OWNER_STEPS.md](NATIVE_LOGIN_OWNER_STEPS.md).
- **WorldId.** A new native topology gets a new WorldId. Set the repository variable `OTERYN_STAGING_NATIVE_WORLD_ID` (D171 fallback) to `native_world_id`, then deploy; the deploy publishes the route and runs `health-check.sh` with the full stack.
- **First administrator.** Register an identity, confirm MFA, then run `php artisan admin:bootstrap <email>` in the platform container. It needs an existing MFA-confirmed identity, so it cannot run on an empty database.
- **Wiki launch content (optional).** `php artisan wiki:launch-content:install <publisher>` after the same MFA confirmation.
- **MariaDB-era witness.** Keep `<dir>.mariadb-era-*` untouched. The P6 rollback (only before the first witnessed write) removes the still-empty PostgreSQL directory and moves this one back; after a successful cutover P6 deletes it together with the MariaDB volume. It is never edited or reused by PostgreSQL.
- **After a P6 rollback.** The bootstrap state directory still records the move and the completed run, so `record-witness` and `run` refuse. Rename that state directory (keep it as cutover evidence) before the next freeze, which then starts a new record.

## Local and CI runs

`--local` runs `php artisan` in a Platform checkout (`OTERYN_PG_BOOTSTRAP_PLATFORM_ROOT`, default this repository, PHP from `OTERYN_PG_BOOTSTRAP_PHP`) with the caller's environment, and requires `OTERYN_PG_BOOTSTRAP_STATE_DIR`. The witness directory comes from `GAME_AUTH_NATIVE_EVIDENCE_HIGH_WATER_DIRECTORY`.

`tests/ci/test_synology_postgres_bootstrap.py` covers the bootstrap from an empty database and an empty directory, with native admission off (APP_ENV production, `production:verify-configuration` passes) and on (APP_ENV preproduction, MariaDB-era directory moved aside), the refused re-runs, and each refusal: witness binding row, trust row, foreign store id, missing-profile directory, recorded directory advanced or removed after the freeze, and a new directory that is not empty after the move. Its live cases use a scratch database per case through `PGSQL_INTEGRATION_HOST`, `_PORT`, `_USER` and `_PASSWORD` and skip without them unless `OTERYN_PG_BOOTSTRAP_REQUIRE_LIVE=1`:

```bash
PGSQL_INTEGRATION_HOST=127.0.0.1 PGSQL_INTEGRATION_USER=... PGSQL_INTEGRATION_PASSWORD=... \
  OTERYN_PG_BOOTSTRAP_REQUIRE_LIVE=1 python3 tests/ci/test_synology_postgres_bootstrap.py
```

## Limits

- `production:verify-configuration` requires `APP_ENV=production`, while `publish-trust` runs only in `testing` or `preproduction`; Synology staging runs `preproduction`, so the script skips the former there and says so.
- `health-check.sh` needs the full stack; the deploy after the bootstrap runs it, not this script.
- `game-auth:native-topology:issue` refuses on PostgreSQL until the ADR 0044 amendment (P4) admits the PostgreSQL preproduction store; keep `OTERYN_NATIVE_ROUTE_PUBLISH_ENABLED=false` until then.
