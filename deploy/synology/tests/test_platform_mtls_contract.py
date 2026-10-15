"""Static contract for the Platform internal mTLS listener (README: Platform internal mTLS listener)."""

from __future__ import annotations

import os
import pathlib
import re
import subprocess
import tempfile

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
SCRIPTS = SYNOLOGY / "scripts"

GAME_NODE_ROUTES = {
    "native-evidence",
    "native-runtime-status",
    "native-scope-assignments",
    "native-account-characters",
    "native-account-characters/watermark",
    "character-bootstrap-intents/read",
    "character-commands/pending",
    "character-commands/read",
    "character-commands/receipt",
}
# Mounted by app/CharacterAuthority/Http/CharacterCommandRoutes.php from the CharacterCommandWire paths.
CHARACTER_COMMAND_ROUTES = {"character-commands/pending", "character-commands/read", "character-commands/receipt"}
WIRE = ROOT / "app" / "CharacterAuthority" / "CharacterCommandWire.php"
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
        "native-account-characters|native-account-characters/watermark|character-bootstrap-intents/read|"
        "character-commands/pending|character-commands/read|character-commands/receipt)$",
        "/",
    ]
    exposed = set(re.search(r"game-auth/\(([^)]*)\)\$", locations[0]).group(1).split("|"))
    assert exposed == GAME_NODE_ROUTES
    assert "native-admissions" not in exposed
    assert "limit_except POST { deny all; }" in text
    assert re.search(r"location / \{\s*return 404;\s*\}", text)

    routes = ROUTES.read_text()
    for route in GAME_NODE_ROUTES - CHARACTER_COMMAND_ROUTES:
        assert f"Route::post('/internal/v1/game-auth/{route}'," in routes, route
    assert "CharacterCommandRoutes::register();" in routes
    wire = set(re.findall(r"_PATH = '/internal/v1/game-auth/([a-z/-]+)';", WIRE.read_text()))
    assert wire == CHARACTER_COMMAND_ROUTES, wire


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

    # rollback.sh checks the last-good image through the full preflight (see the rollback test).
    assert "oteryn_platform_mtls_image_ready" not in ROLLBACK.read_text()

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
    assert check.index("return 1") < check.index("FAST_POSTGRES_ID")


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


def test_legacy_candidate_image_is_finalized_with_the_listener_off() -> None:
    lib = LIB.read_text()
    probe = re.search(r"^oteryn_platform_image_has_fastcgi\(\) \{\n(.*?)^\}", lib, re.M | re.S).group(1)
    assert "then exit 0; fi; exit 3" in probe
    flag = re.search(r"^oteryn_platform_mtls_flag_for_image\(\) \{\n(.*?)^\}", lib, re.M | re.S).group(1)
    assert "0) echo true ;;" in flag and "3) echo false ;;" in flag
    assert re.search(r"\*\) echo .*>&2; return 1 ;;", flag)  # an image that cannot be inspected fails
    pin = re.search(r"^oteryn_pin_platform_mtls_flag\(\) \{\n(.*?)^\}", lib, re.M | re.S).group(1)
    assert "grep -v '^OTERYN_PLATFORM_MTLS_ENABLED='" in pin and "chmod 600" in pin

    deploy = DEPLOY.read_text()
    body = re.search(r"^finalize_previous_candidate_if_healthy\(\) \{\n(.*?)^\}", deploy, re.M | re.S).group(1)
    decide = 'candidate_mtls="$(oteryn_platform_mtls_flag_for_image "${candidate_state[3]}")"'
    pin_call = 'oteryn_pin_platform_mtls_flag "$candidate_env" "$candidate_mtls"'
    reconcile = 'OTERYN_PLATFORM_MTLS_ENABLED="$candidate_mtls" oteryn_reconcile_platform_mtls "${candidate_compose[@]}"'
    for needle in (decide, pin_call, '"OTERYN_PLATFORM_MTLS_ENABLED=$candidate_mtls"', reconcile):
        assert needle in body, needle
    # Decided before any mutation of the stack, and the candidate health check reads the pinned file.
    assert body.index(decide) < body.index(pin_call) < body.index("oteryn_native_admission_ingress_stop")
    assert body.index("oteryn_native_admission_ingress_stop") < body.index(reconcile)
    assert body.index(reconcile) < body.index('OTERYN_ENV_FILE="$candidate_env" bash "$SCRIPT_DIR/health-check.sh"')
    assert "oteryn_reconcile_platform_mtls \"${candidate_compose[@]}\"" not in body.replace(reconcile, "")
    # This deploy's own image is preflighted before the candidate is finalized.
    assert deploy.index('oteryn_platform_mtls_preflight "${compose[@]}" || exit 1') < deploy.index(
        "\nfinalize_previous_candidate_if_healthy\n"
    )


