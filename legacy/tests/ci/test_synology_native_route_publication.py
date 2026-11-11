from __future__ import annotations

import os
import pathlib
import stat
import subprocess

import pytest

ROOT = pathlib.Path(__file__).resolve().parents[2]
LIB = ROOT / "deploy" / "synology" / "scripts" / "lib.sh"
DEPLOY = ROOT / "deploy" / "synology" / "scripts" / "deploy.sh"
WORKFLOW = ROOT / ".github" / "workflows" / "deploy-synology-staging.yml"

WORLD = "0192b3c4-5d6e-7f80-9a1b-2c3d4e5f6a7b"
CHANNEL = "0192b3c4-5d6e-7f80-8a1b-2c3d4e5f6a7c"

# Stands in for the Compose command words: "$@" exec -T platform php artisan <command> ...
FAKE_COMPOSE = """#!/usr/bin/env bash
printf '%s\\n' "$*" >> "$FAKE_LOG"
case "$*" in
    *native-topology:issue*) printf '{"version":1,"world_id":"%s","channel_id":"%s"}\\n' "${FAKE_ISSUED_WORLD}" "$FAKE_CHANNEL"; exit "${FAKE_ISSUE_RC:-0}" ;;
    *native-route:publish*) printf '%s\\n' "${FAKE_PUBLISH_OUT}"; exit "${FAKE_PUBLISH_RC:-0}" ;;
esac
exit 97
"""

VALID = {
    "OTERYN_NATIVE_ADMISSION_ENABLED": "true",
    "OTERYN_NATIVE_ROUTE_PUBLISH_ENABLED": "true",
    "GAME_WORLD_ID": "1",
    "GAME_AUTH_NATIVE_ADMISSION_UNVERIFIED_CHARACTER_WORLD_ID": WORLD,
    "OTERYN_NATIVE_CHANNEL_KEY": "ch1",
    "OTERYN_NATIVE_ROUTE_HOST": "192.168.1.20",
    "OTERYN_NATIVE_ROUTE_PORT": "7172",
    "OTERYN_NATIVE_ROUTE_TLS_SERVER_NAME": "node-1.preprod.oteryn.internal",
    "FAKE_ISSUED_WORLD": WORLD,
    "FAKE_CHANNEL": CHANNEL,
    "FAKE_PUBLISH_OUT": '{"world_id":"%s","channel_id":"%s","route_version":1,"route_revision":"rt.1.abc","native_login_enabled":true}' % (WORLD, CHANNEL),
}


def run(tmp_path: pathlib.Path, **overrides: str) -> tuple[subprocess.CompletedProcess[str], str]:
    fake = tmp_path / "compose"
    fake.write_text(FAKE_COMPOSE, encoding="utf-8")
    fake.chmod(fake.stat().st_mode | stat.S_IXUSR)
    log = tmp_path / "log"
    log.write_text("", encoding="utf-8")
    env = {"PATH": os.environ["PATH"], "FAKE_LOG": str(log), **VALID}
    for key, value in overrides.items():
        if value == "<unset>":
            env.pop(key, None)
        else:
            env[key] = value
    result = subprocess.run(
        ["bash", "-c", f'source "{LIB}"; oteryn_publish_native_route "{fake}"'],
        capture_output=True, text=True, env=env, check=False,
    )
    return result, log.read_text(encoding="utf-8")


def test_publishes_after_issuing_and_matching_the_configured_world(tmp_path: pathlib.Path) -> None:
    result, log = run(tmp_path)
    assert result.returncode == 0, result.stderr
    assert "route_revision" in result.stdout
    lines = log.splitlines()
    assert "native-topology:issue" in lines[0] and "--world-row-id=1" in lines[0] and "--channel-key=ch1" in lines[0]
    assert "native-route:publish" in lines[1]
    for expected in ("--host=192.168.1.20", "--port=7172", "--tls-server-name=node-1.preprod.oteryn.internal", "--login-enabled=true"):
        assert expected in lines[1]


