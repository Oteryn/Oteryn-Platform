# Platform PostgreSQL P6a: trust safeguards design note

Status: proposed. It must be accepted before any P6a implementation starts.

- Governing plan: `PLATFORM_POSTGRES_MIGRATION_PLAN.md`, *P6 / Trust safeguards (P6a)*.
- Decision: ADR 0046 §8.
- Producer contract: `OTERYN_V2_NATIVE_EVIDENCE_PRODUCER_CONTRACT.md`, store provenance and signing trust.

This note changes no code, configuration, schema, deployment, secret or data. Items marked **proposal** are choices for the producer-contract owner or the owner to accept (see *Questions*). Everything else restates the plan or the current code.

## Fixed by the plan

- The first `game-auth:native-admission:publish-trust` attempt on the clean-slate PostgreSQL database advances the trust-state witness before its database transaction commits (`NativeSigningTrustRegistry::publishTrustedKey`, `$advance(1)` before the profile insert). A failed attempt can therefore leave the **missing-profile state**: a trust-state floor of 1 with no profile row. `reconcile --trust` refuses that state (`NativeEvidenceRecoveryReconciler.php:61-71`), and so does a new publication (`NativeSigningTrustRegistry.php:47-50`).
- Before the first publication attempt in step 4, two things must hold:
  - **Provenance.** The PostgreSQL witness-store id and its database binding exist. They come from a provenance-only init that advances no namespace floor and runs at the start of step 4, after the last rollback point.
  - **Attempt evidence.** Durable evidence exists outside the database and outside the witness directory, on storage that no database or Platform volume restore rolls back. It binds a new P6 run id, that store id, the trust scope and the key id being published.
- The missing-profile state has **no automated repair**. If it occurs:
  - the cutover stops and is not declared done;
  - native admission stays off and the scope stays unavailable;
  - the owner decides.
- **A restore never synthesizes a profile.** P6a carries one negative test for this.
- **A successor key is new.** It appears in `used.kids` exactly once and is absent from database history.
- **Forward-only.** P6a never lowers, moves, edits or replaces the PostgreSQL witness, and nothing in it reopens a rollback or a P3 re-run.

## Boundary

P6a designs no repair of the missing-profile state and builds none. It gives P6 three things:

- an init that only creates provenance;
- a read-only observation of trust state;
- an evidence trail that is classified, but never acted on automatically.

Nothing in P6a writes a trust row, a key row or a floor. No classification authorizes a write. Any future repair needs its own design note, its own review and the producer-contract owner's review (plan *Trust safeguards (P6a)*). The proof below is an input to such a design, not an approval of one.

## Facts the design relies on

These facts were verified against `main` at `ba971b8`.

1. **Only `publishTrustedKey` can take the trust-state floor from absent to 1.**
   - `publishNextProfileVersion`, `revokeKey` and `revokeProfile` call `lockedProfile`, which throws before `$advance` when no profile exists.
   - `NativeEvidenceRecoveryReconciler::reconcileSigningTrustAsRevoked` throws on a missing profile, and on a null floor.
   - `NativeEvidenceSource` only `peek`s the trust-state namespace.
   - Two console commands reach `publishTrustedKey`:
     - `game-auth:native-admission:publish-trust`, allowed only in the `testing` and `preproduction` environments;
     - `game-auth:native-trust:publish-key`.
2. **The floor lock spans the database transaction.** `withNamespace` holds an exclusive non-blocking `flock` on `<namespace>.floor.lock` while the callback runs. The callback runs `DB::transaction(..., 3)`, so the lock is held until the commit or rollback returns. A shared lock on the same file is therefore granted only when no trust-state writer is inside its callback.
3. **After the floor reaches 1 with no profile, every writer refuses.**
   - `publishTrustedKey` throws when `$floor !== null` and no profile exists.
   - The other writers throw in `lockedProfile`.
   - The reconciler throws on a missing profile.
   - No application code deletes profile or key-version rows.