def test_rollback_preflights_the_listener_before_stopping_anything() -> None:
    rollback = ROLLBACK.read_text()
    valid = "oteryn_platform_mtls_settings_valid || exit 1"
    preflight = 'oteryn_platform_mtls_preflight "${compose[@]}" || exit 1'
    stop = "oteryn_native_admission_ingress_stop"
    assert rollback.index('"${compose[@]}" pull platform gateway') < rollback.index(valid)
    assert rollback.index(valid) < rollback.index(preflight) < rollback.index(stop)
    assert rollback.index(stop) < rollback.index('oteryn_reconcile_platform_mtls "${compose[@]}"')

    lib = LIB.read_text()
    body = re.search(r"^oteryn_platform_mtls_preflight\(\) \{\n(.*?)^\}", lib, re.M | re.S).group(1)
    assert 'oteryn_platform_mtls_image_ready "${PLATFORM_IMAGE:-}" || return 1' in body
    assert "ip -o -4 addr show" in body  # bind address is on this host
    assert "--entrypoint nginx internal-mtls -t -q -c /bootstrap/internal-mtls-nginx.conf" in body


def test_listener_is_stopped_while_platform_is_replaced_and_migrated() -> None:
    lib = LIB.read_text()
    body = re.search(r"^oteryn_native_admission_ingress_stop\(\) \{\n(.*?)^\}", lib, re.M | re.S).group(1)
    # On: stopped and kept for the way back. Off: any existing listener is removed, whatever its state.
    assert '"$@" stop internal-mtls >&2 || return 1' in body
    assert '"$@" rm -s -f internal-mtls >&2 || return 1' in body
    stop = 'oteryn_native_admission_ingress_stop "${compose[@]}" >/dev/null'
    reconcile = 'oteryn_reconcile_platform_mtls "${compose[@]}"'

    deploy = DEPLOY.read_text()
    main = deploy[deploy.index("\nfinalize_previous_candidate_if_healthy\n"):]
    assert main.index(stop) < main.index('"${compose[@]}" up -d platform')
    assert main.index(stop) < main.index("oteryn_platform_migrate_postgres")
    assert main.index("oteryn_publish_native_admission_trust_or_stop") < main.index(reconcile)

    rollback = ROLLBACK.read_text()
    assert rollback.index(stop) < rollback.index('"${compose[@]}" up -d platform')
    assert rollback.index("oteryn_native_admission_self_check_or_stop") < rollback.index(reconcile)


def test_candidate_keeps_the_listener_off_when_its_platform_runs_without_fastcgi() -> None:
    deploy = DEPLOY.read_text()
    body = re.search(r"^finalize_previous_candidate_if_healthy\(\) \{\n(.*?)^\}", deploy, re.M | re.S).group(1)
    decide = 'candidate_mtls="$(oteryn_platform_mtls_flag_for_image "${candidate_state[3]}")"'
    running = 'running_platform_fastcgi="$(oteryn_platform_fastcgi_flag_of "$("${compose[@]}" ps -q platform)")"'
    off = '[[ "$running_platform_fastcgi" == true ]] || candidate_mtls=false'
    pin_call = 'oteryn_pin_platform_mtls_flag "$candidate_env" "$candidate_mtls"'
    for needle in (running, off):
        assert needle in body, needle
    assert body.index(decide) < body.index(running) < body.index(off) < body.index(pin_call)
    assert body.index(pin_call) < body.index("oteryn_native_admission_ingress_stop")


# Every route that replaces Platform behind the native admission gates stops ingress through
# lib.sh oteryn_native_admission_ingress_stop; that one helper also stops the Game-node mTLS
# listener. The list below is every call site: a new one must be added here together with the way
# it returns the listener.
INGRESS_STOP_CALLERS = {
    "deploy-impact.sh": 1,  # _oteryn_platform_reconcile
    "deploy.sh": 3,  # fast path, previous-candidate finalization, full path
    "generate-native-admission-key.sh": 1,  # key promotion
    "lib.sh": 2,  # _oteryn_recreate_marketplace_runtime_gated, failure restore
    "marketplace-staging.sh": 2,  # enable, prepare-rollback
    "rollback.sh": 1,
}
LISTENER_RETURNS = (
    "oteryn_native_admission_ingress_start",
    "oteryn_native_admission_self_check_or_stop",
    "oteryn_reconcile_platform_mtls",
)

