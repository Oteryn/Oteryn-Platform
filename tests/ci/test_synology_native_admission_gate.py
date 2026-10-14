from __future__ import annotations

import base64
import http.server
import json
import os
import pathlib
import re
import shutil
import socketserver
import subprocess
import threading

import pytest


ROOT = pathlib.Path(__file__).resolve().parents[2]
COMPOSE = ROOT / "deploy" / "synology" / "compose.yml"
LIB = ROOT / "deploy" / "synology" / "scripts" / "lib.sh"
HEALTH_CHECK = ROOT / "deploy" / "synology" / "scripts" / "health-check.sh"
README = ROOT / "deploy" / "synology" / "README.md"
DEPLOY = ROOT / "deploy" / "synology" / "scripts" / "deploy.sh"
KEY_SCRIPT = ROOT / "deploy" / "synology" / "scripts" / "generate-native-admission-key.sh"
ENTRYPOINT = ROOT / "deploy" / "synology" / "docker" / "platform-entrypoint.sh"
ENV_EXAMPLE = ROOT / "deploy" / "synology" / ".env.example"
PREFLIGHT = ROOT / "deploy" / "synology" / "scripts" / "production-target-preflight.sh"
WORKFLOWS = ROOT / ".github" / "workflows"
TICKETS = ROOT / "app" / "GameAuth" / "NativeLogin" / "NativeGameLoginTickets.php"

PROBE_MARKER = '"$expected_gateway_version" "$platform_native_admission" <<\'PY\'\n'
UUID7 = re.compile(r"^[0-9a-f]{8}-[0-9a-f]{4}-7[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$")
NO_STORE = {
    "Cache-Control": "no-store, no-cache, must-revalidate, private",
    "Pragma": "no-cache",
    "Expires": "0",
}


def test_gateway_native_login_is_the_platform_admission_flag() -> None:
    text = COMPOSE.read_text(encoding="utf-8")
    # On for the preproduction stack: deploy.sh and health-check.sh refuse unless the key volume,
    # key id and Platform signing self-check prove the issuer can sign (tests below).
    # Default true; rollback of an image that predates the self-check overrides it for that image only.
    assert 'x-native-admission-enabled: &native_admission_enabled "${OTERYN_NATIVE_ADMISSION_ENABLED:-false}"' in text
    for name in ("RUNTIME_STATUS_ENABLED", "RUNTIME_STATUS_IDENTITIES", "SCOPE_ASSIGNMENT_ENABLED", "SCOPE_ASSIGNMENT_IDENTITIES"):
        assert f"GAME_AUTH_NATIVE_{name}:" in text
    # LCFA is on with the Character Authority projection identity; the D171 fallback is off.
    assert 'GAME_AUTH_NATIVE_ACCOUNT_CHARACTERS_ENABLED: "true"' in text
    assert "GAME_AUTH_NATIVE_ACCOUNT_CHARACTERS_SOURCE_AUTHORITY: oteryn:character-authority:primary\n" in text
    assert """GAME_AUTH_NATIVE_ACCOUNT_CHARACTERS_IDENTITIES: '["CN=oteryn-preprod-character-projection"]'\n""" in text
    assert 'GAME_AUTH_NATIVE_ADMISSION_UNVERIFIED_CHARACTER_OWNERSHIP: "false"' in text
    assert "GAME_AUTH_NATIVE_ADMISSION_UNVERIFIED_CHARACTER_WORLD_ID: ${GAME_AUTH_NATIVE_ADMISSION_UNVERIFIED_CHARACTER_WORLD_ID:-}" in text
    assert "GAME_AUTH_NATIVE_ADMISSION_ENABLED: *native_admission_enabled" in text
    assert "GATEWAY_NATIVE_LOGIN_ENABLED: *native_admission_enabled" in text
    # No literal flag value may bypass the shared anchor.
    assert re.search(r"(GAME_AUTH_NATIVE_ADMISSION_ENABLED|GATEWAY_NATIVE_LOGIN_ENABLED):\s*[\"']?(true|false)", text) is None
    platform = text.index("\n  platform:\n")
    gateway = text.index("\n  gateway:\n")
    assert platform < text.index("GAME_AUTH_NATIVE_ADMISSION_ENABLED: *") < text.index("\n  tls-init:\n")
    assert gateway < text.index("GATEWAY_NATIVE_LOGIN_ENABLED: *")
    # The legacy login switch is untouched by this change.
    assert "GAME_AUTH_LEGACY_LOGIN_ENABLED" not in text


def test_signing_keys_come_from_an_external_read_only_volume_never_from_values() -> None:
    text = COMPOSE.read_text(encoding="utf-8")
    assert "- native_admission_keys:/run/oteryn-native-admission:ro" in text
    assert "native_admission_keys:\n    name: oteryn-native-admission-keys\n    external: true" in text
    for forbidden in ("SIGNING_KEY_FILE", "SIGNING_KEY_ID", "SIGNING_KEY:", "PRIVATE"):
        assert forbidden not in text
    entrypoint = ENTRYPOINT.read_text(encoding="utf-8")
    assert "/run/oteryn-native-admission" in entrypoint
    assert "GAME_AUTH_NATIVE_ADMISSION_SIGNING_KEY_FILE" in entrypoint
    # The entrypoint exports paths and key ids only; it never reads the key file.
    assert "current.key\"" in entrypoint and "cat " not in entrypoint


