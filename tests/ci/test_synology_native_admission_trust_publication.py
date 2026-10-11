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
LIB = ROOT / "deploy" / "synology" / "scripts" / "lib.sh"
DEPLOY = ROOT / "deploy" / "synology" / "scripts" / "deploy.sh"
COMMAND = ROOT / "app" / "Console" / "Commands" / "PublishNativeAdmissionTrust.php"

PUBLISH_CALL = 'oteryn_publish_native_admission_trust "$("${compose[@]}" ps -q platform)"'
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
    smoke = body.index("_oteryn_fast_platform_smoke")
    assert recreate < publish < smoke
    self_check = body.find("game-auth:native-admission:self-check")
    assert self_check == -1 or publish < self_check


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


def test_switch_off_skips_publication(fake_docker: pathlib.Path) -> None:
    result, calls = run_publish(fake_docker, FAKE_FLAG="false")
    assert result.returncode == 0
    assert "Native admission is off" in result.stdout
    assert len(calls) == 1
    assert not any("publish-trust" in call for call in calls)


def test_switch_on_creates_the_witness_owner_only_and_publishes(fake_docker: pathlib.Path) -> None:
    result, calls = run_publish(fake_docker)
    assert result.returncode == 0, result.stderr
    assert TRUSTED in result.stdout
    assert "PUBLIC values for Game's trust configuration" in result.stdout
    publish = "\n".join(calls)
    assert "\nexec platform-1 sh -ec \n" in f"\n{publish}"
    assert re.search(r"umask 077\s+mkdir -p -- \"\$GAME_AUTH_NATIVE_EVIDENCE_HIGH_WATER_DIRECTORY\"", publish)
    assert "exec php artisan game-auth:native-admission:publish-trust --no-interaction" in publish


def test_already_trusted_is_accepted(fake_docker: pathlib.Path) -> None:
    result, _ = run_publish(fake_docker, FAKE_OUTPUT=TRUSTED.replace("published", "already_trusted"))
    assert result.returncode == 0, result.stderr


@pytest.mark.parametrize(
    ("fake", "reason"),
    [
        ({"FAKE_FLAG_RC": "1"}, "Unable to read the Platform native admission flag"),
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