FAKE_DOCKER = """#!/usr/bin/env bash
printf '%s\\n' "$*" >> "$FAKE_DOCKER_LOG"
case "$*" in
    *' ps -q internal-proxy') echo proxy-1 ;;
    *' ps -q gateway') echo gateway-1 ;;
    *' ps -q internal-mtls') printf '%s\\n' "${FAKE_MTLS-mtls-1}" ;;
    *' ps -q platform') echo platform-1 ;;
    'inspect '*) echo "${FAKE_INSPECT-GAME_AUTH_NATIVE_ADMISSION_ENABLED=false}" ;;
esac
exit 0
"""


def _run_lib(script: str, **env: str) -> tuple[subprocess.CompletedProcess[str], list[str]]:
    with tempfile.TemporaryDirectory() as tmp:
        directory = pathlib.Path(tmp)
        (directory / "docker").write_text(FAKE_DOCKER)
        (directory / "docker").chmod(0o755)
        log = directory / "docker.log"
        result = subprocess.run(
            ["bash", "-c", 'source "$1"; ' + script, "bash", str(LIB)],
            text=True,
            capture_output=True,
            check=False,
            env={**os.environ, "PATH": f"{directory}:{os.environ['PATH']}", "FAKE_DOCKER_LOG": str(log), **env},
        )
        calls = log.read_text().splitlines() if log.exists() else []
    return result, calls


def test_every_ingress_stop_caller_returns_the_listener_only_after_its_gates() -> None:
    found: dict[str, int] = {}
    call = re.compile(r"^(?!\s*#).*\boteryn_native_admission_ingress_stop \"\$\{", re.M)
    for script in sorted(SCRIPTS.glob("*.sh")):
        text = script.read_text()
        for match in call.finditer(text):
            found[script.name] = found.get(script.name, 0) + 1
            rest = text[match.end():]
            # The caller's own scope: its function body or case branch, or the rest of a top-level script.
            ends = [i for i in (rest.find("\n}"), rest.find(";;")) if i >= 0]
            scope = rest[: min(ends)] if ends else rest
            scope = "\n".join(line for line in scope.splitlines() if not line.lstrip().startswith("#"))
            returns = [m.start() for name in LISTENER_RETURNS for m in re.finditer(re.escape(name), scope)]
            assert returns, f"{script.name}: ingress stop without a way back for the mTLS listener"
            # Once Platform is (re)created, the listener returns only behind the trust gate. (A route
            # may return ingress unchanged before touching Platform, as a refused key promotion does.)
            replaced = [m.start() for m in re.finditer(r"up -d (--no-deps --force-recreate )?platform\b", scope)]
            after = [r for r in returns if not replaced or r > min(replaced)]
            gate = scope.find("oteryn_publish_native_admission_trust_or_stop", min(replaced) if replaced else 0)
            assert after and 0 <= gate < min(after), f"{script.name}: the listener could return before the trust gate"
    assert found == INGRESS_STOP_CALLERS


def test_ingress_stop_takes_the_listener_down_and_every_way_back_returns_it() -> None:
    lib = LIB.read_text()
    body = re.search(r"^oteryn_native_admission_ingress_stop\(\) \{\n(.*?)^\}", lib, re.M | re.S).group(1)
    assert '"$@" stop internal-mtls' in body

    result, calls = _run_lib(
        'ids="$(oteryn_native_admission_ingress_stop docker compose)"; echo "$ids"; '
        "oteryn_native_admission_ingress_stop docker compose >/dev/null; "
        "oteryn_native_admission_ingress_start; oteryn_native_admission_ingress_start",
        OTERYN_PLATFORM_MTLS_ENABLED="true",
    )
    assert result.returncode == 0, result.stderr
    assert result.stdout.split() == ["proxy-1", "gateway-1", "mtls-1"]
    assert "compose stop internal-mtls" in calls
    # The direct stop kept the listener; the first start returns it, the second has nothing left.
    assert calls.count("start mtls-1") == 1 and calls[-1] == "start mtls-1"

    # Given ids (the printed list) and the kept id are started once together.
    result, calls = _run_lib(
        "oteryn_native_admission_ingress_stop docker compose >/dev/null; oteryn_native_admission_ingress_start proxy-1 gateway-1 mtls-1",
        OTERYN_PLATFORM_MTLS_ENABLED="true",
    )
    assert result.returncode == 0, result.stderr
    assert calls[-1] == "start proxy-1 gateway-1 mtls-1"

    # marketplace-staging.sh returns the proxy and Gateway by hand: the self-check gate returns the listener.
    result, calls = _run_lib(
        "oteryn_native_admission_ingress_stop docker compose >/dev/null; oteryn_native_admission_self_check_or_stop docker compose",
        OTERYN_PLATFORM_MTLS_ENABLED="true",
    )
    assert result.returncode == 0, result.stderr
    assert calls[-1] == "start mtls-1"

    # A reconcile replaces the listener, so nothing stale is started afterwards.
    result, calls = _run_lib(
        "oteryn_platform_mtls_fingerprint() { echo f; }; oteryn_native_admission_ingress_stop docker compose >/dev/null; "
        "oteryn_reconcile_platform_mtls docker compose; oteryn_native_admission_ingress_start",
        OTERYN_PLATFORM_MTLS_ENABLED="true",
    )
    assert result.returncode == 0, result.stderr
    assert calls[-1] == "compose up -d --no-deps --force-recreate internal-mtls"

    # The setting was turned off while a listener still runs (true -> false): every Platform-only
    # route removes it with the rest of the ingress stop and nothing brings it back. With no listener
    # the removal is a no-op.
    for present in ("mtls-1", ""):
        result, calls = _run_lib(
            "oteryn_native_admission_ingress_stop docker compose; oteryn_native_admission_self_check_or_stop docker compose; "
            "oteryn_native_admission_ingress_start proxy-1 gateway-1",
            OTERYN_PLATFORM_MTLS_ENABLED="false",
            FAKE_MTLS=present,
        )
        assert result.returncode == 0, result.stderr
        assert result.stdout.split() == ["proxy-1", "gateway-1"]
        assert "compose rm -s -f internal-mtls" in calls
        assert calls.index("compose rm -s -f internal-mtls") < calls.index("compose ps -q platform")
        assert [c for c in calls if c.startswith("start")] == ["start proxy-1 gateway-1"]


