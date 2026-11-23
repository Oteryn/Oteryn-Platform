# Oracle ARM64 qualification

Governed by Platform issue #1639 and PR #1640. The owner authorized the complete
existing-state migration and direct oteryn.com DNS/HTTPS on Oracle, without Tunnel.

## Historical qualification inputs

- Historical Platform/gateway: `1194d5548760d9d773a1cebb182bc5986a6a42bf` (retired Laravel deployment)
- Current Game runtime: `77af046ce14c81df6ed6cab03ae693a68acff32c`
- Initial isolated Game qualification: `87f770c87ce17518ba69f5effdbd04725d8a7d06`
- Game runtime and PostgreSQL base images are digest-pinned in these files.

The Game image is built by Oteryn-Game CI, never on this host (owner decision
2026-11-19); `game.Dockerfile` here is the recipe being handed to the Game
repository and is no longer used by `actions-deploy.py`. Its
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
OTERYN_GAME_IMAGE=oteryn-game:<40-hex sha>-arm64 bash prepare-game.sh
python3 check-game-db.py
```

The Game image is never built here. For the first install, load the
`image.tar` of Game CI artifact `oteryn-game-image-<sha>` (`docker load`) and
name it in `OTERYN_GAME_IMAGE`; `prepare-game.sh` refuses a malformed name or an
image whose architecture or revision label differs, migrates with it and, only
when `/opt/oteryn-oracle/game-runtime.compose.yml` is absent, installs that file
with both image declarations set to it. Later releases go only through
`actions-deploy.py game deploy-image`, which needs this installed, ready Game.
The Game image carries its own inputs under `/opt/oteryn-game`: the World
bundle `world.otbundle`, the NPC data project `npc-data/` with its
`PROJECT_SHA256`, and the gameplay manifest named by the image's own
`OTERYN_NATIVE_GAMEPLAY_MANIFEST`. The runtime compose mounts nothing there, sets
no manifest path and reads `--npc-data-sha256` from the image at start, so
`deploy-image`, which swaps only the image lines, keeps inputs and pins in step.
The host `node.toml` names the bundle as
`[world_bundle] path = "/opt/oteryn-game/world.otbundle"`.

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
Apex DNS is a proxied CNAME to the Cloudflare Tunnel (below); www, gateway, login
and game use DNS-only records for 158.180.58.90. Existing mail records are preserved.

Native Game shares :443. The edge image (`edge.Dockerfile`: stock Caddy 2.11.7
plus caddy-l4 v0.1.3, both pinned) runs a layer4 listener wrapper ahead of TLS:
a connection whose ClientHello names SNI `game.oteryn.com` and offers ALPN
`oteryn-game/1` is relayed as raw TCP to the Game node at 172.18.0.11:7172 on
the private network. The edge does not terminate it and sends no PROXY header,
so the node sees the edge address, not the client's. Everything else reaches the
HTTP app unchanged. The Game listener is not published on the host. The route
the Platform publishes is host `game.oteryn.com`, port 443, TLS name
`game.oteryn.com`. The HTTPS response for that host is not gameplay readiness.

Build the edge image on the host before switching:
`docker build -f deploy/oracle/edge.Dockerfile -t oteryn-edge:2.11.7-l4-0.1.3 deploy/oracle`,
then `docker compose -f edge.compose.yml up -d`. Rollback: set the edge image back
to the stock `caddy:2-alpine@sha256:d8542f48d34a9cf4e4c11a478865229840e87e4c96ea3f439101f31a5d35f75f`,
remove the `listener_wrappers` block from `Caddyfile` and recreate the edge. The
stock image rejects that block, so change both together.

### Cloudflare Tunnel

`oteryn.com` reaches the edge only through the Cloudflare Tunnel `oteryn-portal`.
`tunnel.compose.yml` runs the pinned `cloudflared` at the fixed private address
172.18.0.20; it opens outbound connections to Cloudflare and forwards to
`https://172.18.0.10:443` with origin server name `oteryn.com`. The `Caddyfile`
trusts forwarding only from 172.18.0.20 and the apex site imports `tunnel_only`,
which aborts any other peer, so a direct request to 158.180.58.90 for oteryn.com is
dropped. www, gateway, login and game stay DNS-only and are unaffected.

