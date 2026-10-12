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
  - `fence_required`: the effective `first_publication_fence` setting is `required` (D8);
  - `floor`: the trust-state floor, null when the `.floor` file is absent, plus the count of `floor-temp` entries for that namespace;
  - `profile`: present, with id, `profile_version`, `issuer_revision` and `revoked`;
  - `key`: the requested kid, `configured_key_id` (from `NativeAdmissionKeyring::currentKeyId()` when the entrypoint exported one, otherwise null), `in_history` (the logic of `historicalKeyIdExists`) and `trusted` (the latest key version for the current profile is trusted);
  - `database`, filled from the new class `NativeEvidenceDatabaseIdentity`. It is null for drivers other than `pgsql`, and the evidence tool then refuses to classify.
    - `identity`: `system_identifier`, `timeline_id`, `postmaster_start_time`, `database_oid`, and the relfilenodes of `native_game_signing_trust_profiles`, `native_game_signing_trust_key_versions` and `native_game_evidence_witness_stores`;
    - `progress`: `wal_lsn` (`pg_current_wal_lsn()`, in its text form) and `xmax` (`pg_snapshot_xmax(pg_current_snapshot())`, as a decimal string);
    - `in_flight`: other backends of this database with an open transaction and the same role (`pg_stat_activity`, `xact_start IS NOT NULL`, excluding itself), and the count from `pg_prepared_xacts`.
- **Reads.** The database reads happen in one read-only transaction, opened after the shared floor lock is taken and closed before it is released.

### D3. Where the attempt evidence lives

- **Location (proposal).** A dedicated Synology shared folder: `/volume1/oteryn-attempt-evidence`, overridable with `OTERYN_ATTEMPT_EVIDENCE_HOST_PATH`.
  - The share has no snapshot schedule, no snapshot replication, and appears in no Hyper Backup task's restore set.
  - It is bind-mounted read-write only into the deploy runner, which runs as root (`user: "0:0"`), at `/var/lib/oteryn-attempt-evidence` (`deploy/synology/runner/compose.yml`). Changing that file means redeploying the runner, which is an owner action before P6.
  - It is also bind-mounted read-only into the `platform` service, at the same path (`deploy/synology/compose.yml`). The producer reads the open intent there to verify a first-publication fence (D8). The service runs as `www-data` (uid and gid 82, `platform.Dockerfile`), so it can read the evidence but never write it.
  - The merged draft's `/volume1/docker/oteryn/attempt-evidence` is rejected. It sits in the same `docker` share as the state directory and the database backups, so a snapshot restore of that share would roll it back with them.
- **Location checks.** `attempt_evidence.py check-location` runs before the first record of every run. It refuses unless every check passes:
  - The runner's own mount for `/var/lib/oteryn-attempt-evidence` has type `bind`. The mount is found with `docker inspect` of the runner container, which is named by its hostname.
  - **The source is canonical.** Docker reports a bind `Source` as written and does not resolve symlinks, so the check never trusts it alone. It reads the kernel's view of the same mount from `/proc/self/mountinfo` in the runner: the device (`major:minor`) and `root`, the path of the mounted directory inside its file system, with every symlink and host bind mount already resolved. It refuses unless `Source` equals its first component (`/volumeN`) followed by `root`. A symlink in any component, or a host bind mount at any component, makes `root` differ, so the check refuses. A layout where the volume is not mounted at the top of its file system also refuses; that fails closed.
  - Every comparison below uses that canonical path. For the runner's own other mounts (the state directory and its backups) it uses their `mountinfo` device and `root` in the same way, so an alias there is resolved as well. Two mounts on different devices are on different volumes.
  - Its canonical path is not under `docker info`'s `DockerRootDir`, so it is not a named volume.
  - Its canonical path is not equal to, inside or containing:
    - any mount source of a container in the Oteryn Compose project, which covers `postgres_data`, `platform_storage`, the WAL spool and the key volume;
    - any other mount of the runner, which covers the state directory and its backups.
  - The one allowed exception is the evidence mount of the `platform` service: the same `Source` string, same target, `RW: false` in `docker inspect`. Any other project container that mounts the source, or a writable Platform mount, is refused.
  - Its shared folder, the first two components of the canonical path (`/volumeN/<share>`), is not the shared folder of any of those mounts. Because the path is canonical, the share the owner excludes from snapshots, replication and Hyper Backup (O1) is the share that really holds the files.
  - **What the check cannot resolve.** The runner cannot see the mount table of the other project containers, so their `Source` strings and `DockerRootDir` are compared as written. For one of them to reach the evidence, root on the NAS would have to place a symlink or bind mount that points into the dedicated evidence share. That is the root-writer boundary of A5. As a cheap detector, the check also refuses when the directory holds any entry other than regular `<uuid>.jsonl` run files and the Synology system directories `@eaDir` and `#recycle`. That catches files written into it through such an alias.
  - It is a directory owned by root, group 82, mode `2750` (setgid, so run files get group 82), writable by the runner (`os.access` with `W_OK | X_OK`), on a file system that is not mounted read-only (`statvfs` without `ST_RDONLY`).
  - The check creates, removes and renames no file. It therefore also passes on an append-only directory, where `unlink` and `rename` are forbidden even to root. The first real write is the run file that `open` creates with `O_EXCL` (D5). If that create fails, the run stops before the init, so nothing irreversible has happened. P6a never removes or renames a file in this directory.