def test_preflight_validates_the_server_certificate_before_any_ingress_mutation() -> None:
    lib = LIB.read_text()
    preflight = re.search(r"^oteryn_platform_mtls_preflight\(\) \{\n(.*?)^\}", lib, re.M | re.S).group(1)
    assert preflight.index("oteryn_platform_mtls_certificate_valid || return 1") < preflight.index("--entrypoint nginx")
    body = re.search(r"^oteryn_platform_mtls_certificate_valid\(\) \{\n(.*?)^\}", lib, re.M | re.S).group(1)
    assert "OTERYN_PLATFORM_MTLS_SERVER_NAME='platform-internal.preprod.oteryn.internal'" in lib
    assert "--network none" in body and body.count(",readonly\"") == 3
    # The client side trusts only the mounted CA and checks the name, so the chain, the validity
    # period, the SAN and the server-auth purpose are all verified; the server side loads the key.
    assert 'ssl.create_default_context(cafile="/m/ca.crt")' in body
    assert "server_hostname=name" in body
    assert 'server.load_cert_chain("/m/server-fullchain.pem", "/m/server.key"' in body
    assert "ssl.TLSVersion.TLSv1_3" in body and 'tls_client.version() != "TLSv1.3"' in body
    # deploy.sh preflights before the previous-candidate finalization and the full path stop ingress;
    # rollback.sh before its ingress stop.
    deploy = DEPLOY.read_text()
    preflight_call = 'oteryn_platform_mtls_preflight "${compose[@]}" || exit 1'
    assert deploy.index(preflight_call) < deploy.index("\nfinalize_previous_candidate_if_healthy\n")
    rollback = ROLLBACK.read_text()
    assert rollback.index(preflight_call) < rollback.index("oteryn_native_admission_ingress_stop")


