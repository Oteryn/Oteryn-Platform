from __future__ import annotations

import pathlib
import subprocess

ROOT = pathlib.Path(__file__).resolve().parents[3]
SCRIPTS = ROOT / "deploy" / "synology" / "scripts"
TOPOLOGY = SCRIPTS / "engine-topology.sh"
RELEASE_STATE = SCRIPTS / "release-state.sh"

WITHOUT_ENGINE = "services:\n  mariadb:\n    image: x\n  platform:\n    image: z\n"
WITH_ENGINE = WITHOUT_ENGINE.replace("  platform:", "  canary:\n    image: y\n  platform:")
ONLY_AS_VOLUME = WITHOUT_ENGINE + "volumes:\n  canary:\n"

SHA = "a" * 40
DIGEST = "ghcr.io/oteryn/img@sha256:" + "b" * 64
ENV = {"PATH": "/usr/bin:/bin"}


def _compose(tmp: pathlib.Path, text: str) -> pathlib.Path:
    path = tmp / "compose.yml"
    path.write_text(text)
    return path


def _run(args: list[str], compose: pathlib.Path) -> subprocess.CompletedProcess[str]:
    return subprocess.run(args, capture_output=True, text=True, env={**ENV, "OTERYN_COMPOSE_FILE": str(compose)})


def test_topology_detection_covers_both_topologies(tmp_path: pathlib.Path) -> None:
    probe = ["bash", "-c", f'source "{TOPOLOGY}"; oteryn_engine_enabled']
    assert _run(probe, _compose(tmp_path, WITH_ENGINE)).stdout.strip() == "1"
    assert _run(probe, _compose(tmp_path, WITHOUT_ENGINE)).stdout.strip() == "0"
    assert _run(probe, _compose(tmp_path, ONLY_AS_VOLUME)).stdout.strip() == "0"


def _write_state(tmp: pathlib.Path, compose: pathlib.Path, image: str) -> subprocess.CompletedProcess[str]:
    out = tmp / "current-release.env"
    args = ["bash", str(RELEASE_STATE), "write", str(out), SHA, SHA, SHA,
            "schema-1", "schema-1", DIGEST, DIGEST, image, "1"]
    return _run(args, compose)


def test_release_state_requires_image_only_when_service_present(tmp_path: pathlib.Path) -> None:
    present = _compose(tmp_path, WITH_ENGINE)
    assert _write_state(tmp_path, present, DIGEST).returncode == 0
    assert _write_state(tmp_path, present, "").returncode != 0

    absent_dir = tmp_path / "absent"
    absent_dir.mkdir()
    absent = _compose(absent_dir, WITHOUT_ENGINE)
    result = _write_state(absent_dir, absent, "")
    assert result.returncode == 0, result.stderr
    validate = _run(["bash", str(RELEASE_STATE), "validate", str(absent_dir / "current-release.env")], absent)
    assert validate.returncode == 0, validate.stderr


def test_scripts_gate_on_topology() -> None:
    for name in ("deploy.sh", "deploy-impact.sh", "rollback.sh", "health-check.sh",
                 "production-target-preflight.sh", "marketplace-staging.sh"):
        assert "engine_enabled" in (SCRIPTS / name).read_text(), name
    rollback = (SCRIPTS / "rollback.sh").read_text()
    assert "up -d platform internal-proxy gateway" in rollback
    assert "pull platform gateway\n" in rollback