4. **`peek` is not read-only.** It goes through `resolvedDirectory`, then `assertStoreProvenance`, then `reconcileStoreProvenance`, which can create the store id or re-bind the database row. An observation must not use it.
5. **How `reconcileStoreProvenance` reconciles today:**

   | Marker | Row | Behaviour |
   | --- | --- | --- |
   | Present | Present | They must match. |
   | Absent | Present | Refuses. |
   | Present | Absent | Re-binds the row. The producer contract allows this after a database restore, and a test in `NativeEvidenceHardeningTest` asserts it. |
   | Absent | Absent | Creates both, unless the database has native history and the directory has no floor. |

6. **After P3 the PostgreSQL witness directory is empty and no binding row exists.**
   - `bootstrap-postgres.sh` refuses an existing binding.
   - So the init can only follow P3.
   - The init itself is the first write to the PostgreSQL witness (plan, step 4).
7. **These PostgreSQL identity functions work for a non-superuser role.** This was checked on PostgreSQL 16 with a temporary unprivileged role:
   - `pg_control_system()`, which gives `system_identifier`;
   - `pg_control_checkpoint()`, which gives `timeline_id`;
   - `pg_postmaster_start_time()`;
   - the `pg_database` oid;
   - `pg_relation_filenode()`;
   - `pg_current_wal_lsn()`;
   - `pg_current_snapshot()`;
   - `pg_prepared_xacts`, and the `xact_start` of same-role sessions in `pg_stat_activity`.
8. **Storage topology on the NAS:**
   - **Named volumes.** `postgres_data`, `platform_storage` (which holds the witness), the WAL spool and the external `oteryn-native-admission-keys` volume (which holds `used.kids`) all live under the Docker data root. The P5 backup work backs them up, or plans to.
   - **The deploy runner** is a container with the Docker socket. Its state directory is the host bind `/volume1/docker/oteryn/state`, which also holds the database backups (`$state_dir/backups`).

## Decisions

### D1. Provenance-only init (proposal: `game-auth:native-evidence:init-provenance`)

- **Shape.** A new public method `NativeEvidenceHighWaterWitness::initializeStoreProvenance(): NativeEvidenceProvenanceResult`.
  - It never calls `withNamespace`, `peek` or `$advance`, and it never writes a `.floor` file.
  - It resolves the directory the way `resolvedDirectory` does, without calling `assertStoreProvenance`.
  - It takes `witness-store.lock` with `LOCK_EX | LOCK_NB`. If that lock is busy, the result is `busy` and nothing changes.
  - It writes the marker with the existing `writeDurableFile` and the row with the existing `insertDatabaseStoreId`, in the existing order: marker first, then row.
- **Classification.** The method sorts every directory entry into one of these classes:

  | Class | Entries |
  | --- | --- |
  | `marker` | `witness-store.id` |
  | `store-lock` | `witness-store.lock` |
  | `floor` | `<64hex>.floor` |
  | `floor-lock` | `<64hex>.floor.lock` |
  | `marker-temp` | `witness-store.id.tmp.<16hex>` |
  | `floor-temp` | `<64hex>.floor.tmp.<16hex>` |
  | `unclassified` | Anything else, including a symbolic link, a subdirectory or a non-regular file |

- **Allowed states.** Each changes at most the missing half of the provenance:

  | State | Change | Result |
  | --- | --- | --- |
  | No marker, no row, no `floor`, `floor-lock` or `floor-temp` entry, and no native database history (`databaseHasNativeHistory()` is false) | Creates the marker, then the row | `created` |
  | Marker, no row, no `floor`, `floor-lock` or `floor-temp` entry, and no native history. This is a crash between the two writes of a previous init. | Inserts the row from the marker | `completed` |
  | Marker and row present and equal | None | `unchanged` |

  For the `unchanged` state, the result also reports the counts of `floor`, `floor-lock` and `floor-temp` entries.
- **Refusals.** Every other state refuses, with nothing changed:
  - a row without a marker;
  - a marker and row that differ;
  - a marker with any floor-class entry or with native history, but no row. This is stricter than the runtime re-bind; see question P1.
  - any floor-class entry without a marker;
  - any `unclassified` entry;
  - a malformed marker or row;
  - more than one witness-store row.