ROUTE_SETTINGS = {
    # Platform setting: (native-defaults.env name, fail-closed default)
    "GAME_AUTH_NATIVE_EVIDENCE_ACTIVATED": ("OTERYN_STAGING_NATIVE_EVIDENCE_ACTIVATED", "false"),
    "GAME_AUTH_NATIVE_EVIDENCE_MTLS_CLIENT_IDENTITY": ("OTERYN_STAGING_NATIVE_EVIDENCE_MTLS_CLIENT_IDENTITY", ""),
    "GAME_AUTH_NATIVE_EVIDENCE_FRESH_ACCOUNT_PURPOSE": ("OTERYN_STAGING_NATIVE_EVIDENCE_FRESH_ACCOUNT_PURPOSE", ""),
    "GAME_AUTH_NATIVE_EVIDENCE_FRESH_ACCOUNT_SCOPE": ("OTERYN_STAGING_NATIVE_EVIDENCE_FRESH_ACCOUNT_SCOPE", ""),
    "GAME_AUTH_NATIVE_EVIDENCE_CLOCK_UNCERTAINTY_SECONDS": ("OTERYN_STAGING_NATIVE_EVIDENCE_CLOCK_UNCERTAINTY_SECONDS", ""),
    "GAME_AUTH_NATIVE_EVIDENCE_SOURCE_AUTHORITY": ("OTERYN_STAGING_NATIVE_EVIDENCE_SOURCE_AUTHORITY", "platform"),
    "GAME_AUTH_NATIVE_EVIDENCE_REQUESTS_PER_MINUTE": ("OTERYN_STAGING_NATIVE_EVIDENCE_REQUESTS_PER_MINUTE", "120"),
    "GAME_AUTH_CHARACTER_BOOTSTRAP_INTENT_MTLS_CLIENT_IDENTITY": (
        "OTERYN_STAGING_NATIVE_BOOTSTRAP_INTENT_MTLS_CLIENT_IDENTITY",
        "",
    ),
    "GAME_AUTH_CHARACTER_BOOTSTRAP_INTENT_TTL_SECONDS": ("OTERYN_STAGING_NATIVE_BOOTSTRAP_INTENT_TTL_SECONDS", ""),
    "CHARACTER_AUTHORITY_COMMANDS_ENABLED": ("OTERYN_STAGING_NATIVE_CHARACTER_COMMANDS_ENABLED", "false"),
    "CHARACTER_AUTHORITY_COMMAND_IDENTITIES": ("OTERYN_STAGING_NATIVE_CHARACTER_COMMAND_IDENTITIES", ""),
    "CHARACTER_AUTHORITY_COMMAND_SOURCE_AUTHORITY": ("OTERYN_STAGING_NATIVE_CHARACTER_COMMAND_SOURCE_AUTHORITY", ""),
    "CHARACTER_AUTHORITY_COMMAND_MAX_PENDING_INTENTS": ("OTERYN_STAGING_NATIVE_CHARACTER_COMMAND_MAX_PENDING_INTENTS", "40"),
    "CHARACTER_AUTHORITY_WEB_CREATE_ENABLED": ("OTERYN_STAGING_NATIVE_CHARACTER_WEB_CREATE_ENABLED", "false"),
}
# Platform settings of the Character Authority commands that the deploy leaves at the config default.
CHARACTER_COMMAND_UNMAPPED = {
    "CHARACTER_AUTHORITY_COMMAND_INTENT_TTL_SECONDS",
    "CHARACTER_AUTHORITY_COMMAND_REQUESTS_PER_MINUTE",
}
# Subject DN (RFC 2253) of the preprod node 1 [platform] client certificate, named by the Game manifest
# deploy/synology-game/preprod-topology.toml (certificates.node_native_evidence).
NODE_PLATFORM_CLIENT_IDENTITY = "CN=oteryn-preprod-node-1-native-evidence"
PRINT_ROUTE_SETTINGS = "; ".join(f'printf "%s=%s\\n" {name} "${name}"' for name in ROUTE_SETTINGS)


