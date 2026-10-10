#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"
GAME_ROOT="${GAME_ROOT:?GAME_ROOT must point to the pinned Oteryn-Game checkout}"
PLATFORM_HEAD_SHA="${PLATFORM_HEAD_SHA:?PLATFORM_HEAD_SHA is required}"
OTERYN_GAME_SHA="${OTERYN_GAME_SHA:?OTERYN_GAME_SHA is required}"

test "$(git -C "$GAME_ROOT" rev-parse HEAD)" = "$OTERYN_GAME_SHA"
[[ "$PLATFORM_HEAD_SHA" =~ ^[0-9a-f]{40}$ ]]

PREFIX="premium-e2e-${GITHUB_RUN_ID:-local}-$$"
NETWORK="${PREFIX}-net"
DB="${PREFIX}-db"
PLATFORM="${PREFIX}-platform"
PROXY="${PREFIX}-proxy"
IMAGE="${PREFIX}-platform-fpm"
PORT="18443"
TMP="$(mktemp -d)"
PKI="$TMP/pki"
mkdir -p "$PKI"

cleanup() {
  docker rm -f "$PROXY" "$PLATFORM" "$DB" >/dev/null 2>&1 || true
  docker network rm "$NETWORK" >/dev/null 2>&1 || true
  docker image rm -f "$IMAGE" >/dev/null 2>&1 || true
  rm -rf "$TMP"
}
trap cleanup EXIT

openssl req -x509 -newkey rsa:2048 -nodes -days 1 -sha256 \
  -subj '/CN=Oteryn Premium E2E CA' -keyout "$PKI/ca.key" -out "$PKI/ca.crt" >/dev/null 2>&1
openssl req -newkey rsa:2048 -nodes -sha256 -subj '/CN=localhost' \
  -keyout "$PKI/server.key" -out "$PKI/server.csr" >/dev/null 2>&1
printf '%s\n' 'subjectAltName=DNS:localhost,IP:127.0.0.1' 'extendedKeyUsage=serverAuth' > "$PKI/server.ext"
openssl x509 -req -days 1 -sha256 -in "$PKI/server.csr" -CA "$PKI/ca.crt" -CAkey "$PKI/ca.key" \
  -CAcreateserial -extfile "$PKI/server.ext" -out "$PKI/server.crt" >/dev/null 2>&1
openssl req -newkey rsa:2048 -nodes -sha256 -subj '/CN=oteryn-premium-e2e' \
  -keyout "$PKI/client.key" -out "$PKI/client.csr" >/dev/null 2>&1
printf '%s\n' 'extendedKeyUsage=clientAuth' > "$PKI/client.ext"
openssl x509 -req -days 1 -sha256 -in "$PKI/client.csr" -CA "$PKI/ca.crt" -CAkey "$PKI/ca.key" \
  -CAcreateserial -extfile "$PKI/client.ext" -out "$PKI/client.crt" >/dev/null 2>&1
cat "$PKI/client.crt" "$PKI/client.key" > "$PKI/client-identity.pem"

docker network create "$NETWORK" >/dev/null
docker run -d --name "$DB" --network "$NETWORK" --network-alias mariadb \
  -e MARIADB_ROOT_PASSWORD=oteryn-premium-root \
  -e MARIADB_DATABASE=oteryn_premium_e2e \
  mariadb:11.8 >/dev/null
for _ in $(seq 1 60); do
  if docker exec "$DB" mariadb-admin ping -uroot -poteryn-premium-root --silent >/dev/null 2>&1; then break; fi
  sleep 1
done
docker exec "$DB" mariadb-admin ping -uroot -poteryn-premium-root --silent >/dev/null

docker build -f "$ROOT/tests/e2e/premium_physical_e2e/Dockerfile.platform-fpm" -t "$IMAGE" "$ROOT"

start_platform() {
  local enabled="$1"
  docker rm -f "$PROXY" "$PLATFORM" >/dev/null 2>&1 || true
  docker run -d --name "$PLATFORM" --network "$NETWORK" --network-alias platform \
    -e APP_ENV=testing \
    -e APP_DEBUG=false \
    -e APP_KEY=base64:AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA= \
    -e DB_CONNECTION=mysql \
    -e DB_HOST=mariadb \
    -e DB_PORT=3306 \
    -e DB_DATABASE=oteryn_premium_e2e \
    -e DB_USERNAME=root \
    -e DB_PASSWORD=oteryn-premium-root \
    -e CACHE_STORE=array \
    -e SESSION_DRIVER=array \
    -e QUEUE_CONNECTION=sync \
    -e MAIL_MAILER=array \
    -e LOG_CHANNEL=stderr \
    -e PRODUCTS_ENTITLEMENTS_PREMIUM_SNAPSHOT_ENABLED="$enabled" \
    -e PRODUCTS_ENTITLEMENTS_PREMIUM_SNAPSHOT_MTLS_CLIENT_IDENTITY='CN=oteryn-premium-e2e' \
    -e PRODUCTS_ENTITLEMENTS_PREMIUM_SNAPSHOT_REQUESTS_PER_MINUTE=1200 \
    -e PLATFORM_BUILD_REVISION="$PLATFORM_HEAD_SHA" \
    "$IMAGE" >/dev/null
  docker run -d --name "$PROXY" --network "$NETWORK" \
    -p "127.0.0.1:${PORT}:8443" \
    -v "$ROOT/tests/e2e/premium_physical_e2e/nginx.conf:/etc/nginx/nginx.conf:ro" \
    -v "$PKI:/certs:ro" \
    nginx:1.27-alpine >/dev/null
}

