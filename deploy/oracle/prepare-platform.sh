#!/usr/bin/env bash
# Prepare application roles against an independently provisioned Rust database.
set -euo pipefail
umask 077
[[ $EUID == 0 ]] || { echo 'Run as root.' >&2; exit 1; }
SOURCE=${1:?Pass the absolute reviewed Platform checkout path}
ROOT=/opt/oteryn-oracle
[[ -f "$SOURCE/portal-rs/Dockerfile" && -f "$SOURCE/deploy/oracle/public-platform.compose.yml" ]]
# Operator supplies separate application and schema-owner PostgreSQL credentials.
# Never generate Laravel settings, copy secrets into an image, or reset storage.
for file in platform.env platform-migrate.env; do
    [[ ! -L "$ROOT/$file" && $(stat -c '%u %a' "$ROOT/$file") == '0 600' ]] \
        || { echo "Missing or unsafe $file." >&2; exit 1; }
done
[[ -d "$ROOT/platform-secrets" && -f "$ROOT/platform-secrets/gateway-token" \
    && -f "$ROOT/platform-secrets/gateway-token.sha256" ]]
docker network inspect oteryn-oracle_private >/dev/null
compose=(docker compose --env-file "$ROOT/platform.env" -f "$SOURCE/deploy/oracle/public-platform.compose.yml")
"${compose[@]}" config --quiet
"${compose[@]}" run --rm -T --no-deps migrate
"${compose[@]}" up -d --no-deps platform worker gateway
"${compose[@]}" ps