def test_game_node_route_settings_reach_platform_and_default_closed() -> None:
    # Every setting Platform reads for the native-evidence and bootstrap-intent routes is mapped into
    # the platform service (an unmapped one is always unset there, so the route always answers 503).
    config = (ROOT / "config" / "game-auth.php").read_text()
    read = set(re.findall(r"env\('((?:GAME_AUTH_NATIVE_EVIDENCE|GAME_AUTH_CHARACTER_BOOTSTRAP_INTENT)_[A-Z_]+)'", config))
    platform = _service_block(COMPOSE.read_text(), "platform")
    mapped = set(re.findall(r"^      ((?:GAME_AUTH_NATIVE_EVIDENCE|GAME_AUTH_CHARACTER_BOOTSTRAP_INTENT)_[A-Z_]+):", platform, re.M))
    assert read and read == mapped, read ^ mapped
    # Character Authority commands and web creation: every setting is mapped except the two that keep
    # the Platform config default (TTL 300 s, 600 requests per minute).
    config = (ROOT / "config" / "character-authority.php").read_text()
    read = set(re.findall(r"env\('(CHARACTER_AUTHORITY_[A-Z_]+)'", config))
    mapped = set(re.findall(r"^      (CHARACTER_AUTHORITY_[A-Z_]+):", platform, re.M))
    assert mapped and read == mapped | CHARACTER_COMMAND_UNMAPPED and not mapped & CHARACTER_COMMAND_UNMAPPED, read ^ mapped
    for name, (_, default) in ROUTE_SETTINGS.items():
        assert f"      {name}: ${{{name}:-{default}}}\n" in platform, name
    # The other Game-node routes: runtime status and scope assignment come from the rendered
    # environment; account characters (and its watermark) are pinned on with the Character Authority
    # projection identity, and the D171 fallback is pinned off.
    for name in ("RUNTIME_STATUS_ENABLED", "RUNTIME_STATUS_IDENTITIES", "SCOPE_ASSIGNMENT_ENABLED", "SCOPE_ASSIGNMENT_IDENTITIES"):
        assert re.search(rf"^      GAME_AUTH_NATIVE_{name}: \$\{{GAME_AUTH_NATIVE_{name}:-(false|null)\}}$", platform, re.M), name
    assert '      GAME_AUTH_NATIVE_ACCOUNT_CHARACTERS_ENABLED: "true"\n' in platform
    assert "      GAME_AUTH_NATIVE_ACCOUNT_CHARACTERS_SOURCE_AUTHORITY: oteryn:character-authority:primary\n" in platform
    assert """      GAME_AUTH_NATIVE_ACCOUNT_CHARACTERS_IDENTITIES: '["CN=oteryn-preprod-character-projection"]'\n""" in platform
    assert '      GAME_AUTH_NATIVE_ADMISSION_UNVERIFIED_CHARACTER_OWNERSHIP: "false"\n' in platform

    # Deploy inputs: the committed defaults name the preprod node 1 [platform] client certificate for
    # both routes (the Game node presents one certificate for evidence and bootstrap-intent reads) and
    # keep the one-way evidence activation off. Anything not committed resolves to its fail-closed value.
    values = dict(line.split("=", 1) for line in DEFAULTS.read_text().splitlines() if line and not line.startswith("#"))
    assert values["OTERYN_STAGING_NATIVE_EVIDENCE_ACTIVATED"] == "false"
    committed = {
        "OTERYN_STAGING_NATIVE_EVIDENCE_ACTIVATED": "false",
        "OTERYN_STAGING_NATIVE_EVIDENCE_MTLS_CLIENT_IDENTITY": NODE_PLATFORM_CLIENT_IDENTITY,
        "OTERYN_STAGING_NATIVE_EVIDENCE_FRESH_ACCOUNT_PURPOSE": "platform_security",
        "OTERYN_STAGING_NATIVE_EVIDENCE_FRESH_ACCOUNT_SCOPE": "fresh_admission",
        "OTERYN_STAGING_NATIVE_EVIDENCE_CLOCK_UNCERTAINTY_SECONDS": "1",
        "OTERYN_STAGING_NATIVE_BOOTSTRAP_INTENT_MTLS_CLIENT_IDENTITY": NODE_PLATFORM_CLIENT_IDENTITY,
        "OTERYN_STAGING_NATIVE_BOOTSTRAP_INTENT_TTL_SECONDS": "300",
        # Character Authority commands and web creation stay off until the runbook's enablement change;
        # the pending-intent bound is the enablement value 40 (CP decision 2026-10-15).
        "OTERYN_STAGING_NATIVE_CHARACTER_COMMANDS_ENABLED": "false",
        "OTERYN_STAGING_NATIVE_CHARACTER_COMMAND_SOURCE_AUTHORITY": "oteryn:character-authority:primary",
        "OTERYN_STAGING_NATIVE_CHARACTER_COMMAND_MAX_PENDING_INTENTS": "40",
        "OTERYN_STAGING_NATIVE_CHARACTER_WEB_CREATE_ENABLED": "false",
    }
    for _, (default_name, _) in ROUTE_SETTINGS.items():
        assert values.get(default_name, "") == committed.get(default_name, ""), default_name
    # The shared identity stays distinct from every other Game-node route identity Platform pins.
    other_identities = re.findall(r"CN=[A-Za-z0-9.-]+", values["OTERYN_STAGING_NATIVE_RUNTIME_STATUS_IDENTITIES"])
    other_identities += re.findall(r"CN=[A-Za-z0-9.-]+", values["OTERYN_STAGING_NATIVE_SCOPE_ASSIGNMENT_IDENTITIES"])
    other_identities += re.findall(r"CN=[A-Za-z0-9.-]+", platform)
    assert other_identities and NODE_PLATFORM_CLIENT_IDENTITY not in other_identities
    result, _ = _run_lib(f"oteryn_resolve_game_node_route_settings || exit 1; {PRINT_ROUTE_SETTINGS}")
    assert result.returncode == 0, result.stderr
    assert result.stdout.splitlines() == [
        f"{name}={committed.get(default_name) or default}" for name, (default_name, default) in ROUTE_SETTINGS.items()
    ]
    assert "oteryn_resolve_game_node_route_settings\n}" in LIB.read_text()  # load_oteryn_env_file runs it

    # A committed value reaches Platform; the environment file (or caller) wins over it.
    with tempfile.TemporaryDirectory() as tmp:
        defaults = pathlib.Path(tmp) / "d.env"
        defaults.write_text(
            "OTERYN_STAGING_NATIVE_EVIDENCE_ACTIVATED=true\n"
            "OTERYN_STAGING_NATIVE_EVIDENCE_MTLS_CLIENT_IDENTITY=CN=evidence-node\n"
            "OTERYN_STAGING_NATIVE_BOOTSTRAP_INTENT_TTL_SECONDS=60\n"
        )
        result, _ = _run_lib(
            f"oteryn_resolve_game_node_route_settings || exit 1; {PRINT_ROUTE_SETTINGS}",
            OTERYN_NATIVE_DEFAULTS_FILE=str(defaults),
            GAME_AUTH_CHARACTER_BOOTSTRAP_INTENT_MTLS_CLIENT_IDENTITY="CN=bootstrap-node",
        )
    assert result.returncode == 0, result.stderr
    resolved = dict(line.split("=", 1) for line in result.stdout.splitlines())
    assert resolved["GAME_AUTH_NATIVE_EVIDENCE_ACTIVATED"] == "true"
    assert resolved["GAME_AUTH_NATIVE_EVIDENCE_MTLS_CLIENT_IDENTITY"] == "CN=evidence-node"
    assert resolved["GAME_AUTH_CHARACTER_BOOTSTRAP_INTENT_TTL_SECONDS"] == "60"
    assert resolved["GAME_AUTH_CHARACTER_BOOTSTRAP_INTENT_MTLS_CLIENT_IDENTITY"] == "CN=bootstrap-node"
    assert resolved["GAME_AUTH_NATIVE_EVIDENCE_REQUESTS_PER_MINUTE"] == "120"

    # Platform treats a non-boolean switch as a configuration error, so the run is refused.
    result, calls = _run_lib("oteryn_resolve_game_node_route_settings", GAME_AUTH_NATIVE_EVIDENCE_ACTIVATED="yes")
    assert result.returncode == 1 and "must be true or false" in result.stderr and calls == []


