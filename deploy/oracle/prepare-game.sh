#!/usr/bin/env bash
# Prepare/migrate the isolated Game store. Never starts an authoritative node.
set -euo pipefail
umask 077
[[ $EUID == 0 ]] || { echo 'Run as root.' >&2; exit 1; }
SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
ROOT=/opt/oteryn-oracle/game-db
if [[ ! -e $ROOT ]]; then
    python3 "$SCRIPT_DIR/prepare-game-db.py"
fi
[[ -d $ROOT && ! -L $ROOT && $(stat -c '%u %a' "$ROOT") == '0 700' ]] \
    || { echo 'Unsafe Game database directory.' >&2; exit 1; }
for file in config/postgresql.conf config/pg_hba.conf init/10-game-roles.sql migrate.env server/server.key client/ca.crt; do
    [[ -f $ROOT/$file && ! -L $ROOT/$file ]] || { echo "Incomplete Game database configuration: $file" >&2; exit 1; }
done
compose=(docker compose -f "$SCRIPT_DIR/game-db.compose.yml")
"${compose[@]}" config --quiet
docker volume create oteryn-oracle-game_game_backup >/dev/null
docker run --rm --network none \
    --mount type=volume,source=oteryn-oracle-game_game_backup,target=/var/lib/oteryn-postgres-backup \
    --entrypoint bash postgres:17.11@sha256:2d2b8998d31037bf721cfdf764d76ba74171b4fab3431b7f72c27c56ddbdf9e3 \
    -ec 'install -d -o postgres -g postgres -m 0700 /var/lib/oteryn-postgres-backup/wal'
"${compose[@]}" up -d --wait postgres
docker image inspect oteryn-game:87f770c87ce1-arm64 >/dev/null
"${compose[@]}" --profile operations run --rm --no-deps migrate
echo 'Game database migrated over verify-full TLS; native node startup remains a separate gated operation.'