- **Temp files.** A `marker-temp` entry is reported and left in place. P6a never deletes, renames or edits any witness entry.
- **Output.** One JSON line: `{"result": "...", "store_id": "<64hex>", "entries": {...}}`. The command exits non-zero on any refusal.
- **Context.** It needs neither native admission nor an activated producer. Only `cutover-postgres.sh` calls it, in step 4 (D5). It is not wired into `deploy.sh` or any other route.

### D2. Read-only trust observation (proposal: `game-auth:native-evidence:observe-trust`)

- **Usage.** `observe-trust --trust=fresh [--key-id=<kid>]`. Without `--key-id` it uses the configured kid. It runs through the Platform image entrypoint, as `publish-trust` does.
- **It never writes.**
  - It creates no file, lock file, row or marker.
  - It does not call `peek`, `resolvedDirectory`, `assertStoreProvenance` or `withNamespace`.
  - It opens `witness-store.lock` and the trust-state `.floor.lock` read-only (mode `rb`), and only if they exist.
  - It takes `LOCK_SH | LOCK_NB` on each lock it opened. A busy lock gives `{"result":"witness_busy"}`.
  - A missing lock file is reported as `absent`, and no lock is taken for it.
  - The new method behind it is `NativeEvidenceHighWaterWitness::observe(string $namespaceHash): array`.
- **It reports one JSON object** with these fields:
  - `store`: the marker, the database row, and `binding: match | mismatch | marker_only | row_only | none`;
  - `scope`: issuer, profile and key purpose, with the purpose read from `game-auth.native_evidence.fresh_key_purpose`. When the purpose is not configured, the result is `scope_unconfigured`.
  - `floor`: the trust-state floor, null when the `.floor` file is absent, plus the count of `floor-temp` entries for that namespace;
  - `profile`: present, with id, `profile_version`, `issuer_revision` and `revoked`;
  - `key`: the requested kid, `configured_key_id` (from `NativeAdmissionKeyring::currentKeyId()` when the entrypoint exported one, otherwise null), `in_history` (the logic of `historicalKeyIdExists`) and `trusted` (the latest key version for the current profile is trusted);
  - `database`, filled from the new class `NativeEvidenceDatabaseIdentity`. It is null for drivers other than `pgsql`, and the evidence tool then refuses to classify.
    - `identity`: `system_identifier`, `timeline_id`, `postmaster_start_time`, `database_oid`, and the relfilenodes of `native_game_signing_trust_profiles`, `native_game_signing_trust_key_versions` and `native_game_evidence_witness_stores`;
    - `progress`: `wal_lsn` and `xmax` (`pg_snapshot_xmax(pg_current_snapshot())`);
    - `in_flight`: other backends of this database with an open transaction and the same role (`pg_stat_activity`, `xact_start IS NOT NULL`, excluding itself), and the count from `pg_prepared_xacts`.
- **Reads.** The database reads happen in one read-only transaction, opened after the shared floor lock is taken and closed before it is released.

### D3. Where the attempt evidence lives

- **Location (proposal).** A dedicated Synology shared folder: `/volume1/oteryn-attempt-evidence`, overridable with `OTERYN_ATTEMPT_EVIDENCE_HOST_PATH`.
  - The share has no snapshot schedule, no snapshot replication, and appears in no Hyper Backup task's restore set.
  - It is bind-mounted only into the deploy runner, at `/var/lib/oteryn-attempt-evidence` (`deploy/synology/runner/compose.yml`). Changing that file means redeploying the runner, which is an owner action before P6.
  - The merged draft's `/volume1/docker/oteryn/attempt-evidence` is rejected. It sits in the same `docker` share as the state directory and the database backups, so a snapshot restore of that share would roll it back with them.
- **Location checks.** `attempt_evidence.py check-location` runs before the first record of every run. It refuses unless every check passes:
  - The runner's own mount for `/var/lib/oteryn-attempt-evidence` has type `bind`. The mount is found with `docker inspect` of the runner container, which is named by its hostname.
  - Its host source is not under `docker info`'s `DockerRootDir`, so it is not a named volume.
  - Its source is not equal to, inside or containing:
    - any mount source of a container in the Oteryn Compose project, which covers `postgres_data`, `platform_storage`, the WAL spool and the key volume;
    - any other mount source of the runner, which covers the state directory and its backups.
  - Its shared folder (the first two path components, `/volumeN/<share>`) is not the shared folder of any of those sources.
  - It is a directory owned by root with mode `0700`. A probe file can be created there with `O_CREAT | O_EXCL` and removed. The probe is the only file P6a ever removes, and only in this check.
