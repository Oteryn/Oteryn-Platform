#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
DEPLOY_DIR="$(cd -- "$SCRIPT_DIR/.." && pwd)"
ENV_FILE="${OTERYN_ENV_FILE:-$DEPLOY_DIR/.env}"
COMPOSE_FILE="$DEPLOY_DIR/compose.yml"

# shellcheck source=deploy/synology/scripts/lib.sh
source "$SCRIPT_DIR/lib.sh"
load_oteryn_env_file "$ENV_FILE"

compose=(docker compose --env-file "$ENV_FILE" -f "$COMPOSE_FILE")

declare -A container_ids=()
for service in mariadb redis canary platform internal-proxy gateway; do
    container_id="$("${compose[@]}" ps -q "$service")"
    if [[ -z "$container_id" ]]; then
        echo "Service is not created: $service" >&2
        exit 1
    fi
    running="$(docker inspect --format '{{.State.Running}}' "$container_id")"
    if [[ "$running" != "true" ]]; then
        echo "Service is not running: $service" >&2
        exit 1
    fi
    container_ids["$service"]="$container_id"
done

assert_binding() {
    local service="$1"
    local container_port="$2"
    local expected_host="$3"
    local expected_port="$4"
    local container_id="${container_ids[$service]}"
    local actual

    actual="$(docker inspect --format "{{with index .NetworkSettings.Ports \"${container_port}/tcp\"}}{{if eq (len .) 1}}{{(index . 0).HostIp}}:{{(index . 0).HostPort}}{{end}}{{end}}" "$container_id")"
    if [[ "$actual" != "${expected_host}:${expected_port}" ]]; then
        echo "Unexpected published binding for ${service} ${container_port}/tcp: ${actual:-none}" >&2
        exit 1
    fi

    echo "Verified binding: ${service} ${container_port}/tcp -> ${actual}"
}

assert_binding platform 8000 "$PLATFORM_BIND_ADDRESS" "$PLATFORM_PORT"
assert_binding gateway 8080 "$GATEWAY_BIND_ADDRESS" "$GATEWAY_PORT"
assert_binding canary "$CANARY_LOGIN_PORT" "$CANARY_LOGIN_BIND_ADDRESS" "$CANARY_LOGIN_PORT"
assert_binding canary "$CANARY_GAME_PORT" "$CANARY_GAME_BIND_ADDRESS" "$CANARY_GAME_PORT"

probe_url() {
    local service="$1"
    local port="$2"
    local path="$3"
    local label="$4"
    local container_id="${container_ids[$service]}"

    for _ in $(seq 1 30); do
        if docker run --rm \
            --network "container:$container_id" \
            alpine:3.22 \
            /bin/sh -ec \
            "wget -qO- -T 5 'http://127.0.0.1:${port}${path}' >/dev/null"; then
            return 0
        fi
        sleep 2
    done

    echo "Health probe failed: $label" >&2
    return 1
}

probe_url platform 8000 /health "Platform /health"
probe_url gateway 8080 /health "Gateway /health"
probe_url gateway 8080 /ready "Gateway /ready"
probe_url gateway 8080 /version "Gateway /version"

expected_gateway_version="${GATEWAY_VERSION:-synology-staging}"
docker run --rm \
    --network host \
    -i \
    python:3.12-alpine \
    python3 - "$PLATFORM_PORT" "$GATEWAY_PORT" "$expected_gateway_version" <<'PY'
import http.client
import json
import sys
import time

platform_port = int(sys.argv[1])
gateway_port = int(sys.argv[2])
expected_gateway_version = sys.argv[3]


def request(port, method, path, *, body=None, headers=None):
    last_error = None
    for attempt in range(15):
        connection = http.client.HTTPConnection('127.0.0.1', port, timeout=5)
        try:
            connection.request(method, path, body=body, headers=headers or {})
            response = connection.getresponse()
            payload = response.read(8192)
            return response.status, {key.lower(): value for key, value in response.getheaders()}, payload
        except OSError as exc:
            last_error = exc
            if attempt == 14:
                break
            time.sleep(2)
        finally:
            connection.close()

    raise ConnectionError(
        f'Host-loopback request failed after bounded retries: {method} {path}'
    ) from last_error


status, headers, body = request(
    gateway_port,
    'GET',
    '/version',
    headers={'Host': 'login.oteryn.molehill.cloud', 'Accept': 'application/json'},
)
if status != 200:
    raise SystemExit(f'Gateway /version returned unexpected status: {status}')
try:
    version_payload = json.loads(body)
except json.JSONDecodeError as exc:
    raise SystemExit('Gateway /version did not return bounded JSON') from exc
if version_payload != {
    'service': 'oteryn-game-gateway',
    'version': expected_gateway_version,
}:
    raise SystemExit('Gateway /version returned the wrong service identity or version')
if not headers.get('content-type', '').lower().startswith('application/json'):
    raise SystemExit('Gateway /version did not return application/json')

status, headers, body = request(
    gateway_port,
    'POST',
    '/v1/login',
    body=b'{}',
    headers={
        'Host': 'login.oteryn.molehill.cloud',
        'Accept': 'application/json',
        'Content-Type': 'application/json',
    },
)
if status != 400:
    raise SystemExit(f'Bounded invalid Gateway login returned unexpected status: {status}')
try:
    login_payload = json.loads(body)
except json.JSONDecodeError as exc:
    raise SystemExit('Bounded invalid Gateway login did not return JSON') from exc
if login_payload != {'error': 'invalid_request'}:
    raise SystemExit('Bounded invalid Gateway login returned an unexpected body')
cache_control = {token.strip().lower() for token in headers.get('cache-control', '').split(',')}
required_cache_control = {'no-store', 'no-cache', 'must-revalidate', 'private'}
if not required_cache_control.issubset(cache_control):
    raise SystemExit('Gateway login response lost required private no-store cache controls')
if headers.get('pragma', '').lower() != 'no-cache' or headers.get('expires') != '0':
    raise SystemExit('Gateway login response lost legacy no-cache controls')

status, _, body = request(
    platform_port,
    'GET',
    '/version',
    headers={'Host': 'oteryn.molehill.cloud', 'Accept': 'application/json'},
)
if b'oteryn-game-gateway' in body:
    raise SystemExit('Platform port cross-routed to the Game Gateway')

status, _, body = request(
    gateway_port,
    'GET',
    '/login?locale=en',
    headers={'Host': 'login.oteryn.molehill.cloud', 'Accept': 'text/html'},
)
if status != 404:
    raise SystemExit(f'Gateway non-contract /login route returned unexpected status: {status}')
body_lower = body.lower()
if b'<form' in body_lower or b'oteryn platform' in body_lower:
    raise SystemExit('Gateway port cross-routed to Platform content')

print('Verified Gateway identity, bounded invalid login, private no-store headers and port isolation.')
PY

platform_container="${container_ids[platform]}"
docker exec -i "$platform_container" php <<'PHP'
<?php
require '/var/www/html/vendor/autoload.php';

$renderer = new App\Identity\Mfa\MfaQrCode();
$uri = 'otpauth://totp/Oteryn%20Staging:test@example.invalid?secret=JBSWY3DPEHPK3PXP&issuer=Oteryn%20Staging';
$dataUri = $renderer->dataUri($uri);

if (! str_starts_with($dataUri, 'data:image/svg+xml;base64,')) {
    fwrite(STDERR, "MFA QR renderer did not return an inline SVG data URI.\n");
    exit(31);
}

$svg = base64_decode(substr($dataUri, strlen('data:image/svg+xml;base64,')), true);
if (! is_string($svg) || ! str_contains($svg, '<svg') || str_contains($svg, $uri)) {
    fwrite(STDERR, "MFA QR renderer output failed the bounded SVG checks.\n");
    exit(32);
}

$context = stream_context_create([
    'http' => [
        'follow_location' => 0,
        'ignore_errors' => true,
        'timeout' => 5,
    ],
]);
$headers = get_headers('http://127.0.0.1:8000/mfa', false, $context);
$statusLine = is_array($headers) ? ($headers[0] ?? null) : null;
if (! is_string($statusLine) || preg_match('/^HTTP\/\S+ 302\b/', $statusLine) !== 1) {
    fwrite(STDERR, "Anonymous MFA settings boundary did not return HTTP 302.\n");
    exit(33);
}

echo "MFA QR renderer and protected anonymous route verified.\n";
PHP

docker exec "$platform_container" grep -q 'Scan with your authenticator app' /var/www/html/resources/views/identity/mfa/settings.blade.php
docker exec "$platform_container" grep -q 'mfa-qr' /var/www/html/public/css/mfa.css
echo "Verified QR-first MFA renderer, deployed assets and protected anonymous MFA boundary."

public_login_html="$(docker run --rm \
    --network host \
    alpine:3.22 \
    /bin/sh -ec \
    "wget -qO- -T 5 \
        --header='Host: oteryn.molehill.cloud' \
        --header='X-Forwarded-Host: oteryn.molehill.cloud' \
        --header='X-Forwarded-Proto: https' \
        --header='X-Forwarded-Port: 443' \
        'http://127.0.0.1:${PLATFORM_PORT}/login?locale=en'")"

if ! grep -Fq 'action="https://oteryn.molehill.cloud/login?locale=en"' <<<"$public_login_html"; then
    echo "Public HTTPS login form did not preserve the external HTTPS origin." >&2
    exit 34
fi
echo "Verified public HTTPS login form action through the host-loopback proxy boundary."

docker exec -i "$platform_container" php <<'PHP'
<?php
require '/var/www/html/vendor/autoload.php';
$app = require '/var/www/html/bootstrap/app.php';
$kernel = $app->make(Illuminate\Contracts\Console\Kernel::class);
$kernel->bootstrap();

$expectedOrigin = 'https://oteryn.molehill.cloud';
if (config('app.url') !== $expectedOrigin) {
    fwrite(STDERR, "Deployed APP_URL is not the canonical public HTTPS origin.\n");
    exit(35);
}
if (config('session.secure') !== true) {
    fwrite(STDERR, "Deployed public staging session cookie is not Secure.\n");
    exit(36);
}

$urls = [
    route('identity.login.create', absolute: true),
    route('password.reset', [
        'token' => 'redacted-health-check-token',
        'email' => 'controlled@example.invalid',
    ], true),
    Illuminate\Support\Facades\URL::temporarySignedRoute(
        'admin.wiki.articles.preview',
        now()->addMinutes(5),
        ['article' => 1, 'locale' => 'en'],
    ),
];

foreach ($urls as $url) {
    $parts = parse_url($url);
    if (! is_array($parts)
        || ($parts['scheme'] ?? null) !== 'https'
        || ($parts['host'] ?? null) !== 'oteryn.molehill.cloud'
        || str_contains($url, '127.0.0.1')
        || str_contains($url, 'localhost')) {
        fwrite(STDERR, "Requestless URL generation escaped the canonical public origin.\n");
        exit(37);
    }
}

echo "Verified requestless login, password-reset and signed-route canonical origins.\n";
PHP

if ! docker run --rm \
    --network "container:${container_ids[canary]}" \
    alpine:3.22 \
    /bin/sh -ec \
    "nc -z -w 3 127.0.0.1 '${CANARY_GAME_PORT}'"; then
    echo "Canary game TCP port is not reachable inside the Canary network namespace." >&2
    exit 1
fi

if [[ "$CANARY_GAME_BIND_ADDRESS" != "127.0.0.1" ]]; then
    if ! timeout 5 bash -c "exec 3<>/dev/tcp/${CANARY_GAME_BIND_ADDRESS}/${CANARY_GAME_PORT}"; then
        echo "Canary game TCP port is not reachable through the configured Synology LAN address." >&2
        exit 1
    fi
    echo "Verified LAN game endpoint: ${CANARY_GAME_BIND_ADDRESS}:${CANARY_GAME_PORT}"
fi

echo "Platform, Gateway, Canary, canonical URL, cache-control, isolation and MFA QR staging probes passed."