- **Append-only directory attribute (hardening, not the integrity anchor).** The check also reads the directory's append-only attribute with the `FS_IOC_GETFLAGS` ioctl and records it as `dir_append_only`.
  - On a directory, the attribute forbids `unlink` and `rename` of its entries, so a run file cannot be deleted or replaced by a rename.
  - It does not protect a file's content. The flag is per inode, a new file does not inherit it, and the runner has no `CAP_LINUX_IMMUTABLE` to set it on each run file. Content integrity comes only from the external anchor below.
  - The owner sets the attribute once on the host (`chattr +a`). A missing attribute is recorded, not refused. See owner question O2.
- **Records.**
  - One file per run: `<run_id>.jsonl`, created with `O_CREAT | O_EXCL | O_APPEND` and mode `0640`. Only root writes it; the Platform user can only read it. Each line is written in a single `write` call.
  - After each line the tool runs `fsync` on the file. After the create it also runs `fsync` on the directory.
  - An existing file is never reopened for create, truncated or rewritten.
- **Integrity: a hash chain plus a mandatory external anchor.**
  - Every line carries `prev`, the SHA-256 of the previous line (`null` on the first line), and `seq`. The SHA-256 of a line's bytes, without the trailing newline, is its *line hash*.
  - After every append, `append` prints an anchor object: `{"run_id", "seq", "type", "line_sha256"}`. It holds no secret.
  - **The anchor is mandatory.** P6 publishes each anchor out of band before its next action, as a comment on the P6 cutover tracking issue in this repository (REST API, the cutover workflow's token with `issues: write`; the issue is owner question O5). The comment is the durable anchor. P6 also writes the anchor to the workflow job summary and uploads the run file as a workflow artifact; both are conveniences, because artifacts and logs expire.
  - **Anchor retry.** Before posting, P6 fetches the issue's comments. If a comment already holds the same anchor object, for example because an earlier post succeeded but its response was lost, P6 reuses it and posts nothing. A failed post can be retried this way any number of times. If the anchor still cannot be posted, the step fails and the cutover stops. The line stays in the file, unanchored, and no further action runs on it.
  - **The `anchored` record (intents only).** An `attempt-intent` is not usable for a publication until P6 has posted its anchor. P6 then appends an `anchored` line (D4) that names the intent's `seq` and line hash and the id of the comment that holds its anchor. `append anchored` refuses unless the previous line is that intent and the comment id is given. The fence (D8) accepts only an intent whose run ends with that `anchored` line, so an intent whose anchor was never posted can never be used for a publication. The `anchored` line has no anchor of its own. Its content is fixed by the comment it names, and `verify --anchors` checks it against that comment instead.
  - `verify --anchors <file>` takes the anchors fetched from that issue, with their comment ids. It checks:
    - each file's chain;
    - that every anchored line exists with the same line hash;
    - that every `anchored` line names a line whose anchor is in the comment with that id;
    - that no file has a line after its last anchored line, other than one `anchored` line.

    `classify` returns `unknown` unless `verify --anchors` passes. `require-open-attempt` and the producer's fence check (D8) run inside the deploy and cannot fetch anchors. They rely on the `anchored` line, which only the P6 step writes, after the post succeeded (A5). `classify` checks the comment afterwards.
  - **What the anchor detects.** Any change to an anchored line after it was anchored: a rewrite, a truncation or a regenerated, self-consistent chain no longer matches the external line hashes. With `dir_append_only` set, deleting or renaming a run file is also refused.
  - **What P6a does not defend against.** A root user on the NAS, or anyone with the Docker socket, can write a false line before it is anchored, delete a run file when the directory attribute is not set, edit the witness directory, or run SQL against the database. P6a gives no protection against a hostile root on the NAS. The evidence is a record of an honest operator's run that cannot be rewritten later without detection. A repository admin can edit an anchor comment (the edit history stays visible) or delete it; that is also outside what P6a defends.
- **The writer.**
  - The writer is `deploy/synology/scripts/attempt_evidence.py`, which uses only the standard library and runs with `python3 -I`.
  - Its subcommands are `check-location`, `open`, `append` (including `append anchored`), `verify`, `classify` and `require-open-attempt`. It makes no network call; P6 posts and fetches the anchors.
  - It starts no subprocess that writes; `check-location` only reads `docker inspect`, `docker info` and `/proc/self/mountinfo`.
  - It is the only writer of the evidence directory. It never reads or writes the witness directory, the database or the key volume. The data it records comes from the JSON output of D1 and D2, passed in by the caller.

### D4. Record format

Every line has the fields `v` (1), `run_id`, `seq`, `prev`, `type` and `at` (UTC, RFC 3339), plus fields that depend on the type:

| `type` | Written | Extra fields |
| --- | --- | --- |
| `run-open` | First line of a run | `store_id: null`, `location` (the check-location result), `pre_init` (D2 observation), `cutover_ref` (the P6 cutover evidence id, if P6 provides one) |
| `init` | After D1 | The D1 result and `store_id` |
| `begin-refused` | When an intent precondition fails | Observation and reason. The run then closes. |
| `attempt-intent` | Immediately before the deploy that publishes | `attempt` (n), `store_id`, `scope`, `key_id`, `kid_record_count` (the `used.kids` count for that kid, from the key tool), `observation` (D2) |
| `anchored` | Right after an intent's anchor comment is posted (D3) | `for_seq` and `for_line_sha256` (the intent's), `anchor_comment_id`. Written only for an `attempt-intent`; its `prev` is that intent's line hash. |
| `attempt-observation` | After that deploy ends | `attempt` (n), `deploy_exit` (operator-supplied, optional), `observation_1` and `observation_2` (D2, at least 5 s apart), `late` (true when written by a later run), `classification` |
| `run-close` | Last line | Final classification, and `done: true/false` |

