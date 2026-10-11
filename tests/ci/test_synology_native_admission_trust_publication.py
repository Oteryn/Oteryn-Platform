from __future__ import annotations

import json
import os
import pathlib
import re
import stat
import subprocess

import pytest


ROOT = pathlib.Path(__file__).resolve().parents[2]
COMPOSE = ROOT / "deploy" / "synology" / "compose.yml"
MARKETPLACE = ROOT / "deploy" / "synology" / "compose.marketplace.yml"
LIB = ROOT / "deploy" / "synology" / "scripts" / "lib.sh"
DEPLOY = ROOT / "deploy" / "synology" / "scripts" / "deploy.sh"
DEPLOY_IMPACT = ROOT / "deploy" / "synology" / "scripts" / "deploy-impact.sh"
ROLLBACK = ROOT / "deploy" / "synology" / "scripts" / "rollback.sh"
MARKETPLACE_STAGING = ROOT / "deploy" / "synology" / "scripts" / "marketplace-staging.sh"
COMMAND = ROOT / "app" / "Console" / "Commands" / "PublishNativeAdmissionTrust.php"

GATE = "oteryn_publish_native_admission_trust_or_stop"
PUBLISH_CALL = GATE + ' "${compose[@]}"'
CANDIDATE_CALL = GATE + ' "${candidate_compose[@]}"'
RESTORE_CALL = GATE + ' command docker compose --env-file "$ENV_FILE" -f "$COMPOSE_FILE"'
SCRIPTS = (LIB, DEPLOY, DEPLOY_IMPACT, ROLLBACK, MARKETPLACE_STAGING)
WITNESS = "/var/www/html/storage/app/native-evidence-witness"
TRUSTED = json.dumps(
    {
        "result": "published",
        "issuer": "urn:oteryn:platform:game-admission",
        "profile": "oteryn-pre-admission-v1",
        "key_purpose": "fresh_admission",
        "key_id": "admission-1",
        "public_key": "A" * 43,
    },
    separators=(",", ":"),
)

FAKE_DOCKER = """#!/usr/bin/env bash
printf '%s\\n' "$*" >> "$FAKE_DOCKER_LOG"
case "$*" in
    *' ps -q platform') printf '%s\\n' "${FAKE_PS-platform-1}"; exit "${FAKE_PS_RC:-0}" ;;
    *' stop gateway internal-proxy platform') exit "${FAKE_STOP_RC:-0}" ;;
    *'GAME_AUTH_NATIVE_ADMISSION_ENABLED'*) printf %s "$FAKE_FLAG"; exit "${FAKE_FLAG_RC:-0}" ;;
    *'publish-trust'*) printf '%s\\n' "$FAKE_OUTPUT"; exit "${FAKE_PUBLISH_RC:-0}" ;;
esac
exit 97
"""


def platform_environment() -> str:
    text = COMPOSE.read_text(encoding="utf-8")
    return text[text.index("\n  platform:\n") : text.index("\n  tls-init:\n")]


def test_compose_scopes_the_trust_registry_and_keeps_the_witness_on_retained_storage() -> None:
    platform = platform_environment()
    assert "GAME_AUTH_NATIVE_EVIDENCE_FRESH_KEY_PURPOSE: fresh_admission\n" in platform
    assert f"GAME_AUTH_NATIVE_EVIDENCE_HIGH_WATER_DIRECTORY: {WITNESS}\n" in platform
    # The witness must outlive container recreation and stay independent of MariaDB.
    assert "- platform_storage:/var/www/html/storage\n" in platform
    assert "mariadb_data" not in platform


def test_marketplace_scheduler_shares_the_platform_witness() -> None:
    text = MARKETPLACE.read_text(encoding="utf-8")
    scheduler = text[text.index("\n  marketplace-scheduler:\n") :]
    platform = platform_environment()
    for name in ("GAME_AUTH_NATIVE_EVIDENCE_FRESH_KEY_PURPOSE", "GAME_AUTH_NATIVE_EVIDENCE_HIGH_WATER_DIRECTORY"):
        expected = re.search(rf"\n      {name}: (\S+)\n", platform)
        actual = re.search(rf"\n      {name}: (\S+)\n", scheduler)
        assert expected is not None and actual is not None, name
        assert actual.group(1) == expected.group(1), name
    assert "- platform_storage:/var/www/html/storage\n" in scheduler


