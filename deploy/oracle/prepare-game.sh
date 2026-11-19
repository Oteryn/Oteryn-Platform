#!/usr/bin/env bash
# Prepare/migrate the isolated Game store. Never starts an authoritative node.
#
# OTERYN_GAME_IMAGE names the Game CI image already loaded on this host
# (oteryn-game:<40-hex sha>-arm64, from the oteryn-game-image-<sha> artifact).
# Later releases change it only through `actions-deploy.py game deploy-image`.
set -euo pipefail
umask 077
IMAGE=${OTERYN_GAME_IMAGE:-}
[[ $IMAGE =~ ^oteryn-game:([0-9a-f]{40})-arm64$ ]] \
    || { echo 'Set OTERYN_GAME_IMAGE=oteryn-game:<40-hex sha>-arm64.' >&2; exit 2; }
SHA=${BASH_REMATCH[1]}
[[ $EUID == 0 ]] || { echo 'Run as root.' >&2; exit 1; }
SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
ROOT=/opt/oteryn-oracle/game-db
RUNTIME=/opt/oteryn-oracle/game-runtime.compose.yml
read -r arch revision < <(docker image inspect --format \
    '{{.Architecture}} {{index .Config.Labels "org.opencontainers.image.revision"}}' "$IMAGE")
[[ $arch == arm64 && $revision == "$SHA" ]] \
    || { echo 'Game image provenance differs (architecture or revision label).' >&2; exit 1; }
if [[ ! -e $ROOT ]]; then
    python3 "$SCRIPT_DIR/prepare-game-db.py"
fi
[[ -d $ROOT && ! -L $ROOT && $(stat -c '%u %a' "$ROOT") == '0 700' ]] \
    || { echo 'Unsafe Game database directory.' >&2; exit 1; }
for file in config/postgresql.conf config/pg_hba.conf init/10-game-roles.sql migrate.env server/server.key client/ca.crt; do
    [[ -f $ROOT/$file && ! -L $ROOT/$file ]] || { echo "Incomplete Game database configuration: $file" >&2; exit 1; }
done
override=$(mktemp)
trap 'rm -f "$override"' EXIT
printf '{"services": {"migrate": {"image": "%s"}}}\n' "$IMAGE" > "$override"
compose=(docker compose -f "$SCRIPT_DIR/game-db.compose.yml" -f "$override")
"${compose[@]}" config --quiet
docker volume create oteryn-oracle-game_game_backup >/dev/null
docker run --rm --network none \
    --mount type=volume,source=oteryn-oracle-game_game_backup,target=/var/lib/oteryn-postgres-backup \
    --entrypoint bash postgres:18.6@sha256:e3d24f6906d0760bf4da0dae872415391ededefad23fd471c47dc53b0ddf3c42 \
    -ec 'install -d -o postgres -g postgres -m 0700 /var/lib/oteryn-postgres-backup/wal'
"${compose[@]}" up -d --wait postgres
game_sql() {
    "${compose[@]}" exec -T postgres gosu postgres psql -XqAt -v ON_ERROR_STOP=1 -d oteryn_game -c "$1"
}
# Game migrations hand Control-issued objects to oteryn_game_control
# (ALTER ... OWNER TO), which needs Control's CREATE on the schema. Control
# holds it only while migrate runs, whether migrate succeeds or fails.
game_sql 'GRANT CREATE ON SCHEMA public TO oteryn_game_control'
trap 'game_sql "REVOKE CREATE ON SCHEMA public FROM oteryn_game_control"; rm -f "$override"' EXIT
"${compose[@]}" --profile operations run --rm --no-deps migrate
game_sql 'REVOKE CREATE ON SCHEMA public FROM oteryn_game_control'
trap 'rm -f "$override"' EXIT
# First install only: the runtime compose takes the same image on both
# declarations; an installed one is never replaced here.
if [[ ! -e $RUNTIME ]]; then
    runtime=$(sed -E "s#^(    image:) oteryn-game:[^ ]+\$#\\1 $IMAGE#" "$SCRIPT_DIR/game-runtime.compose.yml")
    [[ $(grep -cxF "    image: $IMAGE" <<<"$runtime") == 2 ]] \
        || { echo 'Expected exactly Game and ops image declarations.' >&2; exit 1; }
    install -o root -g root -m 0600 /dev/null "$RUNTIME"
    printf '%s\n' "$runtime" > "$RUNTIME"
    echo "Installed $RUNTIME for $IMAGE."
fi
echo 'Game database migrated over verify-full TLS; native node startup remains a separate gated operation.'
