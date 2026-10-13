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
| `OTERYN_STAGING_CUTOVER_WEB_SMOKE_COMMAND` | `/work/Oteryn-Platform/Oteryn-Platform/deploy/synology/scripts/cutover-hooks/web-smoke.sh` | 3. `smoke`, after `health-check.sh` | First a probe in the running `platform` service. Administrator: an active identity holds `admin.access` through its PostgreSQL role rows, and its MFA secret decrypts with the deployed key and is confirmed. A web session for it is started in the deployed session store and registered as the MFA challenge registers one after a correct code. Then a real `GET /admin` with that session's encrypted cookie goes through the HTTP kernel and the full middleware stack (cookie decryption, session, `EnsureIdentitySessionIsCurrent`, `auth`, `mfa.confirmed`, `admin.permission:admin.access`) and must return 200 with the admin dashboard. The session is then revoked and destroyed. Wallet, independent of the Marketplace switch: `wallet_accounts` and `wallet_ledger_entries` load through their models and the ledger reconciles (each balance equals the sum of its ledger deltas, no entry without its account), in a read-only transaction. The probe fails closed when no MFA-confirmed administrator exists: bootstrap the first administrator (`admin:bootstrap` for an MFA-confirmed identity) before this step. Its only write is that one administrator session (a revoked `identity_web_sessions` row stays). Then over host loopback, with the canonical proxy headers: `/health` returns 200. `/login` returns 200 with the canonical HTTPS form action, a CSRF field and a session cookie. `/news` returns 200 HTML. Then, actor to result: a new smoke identity (random `example.invalid` address and password, never printed or kept) registers through the CSRF-protected form, signs in (redirect to `/`), reads `/account` with its address, gets 403 from `/admin` (no MFA, no role), and signs out, after which `/account` redirects to `/login`. `/bazaar` must match the Marketplace switch the probe read: 200 and a 200 `/account/bazaar` when it is on, 404 when it is off. These are PostgreSQL identity, session and security-event writes, none witnessed; the smoke identity stays as test data. Not exercised (stated limit): the administrator typing the password and TOTP code into the sign-in forms, because the runner holds no administrator credentials. |
| `OTERYN_STAGING_CUTOVER_PROVENANCE_INIT_COMMAND` | `/work/Oteryn-Platform/Oteryn-Platform/deploy/synology/scripts/cutover-hooks/provenance-init.sh` | 4. `native-reenable`, called as `init <task_id> <close_run_id>` | The P6a step-4 begin (`PLATFORM_POSTGRES_TRUST_SAFEGUARDS_DESIGN.md` D5). See the steps below this table. |
| `OTERYN_STAGING_CUTOVER_NATIVE_SMOKE_COMMAND` | `/work/Oteryn-Platform/Oteryn-Platform/deploy/synology/scripts/cutover-hooks/native-smoke.sh` | 5. `native-smoke`, after the signing self-check | The native smoke actor is configured, or the owner's waiver names this task (below); otherwise it stops before any write. Native admission is on and paired in the environment file and in both running containers. The step-4 attempt is observed and closed (see below). Then a native login, actor to result, over loopback: a protocol-2 login with an unknown ticket gets 401 `NATIVE_LOGIN_AUTHENTICATION_REQUIRED` from the Platform issuer. Under the owner's waiver that is all, and the hook prints `native_smoke_login=owner-waived` (below). With the actor, it signs in on the web, authorizes the native (Rust) OAuth client with PKCE, exchanges the code at `/oauth/token`, issues a native ticket at `/api/v1/game-auth/tickets` (the same bearer is then refused), and logs in through the Gateway with that ticket and its character. The login must be admitted (200 with the same `attempt_ref`, a World, channel, Registry endpoint and an `oteryn-pre-admission-v1` grant), and a replay of the attempt must return the same admission; a 503 is retried with the same attempt, a bounded number of times. The actor is signed out, and the hook prints `native_smoke_login=end-to-end`. The final trust observation shows binding `match`, a profile, a non-null floor and the configured key trusted, and continues the committed attempt (identity, WAL position, nothing in flight). Writes attempt evidence and anchors, and the OAuth, ticket and admission-attempt rows of the login. |