Host files, never committed: `/opt/oteryn-oracle/tunnel/` (mode 700, owner 65532)
holds `config.yml` (tunnel id, `credentials-file`; ingress first sends path
`^/\.well-known/acme-challenge/` to `http://172.18.0.10:80`, then `oteryn.com` ->
`https://172.18.0.10:443` with `originRequest.originServerName: oteryn.com`, then
`http_status:404`) and the tunnel credentials JSON. The account `cert.pem` from
`cloudflared tunnel login` stays in a separate root-only directory and is mounted
only into one-off `cloudflared tunnel ... route dns` runs. Start with
`docker compose -f tunnel.compose.yml up -d` and wait for four registered
connections in its log before changing DNS. The apex record must be a proxied
CNAME to `<tunnel id>.cfargotunnel.com`; `route dns --overwrite-dns` replaces only
a CNAME, so an existing apex A record is changed to that CNAME in place.

The ACME path rule makes certificate issuance and renewal independent of the
current origin certificate: Let's Encrypt validates HTTP-01 through Cloudflare and
the tunnel to Caddy's port 80 (TLS-ALPN-01 cannot pass the proxy). Verified on
2026-11-21 by issuing a staging certificate for a temporary tunnel-only host; the
TLS hop to `:443` alone answered 502 to the validation while that host had no
certificate.

Rollback: change the apex back to a proxied A record for 158.180.58.90, then in the
`Caddyfile` put the Cloudflare ranges back in `trusted_proxies` and in the
`tunnel_only` matcher (or drop the import), and reload the edge.

### Machine API host

`api.oteryn.com` is a DNS-only A record for 158.180.58.90 with its own ACME
certificate. It serves native clients without Cloudflare in the path, so zone bot
protection (Bot Fight Mode, challenges) never answers a non-browser client. The
site passes only `/v1/game-auth/tickets` (rewritten as on the apex),
`/oauth/token`, `/api/v1/*` and `/updates/*` to the portal and answers 404 for
everything else; browser steps such as `/oauth/authorize` stay on `oteryn.com`.
Protection there is the portal's own authentication and rate limits. The same paths
keep working on `oteryn.com` until every native client uses the api host.

Recreating `portal` also requires recreating `portal-gateway`, which shares its
network namespace; otherwise the gateway keeps a stale namespace and answers 502.

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
root-only one-off ops container. No Game port is published on the host; gameplay
reaches 172.18.0.11:7172 only through the edge's :443 relay. Internal mTLS
uses `internal-mtls.compose.yml` to remove the NAS host port and give the listener
private Docker address 172.18.0.12. The host mounts `/opt/oteryn-game` at
`/srv/oteryn/game` and the separate root ops directory `/opt/oteryn-game-ops` at
`/srv/oteryn/game-ops` (the retired NAS `/volume1/...` container paths are gone);
`node.toml` and `ops.toml` must name these container paths; Game content comes
from the image (`/opt/oteryn-game`), never from the host.

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
writable mount. `status` only reads readiness. A Platform release requires
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
image of an installed one.

The Platform workflow sends
`deploy-signup-legal-v1 <40-character SHA> <run ID> <run attempt>`. Install
the reviewed root-owned helper before enabling releases through this workflow.
The updated helper also accepts the existing `deploy` command with the same
pre-backup check and bilingual signup probes. The candidate Oracle image
checks the published editions before applying migrations when an older helper
handles a legacy request.

### Game image artifact (deploy-image)

The `game` forced command accepts only `status` and
`deploy-image <sha> <run> <attempt> <artifact_id>` with the artifact zip on
stdin; a Game `deploy` that would build on the host is refused. Interface with
Oteryn-Game CI: artifact `oteryn-game-image-<sha>` holding exactly `image.tar`
(`docker save` of the single tag `oteryn-game:<sha>-arm64`), uploaded by the same
qualified `oracle-operations.yml` run that streams the raw artifact zip. The host
writes at most 1 GiB of stream (3 GiB unpacked) to a 0600 file under
`state/actions`, compares its sha256 with the public artifact `digest`, and
requires the artifact id, name, `expired == false`, `workflow_run.id` and
`workflow_run.head_sha` to match. The archive's `manifest.json` must name only
that tag before `docker load`; the loaded image must be arm64 with
`org.opencontainers.image.revision == <sha>`. The transfer files are removed in
every outcome. Then the existing Game steps run unchanged: re-qualification of the
exact main head (a Game merge during the window aborts the deploy), backup,
journal, stop, migrate, image swap and supervisor start.

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
  `/app/static`, `/app/migrations`. It must be arm64 and pass
  `check-image` with networking disabled.