def test_synology_runs_as_preproduction_and_production_guards_stay_closed() -> None:
    assert "APP_ENV=preproduction" in ENV_EXAMPLE.read_text(encoding="utf-8")
    assert "APP_ENV=staging" not in ENV_EXAMPLE.read_text(encoding="utf-8")
    for name in (
        "deploy-synology-staging.yml",
        "recover-synology-staging-schema.yml",
        "character-bazaar-staging-control.yml",
    ):
        workflow = (WORKFLOWS / name).read_text(encoding="utf-8")
        assert "APP_ENV=preproduction" in workflow, name
        assert "APP_ENV=staging" not in workflow, name
    assert 'test "$APP_ENV" = preproduction' in PREFLIGHT.read_text(encoding="utf-8")
    # The issuer still signs only in testing or preproduction; production is not added anywhere.
    issuer = TICKETS.read_text(encoding="utf-8")
    assert "app()->environment(['testing', 'preproduction'])" in issuer
    assert "'production'" not in issuer


def run_pairing(platform: str, gateway: str) -> subprocess.CompletedProcess[str]:
    return subprocess.run(
        ["bash", "-c", 'source "$1"; oteryn_assert_native_login_pairing "$2" "$3"', "bash", str(LIB), platform, gateway],
        text=True,
        capture_output=True,
        check=False,
    )


def test_pairing_accepts_both_off() -> None:
    assert run_pairing("false", "false").returncode == 0


def test_pairing_refuses_gateway_on_without_platform() -> None:
    result = run_pairing("false", "true")
    assert result.returncode != 0
    assert "does not match Platform native admission" in result.stderr


def test_pairing_refuses_missing_platform_flag() -> None:
    result = run_pairing("", "true")
    assert result.returncode != 0
    assert "missing or invalid" in result.stderr


def test_pairing_accepts_both_on_and_still_refuses_a_mismatch() -> None:
    # Both on is allowed: the signing self-check, not the pairing, proves the issuer can sign.
    assert run_pairing("true", "true").returncode == 0
    mismatch = run_pairing("true", "false")
    assert mismatch.returncode != 0
    assert "does not match" in mismatch.stderr


def test_health_check_runs_the_signing_self_check_when_admission_is_on() -> None:
    script = HEALTH_CHECK.read_text(encoding="utf-8")
    on = script.index('if [[ "$platform_native_admission" == true ]]; then')
    check = script.index('oteryn_run_native_admission_self_check "${container_ids[platform]}" || exit 1')
    assert script.index('oteryn_assert_native_login_pairing "$platform_native_admission"') < on < check
    assert check < script.index(PROBE_MARKER)


def test_health_check_gates_admission_on_a_live_account_character_feed() -> None:
    script = HEALTH_CHECK.read_text(encoding="utf-8")
    check = script.index('oteryn_run_native_admission_self_check "${container_ids[platform]}" || exit 1')
    feed = script.index('oteryn_wait_native_account_characters_feed "${container_ids[platform]}" || exit 1')
    assert 'container_env_value platform GAME_AUTH_NATIVE_ACCOUNT_CHARACTERS_ENABLED' in script
    assert check < feed < script.index(PROBE_MARKER)


def _run_feed_gate(tmp_path: pathlib.Path, evidence: str, wait: str = "0") -> subprocess.CompletedProcess[str]:
    fake = tmp_path / "docker"
    fake.write_text(f"#!/bin/sh\nprintf '%s' '{evidence}'\n", encoding="utf-8")
    fake.chmod(0o755)
    return subprocess.run(
        ["bash", "-c", f'source "{LIB}"; oteryn_wait_native_account_characters_feed cid'],
        env={"PATH": f"{tmp_path}:/usr/bin:/bin", "OTERYN_NATIVE_ACCOUNT_CHARACTERS_FEED_WAIT_SECONDS": wait},
        capture_output=True, text=True, check=False,
    )


def test_account_character_feed_gate_passes_only_on_a_live_feed(tmp_path: pathlib.Path) -> None:
    live = _run_feed_gate(tmp_path, "live")
    assert live.returncode == 0, live.stderr
    for evidence in ("stale", "unavailable", ""):
        refused = _run_feed_gate(tmp_path, evidence)
        assert refused.returncode == 1 and "no fresh watermark" in refused.stderr, evidence
    invalid = _run_feed_gate(tmp_path, "live", wait="601")
    assert invalid.returncode == 1 and "FEED_WAIT_SECONDS" in invalid.stderr


