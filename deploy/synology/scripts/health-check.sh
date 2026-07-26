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

echo "Platform, Gateway, Canary and MFA QR staging probes passed."