- **Append-only attribute.** The check also reads the directory's append-only attribute with the `FS_IOC_GETFLAGS` ioctl and records it as `dir_append_only`.
  - The runner has no `CAP_LINUX_IMMUTABLE`, so it cannot set the attribute. The owner sets it once on the host (`chattr +a`).
  - A missing attribute is recorded, not refused. See owner question O2.
- **Records.**
  - One file per run: `<run_id>.jsonl`, created with `O_CREAT | O_EXCL | O_APPEND` and mode `0600`. Each line is written in a single `write` call.
  - After each line the tool runs `fsync` on the file. After the create it also runs `fsync` on the directory.
  - An existing file is never reopened for create, truncated or rewritten.
- **Integrity.**
  - Every line carries `prev`, the SHA-256 of the previous line (`null` on the first line), and `seq`.
  - `verify` checks the chain of every file.
  - P6 prints each line's SHA-256 to the job log.
  - Uploading each run file as a workflow artifact is an option for the owner (O5). It is not needed for the design.
- **The writer.**
  - The writer is `deploy/synology/scripts/attempt_evidence.py`, which uses only the standard library and runs with `python3 -I`.
  - Its subcommands are `check-location`, `open`, `append`, `verify`, `classify` and `require-open-attempt`.
  - It starts no subprocess that writes; `check-location` only reads `docker inspect` and `docker info`.
  - It never reads or writes the witness directory, the database or the key volume. The data it records comes from the JSON output of D1 and D2, passed in by the caller.

### D4. Record format

Every line has the fields `v` (1), `run_id`, `seq`, `prev`, `type` and `at` (UTC, RFC 3339), plus fields that depend on the type:

| `type` | Written | Extra fields |
| --- | --- | --- |
| `run-open` | First line of a run | `store_id: null`, `location` (the check-location result), `pre_init` (D2 observation), `cutover_ref` (the P6 cutover evidence id, if P6 provides one) |
| `init` | After D1 | The D1 result and `store_id` |
| `begin-refused` | When an intent precondition fails | Observation and reason. The run then closes. |
| `attempt-intent` | Immediately before the deploy that publishes | `attempt` (n), `store_id`, `scope`, `key_id`, `kid_record_count` (the `used.kids` count for that kid, from the key tool), `observation` (D2) |
| `attempt-observation` | After that deploy ends | `attempt` (n), `deploy_exit` (operator-supplied, optional), `observation_1` and `observation_2` (D2, at least 5 s apart), `late` (true when written by a later run), `classification` |
| `run-close` | Last line | Final classification, and `done: true/false` |

- `run_id` is a new random UUID (version 4), generated when `open` runs. It is the "new P6 run id" the plan requires. It is never reused and never derived from earlier evidence.
- **Binding.** Every record after `init` carries the same `store_id`, and every attempt record carries the same `scope` and `key_id` as its intent. `verify` refuses a file that breaks either rule.
- **Secrets.** Records hold no secret, private key, password or token. The kid and the public trust values are public.

### D5. Step 4 sequence

`cutover-postgres.sh` belongs to P6, which owns these calls. P6a provides the commands and the tool.

1. **`step4-begin`**
   1. `check-location`.
   2. `open` a new run and write `run-open` with a pre-init observation.
      - The observation must show `binding: none`, `marker_only` or `match`, a null floor and no profile. `marker_only` and `match` occur only when an earlier P6 run stopped after its init.
      - If it does not, write `begin-refused` and stop.
      - If `run-open` cannot be written, the init does not run.
   3. Run the D1 init and write `init`. Any init result other than `created`, `completed` or `unchanged` writes `begin-refused` and stops.
   4. Read the current kid K from the key tool's `show` action. Run the key tool's new history check (D7) for K.
   5. Observe (D2) with `--key-id=K`.
   6. The intent preconditions are:
      - `binding: match` with the store id S from the `init` record;
      - a null floor and no `floor-temp` entry;
      - no profile;
      - K not in history;
      - `configured_key_id` equal to K, or null when native admission is off;
      - a non-null database identity;
      - `in_flight` all zero.
   7. If any precondition fails, write `begin-refused` and stop. Otherwise write `attempt-intent` n=1.
      - If the intent cannot be written, P6 does not turn native admission on.