def test_deploy_ensures_the_key_before_changes_and_self_checks_before_the_gateway() -> None:
    script = DEPLOY.read_text(encoding="utf-8")
    ensure = script.index("oteryn_ensure_native_admission_key || exit 1")
    assert ensure < script.index("fast_candidate_rc=0")
    assert ensure < script.index('"${compose[@]}" up -d --force-recreate tls-init')
    self_check = script.index('oteryn_run_native_admission_self_check "$("${compose[@]}" ps -q platform)" || exit 1')
    assert script.index("game-auth:oauth-client:ensure") < self_check < script.index('"${compose[@]}" up -d gateway')
    # The fast path recreates Platform only, so it re-proves signing before it can finish.
    fast_fn = script.split("_oteryn_fast_deploy_platform() {")[1]
    assert fast_fn.index("oteryn_run_native_admission_self_check") < fast_fn.index("oteryn_native_admission_ingress_start")


def test_rollback_and_candidate_recovery_gate_the_flag_on_the_image() -> None:
    rollback = (ROOT / "deploy" / "synology" / "scripts" / "rollback.sh").read_text(encoding="utf-8")
    assert 'oteryn_native_admission_flag_for_image "$PLATFORM_IMAGE"' in rollback
    assert "oteryn_native_admission_flag_for_image" in DEPLOY.read_text(encoding="utf-8")
    assert "game-auth:native-admission:self-check" in LIB.read_text(encoding="utf-8").split("oteryn_native_admission_flag_for_image()")[1]


def test_deploy_does_not_require_the_native_world_uuid_and_workflow_still_renders_the_optional_input() -> None:
    script = DEPLOY.read_text(encoding="utf-8")
    assert script.index("oteryn_native_admission_projection_ready || exit 1") < script.index("oteryn_ensure_native_admission_key || exit 1")
    ready = LIB.read_text(encoding="utf-8").split("oteryn_native_admission_projection_ready()")[1].split("\n}\n")[0]
    assert "UNVERIFIED_CHARACTER_WORLD_ID" not in ready
    identities = '{"CN=x":["w/c"]}'
    env = {
        "PATH": "/usr/bin:/bin",
        "GAME_AUTH_NATIVE_RUNTIME_STATUS_ENABLED": "true",
        "GAME_AUTH_NATIVE_RUNTIME_STATUS_IDENTITIES": identities,
        "GAME_AUTH_NATIVE_SCOPE_ASSIGNMENT_ENABLED": "true",
        "GAME_AUTH_NATIVE_SCOPE_ASSIGNMENT_IDENTITIES": identities,
    }
    ok = subprocess.run(["bash", "-c", f'source "{LIB}"; oteryn_native_admission_projection_ready'], env=env, capture_output=True, text=True, check=False)
    assert ok.returncode == 0, ok.stderr
    env.pop("GAME_AUTH_NATIVE_SCOPE_ASSIGNMENT_IDENTITIES")
    bad = subprocess.run(["bash", "-c", f'source "{LIB}"; oteryn_native_admission_projection_ready'], env=env, capture_output=True, text=True, check=False)
    assert bad.returncode == 1 and "SCOPE_ASSIGNMENT_IDENTITIES" in bad.stderr
    workflow = (WORKFLOWS / "deploy-synology-staging.yml").read_text(encoding="utf-8")
    assert "GAME_AUTH_NATIVE_ADMISSION_UNVERIFIED_CHARACTER_WORLD_ID=$NATIVE_WORLD_ID_INPUT" in workflow


def test_key_rotation_restarts_platform_so_key_ids_are_reloaded() -> None:
    script = KEY_SCRIPT.read_text(encoding="utf-8")
    assert 'docker restart "$container"' in script
    assert '[[ "$ACTION" == rotate || "$ACTION" == drop-retiring ]]' in script


def test_self_check_command_name_is_the_merged_artisan_command() -> None:
    lib = LIB.read_text(encoding="utf-8")
    command = (ROOT / "app" / "Console" / "Commands" / "VerifyNativeAdmissionSigning.php").read_text(encoding="utf-8")
    assert "game-auth:native-admission:self-check" in lib
    assert "protected $signature = 'game-auth:native-admission:self-check';" in command


def test_key_script_keeps_the_private_key_off_every_output_channel() -> None:
    script = KEY_SCRIPT.read_text(encoding="utf-8")
    assert script.startswith("#!/usr/bin/env bash\nset -euo pipefail") or "\nset -euo pipefail\n" in script
    assert "--network none" in script and "--read-only" in script
    # Only these lines are ever echoed; the seed variable is never passed to echo/fwrite STDOUT.
    assert "echo 'current_public_key='" in script and "echo 'current_key_id='" in script
    assert "0600" in script and "0700" in script
    echoed = re.findall(r"echo [^\n]+", script.split("<<'PHP'")[1].split("\nPHP\n")[0])
    assert all("seed" not in line and "secret" not in line.lower() for line in echoed)
    assert "docker run" in script and "--volume \"$VOLUME_NAME:/keys\"" in script