def test_command_is_restricted_to_testing_or_preproduction_and_prints_one_json_line() -> None:
    source = COMMAND.read_text(encoding="utf-8")
    assert "protected $signature = 'game-auth:native-admission:publish-trust';" in source
    assert "app()->environment(['testing', 'preproduction'])" in source
    assert "'production'" not in source
    assert "$this->line((string) json_encode($published, JSON_UNESCAPED_SLASHES));" in source


def deploy_text() -> str:
    return DEPLOY.read_text(encoding="utf-8")


def test_full_deploy_publishes_after_migration_and_before_any_signing_self_check() -> None:
    text = deploy_text()
    full = text[text.index('OTERYN_ENV_FILE="$ENV_FILE" bash "$SCRIPT_DIR/prepare-fresh-schema-baseline.sh"') :]
    migrate = full.index('"${compose[@]}" exec -T platform php artisan migrate --force --no-interaction')
    publish = full.index(PUBLISH_CALL + " || exit 1")
    ensure = full.index('"${compose[@]}" exec -T platform php artisan game-auth:oauth-client:ensure')
    gateway = full.index('"${compose[@]}" up -d gateway')
    assert migrate < publish < ensure < gateway
    self_check = full.find("game-auth:native-admission:self-check")
    assert self_check == -1 or publish < self_check
    key_ensure = full.find("native-admission:key")
    assert key_ensure == -1 or key_ensure < publish


def test_fast_deploy_publishes_after_platform_recreate_and_before_smoke() -> None:
    text = deploy_text()
    body = text[text.index("_oteryn_fast_deploy_platform() {") :]
    body = body[: body.index("\n}\n")]
    recreate = body.index('"${compose[@]}" up -d --no-deps --force-recreate platform')
    publish = body.index(PUBLISH_CALL + " || return 1")
    scheduler = body.index("_oteryn_reconcile_marketplace_scheduler_after_runtime_change")
    smoke = body.index("_oteryn_fast_platform_smoke")
    # The witness directory exists before the scheduler restarts on the new image.
    assert recreate < publish < scheduler < smoke
    self_check = body.find("game-auth:native-admission:self-check")
    assert self_check == -1 or publish < self_check


def test_platform_reconcile_publishes_after_platform_recreate_and_before_scheduler_and_smoke() -> None:
    text = DEPLOY_IMPACT.read_text(encoding="utf-8")
    body = text[text.index("_oteryn_platform_reconcile() {") :]
    body = body[: body.index("\n}\n")]
    recreate = body.index('"${compose[@]}" up -d --no-deps --force-recreate platform')
    publish = body.index(PUBLISH_CALL + " || return 1")
    scheduler = body.index("_oteryn_reconcile_marketplace_scheduler_after_runtime_change")
    smoke = body.index("_oteryn_platform_smoke")
    health = body.index('bash "$SCRIPT_DIR/health-check.sh"')
    # This route exits before deploy.sh, so it must take the witnessed publication itself.
    assert recreate < publish < scheduler < smoke < health
    self_check = body.find("game-auth:native-admission:self-check")
    assert self_check == -1 or publish < self_check