`provenance-init.sh` runs these steps in order:

1. Run the location check and read the current kid and its `used.kids` count. The count must be 1.
2. Fetch every anchor of the locked anchor issue. If the last line of a run has no trusted anchor, post it now (D3 retry; the hook prints `anchor_retried=<run_id>/<seq>`) and fetch again. `verify --anchors` must pass. Nothing is written before this point.
3. Take a pre-init observation with `--key-id` and post a write probe.
4. Write `run-open`, then `init-intent`. Anchor each one, and write `anchored` for the intent.
5. Run `php artisan game-auth:native-evidence:init-provenance` in the compose `platform` service, then observe and write `init`.
6. Observe with `--key-id`, then write `attempt-intent`. Anchor it and write `anchored`.

A refused `open`, init or intent writes an anchored `begin-refused` and exits non-zero. After an anchor post fails, the next begin posts the missing anchor first; `open` then refuses the unclosed run (`attempt-unclosed`) when it holds an attempt-intent.

Each hook that writes evidence needs three more inputs: `OTERYN_CUTOVER_ANCHOR_ISSUE`, the number of the locked anchor issue (D3); `GITHUB_REPOSITORY`; and a `GH_TOKEN` with `issues: write`. Without them it exits before any evidence write. The *Run cutover step* and *Run cutover evidence step* steps map `vars.OTERYN_STAGING_CUTOVER_ANCHOR_ISSUE` to `OTERYN_CUTOVER_ANCHOR_ISSUE` and set `GH_TOKEN` to the job token. The `deploy` job grants `issues: write`. The owner must create and lock the anchor issue and set `OTERYN_STAGING_CUTOVER_ANCHOR_ISSUE` before the window.

`native-reenable` records its forward-only marker before the hook runs. So, unless it is a dry run, the *Run cutover step* step first runs `provenance-init.sh preflight <task_id> <close_run_id>`: steps 1 and 2 and the write probe, with no evidence write. A missing anchor issue, an unlocked issue, a token without `issues: write` or a failing `verify --anchors` then stops the step before the marker.

### Native smoke actor

**Owner waiver.** The owner waived the successful native login for this cutover: no smoke actor and no published native route are provisioned. The waiver is never a default. It is given per dispatch: the `native-smoke` step is dispatched with the workflow input `native_smoke_waiver` equal to `task_id` (the workflow passes it to the hook as `OTERYN_CUTOVER_NATIVE_SMOKE_WAIVER`), with none of the three actor settings below set. The control plane fills that input only from the owner's own waiver decision for this task; no repository variable or secret holds a waiver. Then `native-smoke` checks only that the Platform issuer refuses an unknown ticket and records `native_smoke_login=owner-waived` in its output. With neither an actor nor a matching waiver, a waiver for another task, or both a waiver and an actor, it stops before any write. Stated limit: there is no successful end-to-end native login (ticket redemption, scope assignment, route resolution and grant signing on PostgreSQL) before MariaDB retirement. The other step-5 checks (attempt evidence, the signing self-check and the final trust observation) are unchanged. The waiver does not cover step 3: an MFA-confirmed administrator (`admin:bootstrap`) is still required before `smoke`.

For the full login, set all three before step 5; they are never printed:

- `OTERYN_STAGING_CUTOVER_NATIVE_SMOKE_EMAIL` (repository variable) and `OTERYN_STAGING_CUTOVER_NATIVE_SMOKE_PASSWORD` (environment secret; the workflow passes it only to the `native-smoke` step, and the hook passes it to the request container by name): an identity without MFA.
- `OTERYN_STAGING_CUTOVER_NATIVE_SMOKE_CHARACTER_ID` (repository variable): the lower-case UUIDv7 of a character that identity owns, bootstrapped on the native route of a ready Game node (`NATIVE_LOGIN_OWNER_STEPS.md` steps 9 and 10, with the route published).