def test_health_check_asserts_pairing_before_probes() -> None:
    script = HEALTH_CHECK.read_text(encoding="utf-8")
    pairing = script.index('oteryn_assert_native_login_pairing "$platform_native_admission"')
    probes = script.index(PROBE_MARKER)
    assert pairing < probes
    assert "body=b'{}'" in script


def probe_source() -> str:
    script = HEALTH_CHECK.read_text(encoding="utf-8")
    start = script.index(PROBE_MARKER) + len(PROBE_MARKER)
    end = script.index("\nPY\n", start)
    return script[start:end]


class _Server(socketserver.ThreadingMixIn, http.server.HTTPServer):
    daemon_threads = True


def serve(handler: type[http.server.BaseHTTPRequestHandler]) -> tuple[_Server, int]:
    server = _Server(("127.0.0.1", 0), handler)
    threading.Thread(target=server.serve_forever, daemon=True).start()
    return server, server.server_address[1]


def gateway_handler(native_answer, seen: list[dict]):
    class Handler(http.server.BaseHTTPRequestHandler):
        def log_message(self, *args) -> None:
            return

        def reply(self, status: int, payload: object, extra: dict[str, str] | None = None) -> None:
            data = json.dumps(payload).encode()
            self.send_response(status)
            self.send_header("Content-Type", "application/json")
            for key, value in (extra or {}).items():
                self.send_header(key, value)
            self.send_header("Content-Length", str(len(data)))
            self.end_headers()
            self.wfile.write(data)

        def do_GET(self) -> None:
            if self.path == "/version":
                self.reply(200, {"service": "oteryn-game-gateway", "version": "synology-staging"})
            else:
                self.reply(404, {"error": "not_found"})

        def do_POST(self) -> None:
            body = json.loads(self.rfile.read(int(self.headers["Content-Length"])))
            if body.get("protocol_version") == 2:
                seen.append(body)
                status, payload = native_answer(body)
                self.reply(status, payload, NO_STORE)
            else:
                self.reply(400, {"error": "invalid_request"}, NO_STORE)

    return Handler


class PlatformHandler(http.server.BaseHTTPRequestHandler):
    def log_message(self, *args) -> None:
        return

    def do_GET(self) -> None:
        data = b'{"service":"oteryn-platform"}'
        self.send_response(200)
        self.send_header("Content-Length", str(len(data)))
        self.end_headers()
        self.wfile.write(data)


def run_probe(native_answer, native_on: str = "false") -> tuple[subprocess.CompletedProcess[str], list[dict]]:
    seen: list[dict] = []
    gateway, gateway_port = serve(gateway_handler(native_answer, seen))
    platform, platform_port = serve(PlatformHandler)
    try:
        result = subprocess.run(
            ["python3", "-I", "-", str(platform_port), str(gateway_port), "synology-staging", native_on],
            input=probe_source(),
            text=True,
            capture_output=True,
            check=False,
            timeout=60,
        )
    finally:
        gateway.shutdown()
        platform.shutdown()
    return result, seen


def refusal(code: str):
    return lambda body: (
        {"NATIVE_LOGIN_UNAVAILABLE": 503, "NATIVE_LOGIN_AUTHENTICATION_REQUIRED": 401}[code],
        {"protocol_version": 2, "error": {"code": code, "public_class": "X"}, "attempt_ref": body["attempt_ref"]},
    )


def test_probe_sends_well_formed_native_login_and_accepts_closed_gateway() -> None:
    result, seen = run_probe(lambda body: (400, {"error": "invalid_request"}))
    assert result.returncode == 0, result.stderr
    assert len(seen) == 1
    request = seen[0]
    assert set(request) == {"protocol_version", "game_login_ticket", "attempt_ref", "character_id", "channel_id", "offer"}
    assert UUID7.match(request["attempt_ref"]) and UUID7.match(request["character_id"])
    assert request["channel_id"] is None
    assert request["offer"]["client_platform"] == "windows"
    assert request["offer"]["transports"] == [{"protocol_major": 1, "transport_profile": 1, "alpn": "oteryn-game/1"}]


def test_probe_fails_when_gateway_routes_native_login_to_the_issuer() -> None:
    for code in ("NATIVE_LOGIN_UNAVAILABLE", "NATIVE_LOGIN_AUTHENTICATION_REQUIRED"):
        result, _ = run_probe(refusal(code))
        assert result.returncode != 0
        assert "while native admission is off" in result.stderr


def test_probe_with_admission_on_requires_the_issuer_to_reject_the_unknown_ticket() -> None:
    result, seen = run_probe(refusal("NATIVE_LOGIN_AUTHENTICATION_REQUIRED"), "true")
    assert result.returncode == 0, result.stderr
    assert len(seen) == 1
    for answer in (refusal("NATIVE_LOGIN_UNAVAILABLE"), lambda body: (400, {"error": "invalid_request"})):
        failed, _ = run_probe(answer, "true")
        assert failed.returncode != 0
        assert "through the Platform issuer" in failed.stderr


