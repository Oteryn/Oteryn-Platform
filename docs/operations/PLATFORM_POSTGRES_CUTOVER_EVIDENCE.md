# Platform PostgreSQL cutover evidence

This file records the dated, non-secret results of the staging MariaDB to PostgreSQL cutover (plan P6, `PLATFORM_POSTGRES_MIGRATION_PLAN.md`). The cutover is driven one step per dispatch by the `cutover` action of the *Deploy Synology Staging* workflow, which runs `deploy/synology/scripts/cutover-postgres.sh`. It records no password, dump, token, cookie or connection string.

Status: **pending – not executed**. Every field below is a placeholder until the owner-approved cutover window runs. Dry-run dispatches (`cutover_dry_run=true`, the default) are not evidence and are not recorded here.

## Run header

| Field | Value |
| --- | --- |
| Task id | `OTERYN-YYYYMMDD-<slug>` (pending) |
| Window (UTC) | pending |
| Repository head (`main`) | pending, 40-hex SHA |
| Platform image | pending, `ghcr.io/oteryn/oteryn-platform@sha256:…` |
| Gateway image | pending, `ghcr.io/oteryn/oteryn-gateway@sha256:…` |
| Runner | pending |
| `MARKETPLACE_ENABLED` | `false` (checked by preflight) |

## Cutover hooks

`cutover-postgres.sh` runs three executable hooks. Each repository variable holds the hook's absolute path in the runner checkout. The workflow maps `vars.<name>` to `OTERYN_CUTOVER_<…>_COMMAND`. Every hook exits non-zero on any failure, so its step fails closed. No hook prints the environment file, a password, a token, a cookie or a key.

| Variable | Stable path | Step | What it checks or writes |
| --- | --- | --- | --- |
| `OTERYN_STAGING_CUTOVER_WEB_SMOKE_COMMAND` | `/work/Oteryn-Platform/Oteryn-Platform/deploy/synology/scripts/cutover-hooks/web-smoke.sh` | 3. `smoke`, after `health-check.sh` | Over host loopback, with the canonical proxy headers: `/health` returns 200. `/login` returns 200 with the canonical HTTPS form action, a CSRF field and a session cookie. `/news` returns 200 HTML. Read-only. |
| `OTERYN_STAGING_CUTOVER_PROVENANCE_INIT_COMMAND` | `/work/Oteryn-Platform/Oteryn-Platform/deploy/synology/scripts/cutover-hooks/provenance-init.sh` | 4. `native-reenable`, called as `init <task_id> <close_run_id>` | The P6a step-4 begin (`PLATFORM_POSTGRES_TRUST_SAFEGUARDS_DESIGN.md` D5). See the steps below this table. |
| `OTERYN_STAGING_CUTOVER_NATIVE_SMOKE_COMMAND` | `/work/Oteryn-Platform/Oteryn-Platform/deploy/synology/scripts/cutover-hooks/native-smoke.sh` | 5. `native-smoke`, after the signing self-check | Native admission is on and paired in the environment file and in both running containers. The step-4 attempt is observed and closed (see below). A protocol-2 login with an unknown ticket, sent to the Gateway over loopback, gets 401 `NATIVE_LOGIN_AUTHENTICATION_REQUIRED` from the Platform issuer. The final trust observation shows binding `match`, a profile, a non-null floor and the configured key trusted. Writes attempt evidence and anchors. |

`provenance-init.sh` runs these steps in order:

1. Run the location check and read the current kid and its `used.kids` count. The count must be 1.
2. Fetch every anchor of the locked anchor issue. If the last line of a run has no trusted anchor, post it now (D3 retry; the hook prints `anchor_retried=<run_id>/<seq>`) and fetch again. `verify --anchors` must pass. Nothing is written before this point.
3. Take a pre-init observation with `--key-id` and post a write probe.
4. Write `run-open`, then `init-intent`. Anchor each one, and write `anchored` for the intent.
5. Run `php artisan game-auth:native-evidence:init-provenance` in the compose `platform` service, then observe and write `init`.
6. Observe with `--key-id`, then write `attempt-intent`. Anchor it and write `anchored`.

A refused `open`, init or intent writes an anchored `begin-refused` and exits non-zero. After an anchor post fails, the next begin posts the missing anchor first; `open` then refuses the unclosed run (`attempt-unclosed`) when it holds an attempt-intent.

Each hook that writes evidence needs three more inputs: `OTERYN_CUTOVER_ANCHOR_ISSUE`, the number of the locked anchor issue (D3); `GITHUB_REPOSITORY`; and a `GH_TOKEN` with `issues: write`. Without them it exits before any evidence write. The *Run cutover step* and *Run cutover evidence step* steps map `vars.OTERYN_STAGING_CUTOVER_ANCHOR_ISSUE` to `OTERYN_CUTOVER_ANCHOR_ISSUE` and set `GH_TOKEN` to the job token. The `deploy` job grants `issues: write`. The owner must create and lock the anchor issue and set `OTERYN_STAGING_CUTOVER_ANCHOR_ISSUE` before the window.

### Step 4 observe (native-smoke)

`native-smoke.sh` first retries pending anchors, then reads the evidence directory:

- One attempt without an observation: take two trust observations at least 5 s apart. While the witness is busy or a transaction is in flight, try again, at most 12 rounds. Write `attempt-observation` (the tool classifies it) and anchor it.
  - `committed`: run the read-only signing self-check and write `self-check`. If it passed, write `run-close` with `done: true`. If it failed, the run stays open; the next `native-smoke` rechecks.
  - `failed-before-witness`: the run stays open. Retry `n+1` is not implemented; the owner decides.
  - Any other class: write `run-close` with `done: false` and exit non-zero. The owner decides.
- One committed attempt whose self-check failed: observe again and write a late `self-check` with `--anchors`. The tool refuses it unless the late observation still classifies `committed`.
- One closed committed attempt: nothing is written.
- Anything else: exit non-zero (`has not recorded an attempt`).

The login smoke and the final trust check run only after that.

### Evidence recovery steps

Anchors count only when the workflow token posts them, so the recovery commands run through the workflow, never by hand. Run `deploy-synology-staging.yml` with `action=cutover`, the task id, `close_run_id`, `evidence_run_id` (the run's uuid4) and one of these `cutover_step` values. The *Run cutover evidence step* step calls `provenance-init.sh <command> <task_id> <close_run_id> <run_id>`; `dry_run` only prints the command.

| `cutover_step` | Command | Use it when | Result |
| --- | --- | --- | --- |
| `evidence-close-init` | `close-init` | step 4 stopped between `init-intent` and `init` | Binding `marker_only` or `match` with an anchored intent: rerun D1, write `init` with `late: true` and `run-close` with `done: false` (`close_init_result=init-late`). Binding `none` and no `anchored` line, or the store unchanged: `begin-refused` `init-not-run`, exit 0. Anything else: `begin-refused` `init-lost`, which blocks every later begin; the owner decides. |
| `evidence-observe` | `observe` | the attempt of `native-reenable` was not observed, for example after a runner loss | The same observe and close as in `native-smoke`, with `late: true`. |

After `init-late` or `init-not-run`, run `native-reenable` again. Any `run-close` with `done: false`, `init-lost` and `failed-before-witness` stop the cutover for the owner.

## Gate checks

| Check | Run id | Result |
| --- | --- | --- |
| Open-gate audit before preflight (`oteryn-edge-audit-result`, `gate_state=open`) | pending | pending |
| Maintenance-close run (`closure_run_id`) | pending | pending |
| Closed-gate audit (`closure_match=yes`) before freeze | pending | pending |
| External check: both canonical hosts return 503, a random label gets no 2xx/3xx | n/a | pending |
| Closed-gate audit before native re-enable / declare-done / retire | pending | pending |
| Maintenance-open run | pending | pending |
| Open-gate audit before unfreeze; both hosts reach their origins | pending | pending |
| `platform-cutover-gate-evidence` artifact (`cutover_complete` or `rollback_verified`) | pending | pending |

## Steps

| Step | Subcommand | Started (UTC) | Finished (UTC) | Duration (s) | Result |
| --- | --- | --- | --- | --- | --- |
| 0. Preflight | `preflight` | pending | pending | pending | pending |
| 1. Freeze | `freeze` | pending | pending | pending | pending |
| 2. Bootstrap (engine flipped to `postgres` by reviewed PR) | `bootstrap` | pending | pending | pending | pending |
| 3. Smoke | `smoke` | pending | pending | pending | pending |
| 4. Native re-enable (last rollback point passed when it starts) | `native-reenable` | pending | pending | pending | pending |
| 5. Native smoke | `native-smoke` | pending | pending | pending | pending |
| 6. Declare done | `declare-done` | pending | pending | pending | pending |
| 6. Retire MariaDB (owner-gated, destructive) | `retire-mariadb` | pending | pending | pending | pending |
| 7. Unfreeze | `unfreeze` | pending | pending | pending | pending |
| Rollback (only before step 4) | `rollback` | not used | not used | not used | not used |

Total frozen window (freeze to maintenance-open): pending. Rollback outcome, if any: pending.

## Bootstrap results

| Field | Value |
| --- | --- |
| `WorldId` (native world) | pending |
| OAuth client id (web) | pending, UUID |
| OAuth client id (Rust) | pending, UUID |
| Witness store id | pending, 64-hex |
| MariaDB-era witness moved aside to | pending, `<dir>.mariadb-era-<ts>` |
| Provenance init (P6a hook) | pending |
| First trust publication | pending |

## MariaDB retirement

| Field | Value |
| --- | --- |
| Owner delete confirmation supplied (phrase value is not recorded) | pending |
| `owner_delete_ack` names the task id | pending |
| Prerequisites: native smoke and declare-done recorded | pending |
| Volume deleted | pending, `<project>_mariadb_data` |
| Container removed | pending |
| MariaDB-era witness directory deleted | pending |
| MariaDB deleted at (UTC) | pending |

## Notes and deferrals

- Rollback is forward-only once step 4 starts; the recorded marker, not operator memory, enforces it.
- Until P7 removes `mariadb` from the compose and deploy path, an ordinary staging deploy would recreate the retired service and an empty volume; run none between retirement and P7.
- `health-check.sh` requires the MariaDB service, so health checks cannot run after retirement.
- Pending: record who approved the window and where the owner decision is archived (no personal data).