def test_every_platform_deploy_route_publishes_trust() -> None:
    # Each script that (re)creates Platform for a release, rollback or Marketplace switch passes the
    # witnessed publication before it promotes or reports success.
    # Every one goes through the fail-closed gate, which stops Platform, internal proxy and Gateway
    # when publication fails; the failure-restore traps gate the restored Platform the same way.
    assert deploy_text().count(PUBLISH_CALL + " || exit 1") == 1
    assert deploy_text().count(PUBLISH_CALL + " || return 1") == 1
    assert deploy_text().count("if ! " + CANDIDATE_CALL + "; then") == 1
    assert deploy_text().count("if " + RESTORE_CALL + "; then") == 1
    impact = DEPLOY_IMPACT.read_text(encoding="utf-8")
    assert impact.count(PUBLISH_CALL + " || return 1") == 1
    assert impact.count("if " + RESTORE_CALL + "; then") == 1
    assert ROLLBACK.read_text(encoding="utf-8").count(PUBLISH_CALL + " || exit 1") == 1
    assert MARKETPLACE_STAGING.read_text(encoding="utf-8").count(PUBLISH_CALL + " || exit 1") == 3
    lib = LIB.read_text(encoding="utf-8")
    assert lib.count(GATE + ' "${marketplace_compose[@]}" || return 1') == 1


def test_only_the_fail_closed_gate_calls_the_bare_publication() -> None:
    bare = 'oteryn_publish_native_admission_trust "'
    assert sum(path.read_text(encoding="utf-8").count(bare) for path in SCRIPTS) == 1
    lib = LIB.read_text(encoding="utf-8")
    gate = lib[lib.index(GATE + "() {") :]
    gate = gate[: gate.index("\n}\n")]
    ordered(gate, 'oteryn_publish_native_admission_trust "$("$@" ps -q platform)"', '"$@" stop gateway internal-proxy platform')
    assert gate.rstrip().endswith("return 1")


@pytest.mark.parametrize(
    ("path", "function", "label"),
    [
        (DEPLOY, "_oteryn_fast_restore_on_exit() {", "Fast deploy recovery"),
        (DEPLOY_IMPACT, "_oteryn_reconcile_restore_on_exit() {", "Platform reconcile recovery"),
    ],
)
def test_failure_restore_gates_the_restored_platform(path: pathlib.Path, function: str, label: str) -> None:
    text = path.read_text(encoding="utf-8")
    body = text[text.index(function) :]
    body = body[: body.index("\n}\n")]
    ordered(
        body,
        "up -d --no-deps --force-recreate platform",
        "if " + RESTORE_CALL + "; then",
        "start internal-proxy gateway",
        "_oteryn_reconcile_marketplace_scheduler_after_runtime_change",
        "else",
        label + " left Platform, internal proxy and Gateway stopped",
        'restored_platform_id="$(',
    )


def ordered(text: str, *needles: str) -> None:
    positions = [text.index(needle) for needle in needles]
    assert positions == sorted(positions), needles


def test_previous_candidate_is_promoted_only_after_trust_publication() -> None:
    text = deploy_text()
    body = text[text.index("finalize_previous_candidate_if_healthy() {") :]
    body = body[: body.index("\n}\n")]
    ordered(
        body,
        '"${candidate_compose[@]}" up -d --no-deps --force-recreate internal-proxy gateway',
        CANDIDATE_CALL,
        'bash "$SCRIPT_DIR/health-check.sh"',
        'cp "$candidate_file" "$current_file.tmp"',
    )


def test_rollback_publishes_after_starting_the_last_good_platform_and_before_promotion() -> None:
    text = ROLLBACK.read_text(encoding="utf-8")
    ordered(
        text,
        '"${compose[@]}" up -d platform internal-proxy gateway',
        PUBLISH_CALL + " || exit 1",
        "_oteryn_reconcile_marketplace_scheduler_after_runtime_change",
        'bash "$SCRIPT_DIR/health-check.sh"',
        '"$state_dir/current-release.env.tmp"',
    )


def test_marketplace_switches_publish_after_each_platform_recreation() -> None:
    text = MARKETPLACE_STAGING.read_text(encoding="utf-8")
    enable = text[text.index("    enable)\n") : text.index("    verify)\n")]
    ordered(
        enable,
        "artisan migrate --force --no-interaction",
        PUBLISH_CALL,
        "marketplace:reconcile",
        "export MARKETPLACE_ENABLED=true",
    )
    second = enable[enable.index("export MARKETPLACE_ENABLED=true") :]
    ordered(second, "--force-recreate platform", PUBLISH_CALL, "up -d marketplace-scheduler", "verify_enabled_runtime")
    rollback = text[text.index("    prepare-rollback)\n") :]
    ordered(rollback, "--force-recreate platform", PUBLISH_CALL, "stop marketplace-scheduler", "Character Bazaar staging is drained")