wait_endpoint() {
  local expected="$1"
  local body='{"schema":"oteryn.premium_snapshot_request.v1","account_id":"01890f4e-7c00-7000-8000-000000000099","nonce":"00000000000000000000000000000000"}'
  for _ in $(seq 1 60); do
    code="$(curl -sS -o /dev/null -w '%{http_code}' --http1.1 \
      --cacert "$PKI/ca.crt" --cert "$PKI/client.crt" --key "$PKI/client.key" \
      -H 'Content-Type: application/json' --data "$body" "https://localhost:${PORT}/internal/v1/products-entitlements/premium-snapshots/read" || true)"
    if [ "$code" = "$expected" ]; then return 0; fi
    sleep 1
  done
  echo "premium endpoint did not reach expected readiness status $expected" >&2
  docker logs "$PROXY" >&2 || true
  docker logs "$PLATFORM" >&2 || true
  return 1
}

cp "$ROOT/tests/e2e/premium_physical_e2e/game_client.rs" "$GAME_ROOT/apps/game-server/tests/platform_premium_physical.rs"

run_game_case() {
  local account="$1"
  local expected="$2"
  (
    cd "$GAME_ROOT"
    PREMIUM_E2E_ORIGIN="https://localhost:${PORT}" \
    PREMIUM_E2E_IDENTITY_PEM="$PKI/client-identity.pem" \
    PREMIUM_E2E_PLATFORM_CA_PEM="$PKI/ca.crt" \
    PREMIUM_E2E_ACCOUNT="$account" \
    PREMIUM_E2E_EXPECT="$expected" \
    PREMIUM_E2E_PLATFORM_REVISION="$PLATFORM_HEAD_SHA" \
    cargo +1.94.0 test --locked -p oteryn-game-server --test platform_premium_physical \
      physical_platform_premium_endpoint -- --exact --nocapture
  )
}

# Disabled producer must be physically reachable yet fail closed before any database state matters.
start_platform false
wait_endpoint 503
run_game_case '01890f4e-7c00-7000-8000-000000000099' 503

# Enable the same exact Platform candidate and create only ephemeral test identities/state.
start_platform true
docker exec "$PLATFORM" php artisan migrate --force
TARGET_ACCOUNT="$(docker exec "$PLATFORM" php tests/e2e/premium_physical_e2e/control.php seed | tr -d '\r\n')"
[[ "$TARGET_ACCOUNT" =~ ^[0-9a-f-]{36}$ ]]
wait_endpoint 404

run_game_case '01890f4e-7c00-7000-8000-000000000099' 404
run_game_case "$TARGET_ACCOUNT" None
docker exec "$PLATFORM" php tests/e2e/premium_physical_e2e/control.php grant >/dev/null
run_game_case "$TARGET_ACCOUNT" Active
docker exec "$PLATFORM" php tests/e2e/premium_physical_e2e/control.php revoke >/dev/null
run_game_case "$TARGET_ACCOUNT" Revoked

echo "Premium physical E2E PASS: Platform=$PLATFORM_HEAD_SHA Game=$OTERYN_GAME_SHA cases=503,404,NONE,ACTIVE,REVOKED"
if [ -n "${GITHUB_STEP_SUMMARY:-}" ]; then
  {
    echo '### Premium physical cross-repository E2E'
    echo "- Platform exact SHA: \`$PLATFORM_HEAD_SHA\`"
    echo "- Game exact SHA: \`$OTERYN_GAME_SHA\`"
    echo '- Real Game `PremiumSnapshotClient` + `snapshot::validate` exercised the real Platform endpoint through TLS 1.3 mutual TLS.'
    echo '- Cases: disabled `503`, unknown AccountId `404`, known/no entitlement `NONE`, operator test grant `ACTIVE`, revocation `REVOKED`.'
  } >> "$GITHUB_STEP_SUMMARY"
fi