- `run_id` is a new random UUID (version 4), generated when `open` runs. It is the "new P6 run id" the plan requires. It is never reused and never derived from earlier evidence.
- **Binding.** Every record after `init` carries the same `store_id`, and every attempt record carries the same `scope` and `key_id` as its intent. `verify` refuses a file that breaks either rule.
- **One store per evidence directory.** Every `init` record in the directory, across all runs, carries the same `store_id`. `append init` refuses a different one, and `verify` refuses a directory that holds two. Each run has its own hash chain; there is no chain across runs. A run of the store that is deleted is still detected, because its anchored lines are missing (D3).
- **Intent hash.** The line hash of an `attempt-intent` line is its *intent hash*. The first-publication fence (D8) names the intent by its run id and intent hash. The producer reads the intent line itself and takes every value it checks from that line, never from the caller. `publish-trust` prints the run id, attempt and intent hash in its result, which binds the publication to that intent in the deploy log.
- **Secrets.** Records hold no secret, private key, password or token. The kid and the public trust values are public.

### D5. Step 4 sequence

`cutover-postgres.sh` belongs to P6, which owns these calls. P6a provides the commands and the tool.

1. **`step4-begin`**
   1. `check-location`.
   2. **Fetch every anchor** from the anchor issue, all pages, into a file. This happens before `open` and before the init. If an earlier run's last line was never anchored because a post failed, P6 first retries that post (D3, *Anchor retry*) and fetches again. A line whose post still fails leaves the check below failing, and the cutover stays stopped.
   3. `open --anchors <file>` a new run and write `run-open` with a pre-init observation. Before it reads any `init` record or the observation, `open` runs the `verify --anchors` checks (D3) over the whole directory against the fetched set. It also refuses when:
      - an anchor names a run id with no file, or a line that is missing or different;
      - a file in the directory has no anchored line at all, or has a line other than one `anchored` line after its last anchored line.

      So a deleted run file, which needs no `chattr +a` to be possible, is detected before the store-continuity rule below reads the directory. A deleted file can no longer make an emptied directory look like a first run.
      - The observation must show `binding: none`, `marker_only` or `match`, a null floor and no profile. `marker_only` and `match` occur only when an earlier P6 run stopped after its init.
      - **Store continuity.** If any run in the directory has an `init` record, with store id S0, the observation must show `binding: marker_only` or `match` with store id S0. `binding: none`, or any other store id, means the witness store that earlier evidence describes is gone: the database and `platform_storage` were restored or rebuilt to before that init. `open` checks this itself.
      - If any condition fails, `open` writes `begin-refused` after `run-open` and exits non-zero. The reason is `evidence-lost` for a failed anchor check, `store-changed` for the continuity rule, and otherwise the failed precondition. The anchor check runs first, so an `evidence-lost` run never reaches the continuity rule. P6 anchors both and stops. The init does not run, so no new store S2 is created.
      - If `run-open` cannot be written or anchored, the init does not run.
   4. Run the D1 init and write `init`. Any init result other than `created`, `completed` or `unchanged` writes `begin-refused` and stops. So does an init whose store id differs from S0, which `append init` refuses.
   5. Read the current kid K from the key tool's `show` action. Run the key tool's new history check (D7) for K.
   6. Observe (D2) with `--key-id=K`.
   7. The intent preconditions are:
      - `binding: match` with the store id S from the `init` record;
      - a null floor and no `floor-temp` entry;
      - no profile;
      - K not in history;
      - `configured_key_id` equal to K, or null when native admission is off;
      - a non-null database identity;
      - `in_flight` all zero.
   8. If any precondition fails, write `begin-refused` and stop. Otherwise write `attempt-intent` n=1.
      - P6 posts the intent's anchor (with the retry of D3) and then writes `anchored` for it. Only then does P6 turn native admission on.
      - If the intent cannot be written or anchored, or `anchored` cannot be written, P6 does not turn native admission on, and the fence refuses that intent.
   9. Every record of steps 1 and 3, except an `anchored` line, is anchored (D3) before the next action.
   10. **Recovery of an intent that was never anchored.** P6 first retries the anchor (D3). If the intent stays unanchored, `step4-observe` closes the attempt: it writes `attempt-observation` and anchors that. Its chain (`prev`) covers the intent line. No publication could have used that intent, because it has no `anchored` line. If the observation shows a null floor and no profile, the classification is `failed-before-witness`, and a retry writes intent n+1. Any other observation is classified as D6 says.
2. **The operator turns native admission on through the deploy.** The deploy runs `publish-trust` and then the self-check (plan, step 4).
   - The step-4 deploy uses the same release and Compose definition as steps 2 and 3, so it recreates neither the `postgres` service nor any table.
   - If it did, the identity changes and the classification is `unknown`. That outcome fails safe.
3. **`step4-observe`**
   1. Two D2 observations at least 5 s apart. A `witness_busy` result or a non-zero `in_flight` count waits and retries within a bounded time. If the window runs out, the result is `unknown`.
   2. Fetch the anchors, classify (D6) and write `attempt-observation`.
   3. If the result is `committed` and the self-check passed, write `run-close` with `done: true`.
   4. If the result is `failed-before-witness`, the operator fixes the cause outside trust state and runs `step4-begin --retry`. That writes `attempt-intent` n+1 in the same run file after a fresh observation, with the same preconditions, anchor and `anchored` line.
   5. Any other result writes `run-close` with `done: false` and stops the cutover for the owner.

Further rules:

- A new intent is refused while the previous attempt has no observation.
- A new run cannot start while a previous run has an unclosed attempt. `step4-observe --run <id>` then writes the missing observation with `late: true`.
- An intent is also refused while any run in the directory has an attempt classified `failed-after-witness`, `restore-or-loss` or `unknown`, or a `begin-refused` with reason `store-changed`.
- Once any run in the directory has a `committed` attempt, step 4 is complete and P6a defines no later run. `open` still writes `run-open`, so the observation is recorded, and then always `begin-refused`: reason `store-changed` when the continuity rule fails, otherwise `step4-complete`.
- **No reset in P6a.** A `store-changed` or `evidence-lost` refusal is permanent for that evidence directory. P6a provides no command that clears it. If the owner decides to start over, that is an owner action outside P6a: a new, empty evidence share and a new anchor issue, with a comment on the old issue that links the new one. The old share and the old anchors are kept. A new share alone does not work, because `verify --anchors` against the old issue fails for every run that is missing.

### D6. Classification (`attempt_evidence.py classify`, information only)

The classification compares one attempt's intent observation I with its post-attempt observations O1 and O2, and also reads the other runs in the directory.

- **Common conditions (C).** All of these must hold:
  - O1 and O2 are equal in every field except `progress`;
  - I, O1 and O2 have the same `database.identity` and the same binding (`match` with store id S), and all report `fence_required: true`;
  - `progress` does not decrease from I to O1 to O2;
  - `in_flight` is zero in O1 and O2.

| Classification | Condition | Meaning, and what P6 does |
| --- | --- | --- |
| `committed` | C holds. Profile present, not revoked. `issuer_revision` equals the floor. K is trusted. | The publication committed. P6 continues. |
| `failed-before-witness` | C holds. Floor still null, no profile. | Nothing was witnessed. A retry is safe (plan: "a retry on a consistent state is idempotent"). |
| `failed-after-witness` | C holds. I has a null floor and no profile. O1 and O2 have floor 1 and no profile. No run in the directory has a `committed` attempt for S. | A failed first publication; see *Proof*. The cutover stops and the owner decides. Nothing is repaired. |
| `witness-ahead` | C holds. Profile present with `issuer_revision` below the floor. | The existing ambiguous state. `reconcile --trust=fresh` applies only on the owner's decision (plan, step 4), and it leaves the profile revoked. |
| `restore-or-loss` | Some earlier attempt for S was `committed`, and O shows no profile, or a floor below that attempt's floor. Also the directory verdict when a run ends `store-changed` and some attempt for S0 was `committed`. | Data loss or a restore after a committed publication. The cutover stops and the owner decides. |
| `unknown` | Anything else, including a changed identity or binding, a null `database`, an attempt observed with a null floor and no profile whose next observation for S, before any later intent, shows a non-null floor or a profile (D8, *Attribution*), a broken hash chain, a failed `verify --anchors`, a profile with K untrusted, or observations that never stabilized. | The cutover stops and the owner decides. |

**Directory verdict.** `classify` also prints one verdict for the whole directory:

- `restore-or-loss` when a run ends with `begin-refused` reason `store-changed` and some attempt for S0 was `committed`;
- `unknown` when a run ends `store-changed` and no attempt for S0 was `committed`. That covers earlier runs that stopped after their init, before an intent, or with any other classification. Without a commit it cannot tell a restore from a rebuild, so the owner decides;
- `unknown` when `verify` finds `init` records for more than one store id;
- `unknown` when a run ends with `begin-refused` reason `evidence-lost`. A lost run file may have held a commit, so nothing narrower can be claimed;
- otherwise the most severe attempt verdict, in the order `unknown`, `restore-or-loss`, `failed-after-witness`, `witness-ahead`, `failed-before-witness`, `committed`.

No classification triggers a write. `classify` prints a verdict and exits; no script maps any verdict to a command that writes trust state or witness state.

### D7. Successor-key and kid-history check (proposal: key tool action `check-kid-history`)

- The new action of `generate-native-admission-key.sh` reads `used.kids` without changing it. It prints, for each role key (`current`, `next`, `retiring`) and for any `--kid` argument, how many records `used.kids` holds for that kid.
- It exits non-zero when a count is not exactly 1.
- `step4-begin` uses it for K. Any later successor-profile procedure uses it for the successor kid, together with D2's `in_history`, which must be false.

### D8. Deploy guard and first-publication fence (proposal)

D8 has two parts. The guard in `lib.sh` picks the open intent. The fence inside the publication makes the identity check and the first witness write one continuous step, so nothing can change between them.

**The guard.** It goes in `oteryn_publish_native_admission_trust` in `deploy/synology/scripts/lib.sh`. It applies when `OTERYN_PLATFORM_DB_ENGINE=postgres` and the native admission flag is `true`, and it runs before `publish-trust`.

1. Observe (D2) with K, the kid that the entrypoint configures.
2. **Profile present for the fresh scope.** Run `publish-trust` as today, without a fence. This covers every deploy after the cutover and every key rotation.
3. **No profile.** Continue only if all of these hold, and refuse otherwise:
   - `binding: match`;
   - a null floor;
   - `in_flight` all zero;
   - `attempt_evidence.py require-open-attempt --store S --key-id K --scope ... --identity <json>` passes. That command checks that:
     - the newest run ends with an intent with no observation, for exactly S, K and that scope, followed only by its `anchored` line (D5 writes it after the anchor is posted);
     - the intent's `database.identity` equals the identity just observed;
     - no run in the directory carries a blocking classification or ends `store-changed`, and every `init` record carries S.

     It prints the fence: `{"run_id", "intent_sha256"}`. The fence only names the intent. It carries no value that the producer trusts.
4. Run `publish-trust --first-publication-fence=<base64url JSON>`.

The guard writes nothing. Its checks give an early, clear refusal. They are not the security boundary, because a caller can skip the guard and call either command directly. The producer repeats every check from the evidence itself (below).