def test_marketplace_runtime_recreation_publishes_before_the_scheduler_restarts() -> None:
    lib = LIB.read_text(encoding="utf-8")
    body = lib[lib.index("_oteryn_recreate_marketplace_runtime() {") :]
    body = body[: body.index("\n}\n")]
    ordered(
        body,
        '"${marketplace_compose[@]}" up -d --no-deps --force-recreate platform',
        GATE + ' "${marketplace_compose[@]}" || return 1',
        '"${marketplace_compose[@]}" up -d --force-recreate marketplace-scheduler',
    )


@pytest.fixture()
def fake_docker(tmp_path: pathlib.Path) -> pathlib.Path:
    bin_dir = tmp_path / "bin"
    bin_dir.mkdir()
    docker = bin_dir / "docker"
    docker.write_text(FAKE_DOCKER, encoding="utf-8")
    docker.chmod(docker.stat().st_mode | stat.S_IXUSR)
    return tmp_path


def run_publish(
    tmp: pathlib.Path, container: str = "platform-1", **fake: str
) -> tuple[subprocess.CompletedProcess[str], list[str]]:
    log = tmp / "docker.log"
    env = {
        **os.environ,
        "PATH": f"{tmp / 'bin'}:{os.environ['PATH']}",
        "FAKE_DOCKER_LOG": str(log),
        "FAKE_FLAG": "true",
        "FAKE_OUTPUT": TRUSTED,
        **fake,
    }
    result = subprocess.run(
        ["bash", "-c", 'source "$1"; oteryn_publish_native_admission_trust "$2"', "bash", str(LIB), container],
        text=True,
        capture_output=True,
        check=False,
        env=env,
    )
    calls = log.read_text(encoding="utf-8").splitlines() if log.exists() else []
    return result, calls


WITNESS_SETUP = re.compile(
    r"test -n \"\$\{GAME_AUTH_NATIVE_EVIDENCE_HIGH_WATER_DIRECTORY:-\}\"\s+umask 077\s+"
    r"mkdir -p -- \"\$GAME_AUTH_NATIVE_EVIDENCE_HIGH_WATER_DIRECTORY\""
)


def test_switch_off_creates_the_witness_and_skips_publication(fake_docker: pathlib.Path) -> None:
    result, calls = run_publish(fake_docker, FAKE_FLAG="false")
    assert result.returncode == 0
    assert "Native admission is off" in result.stdout
    log = "\n".join(calls)
    # Identity security revocations write through the configured witness even with native admission off.
    assert log.startswith("exec platform-1 sh -ec ")
    assert WITNESS_SETUP.search(log)
    assert "publish-trust" not in log


def test_switch_on_creates_the_witness_owner_only_and_publishes(fake_docker: pathlib.Path) -> None:
    result, calls = run_publish(fake_docker)
    assert result.returncode == 0, result.stderr
    assert TRUSTED in result.stdout
    assert "PUBLIC values for Game's trust configuration" in result.stdout
    log = "\n".join(calls)
    assert WITNESS_SETUP.search(log)
    assert log.index("mkdir -p") < log.index("publish-trust")
    # The image entrypoint exports the signing key id and file path from the key volume (as for the self-check).
    assert calls[-1] == (
        "exec platform-1 /usr/local/bin/oteryn-platform-entrypoint "
        "php artisan game-auth:native-admission:publish-trust --no-interaction"
    )


def test_already_trusted_is_accepted(fake_docker: pathlib.Path) -> None:
    result, _ = run_publish(fake_docker, FAKE_OUTPUT=TRUSTED.replace("published", "already_trusted"))
    assert result.returncode == 0, result.stderr


