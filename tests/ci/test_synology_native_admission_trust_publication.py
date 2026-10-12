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
INGRESS_STOP = 'oteryn_native_admission_ingress_stop "${compose[@]}" >/dev/null'
RESTORE = "oteryn_restore_previous_platform_behind_trust_gate"
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
    *' ps -q internal-proxy') printf '%s\\n' "${FAKE_PROXY-proxy-1}"; exit "${FAKE_INGRESS_PS_RC:-0}" ;;
    *' ps -q gateway') printf '%s\\n' "${FAKE_GATEWAY-gateway-1}"; exit "${FAKE_INGRESS_PS_RC:-0}" ;;
    *' stop gateway internal-proxy') exit "${FAKE_STOP_RC:-0}" ;;
    *' rm -s -f gateway internal-proxy platform') exit "${FAKE_RM_RC:-0}" ;;
    *' rm -s -f internal-mtls') exit "${FAKE_MTLS_RM_RC:-0}" ;;
    *' up -d '*) exit "${FAKE_UP_RC:-0}" ;;
    'start '*) exit "${FAKE_START_RC:-0}" ;;
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


def ordered(text: str, *needles: str) -> None:
    positions = [text.index(needle) for needle in needles]
    assert positions == sorted(positions), needles


def function_body(text: str, signature: str) -> str:
    body = text[text.index(signature) :]
    return body[: body.index("\n}\n")]


# Every route that (re)creates Platform: ingress (internal proxy and Gateway) is stopped first, the
# witnessed trust gate passes, and only then does ingress start. Grants are issued only through
# Gateway, so no grant can come from a Platform whose trust publication and witness are unproven.


def test_full_deploy_keeps_ingress_stopped_until_the_trust_gate_passes() -> None:
    text = deploy_text()
    full = text[text.index('OTERYN_ENV_FILE="$ENV_FILE" bash "$SCRIPT_DIR/prepare-fresh-schema-baseline.sh"') :]
    ordered(
        full,
        INGRESS_STOP,
        '"${compose[@]}" up -d platform',
        '"${compose[@]}" exec -T platform php artisan migrate --force --no-interaction',
        PUBLISH_CALL + " || exit 1",
        '"${compose[@]}" exec -T platform php artisan game-auth:oauth-client:ensure',
        '"${compose[@]}" up -d --no-deps --force-recreate internal-proxy',
        '"${compose[@]}" up -d gateway',
        'bash "$SCRIPT_DIR/health-check.sh"',
    )
    publish = full.index(PUBLISH_CALL)
    self_check = full.find("native_admission_self_check")
    assert self_check == -1 or publish < self_check
    # Nothing between the start of the full path and the gate starts ingress.
    before_gate = full[:publish]
    assert "internal-proxy" not in before_gate.replace(INGRESS_STOP, "")
    assert "gateway" not in before_gate.replace(INGRESS_STOP, "")


@pytest.mark.parametrize(
    ("path", "signature", "prefix", "smoke"),
    [
        (DEPLOY, "_oteryn_fast_deploy_platform() {", "FAST", "_oteryn_fast_platform_smoke"),
        (DEPLOY_IMPACT, "_oteryn_platform_reconcile() {", "RECONCILE", "_oteryn_platform_smoke"),
    ],
)
def test_platform_only_routes_keep_ingress_stopped_until_the_trust_gate_passes(
    path: pathlib.Path, signature: str, prefix: str, smoke: str
) -> None:
    body = function_body(path.read_text(encoding="utf-8"), signature)
    ordered(
        body,
        INGRESS_STOP,
        '"${compose[@]}" up -d --no-deps --force-recreate platform',
        PUBLISH_CALL + " || return 1",
        f'oteryn_native_admission_ingress_start "${prefix}_PROXY_ID" "${prefix}_GATEWAY_ID"',
        "_oteryn_reconcile_marketplace_scheduler_after_runtime_change",
        smoke,
    )
    # The same preserved containers start again, so the preserved-identity assertions still hold.
    assert f'internal-proxy "${prefix}_PROXY_ID"' in body
    assert f'gateway "${prefix}_GATEWAY_ID"' in body


