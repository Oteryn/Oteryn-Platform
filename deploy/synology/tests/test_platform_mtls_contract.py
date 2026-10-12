"""Static contract for the Platform internal mTLS listener (README: Platform internal mTLS listener)."""

from __future__ import annotations

import pathlib
import re

ROOT = pathlib.Path(__file__).resolve().parents[3]
SYNOLOGY = ROOT / "deploy" / "synology"
NGINX = SYNOLOGY / "nginx" / "internal-mtls.conf"
COMPOSE = SYNOLOGY / "compose.yml"
DOCKERFILE = SYNOLOGY / "docker" / "platform.Dockerfile"
ENTRYPOINT = SYNOLOGY / "docker" / "platform-entrypoint.sh"
DEFAULTS = SYNOLOGY / "native-defaults.env"
LIB = SYNOLOGY / "scripts" / "lib.sh"
DEPLOY = SYNOLOGY / "scripts" / "deploy.sh"
ROLLBACK = SYNOLOGY / "scripts" / "rollback.sh"
HEALTH = SYNOLOGY / "scripts" / "health-check.sh"
ROUTES = ROOT / "routes" / "internal.php"

GAME_NODE_ROUTES = {
    "native-evidence",
    "native-runtime-status",
    "native-scope-assignments",
    "native-account-characters",
    "native-account-characters/watermark",
    "character-bootstrap-intents/read",
}
STAGING_FILES = {
    "SERVER_CERT_FILE": "/volume1/oteryn/preprod-platform-tls/platform-internal-preprod.fullchain.pem",
    "SERVER_KEY_FILE": "/volume1/oteryn/preprod-platform-tls/platform-internal-preprod.key",
    "CLIENT_CA_FILE": "/volume1/oteryn/preprod-platform-tls/oteryn-preprod-ca.crt",
}
SOCKET = "/run/oteryn-fastcgi/php-fpm.sock"
FASTCGI_MOUNT = (
    "      - type: volume\n"
    "        source: platform_fastcgi\n"
    "        target: /run/oteryn-fastcgi\n"
    "        volume:\n"
    "          nocopy: true\n"
)


def _directives(text: str) -> list[str]:
    return [line.split("#", 1)[0].strip() for line in text.splitlines() if line.split("#", 1)[0].strip()]


def _service_block(compose: str, name: str) -> str:
    match = re.search(rf"^  {re.escape(name)}:\n(.*?)(?=^  [a-z][a-z0-9_-]*:\n|^[a-z])", compose, re.M | re.S)
    assert match, f"compose service {name} not found"
    return match.group(1)


def test_listener_is_tls13_only_with_mandatory_client_certificates() -> None:
    directives = _directives(NGINX.read_text())
    assert "listen 8543 ssl;" in directives
    assert "ssl_protocols TLSv1.3;" in directives
    assert "ssl_verify_client on;" in directives
    assert "ssl_session_tickets off;" in directives
    assert "ssl_client_certificate /etc/nginx/internal-mtls/client-ca.crt;" in directives
    assert "ssl_certificate /etc/nginx/internal-mtls/server-fullchain.pem;" in directives
    assert "ssl_certificate_key /etc/nginx/internal-mtls/server.key;" in directives
    assert not [d for d in directives if d.startswith("ssl_protocols") and "TLSv1.2" in d]
    assert not [d for d in directives if "ssl_verify_client" in d and d != "ssl_verify_client on;"]
    assert not [d for d in directives if d.startswith("listen") and d != "listen 8543 ssl;"]


def test_client_certificate_values_reach_php_only_from_the_tls_session() -> None:
    directives = _directives(NGINX.read_text())
    assert "fastcgi_param SSL_CLIENT_VERIFY $ssl_client_verify;" in directives
    assert "fastcgi_param SSL_PROTOCOL $ssl_protocol;" in directives
    assert "fastcgi_param SSL_CLIENT_S_DN $ssl_client_s_dn;" in directives
    ssl_params = [d for d in directives if re.match(r"fastcgi_param (HTTP_)?SSL_", d)]
    assert len(ssl_params) == 3, ssl_params
    assert not [d for d in directives if d.startswith("include")]
    assert not [d for d in directives if d.startswith(("proxy_pass", "proxy_set_header"))]
    for header in ("HTTP_PROXY", "HTTP_FORWARDED", "HTTP_X_FORWARDED_FOR", "HTTP_X_REAL_IP"):
        assert f'fastcgi_param {header} "";' in directives
    assert f"fastcgi_pass unix:{SOCKET};" in directives
    assert len([d for d in directives if d.startswith("fastcgi_pass ")]) == 1