@pytest.mark.parametrize(
    ("fake", "reason"),
    [
        ({"FAKE_FLAG_RC": "1"}, "Unable to prepare the native evidence witness directory"),
        ({"FAKE_PUBLISH_RC": "1", "FAKE_OUTPUT": "failed: restricted"}, "trust publication refused"),
        ({"FAKE_OUTPUT": '{"result":"ignored"}'}, "returned no trust result"),
    ],
)
def test_refusals_fail_closed(fake_docker: pathlib.Path, fake: dict[str, str], reason: str) -> None:
    result, _ = run_publish(fake_docker, **fake)
    assert result.returncode == 1
    assert reason in result.stderr


def test_missing_container_fails_closed(fake_docker: pathlib.Path) -> None:
    result, calls = run_publish(fake_docker, container="")
    assert result.returncode == 1
    assert "needs the Platform container" in result.stderr
    assert calls == []


def run_gate(tmp: pathlib.Path, *compose: str, **fake: str) -> tuple[subprocess.CompletedProcess[str], list[str]]:
    log = tmp / "docker.log"
    env = {
        **os.environ,
        "PATH": f"{tmp / 'bin'}:{os.environ['PATH']}",
        "FAKE_DOCKER_LOG": str(log),
        "FAKE_FLAG": "true",
        "FAKE_OUTPUT": TRUSTED,
        **fake,
    }
    result = subprocess.run(
        ["bash", "-c", 'source "$1"; shift; ' + GATE + ' "$@"', "bash", str(LIB), *compose],
        text=True,
        capture_output=True,
        check=False,
        env=env,
    )
    calls = log.read_text(encoding="utf-8").splitlines() if log.exists() else []
    return result, calls


STOP = "compose -f staging.yml stop gateway internal-proxy platform"


def test_gate_passes_without_stopping_anything(fake_docker: pathlib.Path) -> None:
    result, calls = run_gate(fake_docker, "docker", "compose", "-f", "staging.yml")
    assert result.returncode == 0, result.stderr
    assert calls[0] == "compose -f staging.yml ps -q platform"
    assert calls[-1].endswith("publish-trust --no-interaction")
    assert STOP not in calls


@pytest.mark.parametrize(
    "fake",
    [
        {"FAKE_PUBLISH_RC": "1", "FAKE_OUTPUT": "failed: restricted"},
        {"FAKE_OUTPUT": '{"result":"ignored"}'},
        {"FAKE_FLAG_RC": "1"},
        {"FAKE_PS": ""},
        {"FAKE_PS_RC": "1", "FAKE_PS": ""},
    ],
)
def test_gate_failure_stops_platform_proxy_and_gateway(fake_docker: pathlib.Path, fake: dict[str, str]) -> None:
    result, calls = run_gate(fake_docker, "docker", "compose", "-f", "staging.yml", **fake)
    assert result.returncode == 1
    assert "stopping Platform, internal proxy and Gateway (fail closed)" in result.stderr
    assert calls[-1] == STOP


def test_gate_still_fails_when_the_stop_fails(fake_docker: pathlib.Path) -> None:
    result, calls = run_gate(
        fake_docker, "docker", "compose", "-f", "staging.yml", FAKE_PUBLISH_RC="1", FAKE_STOP_RC="1"
    )
    assert result.returncode == 1
    assert "Unable to stop Platform, internal proxy and Gateway" in result.stderr
    assert calls[-1] == STOP


def test_gate_accepts_env_prefixed_and_builtin_compose_commands(fake_docker: pathlib.Path) -> None:
    for prefix in (("env", "PLATFORM_IMAGE=x"), ("command",)):
        log = fake_docker / "docker.log"
        log.unlink(missing_ok=True)
        result, calls = run_gate(fake_docker, *prefix, "docker", "compose", "-f", "staging.yml", FAKE_PUBLISH_RC="1")
        assert result.returncode == 1
        assert calls[0] == "compose -f staging.yml ps -q platform"
        assert calls[-1] == STOP


def test_gate_needs_the_compose_command(fake_docker: pathlib.Path) -> None:
    result, calls = run_gate(fake_docker)
    assert result.returncode == 1
    assert "needs the Compose command" in result.stderr
    assert calls == []