**The fence (a producer change; question P6).**

- **Config.**
  - A new setting `game-auth.native_evidence.first_publication_fence` (`GAME_AUTH_NATIVE_EVIDENCE_FIRST_PUBLICATION_FENCE`) takes `required` or `off`, with `off` as the default. `deploy/synology/compose.yml` passes it to the Platform services, and `lib.sh` sets it to `required` when the engine is `postgres`.
  - A new setting `game-auth.native_evidence.attempt_evidence_directory` (`GAME_AUTH_NATIVE_EVIDENCE_ATTEMPT_DIRECTORY`) names the read-only evidence mount (D3). `deploy/synology/compose.yml` sets it to `/var/lib/oteryn-attempt-evidence` for the `platform` service only. When it is unset or unreadable, a fenced first publication refuses.
- **Enforcement.** When the setting is `required`, `NativeSigningTrustRegistry::publishTrustedKey` refuses a first publication (no profile, null floor) unless it receives a `NativeEvidencePublicationFence`.
  - Both commands that reach it, `publish-trust` and `game-auth:native-trust:publish-key`, accept `--first-publication-fence`.
  - A manual `publish-key` therefore cannot bypass the guard.
  - With a profile present, the fence is not needed and is ignored.
- **The fenced path.** With a fence, the first publication runs through a new witness entry, `NativeEvidenceHighWaterWitness::withNamespaceFenced(string $namespaceHash, NativeEvidencePublicationFence $fence, Closure $callback)`:
  1. It takes `witness-store.lock` with `LOCK_SH | LOCK_NB` and holds it until the callback returns.
     - It never calls `reconcileStoreProvenance`. A marker-only state, such as a restored database without the binding row, is refused, not re-bound.
  2. It takes the exclusive floor lock, as `withNamespace` does.
  3. **It verifies the intent from the evidence directory**, under both locks and before the transaction. `NativeEvidencePublicationFence::verify` reads every `*.jsonl` file in the directory and refuses unless all of these hold:
     - every file's hash chain is intact (`v`, `run_id`, `seq` and `prev` as in D3 and D4);
     - the file `<run_id>.jsonl` exists; its last line is an `anchored` line, the line before it is an `attempt-intent` whose line hash equals `intent_sha256`, and the `anchored` line names that intent's `seq` and line hash and carries a comment id;
     - across all files, it is the only `attempt-intent` without an `attempt-observation`;
     - no `attempt-observation` in any file carries `failed-after-witness`, `restore-or-loss` or `unknown`;
     - every `init` record in the directory carries the intent's `store_id`, and no run ends with `begin-refused` reason `store-changed`;
     - the intent's `store_id` equals the marker and the row, its `scope` and `key_id` equal the call's, its `observation` shows a null floor and no profile, and its `observation.database` is not null.

     The verified line, not the caller, supplies the store id, scope, key id, `database.identity` and the `database.progress` baseline (`wal_lsn`, `xmax`) used below.
  4. Inside the database transaction, before `$advance(1)`:
     - it runs `LOCK TABLE` on `native_game_signing_trust_profiles`, `native_game_signing_trust_key_versions` and `native_game_evidence_witness_stores` `IN ROW EXCLUSIVE MODE`;
     - it reads the D2 identity fields and requires them equal to the intent's `database.identity`;
     - it requires `pg_current_wal_lsn() >= <baseline wal_lsn>::pg_lsn` and `pg_snapshot_xmax(pg_current_snapshot()) >= <baseline xmax>::xid8`, both evaluated in that transaction after the table locks;
     - it reads the binding row again and requires it equal to the intent's store id.

     Any refusal in steps 1 to 4 throws before `$advance`, so no witness write happens.
- **Trust boundary of the fence.**
  - The caller supplies only a run id and a hash. The producer accepts them only if they name a line that already exists in the evidence directory and passes the checks above. A caller who can only run `publish-trust` or `publish-key`, as the Platform user or through the guard, cannot make a fence pass for an intent that `attempt_evidence.py` did not write.
  - Writing a line that passes needs write access to the evidence directory: root on the NAS, or the Docker socket. That is the root-writer boundary that P6a does not defend against (D3, assumption A5).
  - The Platform user owns the witness directory and holds the database credentials. Code running as that user can write the floor or the tables directly and bypass the fence. That is the manual-write case excluded by A1 and A2, not something the fence defends against.
  - The producer has no network access and cannot fetch the anchor comment. It requires the `anchored` line instead, which P6 writes only after the post succeeded. An intent whose anchor post failed has no `anchored` line, so the fence refuses it, whoever calls. Writing a false `anchored` line needs root (A5). `classify` checks the named comment afterwards.
- **Attribution.** A floor is attributed to an attempt only by these rules:
  - With the setting `required`, every first publication, the only write that moves the floor from null (Fact 1), passes step 3. At that moment its intent, followed only by its `anchored` line, ends its run and is the only intent without an observation. So exactly one intent is ever eligible.
  - After the floor is 1, no first publication is possible, and every new intent is refused (D5). So the floor cannot be attributed to a later attempt.
  - A floor that first appears between intent n and its observation belongs to attempt n, whoever invoked the call. Several publish calls in one deploy all belong to attempt n.
  - A call holds the exclusive floor lock from step 2 until its transaction ends, so O1 and O2 cannot read the floor while it runs. One late case remains: a call that has not yet taken the lock when O1 and O2 are read (for example, a `publish-trust` still running after its `docker exec` client was killed) can take it and verify intent n before the observation line is appended. Attempt n is then recorded `failed-before-witness`, and the floor appears later. Once the observation line exists, intent n no longer passes step 3, so this can only happen to attempt n. The next `step4-begin --retry` sees the floor and writes `begin-refused`, never an intent, so the floor is never attributed to attempt n+1. `classify` then reports attempt n as `unknown` (D6), and the owner decides.
