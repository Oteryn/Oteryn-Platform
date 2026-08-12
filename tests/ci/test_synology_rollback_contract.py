from __future__ import annotations

import pathlib
import subprocess
import tempfile

ROOT = pathlib.Path(__file__).resolve().parents[2]
SCRIPTS = ROOT / "deploy" / "synology" / "scripts"
RELEASE_STATE = SCRIPTS / "release-state.sh"

OLD_SHA = "1" * 40
NEW_SHA = "2" * 40
DIGEST_A = "a" * 64
DIGEST_B = "b" * 64
DIGEST_C = "c" * 64


def release(path: pathlib.Path, *, sha: str, schema: str, accepts: str, eligible: int = 1) -> None:
    path.write_text(
        "\n".join(
            [
                f"RELEASE_SHA={sha}",
                f"PLATFORM_IMAGE=example/platform@sha256:{DIGEST_A}",
                f"GATEWAY_IMAGE=example/gateway@sha256:{DIGEST_B}",
                f"CANARY_IMAGE=example/canary@sha256:{DIGEST_C}",
                f"SCHEMA_COMPATIBILITY_ID={schema}",
                f"APP_ACCEPTS_SCHEMA_IDS={accepts}",
                "MIGRATION_POLICY=expand-contract",
                f"ROLLBACK_ELIGIBLE={eligible}",
                "",
            ]
        )
    )


def run_compatible(current: pathlib.Path, old: pathlib.Path, schema: str = "schema-v2") -> subprocess.CompletedProcess[str]:
    return subprocess.run(
        ["bash", str(RELEASE_STATE), "compatible", str(current), str(old), schema],
        text=True,
        capture_output=True,
        check=False,
    )


def test_compatible_rollback_is_accepted() -> None:
    with tempfile.TemporaryDirectory() as td:
        root = pathlib.Path(td)
        current, old = root / "candidate.env", root / "last-good.env"
        release(current, sha=NEW_SHA, schema="schema-v2", accepts="schema-v2")
        release(old, sha=OLD_SHA, schema="schema-v1", accepts="schema-v1,schema-v2")
        assert run_compatible(current, old).returncode == 0


def test_incompatible_schema_rollback_is_rejected() -> None:
    with tempfile.TemporaryDirectory() as td:
        root = pathlib.Path(td)
        current, old = root / "candidate.env", root / "last-good.env"
        release(current, sha=NEW_SHA, schema="schema-v2", accepts="schema-v2")
        release(old, sha=OLD_SHA, schema="schema-v1", accepts="schema-v1")
        result = run_compatible(current, old)
        assert result.returncode != 0
        assert "does not declare compatibility" in result.stderr


def test_missing_compatibility_metadata_is_rejected() -> None:
    with tempfile.TemporaryDirectory() as td:
        root = pathlib.Path(td)
        current, old = root / "candidate.env", root / "last-good.env"
        release(current, sha=NEW_SHA, schema="schema-v2", accepts="schema-v2")
        release(old, sha=OLD_SHA, schema="schema-v1", accepts="schema-v1,schema-v2")
        old.write_text(old.read_text().replace("APP_ACCEPTS_SCHEMA_IDS=schema-v1,schema-v2\n", ""))
        assert run_compatible(current, old).returncode != 0


def test_stale_last_good_identity_is_rejected() -> None:
    with tempfile.TemporaryDirectory() as td:
        root = pathlib.Path(td)
        current, old = root / "candidate.env", root / "last-good.env"
        release(current, sha=NEW_SHA, schema="schema-v2", accepts="schema-v2")
        release(old, sha=NEW_SHA, schema="schema-v1", accepts="schema-v1,schema-v2")
        result = run_compatible(current, old)
        assert result.returncode != 0
        assert "stale" in result.stderr.lower()


def test_health_probe_helpers_are_repository_pinned_by_digest() -> None:
    lib = (SCRIPTS / "lib.sh").read_text()
    health = (SCRIPTS / "health-check.sh").read_text()
    assert "OTERYN_HEALTH_ALPINE_IMAGE='alpine@sha256:" in lib
    assert "OTERYN_HEALTH_PYTHON_IMAGE='python@sha256:" in lib
    assert "alpine:3.22) args[$i]=\"$OTERYN_HEALTH_ALPINE_IMAGE\"" in lib
    assert "python:3.12-alpine) args[$i]=\"$OTERYN_HEALTH_PYTHON_IMAGE\"" in lib
    assert health.count("alpine:3.22") >= 1
    assert health.count("python:3.12-alpine") == 1


def test_migration_ambiguity_fails_closed_before_migrate() -> None:
    lib = (SCRIPTS / "lib.sh").read_text()
    assert "printf 'SCHEMA_STATE=unknown\\n'" in lib
    assert "printf 'SCHEMA_STATE=known\\n'" in lib
    before = lib.index("_oteryn_before_platform_migrate", lib.index("docker()"))
    execute = lib.index('command docker "${args[@]}"', before)
    after = lib.index("_oteryn_after_platform_migrate", execute)
    assert before < execute < after


def test_recovery_requires_verified_managed_backup_and_never_runs_implicitly() -> None:
    recovery = (SCRIPTS / "recover-schema.sh").read_text()
    deploy = (SCRIPTS / "deploy.sh").read_text()
    rollback = (SCRIPTS / "rollback.sh").read_text()
    assert "BACKUP_SHA256" in recovery
    assert '"$state_dir"/backups/*/evidence.env' in recovery
    assert "DROP DATABASE IF EXISTS" in recovery
    assert "recover-schema.sh" not in deploy
    assert "bash \"$SCRIPT_DIR/recover-schema.sh\"" not in rollback


def test_image_rollback_never_claims_database_rollback() -> None:
    rollback = (SCRIPTS / "rollback.sh").read_text()
    assert "Database schema was NOT rolled back or changed" in rollback
    assert "migrate:rollback" not in rollback
    assert "migrate:rollback" not in (SCRIPTS / "deploy.sh").read_text()
