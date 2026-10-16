# Oracle ARM64 qualification

Governed by Platform issue #1639 and PR #1640. The owner authorized the complete
existing-state migration and direct oteryn.com DNS/HTTPS on Oracle, without Tunnel.

## Pinned inputs

- Platform/gateway: `1194d5548760d9d773a1cebb182bc5986a6a42bf`
- Current Game runtime: `77af046ce14c81df6ed6cab03ae693a68acff32c`
- Initial isolated Game qualification: `87f770c87ce17518ba69f5effdbd04725d8a7d06`
- Game runtime and PostgreSQL base images are digest-pinned in these files.

Use `game.Dockerfile` with the immutable Game source archive as context. Its
`runtime` target provides the three Game executables as UID/GID 1001; operations
run in separate one-off containers. The `world-bundle` target exports the pinned
World after verifying compiler input identities and rebuilding the bundle twice.
The archive-only Git index carries compiler inputs, with no synthetic history.

## Host preparation

These root-run scripts assume the pinned Platform archive was extracted under
`/opt/oteryn-build/` and the native images have been built. Fetch these deployment
files from one exact reviewed candidate commit; keep them together in an
owner-only directory. Never print the generated environment files or keys.

```bash
bash prepare-platform.sh
bash prepare-game.sh
python3 check-game-db.py
```

`prepare-platform.sh` uses the existing Synology Compose contract and operational
PostgreSQL bootstrap, with project `oteryn-oracle`, generated independent secrets,
TLS 1.3 / verify-full, split database roles, loopback HTTP and native admission off.
It reconciles the existing WAL archive / daily physical and logical backup jobs.
An existing database or witness is never reset on retry.

`prepare-game-db.py` creates configuration once and refuses to replace it. Its
CA private key remains outside all container mounts. The Game store has a separate
internal Docker subnet (`172.30.84.0/24`, checked for overlap on this host), no
published database port, SCRAM / TLS-only authentication, non-superuser login
roles and retained WAL archive. `prepare-game.sh` starts it and runs the shipped
migration executable over verify-full TLS as the schema owner. It starts no node.
`check-game-db.py` proves the runtime role cannot perform DDL and rejects
plaintext and an unknown CA, without printing credentials.

## Verified target results

- Platform, gateway and Game native ARM64 builds passed.
- Platform home/login and gateway readiness return HTTP 200.
- Platform PostgreSQL, scheduler and backup sidecar are healthy.
- Forced Platform WAL archive, physical base backup and logical dump passed.
- Game PostgreSQL TLS 1.3 / verify-full passed; 77 migrations applied.
- Migrated deployed World digest: `1b04cc35a65bcced64dbc933fcb5ac9adad426a369149bb8c170b1aa80d04406`.
- Gameplay manifest and NPC catalogue staged with their shipped digest checks.

## Native bootstrap boundary

The coherent frozen-source migration and both isolated logical restores passed.
Game launch, scope assignment, readiness, receipt-backed web Character creation
and a real controlled restart passed. Platform post-migration physical and logical
backups passed. Browser Account Center repair is merged as PR #1642 and the real owned roster passed; native client
entry remains a separate acceptance item. Never start a second authoritative writer
against retained NAS data.

The fresh-store first-publication route is Synology-specific. It is not used for
this existing-state migration: transfer source trust and its independently
retained custody coherently, then validate them through the shipped read-only
observation and signing self-check. Never manufacture a fresh trust profile,
replace canonical source identities or reset independent floors.

## Direct public edge

Apply `public-platform.compose.yml` with the pinned Platform Compose contract,
and `edge.compose.yml` with `Caddyfile`. Only Caddy publishes 80/443; database,
PHP and internal proxy listeners remain private. The trusted edge and internal
proxy have exact addresses 172.18.0.10 and 172.18.0.5. Canonical APP_URL is
https://oteryn.com with Secure cookies. Certificates persist in named volumes.
Apex DNS is proxied to 158.180.58.90; www, gateway, login and game use DNS-only
records for that address. Existing mail records are preserved. Native Game uses
TLS TCP 7172; the HTTPS explanatory response is not gameplay readiness.

## Existing NAS state migration

The source has existing signed trust, six identities and two Game Characters.
Migrate the databases and independently retained custody together after freezing
all source writers; never initialize new trust over the existing source history.
Keep the fresh Oracle qualification databases as separate private rollback copies.
Preserve source canonical AccountId, CharacterId, WorldId, floors, fences, signing
keys and append-only evidence. Change endpoints only through shipped Registry and
Game ops commands, advancing descriptor/assignment/projection revisions as required.
A migration is not complete until real account, Character and native client checks
pass. The earlier fresh-store bootstrap candidate is retained in Git history only;
it is not used to replace migrated NAS state.

## Game runtime and restart ownership

`game-runtime.compose.yml` runs the non-root native Game node and a separate
root-only one-off ops container. Only Game TCP 7172 is published. Internal mTLS
uses `internal-mtls.compose.yml` to remove the NAS host port and give the listener
private Docker address 172.18.0.12. Existing NAS file paths remain container paths;
the host mounts `/opt/oteryn-game` and the separate root ops directory there.

Install the shipped `oteryn-game.service` only after the final coherent source
transfer, new endpoint descriptor issuance, forward projection resync and the
root-owned migration baseline are qualified. `game-supervisor.py` issues a new
durable launch authorization superseding the retained node, records the new
registered incarnation, replaces/reports its assignment, and waits for readiness.
A retained ambiguous request is reconciled by its exact file. A registration gap
or unresolved outcome leaves a reconciliation marker and stops; no floor reset,
authorization replay or blind Docker restart is permitted. Docker restart is off
for the node; systemd owns the controller at boot. Other services retain their
normal Docker restart policies.