@pytest.mark.parametrize(
    ("path", "signature", "label"),
    [
        (DEPLOY, "_oteryn_fast_restore_on_exit() {", "Fast deploy recovery"),
        (DEPLOY_IMPACT, "_oteryn_reconcile_restore_on_exit() {", "Platform reconcile recovery"),
    ],
)
def test_failure_restore_traps_restore_behind_the_trust_gate(path: pathlib.Path, signature: str, label: str) -> None:
    body = function_body(path.read_text(encoding="utf-8"), signature)
    ordered(body, 'export PLATFORM_IMAGE="$', f'{RESTORE} "{label}" || true', 'restored_platform_id="$(')
    # The trap itself never recreates Platform or starts ingress outside the shared gated restore.
    assert "force-recreate" not in body
    assert "start internal-proxy" not in body


def test_shared_restore_keeps_ingress_stopped_until_the_previous_platform_passes_the_gate() -> None:
    body = function_body(LIB.read_text(encoding="utf-8"), RESTORE + "() {")
    ordered(
        body,
        'oteryn_native_admission_ingress_stop "${base[@]}"',
        '"${base[@]}" up -d --no-deps --force-recreate platform',
        'oteryn_publish_native_admission_trust_or_stop "${base[@]}"',
        '"${base[@]}" up -d --no-deps internal-proxy gateway',
        "_oteryn_reconcile_marketplace_scheduler_after_runtime_change",
    )


def test_previous_candidate_recreates_ingress_only_after_the_trust_gate() -> None:
    body = function_body(deploy_text(), "finalize_previous_candidate_if_healthy() {")
    ordered(
        body,
        'oteryn_native_admission_ingress_stop "${candidate_compose[@]}"',
        "if ! " + CANDIDATE_CALL + "; then",
        '"${candidate_compose[@]}" up -d --no-deps --force-recreate internal-proxy gateway',
        'bash "$SCRIPT_DIR/health-check.sh"',
        'cp "$candidate_file" "$current_file.tmp"',
    )


def test_rollback_starts_ingress_only_after_the_last_good_platform_passes_the_gate() -> None:
    text = ROLLBACK.read_text(encoding="utf-8")
    ordered(
        text,
        INGRESS_STOP,
        '"${compose[@]}" up -d platform\n',
        PUBLISH_CALL + " || exit 1",
        '"${compose[@]}" up -d platform internal-proxy gateway',
        "_oteryn_reconcile_marketplace_scheduler_after_runtime_change",
        'bash "$SCRIPT_DIR/health-check.sh"',
        '"$state_dir/current-release.env.tmp"',
    )
    # The combined start runs only once, after the gate, on the already gated Platform.
    assert text.count("up -d platform internal-proxy gateway") == 1


def test_marketplace_switches_start_ingress_only_after_the_final_trust_gate() -> None:
    text = MARKETPLACE_STAGING.read_text(encoding="utf-8")
    enable = text[text.index("    enable)\n") : text.index("    verify)\n")]
    ordered(
        enable,
        INGRESS_STOP,
        "--force-recreate platform",
        "artisan migrate --force --no-interaction",
        PUBLISH_CALL,
        "marketplace:reconcile",
        "export MARKETPLACE_ENABLED=true",
    )
    second = enable[enable.index("export MARKETPLACE_ENABLED=true") :]
    ordered(
        second,
        "--force-recreate platform",
        PUBLISH_CALL,
        "up -d marketplace-scheduler",
        "--force-recreate internal-proxy",
        '"${compose[@]}" start gateway',
        "verify_enabled_runtime",
    )
    rollback = text[text.index("    prepare-rollback)\n") :]
    ordered(
        rollback,
        INGRESS_STOP,
        "--force-recreate platform",
        PUBLISH_CALL,
        "stop marketplace-scheduler",
        "--force-recreate internal-proxy",
        '"${compose[@]}" start gateway',
        "Character Bazaar staging is drained",
    )


def test_marketplace_runtime_recreation_restarts_only_previously_running_ingress_after_the_gate() -> None:
    body = function_body(LIB.read_text(encoding="utf-8"), "_oteryn_recreate_marketplace_runtime_gated() {")
    ordered(
        body,
        'ingress="$(oteryn_native_admission_ingress_stop "${marketplace_compose[@]}")" || return 1',
        '"${marketplace_compose[@]}" up -d --no-deps --force-recreate platform',
        GATE + ' "${marketplace_compose[@]}" || return 1',
        'oteryn_native_admission_self_check_or_stop "${marketplace_compose[@]}" || return 1',
        "oteryn_native_admission_ingress_start $ingress || return 1",
        '"${marketplace_compose[@]}" up -d --force-recreate marketplace-scheduler',
    )