def test_readme_documents_native_admission_keys_by_name() -> None:
    readme = README.read_text(encoding="utf-8")
    for key in (
        "GAME_AUTH_NATIVE_ADMISSION_ENABLED",
        "GATEWAY_NATIVE_LOGIN_ENABLED",
        "GAME_AUTH_NATIVE_ADMISSION_SIGNING_KEY_FILE",
        "GAME_AUTH_NATIVE_ADMISSION_SIGNING_KEY_ID",
        "generate-native-admission-key.sh",
        "oteryn-native-admission-keys",
        "APP_ENV=preproduction",
    ):
        assert key in readme


def test_native_admission_is_opt_in_and_needs_runtime_identities_and_key_export() -> None:
    deploy = DEPLOY.read_text(encoding="utf-8")
    flag = LIB.read_text(encoding="utf-8").split("oteryn_native_admission_flag_for_image()")[1]
    assert 'OTERYN_NATIVE_ADMISSION_ENABLED:-false}" == true' in deploy
    for name in ("RUNTIME_STATUS", "SCOPE_ASSIGNMENT"):
        assert f"GAME_AUTH_NATIVE_{name}_IDENTITIES" in LIB.read_text(encoding="utf-8")
    assert deploy.count("oteryn_run_native_admission_self_check") >= 2
    assert "load_native_admission_keys" in flag
    assert "game-auth:native-admission:self-check" in flag
    workflow = (WORKFLOWS / "deploy-synology-staging.yml").read_text(encoding="utf-8")
    assert "OTERYN_STAGING_NATIVE_ADMISSION_ENABLED" in workflow
    assert "OTERYN_STAGING_NATIVE_RUNTIME_STATUS_IDENTITIES" in workflow
    assert "OTERYN_STAGING_NATIVE_SCOPE_ASSIGNMENT_IDENTITIES" in workflow


def test_fast_path_is_rejected_when_the_native_admission_flag_changed() -> None:
    deploy = DEPLOY.read_text(encoding="utf-8")
    fast = deploy.split("_oteryn_fast_candidate() {")[1].split("_oteryn_fast_restore_on_exit()")[0]
    assert "GATEWAY_NATIVE_LOGIN_ENABLED=" in fast
    assert "GAME_AUTH_NATIVE_ADMISSION_ENABLED=" in fast
    assert "native admission flag changed" in fast


def test_reconcile_rejects_a_changed_flag_and_rotate_needs_maintenance_while_live() -> None:
    impact = (ROOT / "deploy" / "synology" / "scripts" / "deploy-impact.sh").read_text(encoding="utf-8")
    cand = impact.split("_oteryn_platform_reconcile_candidate() {")[1].split("\n}\n")[0]
    assert "native admission flag changed" in cand
    assert "GATEWAY_NATIVE_LOGIN_ENABLED=" in cand and "GAME_AUTH_NATIVE_ADMISSION_ENABLED=" in cand
    key = KEY_SCRIPT.read_text(encoding="utf-8")
    assert '"$MAINTENANCE" != --maintenance' in key
    assert key.index("Refusing to rotate while native admission is live") < key.index("docker run --rm")
    assert "default off" in README.read_text(encoding="utf-8")


def test_bounded_env_files_pin_the_compatibility_flag_for_every_compose_command() -> None:
    lib = LIB.read_text(encoding="utf-8")
    assert "oteryn_pin_native_admission_flag()" in lib
    rollback = (ROOT / "deploy" / "synology" / "scripts" / "rollback.sh").read_text(encoding="utf-8")
    assert 'oteryn_pin_native_admission_flag "$rollback_env" "$PLATFORM_IMAGE"' in rollback
    assert rollback.index("oteryn_pin_native_admission_flag") < rollback.index("_oteryn_reconcile_marketplace_scheduler_after_runtime_change")
    assert 'oteryn_pin_native_admission_flag "$candidate_env"' in DEPLOY.read_text(encoding="utf-8")


def test_rollback_flag_requires_projection_settings_and_publish_trust() -> None:
    lib = LIB.read_text(encoding="utf-8")
    flag = lib.split("oteryn_native_admission_flag_for_image()")[1]
    assert "oteryn_native_admission_projection_ready" in flag
    assert "game-auth:native-admission:publish-trust" in flag
    assert "oteryn_native_admission_projection_ready || exit 1" in DEPLOY.read_text(encoding="utf-8")


def test_finalize_previous_candidate_checks_the_flag_before_recreating_gateway() -> None:
    deploy = DEPLOY.read_text(encoding="utf-8")
    fin = deploy.split("finalize_previous_candidate_if_healthy() {")[1]
    assert fin.index("running_platform_flag") < fin.index("up -d --no-deps --force-recreate internal-proxy gateway")


