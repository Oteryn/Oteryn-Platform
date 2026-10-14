# Platform PostgreSQL cutover evidence

This file records the dated, non-secret results of the staging MariaDB to PostgreSQL cutover (plan P6, `PLATFORM_POSTGRES_MIGRATION_PLAN.md`). The cutover is driven one step per dispatch by the `cutover` action of the *Deploy Synology Staging* workflow, which runs `deploy/synology/scripts/cutover-postgres.sh`. It records no password, dump, token, cookie or connection string.

Status: **executed on 2026-10-14, steps 0 to 7 (`unfreeze` run 38607404066); MariaDB retired (the recreated step 6 volume was removed on 2026-10-14, see below).** All steps ran for real (task `OTERYN-20261013-pg-cutover`); no archived owner decision verifies the window, see Approval below. One `unfreeze` dry run (38605943395, `OTERYN_CUTOVER_DRY_RUN: 1`, `nothing was executed or recorded`) preceded the real one and is not evidence. Dry-run dispatches (`cutover_dry_run=true`, the default) are not evidence. Every value below comes from the logs of the runs named in it; a value no log proves is `not recorded`. Failed and retried dispatches are not listed; only the successful run of each step is.

## Run header

| Field | Value |
| --- | --- |
| Task id | `OTERYN-20261013-pg-cutover` |
| Window (UTC) | 2026-10-14T06:18:14Z (preflight start) to 2026-10-14T17:33:13Z (`unfreeze` end) |
| Repository head (`main`) | Per step, as checked out by the run: `087c149cae0891af07aa1b118682eb86a55564a7` (preflight, freeze), `033886ee1175ff4a3e162f71b5e094ae0a064a99` (bootstrap), `243246a152d06f95b088b439f405bea2bd3f9130` (smoke), `309038e9b2a6205e5689726ced90190fedc5ec90` (native-reenable, native-smoke, declare-done, retire-mariadb). `fbc8e30d87efdd8b9c0af51f3fd79cbb6703cd7f` (unfreeze, real run 38607404066 and the earlier dry run). |
| Platform image | `ghcr.io/oteryn/oteryn-platform@sha256:8b2a2cfbdaedf94f16dec91560414bdfc1d40d705e3099a4e6b4e98eabf7a59d` as the input of smoke, native-reenable, native-smoke, declare-done and retire-mariadb. Preflight, freeze and bootstrap were dispatched with no image input (not recorded). |
| Gateway image | `ghcr.io/oteryn/oteryn-game-gateway@sha256:5a58034c60581a1f1dc888c8d8f4f12fcb4e51ae7f5447987ddd06452b7b05aa` for the same steps; none for preflight, freeze and bootstrap (not recorded) |
| Runner | `oteryn-synology-platform` (group `platform-runners`, runner version 2.337.0) |
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
| Open-gate audit before preflight (`oteryn-edge-audit-result`, `gate_state=open`) | 38519629846 | `gate_state=open`, `closure_match=not_requested`; preflight run 38521372820 logged `Audit 38519629846 verified: gate_state=open.` |
| Maintenance-close run (`closure_run_id`) | 38521536252 | success (2026-10-14T06:19:29Z to 06:25:28Z); passed as `close_run_id` to every later step |
| Closed-gate audit (`closure_match=yes`) before freeze | 38522229083 | `gate_state=closed`, `closure_match=yes`; freeze run 38523378106 logged `Audit 38522229083 verified: gate_state=closed.` |
| External check: both canonical hosts return 503, a random label gets no 2xx/3xx | n/a | freeze run 38523378106 logged `External check passed: both canonical hostnames return 503.`; the random-label result is not recorded in the log |
| Closed-gate audit before native re-enable / declare-done / retire | 38522229083 | `gate_state=closed` verified in run 38588269252 (native-reenable), 38595000727 (declare-done) and 38595760762 (retire-mariadb) |
| Maintenance-open run | 38601451317 | success (2026-10-14T16:50:13Z to 16:56:11Z); logged `mode=maintenance-open`, `gate_state=open` |
| Open-gate audit before unfreeze; both hosts reach their origins | 38602856648 | `gate_state=open`, `closure_match=not_requested`. The real unfreeze 38607404066 logged `Audit 38602856648 verified: gate_state=open.` and `External check passed: both canonical hostnames reach their origins.` (as did the earlier dry run 38605943395). |
| `platform-cutover-gate-evidence` artifact (`cutover_complete` or `rollback_verified`) | 38595000727 | `gate_evidence_written=cutover_complete`; artifact id 11908977304 (319 bytes, retention 30 days) |