2. **The operator turns native admission on through the deploy.** The deploy runs `publish-trust` and then the self-check (plan, step 4).
   - The step-4 deploy uses the same release and Compose definition as steps 2 and 3, so it recreates neither the `postgres` service nor any table.
   - If it did, the identity changes and the classification is `unknown`. That outcome fails safe.
3. **`step4-observe`**
   1. Two D2 observations at least 5 s apart. A `witness_busy` result or a non-zero `in_flight` count waits and retries within a bounded time. If the window runs out, the result is `unknown`.
   2. Classify (D6) and write `attempt-observation`.
   3. If the result is `committed` and the self-check passed, write `run-close` with `done: true`.
   4. If the result is `failed-before-witness`, the operator fixes the cause outside trust state and runs `step4-begin --retry`. That writes `attempt-intent` n+1 in the same run file after a fresh observation, with the same preconditions.
   5. Any other result writes `run-close` with `done: false` and stops the cutover for the owner.

Further rules:

- A new intent is refused while the previous attempt has no observation.
- A new run cannot start while a previous run has an unclosed attempt. `step4-observe --run <id>` then writes the missing observation with `late: true`.
- An intent is also refused while any run in the directory has an attempt classified `failed-after-witness`, `restore-or-loss` or `unknown`.

### D6. Classification (`attempt_evidence.py classify`, information only)

The classification compares one attempt's intent observation I with its post-attempt observations O1 and O2, and also reads the other runs in the directory.

- **Common conditions (C).** All of these must hold:
  - O1 and O2 are equal in every field except `progress`;
  - I, O1 and O2 have the same `database.identity` and the same binding (`match` with store id S);
  - `progress` does not decrease from I to O1 to O2;
  - `in_flight` is zero in O1 and O2.

| Classification | Condition | Meaning, and what P6 does |
| --- | --- | --- |
| `committed` | C holds. Profile present, not revoked. `issuer_revision` equals the floor. K is trusted. | The publication committed. P6 continues. |
| `failed-before-witness` | C holds. Floor still null, no profile. | Nothing was witnessed. A retry is safe (plan: "a retry on a consistent state is idempotent"). |
| `failed-after-witness` | C holds. I has a null floor and no profile. O1 and O2 have floor 1 and no profile. No run in the directory has a `committed` attempt for S. | A failed first publication; see *Proof*. The cutover stops and the owner decides. Nothing is repaired. |
| `witness-ahead` | C holds. Profile present with `issuer_revision` below the floor. | The existing ambiguous state. `reconcile --trust=fresh` applies only on the owner's decision (plan, step 4), and it leaves the profile revoked. |
| `restore-or-loss` | Some earlier attempt for S was `committed`, and O shows no profile, or a floor below that attempt's floor. | Data loss or a restore after a committed publication. The cutover stops and the owner decides. |
| `unknown` | Anything else, including a changed identity or binding, a null `database`, a broken hash chain, a profile with K untrusted, or observations that never stabilized. | The cutover stops and the owner decides. |

No classification triggers a write. `classify` prints a verdict and exits; no script maps any verdict to a command that writes trust state or witness state.

### D7. Successor-key and kid-history check (proposal: key tool action `check-kid-history`)

- The new action of `generate-native-admission-key.sh` reads `used.kids` without changing it. It prints, for each role key (`current`, `next`, `retiring`) and for any `--kid` argument, how many records `used.kids` holds for that kid.
- It exits non-zero when a count is not exactly 1.
- `step4-begin` uses it for K. Any later successor-profile procedure uses it for the successor kid, together with D2's `in_history`, which must be false.

### D8. Deploy guard for a first publication (proposal)

This guard goes in `oteryn_publish_native_admission_trust` in `deploy/synology/scripts/lib.sh`. It applies when `OTERYN_PLATFORM_DB_ENGINE=postgres` and the native admission flag is `true`, and it runs before `publish-trust`.