The certificate timer checks Caddy's renewed `game.oteryn.com` certificate daily,
installs its private pair as UID 1001 mode 0600 and restarts through that same
durable controller only when bytes changed. A refresh failure retains the serving
node and reports failure. Focused controller custody tests:
`python3 deploy/oracle/test_game_supervisor.py`. Real launch/restart acceptance is
separate from those unit tests and must be recorded against the target host.

Migration extraction must restore original numeric UID/GID and ordinary directory
mode bits from the verified source archive after safe extraction. Python's data
filter deliberately removes directory metadata; retaining its default 0700 on
root-owned World/runtime parents blocks the non-root service. Keep root-owned
state/fence parents at 0755, service runtime directories at 0700 and private files
at 0600. No ownership or mode bypass in Game is authorized.

The frozen final archive SHA256 is
`f52f41c5fea21b2d02e9ef1f4080d9f2fed06b037e66b605c9d4a00d98e544c3`.
Source writer containers and deployment runners are stopped with restart disabled.
Existing source database/backup containers remain available as retained evidence.
Oracle has accepted new writes: rollback requires a controlled reverse transfer,
not simply restarting the stale NAS writers. Fresh Oracle qualification databases
remain separately retained. `migrated-storage.compose.yml` selects the canonical
migrated Platform volume; include it on every subsequent Platform reconciliation.


## Oracle Actions operations

`oracle-operations.yml` replaces NAS deployment on Oracle. It is restricted to
protected main, the component runner group, exact Oracle runner name, ARM64 and
uid1001. Both runners use the pinned upstream 2.338.0 ARM64 distribution, retain
separate private registration volumes and restart automatically with Docker.
Neither runner has a Docker socket, host runtime mount, published port or host
secret. A local SSH key has a root-owned forced command restricted to its single
component, no shell/PTY/forwarding, with independently pinned SSH host identity.

Install `actions-deploy.py` root-owned under `/opt/oteryn-oracle`, not in a runner
writable mount. `status` only reads readiness. `deploy <40-character SHA>` requires
exact current protected main and its latest GitHub Actions aggregate success,
rechecks after build, builds native ARM64, backs up, preserves root-owned private
configuration and migrates through the existing owner connection. Platform keeps
all three Oracle overrides, including the canonical migrated storage volume.
Game keeps deployed World/content and routes every restart through the durable
controller. Failures remain closed for operator reconciliation; no automatic
schema rollback, destructive volume operation or trust reset is performed.
The runner never installs or upgrades the root wrapper/controller itself.

Before enabling these runners, disable the retired Synology runtime workflows,
Synology image/deploy-runner builders and Tunnel endpoint mutation workflows in
GitHub Actions, retaining their source and historical runs. They are incompatible
with Oracle's configuration and must not be re-enabled as an Oracle deployment
path. Preserve unrelated hosted CI and Atlas runners. NAS writers and runners
remain stopped with restart disabled. Re-registration tokens are one-use inputs,
never persistent runner environment variables.

Run `Oracle Operations` on main with action `status` for acceptance, then action
`deploy` and an exact qualified main SHA for a release. The forced command checks
that SHA independently of workflow inputs. Source/build provenance and failures
are retained privately in `/opt/oteryn-oracle/state/actions`. A successful workflow
is deployment/readiness evidence, not native gameplay acceptance.

`oteryn-game-backup.timer` performs daily physical and logical backups in the
separate Game backup spool. Install a local-only `local replication postgres peer`
HBA rule and root-create the `daily` spool directory owned by PostgreSQL mode0700.
This opens no network replication endpoint. Physical backups are extracted and
verified with `pg_verifybackup`, including WAL; logical archives must pass
`pg_restore --list`. Keep failed partials for diagnosis and retain completed
migration-era backups until an explicit retention decision. The timer is enabled
and persistent; the first verified backup completed 20261015T220119Z.


Oracle releases use root-private component candidate/current JSON journals in
`state/actions`, including exact source, image IDs, schema contract and durable
prepared/schema_unknown/schema_known/ready phases. The migrated Oracle host has
no legacy NAS release/schema-state files; no such history is invented. Before
migration the journal is fsynced as unknown; on success it is fsynced as known.
Only actual health, expected images and a new Game incarnation promote current
and a success receipt. Any retained candidate blocks another deployment until
explicit operator reconciliation; a successful receipt never authorizes retry
through an unresolved candidate. Legacy NAS recovery workflows remain disabled.
Main protection, aggregate success and the approved active workflow are rechecked
after backup and before migration. A still-running old Game prevents migration.
Platform readiness includes the scheduler as well as web/gateway/databases.


Operator prerequisites for Actions deployment: host Git, curl, Python3, Docker
and Compose must be installed. Runner tools do not install host packages.
The adapter gives only public Git child processes umask022, so copied source is
readable by the non-root PHP image USER; private operator writes retain077.
Before maintenance it verifies application entry files in an isolated candidate
container inheriting its real USER and WORKDIR. An unreadable image fails before
backup, writer stop or migration. Preserve failed deployment records and reconcile
against the predeployment backup/schema and actual runtime before retry; a PHP
failure to open artisan is not evidence that a schema migration committed.