A refusal names its public code: `NATIVE_LOGIN_ROUTE_UNAVAILABLE` (route record, node readiness or route revision), `NATIVE_LOGIN_CHARACTER_CONFLICT` (wrong character for that World), `NATIVE_LOGIN_RATE_LIMITED` (wait and run the step again). The step can be run again; it acts on the evidence as below each time.

### Step 4 observe (native-smoke)

`native-smoke.sh` takes the task and close run from `OTERYN_CUTOVER_TASK_ID` and `OTERYN_CUTOVER_CLOSE_RUN_ID`, which `cutover-postgres.sh` checks against its state. It acts only on a run whose `run-open` records `cutover_ref` `<task_id>/close-<close_run_id>`; any other selected run stops it before a write. It first retries pending anchors and runs `verify --anchors` over the whole directory; if that fails, it stops before it reads or writes evidence. Then it reads the evidence directory:

- One attempt without an observation: take two trust observations at least 5 s apart. While the witness is busy or a transaction is in flight, try again, at most 12 rounds. Write `attempt-observation` (the tool classifies it) and anchor it.
  - `committed`: run the read-only signing self-check and write `self-check`. If it passed, write `run-close` with `done: true`. If it failed, the run stays open; the next `native-smoke` rechecks.
  - `failed-before-witness`: the run stays open and the step exits non-zero. Fix the cause outside trust state, then run `evidence-retry` for that run, the deploy and `native-smoke` again.
  - Any other class: write `run-close` with `done: false` and exit non-zero. The owner decides.
- One committed attempt whose self-check failed: observe again and write a late `self-check` with `--anchors`. The tool refuses it unless the late observation still classifies `committed`.
- One committed attempt whose last line is a passing `self-check` (the runner stopped before `run-close`): the tool refuses a second self-check, so write `run-close` with `done: true` from that record. The continuity check still runs before the smoke passes.
- An open run whose last attempt is observed and classified other than `committed` or `failed-before-witness` (the runner stopped before `run-close`): write the missing `run-close` with `done: false` and exit non-zero. The owner decides.
- An open run whose last line is a `failed-before-witness` observation: exit non-zero and write nothing; run `evidence-retry` first.
- An attempt-intent without its `anchored` line, or an open run with `init` (`late: false`) and no attempt-intent: exit non-zero and write nothing. Run `evidence-close-init` for that run first.
- One closed committed attempt (a rerun, for example after a runner loss before `NATIVE_SMOKE` was recorded): nothing is written, but the attempt counts only after a fresh continuity check (below).
- Anything else: exit non-zero (`has not recorded an attempt`); while native admission is still off, `evidence-rebegin` begins it again.

The login smoke and the final trust check run only after that.

Continuity check: a fresh trust observation with the attempt's key id, retried while the witness is busy or a transaction is in flight (at most 12 rounds). It must show the first publication (binding `match`, a profile, a non-null floor, the configured key trusted) and pass the tool's step-4 recheck rule against the committed attempt: the same PostgreSQL identity and store (a restart is allowed), a WAL position at or past every recorded observation, nothing in flight, and no blocking state in the directory. A swapped or restored database therefore fails even with the same store id and trust rows. The final trust check is this check; a rerun over a closed attempt also takes it before the login smoke. It writes nothing.

### Evidence recovery steps