1. Observe (D2) with K, the kid that the entrypoint configures.
2. **Profile present for the fresh scope.** Continue without reading evidence. This covers every deploy after the cutover and every key rotation.
3. **No profile.** Continue only if all of these hold, and refuse otherwise:
   - `binding: match`;
   - a null floor;
   - `in_flight` all zero;
   - `attempt_evidence.py require-open-attempt --store S --key-id K --scope ... --identity <json>` passes. That command checks:
     - the newest run has an intent without an observation for exactly S, K and that scope;
     - the intent's `database.identity` equals the identity just observed;
     - no run in the directory carries a blocking classification.

Consequences:

- The first PostgreSQL publication on the NAS can only happen inside an intent window. The guard closes the gap between the intent observation and the write: a restore or table rewrite between them refuses the publication before any witness write.
- The guard writes nothing.
- When the floor is already 1 with no profile, the guard refuses before the registry would. The outcome is the same, with an earlier and clearer message.
- `bootstrap-postgres.sh` with native admission on in Compose mode on PostgreSQL then refuses, as intended. The bootstrap runbook already routes the first witnessed write through P6a. Its `--local` CI mode does not call this function and is not affected.

## Proof: failed publication versus restore

**Claim.** If `classify` returns `failed-after-witness` for attempt n, then the first-publication transaction of that attempt did not commit. The missing-profile state comes from a failed publication, not from a restore or from relational data loss. This holds under the assumptions below.

**Assumptions.**

- A1. Nobody runs manual privileged SQL that deletes profile or key-version rows on the live database between I and O2.
- A2. Nobody edits the witness directory by hand.
- A3. The evidence location meets D3, and it is not restored.

**Steps.**

1. **The floor came from a publication.**
   - I shows a null floor; O1 and O2 show floor 1.
   - By Fact 1, only `publishTrustedKey` moves the trust-state floor away from null, and it moves it to exactly 1, inside `withNamespace`, before its transaction.
   - Under A2, at least one `publishTrustedKey` call entered its callback for the fresh trust-state namespace between I and O1.
2. **No such call is still in flight.**
   - O1 and O2 took the shared floor lock, so no writer was inside `withNamespace` at those moments (Fact 2).
   - `in_flight` is zero, so no other same-role session of this database had an open transaction that a late `COMMIT` could still publish.
   - O1 and O2 agree, so nothing changed across the gap between them.
3. **A commit would still be visible.** Equal `system_identifier`, `timeline_id`, `postmaster_start_time`, `database_oid` and table relfilenodes between I and O exclude each of these between the two observations:
   - a physical restore or point-in-time recovery, which restarts the postmaster and starts a new timeline;
   - a restored volume, which restarts the postmaster;
   - a dropped and recreated database, which gets a new oid;
   - `TRUNCATE`, `DROP`/`CREATE`, `VACUUM FULL`, `CLUSTER` or a rewriting `ALTER` on the three tables, each of which gives a new relfilenode;
   - a `pg_restore --clean` of those tables, which drops and recreates them.

   Non-decreasing `wal_lsn` and `xmax` give an independent check against a rewind. No application code deletes those rows (Fact 3), and A1 excludes manual deletes. So a profile row committed by the publication in step 1 would still be present at O1.
4. **So the transaction did not commit.** O1 shows no profile. The publication's transaction rolled back, or was never committed, after `$advance(1)`. That is a failed publication.
5. **Nothing between I and the write was restored.** `require-open-attempt` (D8) compared the identity right before `publish-trust` with I. The state the publication saw is therefore the state I recorded.

**A restore never yields this verdict:**

- **A restore between I and O.** It changes identity (step 3), which gives `unknown`.
- **A restore before I.** I would show a non-null floor or a profile, so the intent is refused (D5) and there is no attempt to classify.
- **A restore after O2.** It cannot change records that already exist outside every restore scope (A3). A later observation is classified against the earlier `committed` record, which gives `restore-or-loss`.
- **A restore of only the database after a committed attempt.** The marker-only runtime re-bind (Fact 5) restores the binding row. The floor stays at 1 or above, with no profile. Every writer refuses (Fact 3). Any later evidence classifies the state as `restore-or-loss`, because a `committed` record exists for S.