- Preflight: the running release must already pass the container and
  `/readyz` checks below, so a host the probe cannot reach is refused before
  any effect instead of rolling back a good release after migration.
- Signup staging for the first release that enforces explicit approval.
  Build or load the reviewed candidate image before staging. As host root,
  use that image with the live portal service configuration. Select an active, email-verified staff account with
  `admin.access` and `cms.pages.manage`. The CLI records that account as the
  audit actor. Browser MFA cannot apply to a host-root CLI. Prepare the four
  approved UTF-8 Markdown files with real operator details, and choose new
  versions. Pause predecessor signup during publication if it still submits
  the old versions. Substitute the actual image tag, account UUID, date,
  versions and file paths in this sequence. Each Markdown file is redirected into
  the candidate container, so it can stay readable only by host root.

  ```sh
  (
    set -eu
    export CANDIDATE_IMAGE='oteryn-portal:<sha8>-arm64'
    actor='<staff-account-uuid>'
    effective='<YYYY-MM-DD>'
    terms_version='<new-terms-version>'
    privacy_version='<new-privacy-version>'
    candidate_compose=$(mktemp)
    trap 'rm -f -- "$candidate_compose"' EXIT
    cat > "$candidate_compose" <<'YAML'
  services:
    portal:
      image: ${CANDIDATE_IMAGE:?set the reviewed candidate image tag}
  YAML

    publish_edition() {
      docker compose -p oteryn-portal -f /opt/oteryn-portal/compose.yml \
        -f "$candidate_compose" run --rm -T --no-deps -e OTERYN_ENV=production \
        portal publish-legal-edition --actor "$actor" --slug "$1" --locale "$2" \
        --title "$3" --body-file /dev/stdin --version "$4" \
        --effective "$effective" --final-approved < "$5"
    }
    publish_edition terms-of-service en 'Terms of Service' "$terms_version" /root/legal/terms.en.md
    publish_edition terms-of-service pl 'Regulamin Oteryn' "$terms_version" /root/legal/terms.pl.md
    publish_edition privacy-policy en 'Privacy Policy' "$privacy_version" /root/legal/privacy.en.md
    publish_edition privacy-policy pl 'Polityka prywatności' "$privacy_version" /root/legal/privacy.pl.md
  )
  ```

  After all four commands succeed, set these entries in the live portal's
  Compose environment to the exact versions used above.

  ```dotenv
  OTERYN_TERMS_VERSION=<new-terms-version>
  OTERYN_PRIVACY_VERSION=<new-privacy-version>
  ```

  Then dispatch Oracle Operations with the candidate's full protected-main
  source SHA.

  ```sh
  candidate_sha='<full-protected-main-source-sha>'
  gh workflow run oracle-operations.yml --repo Oteryn/Oteryn-Platform --ref main \
    -f action=deploy -f source_sha="$candidate_sha"
  ```

  The host helper runs `portal check-signup-legal` against that configuration
  before backup, migration or service replacement, and again after migration
  before service replacement.
- Release: recheck authority, `pg_dump -Fc` inside `oteryn-portal-portal-pg-1`
  to a staged file checked with `pg_restore --list` and published as
  `state/actions/<sha>-platform.dump` (0600), recheck, journal `prepared` with the
  previous image, save `<sha>-portal.compose.before` and
  `<sha>-portal.gateway.before`, rewrite both files in place (owner and mode
  kept), `docker compose -p oteryn-portal -f /opt/oteryn-portal/compose.yml run
  --rm -T portal migrate`, then `up -d portal portal-worker portal-gateway`.
- Readiness: all four containers running, not restarting and not unhealthy;
  `portal`, `portal-worker` and `portal-gateway` on one image whose revision label
  is the recorded release; `GET /readyz` on port 8080 at the portal's
  `oteryn-oracle_private` address, sent from the host (the image has no HTTP
  client and the port is not published; `/readyz` includes the database check).
  Both localized signup routes, `/en/register` and `/pl/register`, must also
  respond directly with HTTP 200.
  This assumes the portal listens on `0.0.0.0:8080` inside its container, as
  the edge already requires (`OTERYN_BIND` defaults to loopback).
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
presence and runs the Rust migration role. Before stopping services, it checks
the candidate image against the application database and signup configuration.
An existing schema requires approved editions in both locales as described
above. An empty schema can proceed with first-time provisioning. Existing game/money flags remain off
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
value. Its strict trust policy accepts forwarding only from cloudflared at
`172.18.0.20` and rejects spoofed forwarding from other peers, while Rust trusts
only the Caddy container at `172.18.0.10`; the tunnel hop cannot accidentally
become the registration budget key.

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