PLATFORM_START = re.compile(r"up -d (?:--no-deps )?(?:--force-recreate )?platform\b")
INGRESS_START = re.compile(
    r"oteryn_native_admission_ingress_start|up -d (?:--no-deps )?(?:--force-recreate )?(?:internal-proxy|gateway)\b"
    r"|start (?:internal-proxy|gateway)\b"
)
GATE_CALL = re.compile(GATE + r' "|' + GATE + r" command ")
STOPPED = re.compile(r"oteryn_native_admission_ingress_stop \"|stop platform gateway internal-proxy")


@pytest.mark.parametrize("path", SCRIPTS, ids=lambda path: path.name)
def test_no_platform_start_is_reachable_through_ingress_before_the_gate(path: pathlib.Path) -> None:
    # Class check over every script: each Platform (re)creation is preceded by an ingress stop with
    # no ingress start in between, and the next ingress start comes after the next trust gate.
    lines = [line.strip() for line in path.read_text(encoding="utf-8").splitlines()]
    events: list[tuple[int, str]] = []
    for number, line in enumerate(lines):
        if line.startswith("#") or "$joined" in line or line.startswith("echo"):
            continue
        if PLATFORM_START.search(line) and re.search(r"up -d .*\b(?:internal-proxy|gateway)\b", line):
            events.append((number, "both"))
        elif GATE_CALL.search(line) and "() {" not in line:
            events.append((number, "gate"))
        elif STOPPED.search(line) and "() {" not in line:
            events.append((number, "stop"))
        elif PLATFORM_START.search(line):
            events.append((number, "platform"))
        elif INGRESS_START.search(line) and "() {" not in line:
            events.append((number, "ingress"))
    for index, (number, kind) in enumerate(events):
        if kind == "both":
            # Starting Platform together with ingress is allowed only for a Platform that is already
            # up and gated: the last Platform event before it must be the trust gate.
            previous = [k for _, k in events[:index] if k in {"platform", "gate", "both"}]
            assert previous and previous[-1] == "gate", f"{path.name}:{number + 1} starts Platform and ingress together"
            continue
        if kind != "platform":
            continue
        before = [k for _, k in events[:index] if k in {"stop", "ingress"}]
        assert before and before[-1] == "stop", f"{path.name}:{number + 1} starts Platform with ingress up"
        after = [k for _, k in events[index + 1 :] if k in {"gate", "ingress"}]
        assert after and after[0] == "gate", f"{path.name}:{number + 1} starts ingress before the trust gate"


def test_every_platform_deploy_route_publishes_trust() -> None:
    # Each script that (re)creates Platform for a release, rollback or Marketplace switch passes the
    # fail-closed gate before it starts ingress, promotes or reports success.
    assert deploy_text().count(PUBLISH_CALL + " || exit 1") == 1
    assert deploy_text().count(PUBLISH_CALL + " || return 1") == 1
    assert deploy_text().count("if ! " + CANDIDATE_CALL + "; then") == 1
    assert deploy_text().count(RESTORE + ' "Fast deploy recovery"') == 1
    impact = DEPLOY_IMPACT.read_text(encoding="utf-8")
    assert impact.count(PUBLISH_CALL + " || return 1") == 1
    assert impact.count(RESTORE + ' "Platform reconcile recovery"') == 1
    assert ROLLBACK.read_text(encoding="utf-8").count(PUBLISH_CALL + " || exit 1") == 1
    assert MARKETPLACE_STAGING.read_text(encoding="utf-8").count(PUBLISH_CALL + " || exit 1") == 3
    lib = LIB.read_text(encoding="utf-8")
    assert lib.count(GATE + ' "${marketplace_compose[@]}" || return 1') == 1
    assert lib.count("if ! " + GATE + ' "${base[@]}"; then') == 1