Anchors count only when the workflow token posts them, so the recovery commands run through the workflow, never by hand. Run `deploy-synology-staging.yml` with `action=cutover`, the task id, `close_run_id`, `evidence_run_id` (the run's uuid4) and one of these `cutover_step` values. The *Run cutover evidence step* step calls `provenance-init.sh <command> <task_id> <close_run_id> <run_id>` (for `evidence-rebegin` with an empty `evidence_run_id`, without `<run_id>`); `dry_run` only prints the command. Every command refuses before any anchor post or evidence write unless the cutover state (`$OTERYN_STATE_DIR/postgres-cutover/cutover-state.env`) records `STEP_REENABLE_STARTED` with `TASK_ID` equal to `task_id` and `REENABLE_CLOSE_RUN_ID` equal to `close_run_id`, and the run's `run-open` records `cutover_ref` `<task_id>/close-<close_run_id>`.

| `cutover_step` | Command | Use it when | Result |
| --- | --- | --- | --- |
| `evidence-close-init` | `close-init` | step 4 stopped between `init-intent` and `init` | Binding `marker_only` or `match` with an anchored intent: rerun D1, write `init` with `late: true` and `run-close` with `done: false` (`close_init_result=init-late`). Binding `none` and no `anchored` line, or the store unchanged: `begin-refused` `init-not-run`, exit 0. Anything else: `begin-refused` `init-lost`, which blocks every later begin; the owner decides. |
| `evidence-close-init` | `close-init` | step 4 stopped between two other durable lines of its begin | The durable prefix is kept and the begin goes on from the next line. Only `run-open`: retake the `open` decision on the recorded pre-init observation; a refusal is an anchored `begin-refused`, otherwise write `init-intent`, run D1, write `init` and the attempt-intent (`close_init_result=resumed-begin`). `init` with `late: false` and no attempt-intent: observe and write the anchored attempt-intent (`resumed-attempt`). An attempt-intent without `anchored`: write `anchored`, reusing the anchor comment (`resumed-anchor`). `init` with `late: true` and no `run-close`: write `run-close` with `done: false` (`init-late`). The begin resumes only while native admission is off. After a `resumed-*` result, the cutover goes on with the deploy and `native-smoke`. Any other run (closed, ending in `begin-refused`, or with an anchored attempt-intent) is refused and nothing is written. |
| `evidence-observe` | `observe` | the attempt of `native-reenable` was not observed, for example after a runner loss | The same observe and close as in `native-smoke`, with `late: true`. |
| `evidence-retry` | `retry` | the last line of the run is a `failed-before-witness` observation and the cause is fixed outside trust state | D5 `step4-begin --retry`: a fresh observation with `--key-id`, then `attempt-intent` n+1 in the same run, anchored, with its `anchored` line (`retry_result=intent`). The preconditions are those of attempt 1; a refusal is an anchored `begin-refused` that ends the run (then use `evidence-rebegin`). Then run the deploy and `native-smoke`. |
| `evidence-rebegin` | `rebegin` | the newest run of this cutover ended in a `begin-refused` that is not permanent (a refused `open`, init or intent, `init-not-run`) or in the `run-close` of `init-late`; or, with `evidence_run_id` empty, `native-reenable` failed before `open` wrote any run of this cutover | The whole begin of `native-reenable` again, in a new run (`rebegin_of=<run_id>`, then the `init` outputs); `open` decides on the whole directory. Native admission must be off. A permanent reason (`store-changed`, `evidence-lost`, `init-lost`) or an older run is refused and nothing is written. Without a run id (`rebegin_of=none`) it refuses, and names the newest run and its last line, when a run of this cutover exists. Then run the deploy and `native-smoke`. |

`native-reenable` records `REENABLE_STARTED` before its hook runs, and `cutover-postgres.sh` never runs it again. So every state that step 4 can leave has a forward exit through these steps: no run of the cutover yet, because a check, the pre-init observation, the write probe or `open` failed (`evidence-rebegin` with an empty `evidence_run_id`), an unfinished or interrupted begin (`evidence-close-init`), an unobserved attempt (`evidence-observe` or `native-smoke`), a failed or missing self-check (the `native-smoke` recheck), `failed-before-witness` (`evidence-retry`), and a begin that ended without an attempt (`evidence-rebegin`). Only these stop the cutover for the owner: a `run-close` with `done: false` after a classified attempt (`unknown`, `restore-or-loss`, `failed-after-witness`, `witness-ahead`), a permanent `begin-refused` (`store-changed`, `evidence-lost`, `init-lost`), native admission already on when a begin must be taken again, and a self-check that keeps failing.

Each failure of the hook ends with a `next:` line that names the step for the state it leaves. Before `open` writes a run it is `evidence-rebegin` with an empty `evidence_run_id`; from `open` on the hook prints `attempt_run_id=<run_id>` at once and every later failure names that run id. If the log is lost, `evidence-rebegin` without a run id names the newest run of the cutover and its last line instead of opening another.

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