def test_every_non_full_deploy_route_requires_projection_readiness() -> None:
    scripts = ROOT / "deploy" / "synology" / "scripts"
    for name in ("deploy.sh", "deploy-impact.sh"):
        text = (scripts / name).read_text(encoding="utf-8")
        assert "native admission projection settings incomplete" in text
    market = (scripts / "marketplace-staging.sh").read_text(encoding="utf-8")
    assert "oteryn_native_admission_projection_ready || exit 1" in market


def test_identity_maps_are_proven_with_the_application_settings_and_bazaar_control_renders_native_env() -> None:
    lib = LIB.read_text(encoding="utf-8").split("oteryn_run_native_admission_self_check()")[1]
    assert "NativeRuntimeStatusSettings::current()" in lib and "NativeScopeAssignmentSettings::current()" in lib
    workflow = (WORKFLOWS / "character-bazaar-staging-control.yml").read_text(encoding="utf-8")
    for name in ("OTERYN_NATIVE_ADMISSION_ENABLED=", "GAME_AUTH_NATIVE_RUNTIME_STATUS_IDENTITIES=", "GAME_AUTH_NATIVE_SCOPE_ASSIGNMENT_IDENTITIES=", "GAME_AUTH_NATIVE_ADMISSION_UNVERIFIED_CHARACTER_WORLD_ID="):
        assert name in workflow
    task = (ROOT / "docs" / "agents" / "tasks" / "archive" / "OTERYN-20261011-b7-deploy-native-login.md").read_text(encoding="utf-8")
    assert "## Closure audit" in task


def test_staged_key_rotation_is_a_separate_path_and_the_rotate_guard_stays() -> None:
    key = KEY_SCRIPT.read_text(encoding="utf-8")
    assert "stage | promote" in key and "case 'stage':" in key and "case 'promote':" in key
    promote = key.split('if [[ "$ACTION" == promote ]]; then')[2].split('if [[ "$ACTION" == drop-retiring ]]')[0]
    order = [promote.index(x) for x in ("oteryn_native_admission_ingress_stop", "run_key_tool promote", "force-recreate platform", "oteryn_publish_native_admission_trust_or_stop", "_oteryn_reconcile_marketplace_scheduler_after_runtime_change", "oteryn_native_admission_self_check_or_stop", "sleep ", "oteryn_native_admission_ingress_start $ingress ||")]
    assert order == sorted(order)
    assert '"$MAINTENANCE" != --maintenance' in key
    assert "next.key" not in ENTRYPOINT.read_text(encoding="utf-8")


def test_previous_platform_restore_propagates_ingress_and_scheduler_failures() -> None:
    fn = LIB.read_text(encoding="utf-8").split("oteryn_restore_previous_platform_behind_trust_gate() {")[1].split("\n}\n")[0]
    assert fn.count("rc=1") == 2 and 'return "$rc"' in fn


def test_autostart_repair_shares_the_deploy_concurrency_group() -> None:
    text = (WORKFLOWS / "repair-synology-autostart.yml").read_text(encoding="utf-8")
    assert "group: synology-staging-deployment" in text


def test_promote_fails_closed_loads_the_env_and_propagates_trust() -> None:
    key = KEY_SCRIPT.read_text(encoding="utf-8")
    promote = key.split('if [[ "$ACTION" == promote ]]; then')[2].split('if [[ "$ACTION" == drop-retiring ]]')[0]
    assert promote.index('load_oteryn_env_file "$OTERYN_ENV_FILE"') < promote.index("OTERYN_NATIVE_ADMISSION_ENABLED:-false")
    assert "trap 'rc=$?; ((rc == 0)) || promote_fail_closed" in promote
    assert "rm -s -f gateway internal-proxy platform" in promote
    assert "OTERYN_NATIVE_TRUST_PROPAGATION_SECONDS:-10" in key and 'sleep "$propagation_seconds"' in promote
    assert "((promote_rc == 1))" in promote
    tool = key.split("case 'promote':")[1].split("case 'rotate':")[0]
    assert "array_reverse($done)" in tool and "exit($clean && $done === [] ? 1 : 4)" in tool


def test_promote_rejects_a_propagation_wait_below_the_contract_bound_before_any_mutation(tmp_path: pathlib.Path) -> None:
    env_file = tmp_path / "env"
    env_file.write_text("", encoding="utf-8")
    for bad in ("0", "abc", "9", "-1", "", "1e3"):
        if bad == "":
            continue
        result = subprocess.run(
            ["bash", str(KEY_SCRIPT), "promote"],
            env={"PATH": "/usr/bin:/bin", "PLATFORM_IMAGE": "x", "OTERYN_ENV_FILE": str(env_file), "OTERYN_NATIVE_TRUST_PROPAGATION_SECONDS": bad},
            capture_output=True, text=True, check=False,
        )
        assert result.returncode == 2, (bad, result.stderr)
        assert "at least 10" in result.stderr