## Staging

Oracle staging runs from a root shell on the host. It accepts the current
full SHA of a `codex/*` or `staging/*` branch in
`Oteryn/Oteryn-Platform`. Install all three files from the same reviewed
revision once, then use the installed copies for every branch build.

```sh
install -d -m 0700 /opt/oteryn-staging
install -m 0755 deploy/oracle/staging-deploy.py /opt/oteryn-staging/
install -m 0644 deploy/oracle/staging.Dockerfile deploy/oracle/staging.compose.yml /opt/oteryn-staging/
```

The host needs Python 3.11+, Git, OpenSSL, Docker Engine 28+ with BuildKit and
Compose v2, at least four CPUs and 10 GiB RAM. The largest running staging
workload, including the stack and browser qualification, is capped at 2.4 CPUs
and under 6 GiB RAM. Cargo builds use two jobs and staging-specific cache IDs.
The CLI refuses a held production deployment lock or a pending production
deployment when it starts, before building, and before replacing staging.
Each check releases its shared read lock immediately.

```sh
python3 /opt/oteryn-staging/staging-deploy.py deploy <full-sha>
python3 /opt/oteryn-staging/staging-deploy.py deploy <full-sha> ''
python3 /opt/oteryn-staging/staging-deploy.py deploy <full-sha> specs/legal-pages.spec.mjs
python3 /opt/oteryn-staging/staging-deploy.py deploy <full-sha> specs/legal-pages.spec.mjs --pg-tests oteryn-db,oteryn-web
python3 /opt/oteryn-staging/staging-deploy.py status
python3 /opt/oteryn-staging/staging-deploy.py logs
```

A successful launch returns as soon as the deploy is queued. `status` shows the
serving SHA and live readiness, plus the latest deploy phase and summary.
`logs` prints that deploy's phase history and summary. Both commands return 0
for a passed run, 1 for a failed run and 2 while work is pending or before the
first deploy. The detailed state is under `/opt/oteryn-staging/run/<sha>/`.
Keep the generated environment files and captured mail private.

Each replacement recreates PostgreSQL, migrates it, installs the testing legal
pages, seeds test accounts through the portal CLI and loads the sample data
from the deployed SHA. This uses the updated host deploy script, Compose file
and staging Dockerfile together. Candidate branches need the staging seed CLI,
the sample SQL and the atlas files. The staging portal is reachable on `127.0.0.1:18080` on the host, including `/readyz`. Port 18080
is served by `portal-public`, a second `serve` on the
same database whose origin comes from `/opt/oteryn-staging/public-origin`
(one `https://<hostname>` line; without the file it is the loopback origin).
Put the tunnel hostname there when staging is published behind Cloudflare
Access, so CSRF, passkeys and mail links match the address people use. The
staff panel runs only on its own
host, named in `/opt/oteryn-staging/admin-origin` (one `https://<hostname>`
line, different from `public-origin`, routed by the same tunnel to port 18080).
Cookies are host-only, so staff sign in there separately from the player
portal; without the file the staff panel stays off. Passkeys are bound to the
public host, so staff use TOTP on the staff host. Mail goes to the stack's authenticated
STARTTLS catcher. No Game server or payment provider is connected.

### Staging test accounts

Use `<name>@staging.invalid` on the player portal or, for staff, on the
configured staff host (`staging-admin.oteryn.com`). The deploy creates
`/opt/oteryn-staging/test-accounts.json` once as `root:root` with mode `0600`.
It contains the passwords and, where applicable, the Base32 TOTP secrets
(SHA-1, six digits, 30 seconds). The next deploy reuses the file and restores
the original login setup in the fresh database.