def test_only_the_fail_closed_gate_calls_the_bare_publication() -> None:
    bare = 'oteryn_publish_native_admission_trust "'
    assert sum(path.read_text(encoding="utf-8").count(bare) for path in SCRIPTS) == 1
    gate = function_body(LIB.read_text(encoding="utf-8"), GATE + "() {")
    ordered(gate, 'oteryn_publish_native_admission_trust "$("$@" ps -q platform)"', '"$@" rm -s -f gateway internal-proxy platform')
    assert gate.rstrip().endswith("return 1")


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
    r"mkdir -p -- \"\$GAME_AUTH_NATIVE_EVIDENCE_HIGH_WATER_DIRECTORY\"\s+"
    # An existing directory is normalized and verified on every run (review P2 on PR #1546).
    r"test ! -L \"\$GAME_AUTH_NATIVE_EVIDENCE_HIGH_WATER_DIRECTORY\"\s+"
    r"chmod 700 -- \"\$GAME_AUTH_NATIVE_EVIDENCE_HIGH_WATER_DIRECTORY\"\s+"
    r"test -O \"\$GAME_AUTH_NATIVE_EVIDENCE_HIGH_WATER_DIRECTORY\"\s+"
    r"test \"\$\(stat -c %a -- \"\$GAME_AUTH_NATIVE_EVIDENCE_HIGH_WATER_DIRECTORY\"\)\" = 700"
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


REMOVE = "compose -f staging.yml rm -s -f gateway internal-proxy platform"


def test_gate_passes_without_removing_anything(fake_docker: pathlib.Path) -> None:
    result, calls = run_gate(fake_docker, "docker", "compose", "-f", "staging.yml")
    assert result.returncode == 0, result.stderr
    assert calls[0] == "compose -f staging.yml ps -q platform"
    assert calls[-1].endswith("publish-trust --no-interaction")
    assert REMOVE not in calls


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
def test_gate_failure_removes_platform_proxy_and_gateway(fake_docker: pathlib.Path, fake: dict[str, str]) -> None:
    # Removed, not stopped: a stopped `restart: always` container returns on a daemon or NAS restart.
    result, calls = run_gate(fake_docker, "docker", "compose", "-f", "staging.yml", **fake)
    assert result.returncode == 1
    assert "removing the Platform, internal proxy and Gateway containers (fail closed)" in result.stderr
    assert calls[-1] == REMOVE


def test_gate_still_fails_when_the_removal_fails(fake_docker: pathlib.Path) -> None:
    result, calls = run_gate(
        fake_docker, "docker", "compose", "-f", "staging.yml", FAKE_PUBLISH_RC="1", FAKE_RM_RC="1"
    )
    assert result.returncode == 1
    assert "Unable to remove Platform, internal proxy and Gateway" in result.stderr
    assert calls[-1] == REMOVE


def test_gate_accepts_env_prefixed_and_builtin_compose_commands(fake_docker: pathlib.Path) -> None:
    for prefix in (("env", "PLATFORM_IMAGE=x"), ("command",)):
        log = fake_docker / "docker.log"
        log.unlink(missing_ok=True)
        result, calls = run_gate(fake_docker, *prefix, "docker", "compose", "-f", "staging.yml", FAKE_PUBLISH_RC="1")
        assert result.returncode == 1
        assert calls[0] == "compose -f staging.yml ps -q platform"
        assert calls[-1] == REMOVE


def test_gate_needs_the_compose_command(fake_docker: pathlib.Path) -> None:
    result, calls = run_gate(fake_docker)
    assert result.returncode == 1
    assert "needs the Compose command" in result.stderr
    assert calls == []


def run_lib(tmp: pathlib.Path, script: str, *args: str, **fake: str) -> tuple[subprocess.CompletedProcess[str], list[str]]:
    log = tmp / "docker.log"
    log.unlink(missing_ok=True)
    env = {
        **os.environ,
        "PATH": f"{tmp / 'bin'}:{os.environ['PATH']}",
        "FAKE_DOCKER_LOG": str(log),
        "FAKE_FLAG": "true",
        "FAKE_OUTPUT": TRUSTED,
        "ENV_FILE": "staging.env",
        "COMPOSE_FILE": "staging.yml",
        **fake,
    }
    result = subprocess.run(
        ["bash", "-c", 'source "$1"; shift; ' + script, "bash", str(LIB), *args],
        text=True,
        capture_output=True,
        check=False,
        env=env,
    )
    calls = log.read_text(encoding="utf-8").splitlines() if log.exists() else []
    return result, calls