def test_drop_retiring_revokes_the_retired_key_in_the_registry_before_destroying_it() -> None:
    key = KEY_SCRIPT.read_text(encoding="utf-8")
    block = key.split('if [[ "$ACTION" == drop-retiring ]]; then')[1].split('run_key_tool "$ACTION"')[0]
    assert "revokeKey(" in block and "trustedPublicKey(" in block
    assert "the retiring key was kept" in block and "nothing was removed" in block
    assert key.index('if [[ "$ACTION" == drop-retiring ]]; then') < key.index('run_key_tool "$ACTION"')


def test_marketplace_runtime_recreation_self_checks_the_final_platform_before_ingress() -> None:
    fn = LIB.read_text(encoding="utf-8").split("_oteryn_recreate_marketplace_runtime_gated() {")[1].split("\n}\n")[0]
    order = [fn.index(x) for x in ("oteryn_publish_native_admission_trust_or_stop", "oteryn_native_admission_self_check_or_stop", "oteryn_native_admission_ingress_start $ingress")]
    assert order == sorted(order)


def test_marketplace_recreation_fails_closed_after_the_ingress_stop() -> None:
    lib = LIB.read_text(encoding="utf-8")
    outer = lib.split("_oteryn_recreate_marketplace_runtime() {")[1].split("\n}\n")[0]
    assert "_oteryn_recreate_marketplace_runtime_gated" in outer and "rm -s -f gateway internal-proxy platform" in outer
    gated = lib.split("_oteryn_recreate_marketplace_runtime_gated() {")[1].split("\n}\n")[0]
    assert "--force-recreate platform || return 1" in gated and "marketplace-scheduler || return 1" in gated


def test_self_check_gate_treats_inspect_failure_as_a_failure_and_removes_ingress() -> None:
    fn = LIB.read_text(encoding="utf-8").split("oteryn_native_admission_self_check_or_stop() {")[1].split("\n}\n")[0]
    assert "&& env_out=\"$(command docker inspect" in fn and "rm -s -f gateway internal-proxy platform" in fn
    assert "|| true" not in fn


def test_every_route_runs_the_self_check_gate_before_ingress() -> None:
    root = ROOT / "deploy" / "synology" / "scripts"
    rollback = (root / "rollback.sh").read_text(encoding="utf-8")
    assert rollback.index("oteryn_native_admission_self_check_or_stop") < rollback.index("up -d platform internal-proxy gateway")
    mp = (root / "marketplace-staging.sh").read_text(encoding="utf-8")
    assert mp.count("oteryn_native_admission_self_check_or_stop") == 2
    assert "oteryn_native_admission_self_check_or_stop" in KEY_SCRIPT.read_text(encoding="utf-8")
    assert "must be true or false" in KEY_SCRIPT.read_text(encoding="utf-8")


def test_native_defaults_file_is_non_secret_complete_and_overridable_by_variables() -> None:
    defaults = (ROOT / "deploy" / "synology" / "native-defaults.env").read_text(encoding="utf-8")
    values = dict(line.split("=", 1) for line in defaults.splitlines() if line and not line.startswith("#"))
    # The D171 World tie is dropped: the client directory lists the World from the native route record.
    assert "OTERYN_STAGING_NATIVE_WORLD_ID" not in values
    world = re.search(r"[0-9a-f]{8}-[0-9a-f]{4}-7[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}", values["OTERYN_STAGING_NATIVE_SCOPE_ASSIGNMENT_IDENTITIES"]).group(0)
    assert values["OTERYN_STAGING_NATIVE_ADMISSION_ENABLED"] == "true"
    for name in ("RUNTIME_STATUS_IDENTITIES", "SCOPE_ASSIGNMENT_IDENTITIES"):
        identities = json.loads(values[f"OTERYN_STAGING_NATIVE_{name}"])
        assert identities and all(s.startswith("CN=") and scopes for s, scopes in identities.items())
        assert all(scope.startswith(f"{world}/") for scopes in identities.values() for scope in scopes)
    assert re.search(r"(?i)secret|password|token|private", "\n".join(f"{k}={v}" for k, v in values.items())) is None
    for name in ("deploy-synology-staging.yml", "character-bazaar-staging-control.yml"):
        workflow = (WORKFLOWS / name).read_text(encoding="utf-8")
        assert workflow.count('oteryn_native_default OTERYN_STAGING_NATIVE_') == 4
        assert "vars.OTERYN_STAGING_NATIVE_WORLD_ID" in workflow


def test_native_default_resolver_prefers_the_variable_then_the_file(tmp_path: pathlib.Path) -> None:
    file = tmp_path / "d.env"
    file.write_text('OTERYN_STAGING_NATIVE_WORLD_ID=from-file\nOTERYN_STAGING_NATIVE_ADMISSION_ENABLED=false\n', encoding="utf-8")

    def resolve(name: str, current: str) -> str:
        result = subprocess.run(
            ["bash", "-c", f'source "{LIB}"; oteryn_native_default {name} "$1"', "x", current],
            env={"PATH": "/usr/bin:/bin", "OTERYN_NATIVE_DEFAULTS_FILE": str(file)},
            capture_output=True, text=True, check=False,
        )
        assert result.returncode == 0, result.stderr
        return result.stdout

    assert resolve("OTERYN_STAGING_NATIVE_WORLD_ID", "") == "from-file"
    assert resolve("OTERYN_STAGING_NATIVE_WORLD_ID", "override") == "override"
    assert resolve("OTERYN_STAGING_NATIVE_ADMISSION_ENABLED", "") == "false"
    assert resolve("OTERYN_STAGING_NATIVE_SCOPE_ASSIGNMENT_IDENTITIES", "") == ""