## Steps

Times are the start and end of the *Run cutover step* step of the successful run, in UTC, to the second as GitHub records them.

| Step | Subcommand | Started (UTC) | Finished (UTC) | Duration (s) | Result |
| --- | --- | --- | --- | --- | --- |
| 0. Preflight | `preflight` | 2026-10-14T06:18:14Z | 2026-10-14T06:18:17Z | 3 | `preflight_result=passed` (run 38521372820) |
| 1. Freeze | `freeze` | 2026-10-14T06:38:38Z | 2026-10-14T06:41:34Z | 176 | `freeze_result=done` (run 38523378106) |
| 2. Bootstrap (engine flipped to `postgres` by reviewed PR) | `bootstrap` | 2026-10-14T07:23:14Z | 2026-10-14T07:28:54Z | 340 | `bootstrap_result=complete` (run 38527939812) |
| 3. Smoke | `smoke` | 2026-10-14T12:16:50Z | 2026-10-14T12:21:59Z | 309 | `smoke_result=passed`, `web_smoke_result=passed` (run 38562536892) |
| 4. Native re-enable (last rollback point passed when it starts) | `native-reenable` | 2026-10-14T15:34:13Z | 2026-10-14T15:37:33Z | 200 | `native_reenable_result=provenance_recorded` (run 38588269252) |
| 5. Native smoke | `native-smoke` | 2026-10-14T16:02:40Z | 2026-10-14T16:03:50Z | 70 | `native_smoke_result=passed`, `native_smoke_login=owner-waived` (run 38594579918) |
| 6. Declare done | `declare-done` | 2026-10-14T16:05:26Z | 2026-10-14T16:05:29Z | 3 | `declare_done_result=recorded` (run 38595000727) |
| 6. Retire MariaDB (owner-gated, destructive) | `retire-mariadb` | 2026-10-14T16:10:41Z | 2026-10-14T16:12:26Z | 105 | `retire_mariadb_result=done` (run 38595760762) |
| 7. Unfreeze | `unfreeze` | 2026-10-14T17:32:15Z | 2026-10-14T17:33:13Z | 58 | `unfreeze_result=done`, `marketplace-scheduler` recreated and started (run 38607404066). The earlier dry run 38605943395 is not evidence. |
| Rollback (only before step 4) | `rollback` | not used | not used | not used | not used |