def test_is_off_by_default_and_runs_nothing(tmp_path: pathlib.Path) -> None:
    result, log = run(tmp_path, OTERYN_NATIVE_ROUTE_PUBLISH_ENABLED="false")
    assert result.returncode == 0 and log == ""
    result, log = run(tmp_path, OTERYN_NATIVE_ROUTE_PUBLISH_ENABLED="<unset>")
    assert result.returncode == 0 and log == ""


@pytest.mark.parametrize(
    "name",
    [
        "OTERYN_NATIVE_CHANNEL_KEY",
        "OTERYN_NATIVE_ROUTE_HOST",
        "OTERYN_NATIVE_ROUTE_PORT",
        "OTERYN_NATIVE_ROUTE_TLS_SERVER_NAME",
        "GAME_AUTH_NATIVE_ADMISSION_UNVERIFIED_CHARACTER_WORLD_ID",
    ],
)
def test_missing_required_variable_fails_closed_before_any_command(tmp_path: pathlib.Path, name: str) -> None:
    result, log = run(tmp_path, **{name: ""})
    assert result.returncode == 1 and log == ""
    assert name in result.stderr


@pytest.mark.parametrize(
    "overrides",
    [
        {"OTERYN_NATIVE_ROUTE_PORT": "70000"},
        {"OTERYN_NATIVE_ROUTE_PORT": "0"},
        {"OTERYN_NATIVE_CHANNEL_KEY": "Ch 1"},
        {"OTERYN_NATIVE_ROUTE_HOST": "https://192.168.1.20"},
        {"OTERYN_NATIVE_ROUTE_TLS_SERVER_NAME": "node 1"},
        {"OTERYN_NATIVE_ROUTE_LOGIN_ENABLED": "yes"},
        {"OTERYN_NATIVE_ADMISSION_ENABLED": "false"},
        {"OTERYN_NATIVE_ROUTE_PUBLISH_ENABLED": "maybe"},
    ],
)
def test_malformed_values_fail_closed_before_any_command(tmp_path: pathlib.Path, overrides: dict[str, str]) -> None:
    result, log = run(tmp_path, **overrides)
    assert result.returncode == 1 and log == ""


def test_world_id_mismatch_stops_before_publishing_and_names_the_issued_world(tmp_path: pathlib.Path) -> None:
    other = "0192b3c4-5d6e-7f80-9a1b-2c3d4e5f6a99"
    result, log = run(tmp_path, FAKE_ISSUED_WORLD=other)
    assert result.returncode == 1
    assert other in result.stderr and "native-route:publish" not in log


def test_command_refusals_fail_the_deploy(tmp_path: pathlib.Path) -> None:
    assert run(tmp_path, FAKE_ISSUE_RC="1")[0].returncode == 1
    result, _ = run(tmp_path, FAKE_PUBLISH_RC="1")
    assert result.returncode == 1
    result, _ = run(tmp_path, FAKE_PUBLISH_OUT="{}")
    assert result.returncode == 1


def test_deploy_calls_the_step_after_world_registration_and_workflow_passes_the_variables() -> None:
    deploy = DEPLOY.read_text(encoding="utf-8")
    assert deploy.count("oteryn_publish_native_route") == 2
    assert deploy.index("game-auth:world:ensure", deploy.index("oteryn_publish_native_route") - 900) < deploy.index("oteryn_publish_native_route")
    workflow = WORKFLOW.read_text(encoding="utf-8")
    for variable in (
        "OTERYN_STAGING_NATIVE_ROUTE_PUBLISH_ENABLED",
        "OTERYN_STAGING_NATIVE_CHANNEL_KEY",
        "OTERYN_STAGING_NATIVE_ROUTE_HOST",
        "OTERYN_STAGING_NATIVE_ROUTE_PORT",
        "OTERYN_STAGING_NATIVE_ROUTE_TLS_SERVER_NAME",
        "OTERYN_STAGING_NATIVE_ROUTE_LOGIN_ENABLED",
    ):
        assert f"vars.{variable}" in workflow