**Limits.**

- The converse is not claimed. `unknown` and `restore-or-loss` do not prove that a restore happened.
- `failed-after-witness` does not say which writer failed (Fact 1 lists two commands). It only says that a first-publication transaction failed.
- A1 is the residual gap. A privileged `DELETE` inside the window cannot be told apart from a failed commit. Closing it would need a commit witness written in the same transaction, which is a producer change (question P3).

## Negative test: a restore never synthesizes a profile

This is `tests/Feature/GameAuth/NativeEvidenceRestoreNeverSynthesizesProfileTest.php`. It runs in both the default suite and the `phpunit.pgsql.xml` suite.

1. **Arrange.** Init (D1) creates the provenance. A real `publishTrustedKey` commits for kid K1. The floor is 1, and one profile and one key version exist.
2. **Restore A.** Delete the profile and key-version rows, which simulates a database restore to before the publication. Keep the witness.
3. **Restore B.** Restore A, plus delete the witness-store row, which simulates a restore to before the init.
4. **Act.** For each restore, run each of these entry points and record its outcome, whether it succeeds or throws:
   - D1 init;
   - D2 observe;
   - `publish-trust`, with the configured kid K1 and with a new kid K2;
   - `game-auth:native-trust:publish-key` with K2;
   - `reconcile --trust=fresh`;
   - `publishNextProfileVersion`, `revokeKey` and `revokeProfile`;
   - one native evidence read through `NativeEvidenceSource`;
   - `attempt_evidence.py classify` on fixture records.
5. **Assert.** After every step and at the end:
   - zero rows in `native_game_signing_trust_profiles` and `native_game_signing_trust_key_versions`;
   - the trust-state floor is still 1, with byte-identical `.floor` content;
   - the content of `witness-store.id` is unchanged;
   - no trust-state `.floor.tmp.*` entry is left and no other trust-state floor appears;
   - init gives `unchanged` for A, and refuses with nothing changed for B (marker with floors, no row; see P1);
   - every writer throws `NativeEvidenceUnavailable` or `LogicException`.
6. **Runtime re-bind.** A separate assertion for B runs one runtime call that re-binds the row (Fact 5). The row comes back with the marker's id, and the profile and key-version tables are still empty. This documents that the re-bind restores provenance and never trust.

## Files and tests the implementation will touch

**Code (P6a).**

| Path | Change |
| --- | --- |
| `app/GameAuth/NativeEvidence/NativeEvidenceHighWaterWitness.php` | `initializeStoreProvenance()` and `observe()`, and the entry classifier. No change to `withNamespace`, `peek` or `reconcileStoreProvenance`. |
| `app/GameAuth/NativeEvidence/NativeEvidenceDatabaseIdentity.php` (new) | The `pgsql` identity, progress and in-flight reads; null on other drivers. |
| `app/Console/Commands/InitNativeEvidenceProvenance.php` (new) | `game-auth:native-evidence:init-provenance` |
| `app/Console/Commands/ObserveNativeSigningTrust.php` (new) | `game-auth:native-evidence:observe-trust` |
| `deploy/synology/scripts/attempt_evidence.py` (new) | The D3 to D6 tool |
| `deploy/synology/scripts/lib.sh` | The D8 guard in `oteryn_publish_native_admission_trust` |
| `deploy/synology/scripts/generate-native-admission-key.sh` | The D7 `check-kid-history` action |
| `deploy/synology/runner/compose.yml` | The evidence bind mount (D3) |
| `deploy/synology/README.md` | Evidence location, the host setup (shared folder, `chattr +a`, backup exclusions) and the guard |

**Tests (P6a).**