- **Why the fence is continuous.**
  - The row-exclusive table locks conflict with the `ACCESS EXCLUSIVE` lock that `TRUNCATE`, `DROP`, `VACUUM FULL`, `CLUSTER` and a rewriting `ALTER` need. No relfilenode can change between the identity read and the commit.
  - The open connection blocks `DROP DATABASE`.
  - A postmaster restart or a physical restore ends the transaction, which then cannot commit.
  - The shared store lock blocks the init and the runtime re-bind, which both need it exclusive, for the whole step.

  So the identity compared inside the transaction is the identity of the database that receives the commit, and the floor advances only after that comparison.
- **Result.** `publish-trust` prints `fence: {"run_id", "attempt", "intent_sha256"}` in its JSON result, with `attempt` taken from the verified intent line.

Consequences:

- The first PostgreSQL publication on the NAS can only happen while exactly one intent written by `attempt_evidence.py` is open, and only after its anchor was posted and its `anchored` line written. `classify` checks the anchor comment afterwards.
- A restore, a re-bind, a table rewrite or a rewind after the intent refuses the publication before the witness write. The guard's observation or the fence inside the transaction sees the changed identity, binding or progress.
- A PostgreSQL restart after `$advance(1)` but before the commit leaves the floor at 1 with no profile, and changes `postmaster_start_time`. The classification is then `unknown`, not `failed-after-witness`, which fails safe.
- A crash of only the PHP process after `$advance(1)` rolls the transaction back without changing the identity. That is a failed publication of attempt n, classified `failed-after-witness` if C holds.
- When the floor is already 1 with no profile, the guard refuses before the registry would. The outcome is the same, with an earlier and clearer message.
- `bootstrap-postgres.sh` with native admission on in Compose mode on PostgreSQL then refuses, as intended. The bootstrap runbook already routes the first witnessed write through P6a. Its `--local` CI mode does not call this function, does not set the fence to `required`, and is not affected.

## Proof: failed publication versus restore

**Claim.** If `classify` returns `failed-after-witness` for attempt n, then the first-publication transaction of that attempt did not commit. The missing-profile state comes from a failed publication, not from a restore or from relational data loss. This holds under the assumptions below.

**Assumptions.**

- A1. Nobody runs manual privileged SQL that deletes profile or key-version rows on the live database between I and O2.
- A2. Nobody edits the witness directory by hand.
- A3. The evidence location meets D3, and it is not restored.
- A4. The records used (I, O1, O2) pass `verify --anchors` (D3). No line was changed after it was anchored.
- A5. Only `attempt_evidence.py`, run by the P6 steps, writes the evidence directory. Nobody with root on the NAS or the Docker socket writes a line by hand (D3, *What P6a does not defend against*).

**Steps.**

1. **The floor came from a publication.**
   - I shows a null floor; O1 and O2 show floor 1.
   - By Fact 1, only `publishTrustedKey` moves the trust-state floor away from null. It moves it to exactly 1, under the floor lock, inside its database transaction, before the profile insert and the commit.
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
5. **The publication ran on the database that I describes.**
   - I, O1 and O2 all report `fence_required: true` (D2), so a first publication could only take the fenced path (D8).
   - That path held the shared store lock and the exclusive floor lock, and read I itself from the evidence directory. Under A5, a line that passes its checks was written by the P6 steps. While I was the only open intent, no other intent could pass (D8, *Attribution*), so the publication ran under attempt n.
   - Inside its transaction, under the table locks, it compared the database identity, progress and binding with the values in I, and only then advanced the floor.
   - The table locks held from that comparison to the end of the transaction exclude a rewrite in between, and a restart aborts the transaction.
   - So the transaction that advanced the floor ran on the database that I describes. No restore came between I and the write.

**A restore never yields this verdict:**

- **A restore between I and O.** It changes identity (step 3), which gives `unknown`.
- **A restore before I.** I would show a non-null floor or a profile, so the intent is refused (D5) and there is no attempt to classify.
- **A restore of both the database and `platform_storage` to before the init, after a committed run.** The pre-init observation shows `binding: none`. The store-continuity rule (D5) refuses the run before the init, so no new store, intent or publication follows. The directory verdict is `restore-or-loss`.
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
   - `publish-trust`, with the configured kid K1 and with a new kid K2, each with the fence setting `off` and again with `required` and a fence naming the open intent of a fixture evidence directory;
   - `game-auth:native-trust:publish-key` with K2, with and without that fence;
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
| `app/GameAuth/NativeEvidence/NativeEvidenceHighWaterWitness.php` | `initializeStoreProvenance()`, `observe()`, `withNamespaceFenced()` and the entry classifier. No change to `withNamespace`, `peek` or `reconcileStoreProvenance`. |
| `app/GameAuth/NativeEvidence/NativeEvidenceDatabaseIdentity.php` (new) | The `pgsql` identity, progress and in-flight reads; null on other drivers. |
| `app/GameAuth/NativeEvidence/NativeEvidencePublicationFence.php` (new) | The fence: parsing `{run_id, intent_sha256}`, reading and verifying the intent from the evidence directory, and the identity and progress comparison inside the transaction |
| `app/GameAuth/NativeEvidence/NativeSigningTrustRegistry.php` | `publishTrustedKey` takes an optional fence. With the setting `required`, a first publication refuses without a fence and runs through `withNamespaceFenced` with one. |
| `config/game-auth.php` | `native_evidence.first_publication_fence` and `native_evidence.attempt_evidence_directory` |
| `app/Console/Commands/PublishNativeAdmissionTrust.php` | `--first-publication-fence`, and the `fence` field in the result |
| `app/Console/Commands/PublishNativeTrustedKey.php` | `--first-publication-fence` |
| `app/Console/Commands/InitNativeEvidenceProvenance.php` (new) | `game-auth:native-evidence:init-provenance` |
| `app/Console/Commands/ObserveNativeSigningTrust.php` (new) | `game-auth:native-evidence:observe-trust` |
| `deploy/synology/scripts/attempt_evidence.py` (new) | The D3 to D6 tool |
| `deploy/synology/scripts/lib.sh` | The D8 guard in `oteryn_publish_native_admission_trust`, and the fence setting exported from the engine |
| `deploy/synology/compose.yml` | Passes `GAME_AUTH_NATIVE_EVIDENCE_FIRST_PUBLICATION_FENCE` to the Platform services; the read-only evidence mount and `GAME_AUTH_NATIVE_EVIDENCE_ATTEMPT_DIRECTORY` for the `platform` service (D3) |
| `deploy/synology/scripts/generate-native-admission-key.sh` | The D7 `check-kid-history` action |
| `deploy/synology/runner/compose.yml` | The evidence bind mount (D3) |
| `deploy/synology/README.md` | Evidence location, the host setup (shared folder, owner, group 82 and mode `2750`, `chattr +a`, backup exclusions), the anchor issue, the guard and the fence |