| Account | What it tests |
| --- | --- |
| `player-basic` | Verified password login, two characters and a party post |
| `player-mfa` | Verified player with TOTP and a boss party post |
| `player-premium` | Active 30-day premium grant, without a purchase |
| `player-guild-leader` | Two characters, including Galen Ashbrook, leader of Lantern Watch |
| `player-new` | Account with unverified email |
| `player-restricted` | Active restriction, including blocked party creation |
| `founder`, `staff-platform-admin` | Platform administrator with TOTP |
| `staff-security-admin` | Account roles and audit access with TOTP |
| `staff-content-editor` | Content editing and publishing with TOTP |
| `staff-moderator` | Moderation with TOTP |
| `staff-support` | Tickets and reports with TOTP |
| `staff-operations` | Portal settings and game catalogue administration with TOTP |
| `staff-game-admin` | Game administration role with TOTP |
| `staff-game-analyst` | Game analytics role with TOTP |
| `staff-game-dba` | Game database operations role with TOTP |
| `staff-game-master-senior` | Senior game master role with TOTP |
| `staff-game-operator` | Game operations role with TOTP |
| `staff-game-release` | Game release role with TOTP |
| `staff-game-security` | Game security role with TOTP |
| `staff-game-support` | Game support role with TOTP |
| `staff-game-viewer` | Read-only game staff role with TOTP |

Sample data covers Ashen Coast, Silverfen and Cinderpeak, with 50 named
characters, highscores, growth and daily statistics, guilds, deaths, houses,
party listings, wiki entries, daily boosts, an atlas tile and league standings.
Collection shows two sample cards to signed-in, verified players. To try the
compare form, paste the character IDs
`019a0000-0000-7002-8000-000000000001,019a0000-0000-7002-8000-000000000002`.
The sample feed refreshes short-lived read-model watermarks while staging is up.
World registry status remains unknown without a Game server.

Default deploys build the serving image and finish once staging is ready. An
explicit qualification releases the deployment lock after readiness and uses
its own lock and Compose project so another branch can reach staging while
tests run. The qualification summary stays in the run directory for its SHA.

Browser qualification runs the deployed revision's portal binary in a disposable
CI fixture stack via `portal-rs/e2e/local.mjs` and `run.mjs`. The browser and its
temporary PostgreSQL cluster share an isolated network namespace and never
connect to the tunnel-facing deployment. The synthetic Game producers and
payment fixture run there with throwaway keys. The launcher owns the flags,
legal pages, test CA, mail sink and seed data. The browser database also gets
the staging password-range fixture for password changes and recovery on the
offline network. A read of the sandbox admin page once a minute keeps the
shared staff fixture active during the full project. The character creation
harness runs generated scripts from the browser's private executable tmpfs.
A default deploy builds
and starts the runtime without a test job. On the host CLI an explicit empty spec
argument requests the full `pl-1440` project, and a spec path requests that file.
The separate Portal RS E2E workflow covers all four projects. The staging
report retains counts, test names and failure types.

### Staging from GitHub Actions

`oracle-staging.yml` deploys every push to a `staging/**` branch (and, by
dispatch, any eligible `codex/*` or `staging/*` head) and waits for the
deployment result. A dispatch that supplies a spec also waits for that spec. It runs only on the third runner service in
`runners.compose.yml`, `oteryn-oracle-staging`, registered in its own org
runner group `staging-runners` with labels `oteryn-staging` and
`oteryn-oracle-staging`. That runner volume holds one key,
`/home/runner/.ssh/oracle-staging`, and no production key; branch code never
runs on the production runners. Root's `authorized_keys` binds the key with
`restrict,command="/usr/bin/python3 /opt/oteryn-staging/staging-ssh.py"`.
The adapter accepts only `status`, `logs` and
`deploy <sha> [specs/<file>.spec.mjs] [--pg-tests a,b]`, passes an argument
list to the installed staging CLI, and refuses a deploy while another SHA is
still pending. Install it next to the CLI:

```sh
install -m 0755 deploy/oracle/staging-ssh.py /opt/oteryn-staging/
```

The workflow waits for any pending deploy, queues its own SHA, polls `status`
until that SHA leaves the pending state, prints `logs` and fails with the run.

Optional PostgreSQL tests run one crate at a time in the staging builder image,
each with a fresh database and a role that cannot connect to the portal database.
The stack stays available after testing. The next deployment replaces it and
removes staging images and checkouts beyond the last three distinct attempted
SHAs, while preserving the serving checkout if a replacement never starts.