def test_character_command_settings_refuse_a_surface_that_cannot_work() -> None:
    enabled = {
        "CHARACTER_AUTHORITY_COMMANDS_ENABLED": "true",
        "CHARACTER_AUTHORITY_COMMAND_IDENTITIES": '["CN=oteryn-preprod-character-commands"]',
    }
    # The committed source authority and bound complete an enabled surface; web creation follows it.
    result, calls = _run_lib(
        f"oteryn_resolve_game_node_route_settings || exit 1; {PRINT_ROUTE_SETTINGS}",
        CHARACTER_AUTHORITY_WEB_CREATE_ENABLED="true",
        **enabled,
    )
    assert result.returncode == 0 and calls == [], result.stderr
    resolved = dict(line.split("=", 1) for line in result.stdout.splitlines())
    assert resolved["CHARACTER_AUTHORITY_COMMANDS_ENABLED"] == "true"
    assert resolved["CHARACTER_AUTHORITY_COMMAND_IDENTITIES"] == '["CN=oteryn-preprod-character-commands"]'
    assert resolved["CHARACTER_AUTHORITY_COMMAND_SOURCE_AUTHORITY"] == "oteryn:character-authority:primary"
    assert resolved["CHARACTER_AUTHORITY_COMMAND_MAX_PENDING_INTENTS"] == "40"
    assert resolved["CHARACTER_AUTHORITY_WEB_CREATE_ENABLED"] == "true"

    refused = [
        ({"CHARACTER_AUTHORITY_COMMANDS_ENABLED": "1"}, "CHARACTER_AUTHORITY_COMMANDS_ENABLED must be true or false"),
        ({"CHARACTER_AUTHORITY_WEB_CREATE_ENABLED": "on"}, "CHARACTER_AUTHORITY_WEB_CREATE_ENABLED must be true or false"),
        ({"CHARACTER_AUTHORITY_WEB_CREATE_ENABLED": "true"}, "needs CHARACTER_AUTHORITY_COMMANDS_ENABLED=true"),
        ({"CHARACTER_AUTHORITY_COMMANDS_ENABLED": "true"}, "needs CHARACTER_AUTHORITY_COMMAND_IDENTITIES"),
        ({"CHARACTER_AUTHORITY_COMMAND_MAX_PENDING_INTENTS": "257"}, "from 1 to 256"),
        ({"CHARACTER_AUTHORITY_COMMAND_MAX_PENDING_INTENTS": "0"}, "from 1 to 256"),
        ({"CHARACTER_AUTHORITY_COMMAND_MAX_PENDING_INTENTS": "40s"}, "from 1 to 256"),
    ]
    for env, message in refused:
        result, calls = _run_lib("oteryn_resolve_game_node_route_settings", **env)
        assert result.returncode == 1 and message in result.stderr and calls == [], (env, result.stderr)
    # An enabled surface without a source authority (none committed) is refused too.
    with tempfile.TemporaryDirectory() as tmp:
        defaults = pathlib.Path(tmp) / "d.env"
        defaults.write_text("OTERYN_STAGING_NATIVE_CHARACTER_COMMAND_MAX_PENDING_INTENTS=256\n")
        result, calls = _run_lib("oteryn_resolve_game_node_route_settings", OTERYN_NATIVE_DEFAULTS_FILE=str(defaults), **enabled)
    assert result.returncode == 1 and "CHARACTER_AUTHORITY_COMMAND_SOURCE_AUTHORITY" in result.stderr and calls == []