The native-smoke dispatch passed `native_smoke_waiver=OTERYN-20261013-pg-cutover` (the owner's waiver, logged as `OTERYN_CUTOVER_NATIVE_SMOKE_WAIVER`). It checked only that the Platform issuer refuses an unknown ticket (`native_smoke_unknown_ticket=refused`). The attempt classified `committed` and the self-check passed.

Smoke details (run 38562536892): `web_smoke_admin_access=granted`, `web_smoke_admin_mfa=confirmed`, `web_smoke_admin_page=read`, `web_smoke_wallet=read-reconciled` (0 accounts, 0 ledger entries), `web_smoke_account=registered-signed-in-signed-out`, `web_smoke_admin_boundary=refused-without-mfa`, `web_smoke_marketplace=off`.

Total frozen window (freeze to maintenance-open): 2026-10-14T06:38:38Z (freeze step start) to 2026-10-14T16:56:11Z (maintenance-open run end), 10 h 17 min 33 s (37053 s). The edge itself was closed earlier, by the maintenance-close run 38521536252 (06:19:29Z to 06:25:28Z). Rollback outcome: rollback not used.

## Bootstrap results

| Field | Value |
| --- | --- |
| `WorldId` (native world) | not issued (`native_world_id=not-issued`, run 38527939812) |
| OAuth client id (web) | `01a1394f-18c1-7241-a21c-d9e1abd75b17` (`oauth_client_id`, run 38527939812) |
| OAuth client id (Rust) | `01a1394f-1e08-701c-92d0-25629057faca` (`oauth_rust_client_id`, run 38527939812) |
| Witness store id | `44b47f1a5ebd07311de57502329ffae2e186da8a237a9cf42c2b33afc2dd3936` (PostgreSQL era, `witness_store_id` of native-reenable run 38588269252). The MariaDB-era store id, recorded at freeze (run 38523378106), was `f00aad784e0b1c62796bea784a0e8105db40b6d176363e01604d7d7c9c622bfc`. |
| MariaDB-era witness moved aside to | `/var/www/html/storage/app/native-evidence-witness.mariadb-era-20261014T072456Z` (`witness_moved_aside_to`, run 38527939812); `witness_aside_deleted=true` at `retire-mariadb` |
| Provenance init (P6a hook) | `provenance_preflight=passed`, `provenance_init_result=unchanged`, `attempt_run_id=2bb6c150-6a42-4af5-882b-cb9263c51382`, `attempt_intent_line_sha256=3f551c1be4df67df4ca676bcac9135d42eb4c51eb808e8dc94fde7de44de7c34` (run 38588269252) |
| First trust publication | `attempt_classification=committed`, `attempt_result=committed`, `self_check=passed`, key id `oteryn-admission-20261011-165131-58b205` (run 38594579918). The binding, floor and WAL position values are not recorded in the log. |

## MariaDB retirement

All values are from run 38595760762.

| Field | Value |
| --- | --- |
| Owner delete confirmation supplied (phrase value is not recorded) | yes (`OTERYN_OWNER_DELETE_CONFIRMATION` was set; value not recorded) |
| `owner_delete_ack` names the task id | yes (`OTERYN_OWNER_DELETE_ACK` was `OTERYN-20261013-pg-cutover`) |
| Prerequisites: native smoke and declare-done recorded | yes: native-smoke run 38594579918 (`native_smoke_result=passed`) and declare-done run 38595000727 (`declare_done_result=recorded`) precede it; the step ran only after the closed-gate audit 38522229083 verified |
| Volume deleted | Done after a second removal. The delete step removed `oteryn-staging_mariadb_data` (`retire_mariadb_result=done`), but a later compose line in the same log shows a volume of that name `Created` for the witness-deletion container, and `docker volume ls` on the NAS still listed it on 2026-10-14 (created 16:11:43Z, no container attached). Done outside the run, as a manual operator step that is not evidence from run 38595760762. On 2026-10-14 at about 21:20Z the owner-authorized `docker volume rm oteryn-staging_mariadb_data` ran on the NAS with no container attached; `docker volume ls` then listed no MariaDB volume. Source: operator record https://github.com/Oteryn/Oteryn-Platform/issues/1474#issuecomment-6179296065. |
| Container removed | `oteryn-staging-mariadb-1` (`Container … Removed`) |
| MariaDB-era witness directory deleted | yes (`witness_aside_deleted=true`) |
| MariaDB deleted at (UTC) | 2026-10-14T16:12:26Z (`mariadb_deleted_at`) |

## Notes and deferrals

- Rollback is forward-only once step 4 starts; the recorded marker, not operator memory, enforces it.
- P7 (PR #1599, merged 2026-10-14 as `fbc8e30d`) removed `mariadb` from the compose and deploy path, so an ordinary staging deploy no longer recreates the retired service, and `health-check.sh` no longer checks MariaDB.
- Approval: unverified by this record. The runs prove only that the confirmation phrase, the acknowledgement and the waiver passed their gates; no archived owner decision for the window, the `native_smoke_waiver` or `retire-mariadb` is linked here.
- Closed: the recreated `oteryn-staging_mariadb_data` volume was removed and verified absent (see step 6 above).
- Gap found after the cutover: the PostgreSQL store started with no `native_scope_assignments` row, so the Platform refused every `ReportRuntimeStatusV1` from the running node (409 `PLATFORM_CONFLICT`). The site showed "Dane niedostępne" and the client directory listed no world. Fixed on 2026-10-14 at about 21:49Z. As root, the operator ran the Game ops `assignment report` from the Game README "Epoch raise" note. It re-reports the committed Game assignment without writing a new one; the binary was the previous `oteryn-game-ops` in the game runner container, the host that has the needed glibc. The answer was `result=accepted` with ownership generation 3. Runtime reports then answered 200, and `GET /v1/client/directory` listed `Oteryn Staging`. A later cutover must re-report the scope assignment right after the native re-enable step. Source: operator record https://github.com/Oteryn/Oteryn-Platform/issues/1474#issuecomment-6179296065.
