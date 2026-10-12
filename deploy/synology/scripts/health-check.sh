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
for service in mariadb platform internal-proxy gateway marketplace-scheduler; do
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

# The always-on scheduler publishes no port and carries its own container healthcheck.
scheduler_deadline=$((SECONDS + 120))
until [[ "$(docker inspect --format '{{if .State.Health}}{{.State.Health.Status}}{{else}}none{{end}}' "${container_ids[marketplace-scheduler]}")" == "healthy" ]]; do
    if (( SECONDS >= scheduler_deadline )); then
        echo "Scheduler did not become healthy." >&2
        exit 1
    fi
    sleep 3
done
echo "Verified scheduler: marketplace-scheduler is healthy"

assert_binding platform 8000 "$PLATFORM_BIND_ADDRESS" "$PLATFORM_PORT"
assert_binding gateway 8080 "$GATEWAY_BIND_ADDRESS" "$GATEWAY_PORT"

container_env_value() {
    local service="$1"
    local key="$2"

    docker inspect --format '{{range .Config.Env}}{{println .}}{{end}}' "${container_ids[$service]}" \
        | awk -v prefix="${key}=" 'index($0, prefix) == 1 && !found { print substr($0, length(prefix) + 1); found = 1 }'
}

# Native login is on only while the Platform issuer can really sign: the pairing check keeps the two
# flags in step, and with the flag on the Platform signing self-check must pass (lib.sh).
platform_native_admission="$(container_env_value platform GAME_AUTH_NATIVE_ADMISSION_ENABLED)"
gateway_native_login="$(container_env_value gateway GATEWAY_NATIVE_LOGIN_ENABLED)"
oteryn_assert_native_login_pairing "$platform_native_admission" "$gateway_native_login" || exit 1
if [[ "$platform_native_admission" == true ]]; then
    oteryn_run_native_admission_self_check "${container_ids[platform]}" || exit 1
    # With the D171 fallback off, login needs a fresh account-character projection: prove the feed.
    if [[ "$(container_env_value platform GAME_AUTH_NATIVE_ACCOUNT_CHARACTERS_ENABLED)" == true ]]; then
        oteryn_wait_native_account_characters_feed "${container_ids[platform]}" || exit 1
    fi
else
    echo "::warning::Native game login is disabled on this deployment: Platform native admission is off, so the Gateway refuses every login."
fi

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

probe_internal_tls() {
    local url="$1"
    local label="$2"
    local project="${COMPOSE_PROJECT_NAME:-oteryn-staging}"
    local network="${project}_private"
    local tls_volume="${project}_internal_tls"

    for _ in $(seq 1 12); do
        if docker run --rm \
            --network "$network" \
            -v "${tls_volume}:/etc/oteryn/tls:ro" \
            -e SSL_CERT_FILE=/etc/oteryn/tls/ca.crt \
            python:3.12-alpine \
            python3 - "$url" <<'PY'
import ssl
import sys
import urllib.request

url = sys.argv[1]
context = ssl.create_default_context(cafile='/etc/oteryn/tls/ca.crt')
try:
    with urllib.request.urlopen(url, timeout=5, context=context) as response:
        if response.status != 200:
            raise SystemExit(f'unexpected HTTP status: {response.status}')
        response.read(8192)
except Exception as exc:
    print(f'internal TLS probe failed: {exc}', file=sys.stderr)
    raise SystemExit(1) from exc
PY
        then
            echo "Verified internal TLS dependency: $label"
            return 0
        fi
        sleep 2
    done

    echo "Internal TLS dependency failed: $label" >&2
    docker logs --tail 80 "${container_ids[internal-proxy]}" >&2 || true
    return 1
}

