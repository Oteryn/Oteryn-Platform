# Oracle ARM64 qualification

Governed by Platform issue #1639 and PR #1640. The owner authorized the complete
existing-state migration and direct oteryn.com DNS/HTTPS on Oracle, without Tunnel.

## Historical qualification inputs

- Historical Platform/gateway: `1194d5548760d9d773a1cebb182bc5986a6a42bf` (retired Laravel deployment)
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
bash prepare-platform.sh /opt/oteryn-build/<reviewed-platform-checkout>
bash prepare-game.sh
python3 check-game-db.py
```

`prepare-platform.sh` validates operator-supplied Rust application and migration
configuration and starts the Rust roles described below. PostgreSQL provisioning,
TLS, split roles, WAL and backup retention remain independently managed. It never
resets an existing database or witness.

`prepare-game-db.py` creates configuration once and refuses to replace it. Its
CA private key remains outside all container mounts. The Game store has a separate
internal Docker subnet (`172.30.84.0/24`, checked for overlap on this host), no
published database port, SCRAM / TLS-only authentication, non-superuser login
roles and retained WAL archive. `prepare-game.sh` starts it and runs the shipped
migration executable over verify-full TLS as the schema owner. It starts no node.
`check-game-db.py` proves the runtime role cannot perform DDL and rejects
plaintext and an unknown CA, without printing credentials.

### Game PostgreSQL TLS maintenance

Fresh configuration issues a 10-year CA. Install the reviewed
`prepare-game-db.py` under `/opt/oteryn-oracle` and the separate
`oteryn-game-db-certificate.service` and `oteryn-game-db-certificate.timer`
under `/etc/systemd/system`. With the database provisioned, run as root:

```bash
systemctl daemon-reload
systemctl start oteryn-game-db-certificate.service
systemctl enable --now oteryn-game-db-certificate.timer
```

The daily timer renews a 30-day leaf when it is within seven days of expiry.
It signs for `game-postgres` with the retained CA and server key, verifies the
chain and name, replaces the mounted certificate atomically, then sends
PostgreSQL SIGHUP. A failed signal is retried on the next run. This timer is
separate from the public Game TLS timer.

For an existing 30-day CA, first run
`python3 /opt/oteryn-oracle/prepare-game-db.py extend-ca` as root. It extends the
same CA identity without changing its key, and updates the CA bundle used by
migrations and database checks. Before the original expiry, distribute the
updated `/opt/oteryn-oracle/game-db/client/ca.crt` to the database
`root_ca_file` paths used by the retained Game runtime and ops settings. Reload
client trust through the normal Game supervisor lifecycle and verify new
connections with hostname verification. Then start and enable the timer as
above. Keep runtime verify-full TLS enabled during the transition.

## Historical verified target results

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

Step-pacing receipts (Game `OTERYN_STEP_WAIT_RECEIPTS`) stay off by default.
For each new incarnation the controller reads the owner marker
`/opt/oteryn-game/run/step-wait-receipts` (Game `BASE/run/step-wait-receipts`)
and exports `OTERYN_STEP_WAIT_RECEIPTS=1` to the `game` service only when that
path is a regular, non-symlink file whose entire content is exactly `1` or `1`
plus one newline; otherwise it exports `0`. The value is logged as
`oracle_game_step_wait_receipts=<0|1>` and visible as the container label
`io.oteryn.step-wait-receipts`. Game reads the switch once per process, so enable
or roll back by writing or removing the marker and then
`systemctl restart oteryn-game.service`, which issues a fresh durable incarnation.

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
Platform readiness includes the Rust worker as well as web/gateway/databases.


Operator prerequisites for Actions deployment: host Git, curl, Python3, Docker
and Compose must be installed. Runner tools do not install host packages.
The adapter gives only public Git child processes umask022, so copied source is
readable by the non-root Rust image USER; private operator writes retain077.
Before stopping writers it runs the Rust `check-image` command in an isolated
candidate container inheriting its real USER and WORKDIR. An unreadable image fails before
backup, writer stop or migration. Preserve failed deployment records and reconcile
against the predeployment backup/schema and actual runtime before retry; a failed
image preflight is not evidence that a schema migration committed.

### Runner deploy of the live portal (/opt/oteryn-portal)

The `platform` branch of `actions-deploy.py` deploys the live portal layout on
oteryn-vps. It does not create or migrate that layout; it only replaces the app
image of an installed one. The Game branch and the forced-command grammar are
unchanged.

- Layout it expects: `/opt/oteryn-portal/compose.yml` (project `oteryn-portal`,
  services `portal-pg`, `portal`, `portal-worker`, `portal-gateway`, external
  network `oteryn-oracle_private`) with exactly two `image: oteryn-portal:<tag>`
  lines (the `x-portal` anchor and `portal-gateway`), and `gateway.env` with at
  most one `GATEWAY_VERSION=` line. Anything else is refused before any effect.
  The manual checkout under `src/` is never used.
- Build: the qualified source archive under `/opt/oteryn-build`,
  `docker build -f deploy/oracle/portal.Dockerfile portal-rs` (timeout 2 h),
  tagged `oteryn-portal:<sha8>-arm64` and labelled with the full revision. The
  recipe keeps the live runtime: Ubuntu 26.04, uid 10001, `/app/oteryn-portal`,
  `/app/static`, `/app/migrations`. It must be arm64 and pass `check-image`.
- Release: recheck authority, `pg_dump -Fc` inside `oteryn-portal-portal-pg-1`
  to a staged file checked with `pg_restore --list` and published as
  `state/actions/<sha>-platform.dump` (0600), recheck, journal `prepared` with the
  previous image, save `<sha>-portal.compose.yml.before` and
  `<sha>-portal.gateway.env.before`, rewrite both files in place (owner and mode
  kept), `docker compose -p oteryn-portal -f /opt/oteryn-portal/compose.yml run
  --rm -T portal migrate`, then `up -d portal portal-worker portal-gateway`.
- Readiness: all four containers running, not restarting and not unhealthy;
  `portal`, `portal-worker` and `portal-gateway` on one image whose revision label
  is the recorded release; `GET /readyz` on port 8080 at the portal's
  `oteryn-oracle_private` address, sent from the host (the image has no HTTP
  client and the port is not published; `/readyz` includes the database check).
  A release is ready after two consecutive passing samples with unchanged
  restart counts, within 180 s.
- Failure after the rewrite (migration, start or readiness): both files are
  restored, the previous image is started again with `up -d`, and the pending
  journal stays, so `status` and the next `deploy` report reconciliation. The
  database is never dropped or restored automatically.
- `status` fails until the first runner deploy records a release, since there
  is no recorded revision to compare with before that.

### Rust application deployment

The retained `/opt/oteryn-oracle` composition below (`public-platform.compose.yml`)
is no longer what `actions-deploy.py` deploys; see the section above. It builds
`portal-rs/Dockerfile` with `portal-rs` as context. One immutable image runs `serve`, `worker`, `migrate`
and `gateway` via `public-platform.compose.yml`. Caddy forwards directly to the
portal on port 8080, preserving the native ticket path alias to the Rust API. Rust internal mTLS is configured
on the portal using its existing `OTERYN_INTERNAL_*` settings and certificate files;
the retired nginx listener is not part of application deployment. The gateway
shares the portal network namespace to reach the Rust issuer on loopback port
8091; Caddy reaches gateway HTTP at `platform:8081`, separately from portal HTTP.
The admission and protocol flags retain their existing default-off behavior. The retained
nginx repair helper supports an opaque TCP relay for separately retained ingress
installations with nginx's stream module. It never terminates TLS or manufactures
certificate identity; the Rust listener uses `OTERYN_INTERNAL_TLS_BIND` on port
8443 in that topology. New application deployment uses direct Rust mTLS instead.

Provision the external `oteryn-oracle_private` network and PostgreSQL separately.
Supply root-owned mode-0600 `/opt/oteryn-oracle/platform.env` with the Rust
`OTERYN_DATABASE_URL` for the application role, and `platform-migrate.env` with the
schema-owner URL. Mount certificate/key files, `gateway-token` and its hexadecimal SHA-256 digest
`gateway-token.sha256` in the
read-only `platform-secrets` directory, readable by image UID 65532. These files
remain host-private; `prepare-platform.sh <reviewed-checkout>` validates their
presence and runs the Rust migration role. Existing game/money flags remain off
unless the operator explicitly enables them.

Both Platform deployment paths provision
`/opt/oteryn-oracle/native-evidence-witness` with owner UID/GID 65532 and mode 0700.
Only the portal service mounts it read-write. Its
`GAME_AUTH_NATIVE_EVIDENCE_HIGH_WATER_DIRECTORY` points to `/var/lib/oteryn/witness`.
Retain the directory across container recreation. If the database already records
witness provenance, move the matching marker and floors from the retained store
before recreating the service. `GAME_AUTH_NATIVE_EVIDENCE_ENABLED` defaults to
off. On the public production deployment, Platform reserves Protocol 3 and the
sandbox coin checkout for testing/preproduction.

The release receipt records the latest embedded Rust migration version and
accepts only predecessors present in the candidate migration inventory. A legacy
Laravel schema receipt requires explicit operator reconciliation: the removed
application is not a migration source. Before stopping Rust writers, deployment
streams a PostgreSQL custom-format dump to a private per-attempt file, fsyncs it
and validates its archive table of contents before publishing the SHA-named dump.
Failed attempts leave the final path untouched. The backup uses the retained
PostgreSQL image as a one-off client with the migration role's exact database URL,
TLS configuration and read-only certificates. The client shell expands the URL
inside the container, keeping it out of host-side Docker arguments and logs.
Failure prevents disruption. No automatic schema rollback is attempted.

Caddy forwards its already-resolved `{client_ip}` as a single `X-Forwarded-For`
value. Its strict Cloudflare trust policy rejects spoofed forwarding from other
peers, while Rust trusts only the Caddy container at `172.18.0.10`; a Cloudflare
transport hop cannot accidentally become the registration budget key.

Registration requires mail delivery in both `serve` and `worker`. Supply
`OTERYN_SMTP_HOST`, `OTERYN_SMTP_USERNAME`, `OTERYN_SMTP_PASSWORD`, and
`OTERYN_MAIL_FROM` in the private `platform.env`. Both roles receive
`OTERYN_PUBLIC_ORIGIN=https://oteryn.com` from the compose configuration; this
also supplies the origin used by password reset, email change, referrals and
Bazaar workers when their existing flags are enabled. `OTERYN_SMTP_TLS_MODE`
(default `starttls`) and `OTERYN_SMTP_PORT` may select the authenticated TLS
transport. The worker refuses startup when registration delivery is incomplete
(some mail settings present, others missing or invalid); with no mail settings at
all it starts and keeps verification jobs queued as a disabled consumer. In both
cases serve refuses registration POSTs with 503 until restarted with valid delivery
configuration. Existing queued verification jobs remain untouched during that
configuration failure. Configure enabled reset/email-change/referral/Bazaar
consumers' additional settings before starting the worker.

The gateway also trusts only Caddy at `172.18.0.10`, and receives the same single
resolved client address header. Native login/recovery and browser account/source
budgets therefore keep independent client identities behind the edge.
