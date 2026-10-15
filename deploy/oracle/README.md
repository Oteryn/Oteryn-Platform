# Oracle ARM64 qualification

Governed by Platform issue #1639 and draft PR #1640. No NAS database, private key or independent witness has been moved. The
owner subsequently authorized direct oteryn.com DNS and HTTPS on Oracle.

## Pinned inputs

- Platform/gateway: `1194d5548760d9d773a1cebb182bc5986a6a42bf`
- Game: `87f770c87ce17518ba69f5effdbd04725d8a7d06`
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
- Pinned World digest: `7429f13b9a6caf3bc6017d5c710076d76d6f1394a11b550f46dc2c620d88f806`.
- Gameplay manifest and NPC catalogue staged with their shipped digest checks.

## Native bootstrap boundary

This is not a full-stack readiness claim. Game launch/assignment, native login,
web Character creation, restore qualification and final data migration
remain unqualified. Never start a second authoritative writer against NAS data.

The current operational native trust bootstrap is bound to the protected
Synology GitHub Actions workflow and its cutover state. It requires a dedicated
restore-excluded evidence mount, provenance-only initialization, durable attempt
records anchored by `github-actions[bot]`, then first trust publication with the
fence enabled. See `deploy/synology/scripts/cutover-hooks/provenance-init.sh` and
`common.sh`, and `deploy/synology/scripts/attempt_evidence.py`.

An Oracle bootstrap executor must be integrated and reviewed before activating
native admission here. Do not substitute manual database/trust writes, forge
Synology cutover state or anchors, or disable the first-publication fence. The
same gates must precede the real login/create/list qualification and NAS cutover.

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