# Platform internal mTLS listener for the Game node (README: Platform internal mTLS listener). On:
# the terminator runs with its exact LAN binding, nginx accepts its configuration, a TLS 1.3
# handshake verifies the server chain against the preprod CA for the Game peer name and a request
# without a client certificate is refused (400), and the Platform FastCGI pool accepts connections.
# Off: no listener runs.
check_platform_mtls_listener() {
    local mtls_id running actual platform_flag
    mtls_id="$("${compose[@]}" ps -q internal-mtls)"
    if [[ "${OTERYN_PLATFORM_MTLS_ENABLED:-false}" != true ]]; then
        if [[ -n "$mtls_id" && "$(docker inspect --format '{{.State.Running}}' "$mtls_id")" == true ]]; then
            echo "Platform mTLS listener is running although OTERYN_PLATFORM_MTLS_ENABLED is not true." >&2
            return 1
        fi
        echo "Platform mTLS listener is off."
        return 0
    fi
    [[ -n "$mtls_id" ]] || { echo "Service is not created: internal-mtls" >&2; return 1; }
    running="$(docker inspect --format '{{.State.Running}}' "$mtls_id")"
    [[ "$running" == true ]] || { echo "Service is not running: internal-mtls" >&2; return 1; }
    container_ids[internal-mtls]="$mtls_id"
    assert_binding internal-mtls 8543 "$PLATFORM_MTLS_BIND_ADDRESS" "$PLATFORM_MTLS_PORT"
    docker exec "$mtls_id" nginx -t -q -c /bootstrap/internal-mtls-nginx.conf || {
        echo "Platform mTLS listener configuration is refused by nginx." >&2
        return 1
    }

    if docker exec "${container_ids[platform]}" sh -ec 'command -v php-fpm >/dev/null'; then
        platform_flag="$(oteryn_platform_fastcgi_flag_of "${container_ids[platform]}")" || return 1
        [[ "$platform_flag" == true ]] || {
            echo "Platform FastCGI is off while the mTLS listener is on; run a full deploy." >&2
            return 1
        }
        docker exec "${container_ids[platform]}" php -r '
            $socket = @stream_socket_client("unix:///run/oteryn-fastcgi/php-fpm.sock", $code, $message, 3);
            exit($socket === false ? 1 : 0);
        ' || {
            echo "Platform FastCGI pool is not accepting connections." >&2
            return 1
        }
        # The nginx workers (user nginx) must be able to open the socket: search on the directory and
        # write on the socket, both set by the platform_fastcgi volume options.
        docker exec -u nginx "$mtls_id" test -w /run/oteryn-fastcgi/php-fpm.sock || {
            echo "The mTLS listener's nginx workers cannot open the Platform FastCGI socket." >&2
            return 1
        }
        echo "Verified Platform FastCGI pool for the mTLS listener."
    else
        # A listener without a pool accepts TLS and answers 502 for every Game-node request, while the
        # certificate-less probe below would still pass; this is a failed deployment, not a warning.
        echo "Platform mTLS listener is on but the running Platform image has no FastCGI pool (built before the listener); every Game-node request would fail with 502." >&2
        return 1
    fi

    for _ in $(seq 1 10); do
        if docker run --rm -i \
            --network host \
            -v "${PLATFORM_MTLS_CLIENT_CA_FILE}:/etc/oteryn/internal-mtls-ca.crt:ro" \
            python:3.12-alpine \
            python3 - "$PLATFORM_MTLS_BIND_ADDRESS" "$PLATFORM_MTLS_PORT" <<'PY'
import socket
import ssl
import sys

host, port = sys.argv[1], int(sys.argv[2])
name = 'platform-internal.preprod.oteryn.internal'
context = ssl.create_default_context(cafile='/etc/oteryn/internal-mtls-ca.crt')
context.minimum_version = ssl.TLSVersion.TLSv1_3
try:
    with socket.create_connection((host, port), timeout=5) as raw:
        with context.wrap_socket(raw, server_hostname=name) as tls:
            if tls.version() != 'TLSv1.3':
                raise SystemExit(f'unexpected TLS version: {tls.version()}')
            tls.sendall(
                b'POST /internal/v1/game-auth/native-runtime-status HTTP/1.1\r\n'
                b'Host: ' + name.encode() + b'\r\nContent-Length: 0\r\nConnection: close\r\n\r\n'
            )
            status_line = tls.recv(64).split(b'\r\n', 1)[0]
except (OSError, ssl.SSLError) as exc:
    print(f'platform mTLS probe failed: {exc}', file=sys.stderr)
    raise SystemExit(1) from exc
if not status_line.startswith(b'HTTP/1.1 400 '):
    raise SystemExit(f'request without a client certificate was not refused: {status_line!r}')
PY
        then
            echo "Verified Platform mTLS listener: TLS 1.3, server chain for platform-internal.preprod.oteryn.internal, client certificate required."
            return 0
        fi
        sleep 2
    done
    echo "Platform mTLS listener probe failed." >&2
    docker logs --tail 40 "$mtls_id" >&2 || true
    return 1
}

check_platform_mtls_listener || exit 1

probe_url platform 8000 /health "Platform /health"
probe_internal_tls "https://platform-internal:8443/health" "Gateway -> Platform"
probe_url gateway 8080 /health "Gateway /health"
if ! probe_url gateway 8080 /ready "Gateway /ready"; then
    echo "Gateway aggregate readiness failed after the internal dependency passed." >&2
    docker logs --tail 80 "${container_ids[gateway]}" >&2 || true
    exit 1