def test_only_game_node_routes_are_exposed() -> None:
    text = NGINX.read_text()
    locations = re.findall(r"^\s*location\s+([^{]+)\{", text, re.M)
    assert [loc.strip() for loc in locations] == [
        "~ ^/internal/v1/game-auth/(native-evidence|native-runtime-status|native-scope-assignments|"
        "native-account-characters|native-account-characters/watermark|character-bootstrap-intents/read)$",
        "/",
    ]
    exposed = set(re.search(r"game-auth/\(([^)]*)\)\$", locations[0]).group(1).split("|"))
    assert exposed == GAME_NODE_ROUTES
    assert "native-admissions" not in exposed
    assert "limit_except POST { deny all; }" in text
    assert re.search(r"location / \{\s*return 404;\s*\}", text)

    routes = ROUTES.read_text()
    for route in GAME_NODE_ROUTES:
        assert f"Route::post('/internal/v1/game-auth/{route}'," in routes, route


def test_terminator_mounts_host_files_read_only_and_never_uses_tls_init() -> None:
    compose = COMPOSE.read_text()
    service = _service_block(compose, "internal-mtls")
    assert 'profiles: ["internal-mtls"]' in service
    assert "${PLATFORM_MTLS_BIND_ADDRESS:-127.0.0.1}:${PLATFORM_MTLS_PORT:-8543}:8543" in service
    for suffix, path in STAGING_FILES.items():
        assert f"source: ${{PLATFORM_MTLS_{suffix}:-{path}}}" in service
    assert service.count("type: bind") == 3
    assert service.count("read_only: true") == 3
    assert service.count("create_host_path: false") == 3
    assert "bootstrap_files:/bootstrap:ro" in service
    assert FASTCGI_MOUNT in service
    assert "internal_tls" not in service and "tls-init" not in service and "tls_init" not in service
    networks = re.search(r"networks:\n((?:\s+- .+\n)+)", service).group(1)
    assert [n.strip() for n in networks.splitlines()] == ["- mtls_edge"]
    assert re.search(r"^  mtls_edge:\n    driver: bridge", compose, re.M)

    platform = _service_block(compose, "platform")
    assert "OTERYN_PLATFORM_FASTCGI_ENABLED: *internal_mtls_enabled" in platform
    assert FASTCGI_MOUNT in platform
    # Short syntax would let the first mount copy the image directory's owner onto the tmpfs.
    assert "- platform_fastcgi:/run/oteryn-fastcgi" not in compose
    assert compose.count(FASTCGI_MOUNT) == 2
    assert re.search(r'^  platform_fastcgi:\n    driver: local\n    driver_opts:\n      type: tmpfs', compose, re.M)

    nginx = NGINX.read_text()
    assert "tls-init" in nginx  # documented independence only
    assert "/etc/nginx/tls" not in nginx and "/tls/" not in nginx


def test_fastcgi_pool_is_unix_socket_only_and_off_by_default() -> None:
    dockerfile = DOCKERFILE.read_text()
    assert f"'listen = {SOCKET}'" in dockerfile
    assert not re.search(r"'listen = [^/]", dockerfile)
    assert "rm -f /usr/local/etc/php-fpm.d/*.conf" in dockerfile
    assert "'clear_env = no'" in dockerfile
    assert "RUN php-fpm --test" in dockerfile
    assert "STOPSIGNAL SIGTERM" in dockerfile

    entrypoint = ENTRYPOINT.read_text()
    assert "OTERYN_PLATFORM_FASTCGI_ENABLED" in entrypoint
    assert '[ "$$" = 1 ] || return 0' in entrypoint
    assert "start_platform_fastcgi || exit 7" in entrypoint

    compose = COMPOSE.read_text()
    assert '&internal_mtls_enabled "${OTERYN_PLATFORM_MTLS_ENABLED:-false}"' in compose


