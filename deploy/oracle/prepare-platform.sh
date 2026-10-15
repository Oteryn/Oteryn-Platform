#!/usr/bin/env bash
# Fresh, isolated Oracle Platform. Reuse the reviewed PostgreSQL bootstrap;
# never reset an existing database/witness or publish native admission here.
set -euo pipefail
umask 077
[[ $EUID == 0 ]] || { echo 'Run as root.' >&2; exit 1; }
SOURCE=/opt/oteryn-build/Oteryn-Platform-1194d5548760d9d773a1cebb182bc5986a6a42bf
ROOT=/opt/oteryn-oracle
export OTERYN_ENV_FILE="$ROOT/platform.env"
[[ -f "$SOURCE/deploy/synology/compose.yml" ]] || { echo 'Pinned Platform source is missing.' >&2; exit 1; }
install -d -m 0700 "$ROOT" "$ROOT/state" "$ROOT/attempt-evidence"
if [[ ! -e "$OTERYN_ENV_FILE" ]]; then
    python3 - "$OTERYN_ENV_FILE" <<'PY'
import base64, hashlib, os, secrets, sys
token = secrets.token_hex(32)
env = {
    'COMPOSE_PROJECT_NAME': 'oteryn-oracle',
    'PLATFORM_BIND_ADDRESS': '127.0.0.1', 'PLATFORM_PORT': '8000',
    'GATEWAY_BIND_ADDRESS': '127.0.0.1', 'GATEWAY_PORT': '8080',
    'PLATFORM_IMAGE': 'oteryn-platform:1194d5548760-arm64',
    'GATEWAY_IMAGE': 'oteryn-gateway:1194d5548760-arm64',
    'GATEWAY_VERSION': 'oracle-1194d5548760-arm64',
    'APP_NAME': 'Oteryn Oracle Qualification', 'APP_ENV': 'preproduction',
    'APP_KEY': 'base64:' + base64.b64encode(secrets.token_bytes(32)).decode(),
    'APP_DEBUG': 'false', 'APP_URL': 'http://127.0.0.1:8000',
    'LOG_LEVEL': 'info', 'SESSION_SECURE_COOKIE': 'false',
    'PLATFORM_DB_NAME': 'oteryn_platform', 'MARKETPLACE_ENABLED': 'false',
    'OTERYN_PLATFORM_SERVICE_TOKEN': token,
    'GAME_AUTH_GATEWAY_SERVICE_TOKEN_SHA256': hashlib.sha256(token.encode()).hexdigest(),
    'GAME_WORLD_ID': '1', 'GAME_WORLD_SLUG': 'oteryn-oracle-qualification',
    'GAME_WORLD_NAME': 'Oteryn Oracle Qualification', 'GAME_WORLD_REGION': 'FRA',
    'GAME_WORLD_HOST': '127.0.0.1', 'GAME_WORLD_PORT': '7172',
    'OTERYN_STATE_DIR': '/opt/oteryn-oracle/state',
    'OTERYN_ATTEMPT_EVIDENCE_HOST_PATH': '/opt/oteryn-oracle/attempt-evidence',
    'OTERYN_NATIVE_ADMISSION_ENABLED': 'false',
    'OTERYN_PLATFORM_MTLS_ENABLED': 'false',
    'GAME_AUTH_NATIVE_EVIDENCE_ACTIVATED': 'false',
    'GAME_AUTH_NATIVE_RUNTIME_STATUS_ENABLED': 'false',
    'GAME_AUTH_NATIVE_SCOPE_ASSIGNMENT_ENABLED': 'false',
    'OTERYN_NATIVE_ROUTE_PUBLISH_ENABLED': 'false',
    'CHARACTER_AUTHORITY_COMMANDS_ENABLED': 'false',
    'CHARACTER_AUTHORITY_WEB_CREATE_ENABLED': 'false',
    'OTERYN_PLATFORM_FIRST_PUBLICATION_FENCE': 'required',
}
for role in ('SUPERUSER', 'OWNER', 'APP', 'BACKUP', 'DUMP'):
    env[f'PLATFORM_POSTGRES_{role}_PASSWORD'] = secrets.token_hex(32)
fd = os.open(sys.argv[1], os.O_WRONLY | os.O_CREAT | os.O_EXCL, 0o600)
with os.fdopen(fd, 'w') as f:
    for k, v in env.items():
        f.write(f'{k}={v}\n')
    f.flush()
    os.fsync(f.fileno())
PY
fi
[[ ! -L "$OTERYN_ENV_FILE" && $(stat -c '%u %a' "$OTERYN_ENV_FILE") == '0 600' ]] \
    || { echo 'Unsafe environment file.' >&2; exit 1; }
DEPLOY_DIR="$SOURCE/deploy/synology"
source "$DEPLOY_DIR/scripts/lib.sh"
load_oteryn_env_file "$OTERYN_ENV_FILE"
[[ $COMPOSE_PROJECT_NAME == oteryn-oracle && $OTERYN_NATIVE_ADMISSION_ENABLED == false ]] \
    || { echo 'This preparation requires the isolated project and native admission off.' >&2; exit 1; }
compose=(docker compose --env-file "$OTERYN_ENV_FILE" -f "$DEPLOY_DIR/compose.yml")
# The existing Compose contract expects this external volume even while admission
# is off. Creating it does not generate or publish a signing key.
docker volume create oteryn-native-admission-keys >/dev/null
"${compose[@]}" create tls-init >/dev/null
tls_container="$("${compose[@]}" ps -a -q tls-init)"
[[ -n $tls_container ]]
docker cp "$DEPLOY_DIR/tls/init.sh" "$tls_container:/bootstrap/tls-init.sh"
docker cp "$DEPLOY_DIR/nginx/internal.conf" "$tls_container:/bootstrap/internal-nginx.conf"
docker cp "$DEPLOY_DIR/nginx/internal-mtls.conf" "$tls_container:/bootstrap/internal-mtls-nginx.conf"
docker cp "$DEPLOY_DIR/postgres/postgresql.conf" "$tls_container:/bootstrap/postgres-postgresql.conf"
docker cp "$DEPLOY_DIR/postgres/pg_hba.conf" "$tls_container:/bootstrap/postgres-pg_hba.conf"
docker cp "$DEPLOY_DIR/postgres/init/10-platform-roles.sh" "$tls_container:/bootstrap/postgres-roles.sh"
docker cp "$DEPLOY_DIR/postgres/backup/." "$tls_container:/bootstrap/postgres-backup"
"${compose[@]}" up --no-deps --exit-code-from tls-init tls-init
oteryn_reconcile_postgres "${compose[@]}"
if [[ ! -e "$OTERYN_STATE_DIR/postgres-bootstrap/bootstrap-complete.env" ]]; then
    bash "$DEPLOY_DIR/scripts/bootstrap-postgres.sh" record-witness
    bash "$DEPLOY_DIR/scripts/bootstrap-postgres.sh" run
fi
"${compose[@]}" up -d --no-deps platform internal-proxy gateway marketplace-scheduler
"${compose[@]}" ps
echo 'Platform prepared on loopback; native admission and Character creation remain gated.'