**Tests (P6a).**

| Path | Cases |
| --- | --- |
| `tests/Feature/GameAuth/NativeEvidenceProvenanceInitCommandTest.php` (new) | `created`, `completed` after a simulated crash, `unchanged`, each refusal in D1 with nothing changed, `busy`, temp entries reported and kept, no `.floor` ever written |
| `tests/Feature/GameAuth/NativeSigningTrustObservationCommandTest.php` (new) | No file, lock or row is created on an empty directory. `witness_busy` while a namespace lock is held. Each binding value. The profile and key fields. Identity cases run only when the driver is `pgsql`. |
| `tests/Feature/GameAuth/NativeEvidenceRestoreNeverSynthesizesProfileTest.php` (new) | The negative test above |
| `tests/Feature/GameAuth/NativeEvidencePublicationFenceTest.php` (new) | With `required`, a first publication without a fence refuses and writes no floor. Each of these refuses before `$advance`, with no `.floor` file: a fabricated fence whose hash names no line; an `init` record for another store id anywhere in the directory; a run ending `store-changed`; an intent that is not followed by its `anchored` line, as after a failed anchor post; an `anchored` line that names another `seq` or hash, or has no comment id; an intent followed by a line other than its `anchored` line; a second intent without an observation in another run; a broken hash chain in any file; a blocking classification in any run; a missing or unreadable evidence directory; an intent whose store id, scope or key id differs; a current identity that differs from the intent's; a current `wal_lsn` or `xmax` below the intent's baseline. A marker-only state refuses, and the row is not re-bound. With a profile present the fence is ignored. On `pgsql` only: the identity and progress refusals, a fenced first publication that commits and prints the intent's run id, attempt and hash, and a concurrent `TRUNCATE` of a trust table that waits until the fenced transaction ends. |
| `tests/Feature/GameAuth/NativeAdmission/NativeAdmissionPublishTrustCommandTest.php` (extend) | `--first-publication-fence` parsing, and the `fence` field in the result |
| `tests/ci/test_synology_attempt_evidence.py` (new) | Location refusals (named volume, Docker root, inside or containing a project mount, same share, not writable, read-only file system, wrong owner, group or mode, a Platform mount that is writable or at another target, a second project container mounting the source). Canonical path, from fixture `mountinfo` and `docker inspect` data: a symlinked share, a symlink in an intermediate component, and a host bind mount at the source each refuse because `root` differs from `Source`; a runner state mount whose `Source` is an alias into the evidence share refuses on its `mountinfo` path; a volume not mounted at the top of its file system refuses; a foreign entry in the directory refuses. The read-only `platform` mount passes. `check-location` calls no `unlink`, `rename` or create, checked by patching those calls to fail. Exclusive create; an existing file is not reopened. Anchors: a rewritten anchored line, a regenerated self-consistent chain, a truncated file, an unanchored tail and an `anchored` line whose comment does not hold that anchor each fail `verify --anchors`, and `classify` then gives `unknown`. `append anchored` refuses when the previous line is not an intent or no comment id is given. An unanchored intent closed by an anchored observation passes, and classifies `failed-before-witness` on a null floor. Each D6 classification from fixtures. Anchors before `open`: with the run file of a `committed` S1 run deleted (no `chattr +a`) and a pre-init observation with `binding: none`, `open` writes `begin-refused` `evidence-lost` and never reads the continuity rule, and the directory verdict is `unknown`; the same for an anchor naming a missing line, a file with no anchored line, and an anchor set that `open` was not given. Store continuity: after a `committed` run for S1, a pre-init observation with `binding: none` or another store id makes `open` write `begin-refused` `store-changed`, and the directory verdict is `restore-or-loss`; the same after runs for S1 that stopped after init, before an intent, or `failed-before-witness` gives `unknown`; `append init` with a store id other than S1 refuses; a directory with `init` records for two store ids fails `verify`; `open` after a `committed` run with an unchanged store writes `step4-complete`. Also: `unknown` for a floor that first appears after a `failed-before-witness` observation. `require-open-attempt` refusals, including an intent without its `anchored` line, and the printed fence. No network call, and no subprocess with a write path. |
| `tests/ci/test_synology_native_admission_trust_publication.py` (extend) | Guard: with a profile present it continues without a fence; with no profile and no open attempt it refuses; an identity mismatch refuses; a blocking classification refuses; with an open attempt it passes the fence to `publish-trust`; MariaDB is unaffected and gets the setting `off`. |
| `tests/ci/test_synology_native_admission_gate.py` (extend) | `check-kid-history`: count 1 passes, 0 or 2 fails, read-only |