def test_staging_values_are_documented_and_resolved_by_the_deploy_scripts() -> None:
    defaults = DEFAULTS.read_text()
    assert "OTERYN_STAGING_NATIVE_PLATFORM_MTLS_ENABLED=true" in defaults
    assert "OTERYN_STAGING_NATIVE_PLATFORM_MTLS_BIND_ADDRESS=192.168.1.2" in defaults
    assert "OTERYN_STAGING_NATIVE_PLATFORM_MTLS_PORT=8543" in defaults
    for suffix, path in STAGING_FILES.items():
        assert f"OTERYN_STAGING_NATIVE_PLATFORM_MTLS_{suffix}={path}" in defaults
        assert "-----BEGIN" not in defaults

    lib = LIB.read_text()
    assert "oteryn_resolve_platform_mtls_settings" in lib
    assert "oteryn_platform_mtls_settings_valid" in lib
    assert "OTERYN_PLATFORM_MTLS_ENABLED:ENABLED:false" in lib

    deploy = DEPLOY.read_text()
    assert "oteryn_platform_mtls_settings_valid || exit 1" in deploy
    assert '"$DEPLOY_DIR/nginx/internal-mtls.conf" "$tls_container:/bootstrap/internal-mtls-nginx.conf"' in deploy
    assert deploy.index("stage_bootstrap_files") < deploy.index('oteryn_platform_mtls_preflight "${compose[@]}"')
    assert 'oteryn_reconcile_platform_mtls "${compose[@]}"' in deploy
    assert 'oteryn_reconcile_platform_mtls "${compose[@]}"' in ROLLBACK.read_text()

    health = HEALTH.read_text()
    assert "check_platform_mtls_listener" in health
    assert "assert_binding internal-mtls 8543" in health
    assert "name = 'platform-internal.preprod.oteryn.internal'" in health
    assert "server_hostname=name" in health
    assert "context.minimum_version = ssl.TLSVersion.TLSv1_3" in health
    assert "b'HTTP/1.1 400 '" in health


def test_platform_image_without_fastcgi_fails_instead_of_serving_502() -> None:
    lib = LIB.read_text()
    assert "oteryn_platform_image_has_fastcgi()" in lib
    assert "test -f /usr/local/etc/php-fpm.d/oteryn-internal-mtls.conf" in lib
    assert "/usr/local/etc/php-fpm.d/oteryn-internal-mtls.conf" in DOCKERFILE.read_text()
    ready = re.search(r"^oteryn_platform_mtls_image_ready\(\) \{\n(.*?)^\}", lib, re.M | re.S).group(1)
    assert 'oteryn_platform_image_has_fastcgi "$image"' in ready
    assert "return 1" in ready
    preflight = re.search(r"^oteryn_platform_mtls_preflight\(\) \{\n(.*?)^\}", lib, re.M | re.S).group(1)
    assert 'oteryn_platform_mtls_image_ready "${PLATFORM_IMAGE:-}" || return 1' in preflight

    rollback = ROLLBACK.read_text()
    image_check = 'oteryn_platform_mtls_image_ready "$PLATFORM_IMAGE" || exit 1'
    assert image_check in rollback
    assert rollback.index('"${compose[@]}" pull platform gateway') < rollback.index(image_check)
    assert rollback.index(image_check) < rollback.index("oteryn_native_admission_ingress_stop")

    health = HEALTH.read_text()
    listener = re.search(r"^check_platform_mtls_listener\(\) \{\n(.*?)^\}", health, re.M | re.S).group(1)
    assert "::warning::" not in listener
    no_pool = listener[listener.index("command -v php-fpm"):listener.index("for _ in $(seq 1 10)")]
    else_branch = no_pool[no_pool.rindex("    else\n"):]
    assert "has no FastCGI pool" in else_branch
    assert "return 1" in else_branch