def test_check_kid_history_is_a_read_only_action_that_needs_exactly_one_record() -> None:
    text = KEY_SCRIPT.read_text(encoding="utf-8")
    assert "check-kid-history" in text.split("set -euo pipefail", 1)[0]
    tool = text[text.index("if ($action === 'check-kid-history')") : text.index("secure($dir, $uid, $gid, 0700);\n\nswitch")]
    # Runs before secure(): no chown, chmod, write, rename or generation on this path.
    for forbidden in ("secure(", "chown", "chmod", "fopen", "file_put_contents", "rename(", "unlink", "generate("):
        assert forbidden not in tool
    assert "$count === 1" in tool
    assert "exit($ok ? 0 : 1)" in tool


def _run_key_tool(keys: pathlib.Path, *args: str) -> subprocess.CompletedProcess[str]:
    # The real PHP key tool, pointed at a temp custody directory and the current user as the owner.
    probe = 'exit(function_exists("sodium_crypto_sign_seed_keypair") && function_exists("posix_getpwuid") ? 0 : 1);'
    if shutil.which("php") is None or subprocess.run(["php", "-r", probe], check=False).returncode != 0:
        pytest.skip("php with sodium and posix is not installed")
    text = KEY_SCRIPT.read_text(encoding="utf-8")
    tool = text.split("read -r -d '' PHP_TOOL <<'PHP' || true\n", 1)[1].split("\nPHP\n", 1)[0]
    assert tool.count("$dir = '/keys';") == 1 and tool.count("posix_getpwnam('www-data')") == 1
    tool = tool.replace("$dir = '/keys';", "$dir = getenv('OTERYN_TEST_KEYS');")
    tool = tool.replace("posix_getpwnam('www-data')", "posix_getpwuid(posix_geteuid())")
    return subprocess.run(
        ["php", "-d", "display_errors=stderr", "-r", tool, "--", *args],
        env={"PATH": "/usr/bin:/bin", "OTERYN_TEST_KEYS": str(keys)},
        capture_output=True, text=True, check=False,
    )


def _custody(tmp_path: pathlib.Path) -> tuple[pathlib.Path, str, str]:
    keys = tmp_path / "keys"
    keys.mkdir(mode=0o700)
    seed = base64.urlsafe_b64encode(os.urandom(32)).decode().rstrip("=")
    kid = "oteryn-admission-20261013-000000-abcdef"
    for name, value in (("current.key", seed), ("current.kid", kid), ("used.kids", kid)):
        (keys / name).write_text(value + "\n", encoding="utf-8")
        (keys / name).chmod(0o600)
    return keys, seed, kid


def test_check_kid_history_counts_a_well_formed_custody(tmp_path: pathlib.Path) -> None:
    keys, seed, kid = _custody(tmp_path)
    result = _run_key_tool(keys, "check-kid-history")
    assert result.returncode == 0, result.stderr
    assert result.stdout == f"kid_history current {kid} count=1\n"
    assert seed not in result.stdout + result.stderr


def test_check_kid_history_never_prints_a_symlinked_role_kid(tmp_path: pathlib.Path) -> None:
    # Codex P2 on #1591: current.kid -> current.key must not print the Ed25519 seed as a key id.
    keys, seed, _ = _custody(tmp_path)
    (keys / "current.kid").unlink()
    (keys / "current.kid").symlink_to("current.key")
    result = _run_key_tool(keys, "check-kid-history")
    assert result.returncode != 0
    assert "kid_history" not in result.stdout
    assert seed not in result.stdout + result.stderr


def test_check_kid_history_refuses_unsafe_custody_and_malformed_kids_before_printing(tmp_path: pathlib.Path) -> None:
    keys, seed, _ = _custody(tmp_path)
    (keys / "current.kid").chmod(0o644)
    result = _run_key_tool(keys, "check-kid-history")
    assert result.returncode != 0 and result.stdout == "" and "unsafe ownership" in result.stderr
    (keys / "current.kid").chmod(0o600)
    (keys / "used.kids").unlink()
    (keys / "used.kids").symlink_to("current.key")
    result = _run_key_tool(keys, "check-kid-history")
    assert result.returncode != 0 and result.stdout == "" and seed not in result.stderr
    (keys / "used.kids").unlink()
    (keys / "used.kids").write_text("x\n", encoding="utf-8")
    (keys / "used.kids").chmod(0o600)
    result = _run_key_tool(keys, "check-kid-history", "--kid", "bad kid\n")
    assert result.returncode != 0 and result.stdout == ""