def test_every_fail_closed_removal_also_removes_the_listener() -> None:
    lib = LIB.read_text()
    remove = re.compile(r"^(\s*)(.*) rm -s -f gateway internal-proxy platform\b.*$", re.M)
    found = 0
    for match in remove.finditer(lib):
        found += 1
        before = lib[: match.start()].rstrip("\n").splitlines()[-1]
        assert before.strip().startswith(f"oteryn_platform_mtls_remove_fail_closed {match.group(2)}"), before
    assert found == 4
    helper = re.search(r"^oteryn_platform_mtls_remove_fail_closed\(\) \{\n(.*?)^\}", lib, re.M | re.S).group(1)
    assert '"$@" rm -s -f internal-mtls' in helper and "OTERYN_PLATFORM_MTLS_STOPPED_ID=''" in helper

    removal = ["compose rm -s -f internal-mtls", "compose rm -s -f gateway internal-proxy platform"]
    gates = {
        "trust gate": ("oteryn_publish_native_admission_trust() { return 1; }; ", "oteryn_publish_native_admission_trust_or_stop", {}),
        "self-check gate": (
            "oteryn_run_native_admission_self_check() { return 1; }; ",
            "oteryn_native_admission_self_check_or_stop",
            {"FAKE_INSPECT": "GAME_AUTH_NATIVE_ADMISSION_ENABLED=true"},
        ),
    }
    for label, (failing, gate, env) in gates.items():
        # The ingress stop only stopped the listener; the failed gate removes it, and nothing starts it.
        result, calls = _run_lib(
            f"{failing}oteryn_native_admission_ingress_stop docker compose >/dev/null; "
            f"! {gate} docker compose || exit 9; oteryn_native_admission_ingress_start",
            OTERYN_PLATFORM_MTLS_ENABLED="true",
            **env,
        )
        assert result.returncode == 0, (label, result.stderr)
        assert "compose stop internal-mtls" in calls, label
        assert calls[-2:] == removal, (label, calls)

    # The failure restore whose ingress stop failed.
    result, calls = _run_lib(
        "oteryn_native_admission_ingress_stop() { return 1; }; ! oteryn_restore_previous_platform_behind_trust_gate t || exit 9",
        ENV_FILE="e", COMPOSE_FILE="c",
    )
    assert result.returncode == 0, result.stderr
    assert calls == [f"compose --env-file e -f c {c.removeprefix('compose ')}" for c in removal]



def test_failure_restore_recreates_the_listener_a_failed_forward_gate_removed() -> None:
    # The forward trust gate failed: its fail-closed removal took internal-mtls away and dropped the
    # kept id. The EXIT-trap restore must bring the listener back for the restored Platform, not
    # report success with the Game node unreachable.
    script = (
        "oteryn_platform_mtls_fingerprint() { echo f; }; "
        "_oteryn_reconcile_marketplace_scheduler_after_runtime_change() { :; }; "
        "oteryn_publish_native_admission_trust() { return 1; }; "
        "oteryn_native_admission_ingress_stop docker compose >/dev/null; "
        "! oteryn_publish_native_admission_trust_or_stop docker compose || exit 9; "
        "oteryn_publish_native_admission_trust() { return 0; }; "
        "oteryn_restore_previous_platform_behind_trust_gate restore"
    )
    recreate = "compose --env-file e -f c up -d --no-deps --force-recreate internal-mtls"
    result, calls = _run_lib(script, OTERYN_PLATFORM_MTLS_ENABLED="true", ENV_FILE="e", COMPOSE_FILE="c")
    assert result.returncode == 0, result.stderr
    assert "compose rm -s -f internal-mtls" in calls
    assert recreate in calls
    assert calls.index(recreate) > calls.index("compose --env-file e -f c up -d --no-deps internal-proxy gateway")
    assert not [c for c in calls if c.startswith("start ")], calls

    # With the setting off the restore removes any listener and recreates none.
    result, calls = _run_lib(script, OTERYN_PLATFORM_MTLS_ENABLED="false", ENV_FILE="e", COMPOSE_FILE="c")
    assert result.returncode == 0, result.stderr
    assert recreate not in calls and calls[-1] == "compose --env-file e -f c rm -s -f internal-mtls"


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