**Not P6a.** `deploy/synology/scripts/cutover-postgres.sh` (`step4-begin`, `step4-observe`) and the workflow input that drives it belong to P6, which calls the commands and the tool above.

Compared with the plan's expected list (`app/GameAuth/NativeEvidence/`, the console commands, `tests/Feature/GameAuth/`), the list above adds `config/game-auth.php`, the deploy scripts, the two Compose files and `tests/ci/` (owner question O3).

## Residual risks

- **Manual SQL in the window (A1).** See *Proof*, *Limits*.
- **A whole-NAS volume restore** would roll back every share, including the evidence share. D3 can only keep the evidence out of database, volume and share-level restores. This is owner question O1.
- **A root writer on the NAS.** P6a does not defend against one (D3, *What P6a does not defend against*). The external anchor makes later changes to anchored lines detectable. It cannot stop a false line written before anchoring, including a false intent that the fence would then accept (A5), or changes to the witness or the database.
- **Unresolved sources of other containers.** `check-location` resolves the evidence mount and the runner's own mounts through `mountinfo`, but compares the other project containers' sources and `DockerRootDir` as written (D3). An alias from one of them into the evidence share needs root on the NAS (A5); the foreign-entry check detects writes through it.
- **No append-only attribute** (O2). Without it, a root user can delete a run file. The missing file then has anchors with no matching line. The next `open` checks the anchors before anything else and refuses with `evidence-lost` (D5), so no init or new store follows, and the directory verdict is `unknown`.
- **A crash during the first publication.** A PostgreSQL restart after `$advance(1)` and before the commit gives `unknown`, not `failed-after-witness` (D8). That fails safe, but the owner then gets a less specific verdict.
- **Producer calls may fail as busy.** The observation's shared lock can make a concurrent producer call fail closed as busy for the length of one read. Step 4 runs with the gate closed and the scheduler stopped, so no such call is expected.

## Questions

**For the producer-contract owner.**

- **P1. A stricter init.** The init refuses a marker with floors and no row, while the runtime re-binds that state (contract, store provenance). Recommendation: accept. The init runs only on a clean slate, and the runtime rule is unchanged.
- **P2. Verdicts as information.** Classifications are information only and authorize no write. Recommendation: accept.
- **P3. A commit witness.** Should a later producer change add a commit witness (for example the transaction id written in the profile transaction) to close the manual-SQL gap? Recommendation: not for P6. Treat it as a prerequisite for any future repair design.
- **P4. A runtime re-bind rule.** Should the runtime re-bind refuse when a trust floor exists without a profile? Recommendation: no. The writers already refuse, and changing provenance rules belongs to the contract.
- **P5. Busy failures.** Accept that the observation's shared lock can make a concurrent producer call fail closed as busy. Recommendation: accept.
- **P6. The first-publication fence.** Accept the D8 producer change:
  - the `first_publication_fence` setting;
  - `publishTrustedKey` refusing an unfenced first publication when the setting is `required`;
  - the fenced path, which holds the store lock shared, never re-binds, verifies the named intent and its `anchored` line from the read-only evidence mount, and checks identity, progress and binding against that intent under table locks before `$advance(1)`;
  - the producer reading the evidence directory (`attempt_evidence_directory`), which ties the Platform's first publication to the evidence format of D4.

  Recommendation: accept. It changes nothing when the setting is `off` or a profile exists.

**For the owner.**

- **O1. The evidence host path.** Is `/volume1/oteryn-attempt-evidence` the location? It must be a dedicated shared folder, outside every snapshot, replication, Hyper Backup and restore task, owned by root with group 82 and mode `2750`, mounted read-write into the runner and read-only into the `platform` service (D3). The path must be the real share, not a symlink or a bind mount; `check-location` refuses an alias. Options: that path; another dedicated share. Recommendation: that path.
- **O2. The append-only attribute.** Set `chattr +a` on the evidence directory on the host. It is hardening against deleting and renaming files, not the integrity anchor. If the share's file system refuses it, continue with `dir_append_only: false` recorded? Recommendation: set it, and continue without it only if the file system refuses.
- **O3. Wider paths.** Accept that P6a's paths go beyond the plan's expected list: `config/game-auth.php`, the deploy scripts, the two Compose files and `tests/ci/`. Recommendation: accept, because the evidence must be written outside the Platform container and the fence lives in the producer.
- **O4. The deploy guard.** Accept the D8 first-publication guard in `lib.sh`, including that bootstrap Compose mode with native admission on now refuses on PostgreSQL. Recommendation: accept.
- **O5. The anchor issue.** Which issue holds the mandatory anchors, and may the cutover workflow's token get `issues: write`? Options: a dedicated P6 cutover issue in this repository; the governing issue #1474. Recommendation: a dedicated P6 cutover issue. Lock it, so that only collaborators can comment, if the workflow token can still post there.

## Owner approval

Accepting this note approves D1 to D8 as the implementation contract for P6a, subject to the answers to P1 to P6 and O1 to O5. It approves no missing-profile repair. P6 step 4 cannot start until all of these hold:

- P6a is merged and deployed;
- the runner has the evidence mount, and the `platform` service its read-only mount;
- `check-location` passes on the NAS;
- the anchor issue exists, and the cutover workflow can post to it.