def test_fast_deploy_path_reconciles_when_listener_inputs_change() -> None:
    lib = LIB.read_text()
    fingerprint = re.search(r"^oteryn_platform_mtls_fingerprint\(\) \{\n(.*?)^\}", lib, re.M | re.S).group(1)
    for name in (
        "PLATFORM_MTLS_BIND_ADDRESS",
        "PLATFORM_MTLS_PORT",
        "PLATFORM_MTLS_SERVER_CERT_FILE",
        "PLATFORM_MTLS_SERVER_KEY_FILE",
        "PLATFORM_MTLS_CLIENT_CA_FILE",
    ):
        assert f'"${name}"' in fingerprint, name
    assert "--network none" in fingerprint
    assert fingerprint.count(",readonly\"") == 3
    assert "sha256sum /m/server-fullchain.pem /m/server.key /m/client-ca.crt" in fingerprint
    assert 'sha256sum <"$conf"' in fingerprint
    assert "nginx/internal-mtls.conf" in fingerprint
    assert "|| return 1" in fingerprint

    reconcile = re.search(r"^oteryn_reconcile_platform_mtls\(\) \{\n(.*?)^\}", lib, re.M | re.S).group(1)
    assert 'OTERYN_PLATFORM_MTLS_FINGERPRINT="$fingerprint" "$@" up -d --no-deps --force-recreate internal-mtls' in reconcile

    service = _service_block(COMPOSE.read_text(), "internal-mtls")
    assert "oteryn.platform-mtls.fingerprint: ${OTERYN_PLATFORM_MTLS_FINGERPRINT:-unset}" in service

    deploy = DEPLOY.read_text()
    fast = re.search(r"^_oteryn_fast_candidate\(\) \{\n(.*?)^\}", deploy, re.M | re.S).group(1)
    check = fast[fast.index('if [[ "$want_mtls" == true ]]; then'):]
    assert 'want_mtls_fingerprint="$(oteryn_platform_mtls_fingerprint)"' in check
    assert '"$(oteryn_platform_mtls_fingerprint_of "$mtls_id")" != "$want_mtls_fingerprint"' in check
    assert "FAST_DEPLOY_REASON='platform mTLS listener configuration or certificate files changed'" in check
    assert check.index("return 1") < check.index("FAST_MARIADB_ID")


def test_entrypoint_supervises_the_fastcgi_pool_as_pid_1() -> None:
    entrypoint = ENTRYPOINT.read_text()
    assert "fastcgi_pid=$!" in entrypoint
    # BusyBox ash `wait -n` misses signal-killed children; only comments may mention it.
    assert not [d for d in _directives(entrypoint) if "wait -n" in d]
    alive = re.search(r"^process_alive\(\) \{\n(.*?)^\}", entrypoint, re.M | re.S).group(1)
    assert "/proc/$1/status" in alive and "Z" in alive
    run = re.search(r"^run_supervised\(\) \{\n(.*?)^\}", entrypoint, re.M | re.S).group(1)
    assert 'while process_alive "$main_pid" && process_alive "$fastcgi_pid"; do' in run
    assert "status=8" in run and "status=9" in run
    assert "trap 'stop_supervised; exit 143' TERM INT" in run
    assert 'exit "$status"' in run
    tail = entrypoint[entrypoint.rindex('if [ -n "$fastcgi_pid" ]; then'):]
    assert tail.index('run_supervised "$@"') < tail.index('exec "$@"')


def test_request_headers_reach_php_and_nginx_workers_can_open_the_socket() -> None:
    directives = _directives(NGINX.read_text())
    assert "fastcgi_pass_request_headers on;" in directives
    assert not [d for d in directives if d.startswith("fastcgi_pass_request_headers") and d.endswith("off;")]
    for header in ("HTTP_TRANSFER_ENCODING", "HTTP_CONTENT_ENCODING"):
        assert not [d for d in directives if d.startswith(f"fastcgi_param {header} ")], header

    health = HEALTH.read_text()
    listener = re.search(r"^check_platform_mtls_listener\(\) \{\n(.*?)^\}", health, re.M | re.S).group(1)
    assert f'docker exec -u nginx "$mtls_id" test -w {SOCKET}' in listener


def main() -> None:
    tests = [
        value
        for name, value in sorted(globals().items())
        if name.startswith("test_") and callable(value)
    ]
    for test in tests:
        test()
    print(f"synology platform mTLS contract: PASS ({len(tests)} tests)")


if __name__ == "__main__":
    main()