fi
probe_url gateway 8080 /version "Gateway /version"

expected_gateway_version="${GATEWAY_VERSION:-synology-staging}"
docker run --rm \
    --network host \
    -i \
    python:3.12-alpine \
    python3 - "$PLATFORM_PORT" "$GATEWAY_PORT" "$expected_gateway_version" "$platform_native_admission" <<'PY'
import http.client
import json
import secrets
import sys
import time

platform_port = int(sys.argv[1])
gateway_port = int(sys.argv[2])
expected_gateway_version = sys.argv[3]
native_admission_on = sys.argv[4] == 'true'


def uuid7():
    value = (int(time.time() * 1000) & ((1 << 48) - 1)) << 80
    value |= 0x7 << 76
    value |= secrets.randbits(12) << 64
    value |= 0b10 << 62
    value |= secrets.randbits(62)
    text = f'{value:032x}'
    return f'{text[:8]}-{text[8:12]}-{text[12:16]}-{text[16:20]}-{text[20:]}'


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
    headers={'Host': 'gateway.molehill.cloud', 'Accept': 'application/json'},
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
        'Host': 'gateway.molehill.cloud',
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

# A well-formed native (protocol_version 2) login with an unknown ticket. With native admission off
# the Gateway must refuse it without reaching a dependency; with it on (the signing self-check has
# passed) the Gateway must reach the Platform issuer, which rejects the unknown ticket as 401
# NATIVE_LOGIN_AUTHENTICATION_REQUIRED. A 503 NATIVE_LOGIN_UNAVAILABLE means the issuer is down.
attempt_ref = uuid7()
native_login = json.dumps(
    {
        'protocol_version': 2,
        'game_login_ticket': 'oteryn-health-probe-' + secrets.token_hex(16),
        'attempt_ref': attempt_ref,
        'character_id': uuid7(),
        'channel_id': None,
        'offer': {
            'client_build': 'health-probe',
            'client_platform': 'windows',
            'transports': [{'protocol_major': 1, 'transport_profile': 1, 'alpn': 'oteryn-game/1'}],
        },
    },
    separators=(',', ':'),
).encode()
status, headers, body = request(
    gateway_port,
    'POST',
    '/v1/login',
    body=native_login,
    headers={
        'Host': 'gateway.molehill.cloud',
        'Accept': 'application/json',
        'Content-Type': 'application/json',
    },
)
try:
    native_payload = json.loads(body)
except json.JSONDecodeError as exc:
    raise SystemExit(f'Well-formed native Gateway login did not return JSON (status {status})') from exc
if native_admission_on:
    native_error = native_payload.get('error') if isinstance(native_payload, dict) else None
    native_code = native_error.get('code') if isinstance(native_error, dict) else None
    if status != 401 or native_code != 'NATIVE_LOGIN_AUTHENTICATION_REQUIRED':
        raise SystemExit(f'Gateway did not reject the unknown native ticket through the Platform issuer: status {status}')
elif status != 400 or native_payload != {'error': 'invalid_request'}:
    raise SystemExit(f'Gateway answered a native login while native admission is off: status {status}')
cache_control = {token.strip().lower() for token in headers.get('cache-control', '').split(',')}
if not required_cache_control.issubset(cache_control):
    raise SystemExit('Native Gateway login response lost required private no-store cache controls')

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
    headers={'Host': 'gateway.molehill.cloud', 'Accept': 'text/html'},
)
if status != 404:
    raise SystemExit(f'Gateway non-contract /login route returned unexpected status: {status}')
body_lower = body.lower()
if b'<form' in body_lower or b'oteryn platform' in body_lower:
    raise SystemExit('Gateway port cross-routed to Platform content')

print('Verified Gateway identity, bounded invalid and well-formed native login, private no-store headers and port isolation.')
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

docker exec "$platform_container" grep -q 'portal.identity.mfa_scan' /var/www/html/resources/views/identity/mfa/settings.blade.php
docker exec "$platform_container" grep -q 'mfa-qr-panel' /var/www/html/resources/views/identity/mfa/settings.blade.php
docker exec "$platform_container" grep -q 'mfa-qr-code' /var/www/html/resources/views/identity/mfa/settings.blade.php
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

echo "Platform, Gateway, canonical URL, cache-control, isolation and MFA QR staging probes passed."