| Path | Cases |
| --- | --- |
| `tests/Feature/GameAuth/NativeEvidenceProvenanceInitCommandTest.php` (new) | `created`, `completed` after a simulated crash, `unchanged`, each refusal in D1 with nothing changed, `busy`, temp entries reported and kept, no `.floor` ever written |
| `tests/Feature/GameAuth/NativeSigningTrustObservationCommandTest.php` (new) | No file, lock or row is created on an empty directory. `witness_busy` while a namespace lock is held. Each binding value. The profile and key fields. Identity cases run only when the driver is `pgsql`. |
| `tests/Feature/GameAuth/NativeEvidenceRestoreNeverSynthesizesProfileTest.php` (new) | The negative test above |
| `tests/ci/test_synology_attempt_evidence.py` (new) | Location refusals (named volume, Docker root, inside or containing a project mount, same share, not writable). Exclusive create. An existing file is not reopened. Chain verify, including tamper detection. Each D6 classification from fixtures. `require-open-attempt` refusals. No subprocess with a write path. |
| `tests/ci/test_synology_native_admission_trust_publication.py` (extend) | Guard: profile present continues; no profile without an open attempt refuses; an identity mismatch refuses; a blocking classification refuses; MariaDB is unaffected |
| `tests/ci/test_synology_native_admission_gate.py` (extend) | `check-kid-history`: count 1 passes, 0 or 2 fails, read-only |

**Not P6a.** `deploy/synology/scripts/cutover-postgres.sh` (`step4-begin`, `step4-observe`) and the workflow input that drives it belong to P6, which calls the commands and the tool above.

Compared with the plan's expected list (`app/GameAuth/NativeEvidence/`, the console commands, `tests/Feature/GameAuth/`), the list above adds the deploy scripts, the runner Compose file and `tests/ci/` (owner question O3).

## Residual risks

- **Manual SQL in the window (A1).** See *Proof*, *Limits*.
- **A whole-NAS volume restore** would roll back every share, including the evidence share. D3 can only keep the evidence out of database, volume and share-level restores. This is owner question O1.
- **No append-only attribute** (O2). Without it, a root user on the host can delete or rewrite a run file. The hash chain and the job-log hashes make that detectable, but not impossible.
- **Producer calls may fail as busy.** The observation's shared lock can make a concurrent producer call fail closed as busy for the length of one read. Step 4 runs with the gate closed and the scheduler stopped, so no such call is expected.

## Questions

**For the producer-contract owner.**

- **P1. A stricter init.** The init refuses a marker with floors and no row, while the runtime re-binds that state (contract, store provenance). Recommendation: accept. The init runs only on a clean slate, and the runtime rule is unchanged.
- **P2. Verdicts as information.** Classifications are information only and authorize no write. Recommendation: accept.
- **P3. A commit witness.** Should a later producer change add a commit witness (for example the transaction id written in the profile transaction) to close the manual-SQL gap? Recommendation: not for P6. Treat it as a prerequisite for any future repair design.
- **P4. A runtime re-bind rule.** Should the runtime re-bind refuse when a trust floor exists without a profile? Recommendation: no. The writers already refuse, and changing provenance rules belongs to the contract.
- **P5. Busy failures.** Accept that the observation's shared lock can make a concurrent producer call fail closed as busy. Recommendation: accept.

**For the owner.**

- **O1. The evidence host path.** Is `/volume1/oteryn-attempt-evidence` the location? It must be a dedicated shared folder, outside every snapshot, replication, Hyper Backup and restore task. Options: that path; another dedicated share. Recommendation: that path.
- **O2. The append-only attribute.** Set `chattr +a` on the directory on the host. If the share's file system refuses it, continue with `dir_append_only: false` recorded? Recommendation: set it, and continue without it only if the file system refuses.
- **O3. Wider paths.** Accept that P6a's paths go beyond the plan's expected list: the deploy scripts, the runner Compose file and `tests/ci/`. Recommendation: accept, because the evidence must be written outside the Platform container.
- **O4. The deploy guard.** Accept the D8 first-publication guard in `lib.sh`, including that bootstrap Compose mode with native admission on now refuses on PostgreSQL. Recommendation: accept.
- **O5. Workflow artifacts.** Also upload each run file as a workflow artifact? Recommendation: optional. The job-log hashes are enough for the design.

## Owner approval

Accepting this note approves D1 to D8 as the implementation contract for P6a, subject to the answers to P1 to P5 and O1 to O5. It approves no missing-profile repair. P6 step 4 cannot start until P6a is merged and deployed, the runner has the evidence mount, and `check-location` passes on the NAS.