def test_ingress_stop_reports_the_running_ingress_in_start_order(fake_docker: pathlib.Path) -> None:
    result, calls = run_lib(fake_docker, 'oteryn_native_admission_ingress_stop "$@"', "docker", "compose", "-f", "x.yml")
    assert result.returncode == 0, result.stderr
    assert result.stdout.split() == ["proxy-1", "gateway-1"]
    assert calls == [
        "compose -f x.yml ps -q internal-proxy",
        "compose -f x.yml ps -q gateway",
        "compose -f x.yml stop gateway internal-proxy",
        # The mTLS listener is off here, so any existing one is removed (lib.sh).
        "compose -f x.yml rm -s -f internal-mtls",
    ]


def test_ingress_stop_on_a_first_deploy_has_nothing_to_restart(fake_docker: pathlib.Path) -> None:
    result, calls = run_lib(
        fake_docker, 'oteryn_native_admission_ingress_stop "$@"', "docker", "compose", "-f", "x.yml", FAKE_PROXY="", FAKE_GATEWAY=""
    )
    assert result.returncode == 0, result.stderr
    assert result.stdout == ""
    assert calls[-2:] == ["compose -f x.yml stop gateway internal-proxy", "compose -f x.yml rm -s -f internal-mtls"]


@pytest.mark.parametrize("fake", [{"FAKE_STOP_RC": "1"}, {"FAKE_INGRESS_PS_RC": "1"}])
def test_ingress_stop_failure_is_reported(fake_docker: pathlib.Path, fake: dict[str, str]) -> None:
    result, _ = run_lib(fake_docker, 'oteryn_native_admission_ingress_stop "$@"', "docker", "compose", "-f", "x.yml", **fake)
    assert result.returncode != 0


def test_ingress_start_starts_only_the_given_containers(fake_docker: pathlib.Path) -> None:
    result, calls = run_lib(fake_docker, 'oteryn_native_admission_ingress_start "$@"', "", "proxy-1", "", "gateway-1")
    assert result.returncode == 0, result.stderr
    assert calls == ["start proxy-1 gateway-1"]
    result, calls = run_lib(fake_docker, 'oteryn_native_admission_ingress_start "$@"', "")
    assert result.returncode == 0
    assert calls == []


BASE = "compose --env-file staging.env -f staging.yml"


def test_restore_starts_ingress_only_after_the_previous_platform_passes(fake_docker: pathlib.Path) -> None:
    result, calls = run_lib(fake_docker, RESTORE + ' "Fast deploy recovery"')
    assert result.returncode == 0, result.stderr
    stop = calls.index(f"{BASE} stop gateway internal-proxy")
    recreate = calls.index(f"{BASE} up -d --no-deps --force-recreate platform")
    publish = next(i for i, call in enumerate(calls) if call.endswith("publish-trust --no-interaction"))
    ingress = calls.index(f"{BASE} up -d --no-deps internal-proxy gateway")
    assert stop < recreate < publish < ingress
    assert f"{BASE} rm -s -f gateway internal-proxy platform" not in calls


def test_restore_leaves_the_runtime_removed_when_the_previous_platform_fails_the_gate(fake_docker: pathlib.Path) -> None:
    result, calls = run_lib(fake_docker, RESTORE + ' "Fast deploy recovery"', FAKE_PUBLISH_RC="1", FAKE_OUTPUT="refused")
    assert result.returncode == 1
    assert "Fast deploy recovery left Platform, internal proxy and Gateway removed" in result.stderr
    assert calls[-1] == f"{BASE} rm -s -f gateway internal-proxy platform"
    assert f"{BASE} up -d --no-deps internal-proxy gateway" not in calls


def test_restore_never_recreates_platform_while_ingress_could_not_be_stopped(fake_docker: pathlib.Path) -> None:
    result, calls = run_lib(fake_docker, RESTORE + ' "Platform reconcile recovery"', FAKE_STOP_RC="1")
    assert result.returncode == 1
    assert "removed the runtime containers (fail closed)" in result.stderr
    assert f"{BASE} up -d --no-deps --force-recreate platform" not in calls
    assert calls[-1] == f"{BASE} rm -s -f gateway internal-proxy platform"
