from __future__ import annotations

import os
import pathlib
import subprocess
import tempfile

ROOT = pathlib.Path(__file__).resolve().parents[2]
LIB = ROOT / "deploy" / "synology" / "scripts" / "lib.sh"

LEGACY_KEYS = (
    "MARKETPLACE_ESCROW_CANARY_ACCOUNT_ID=42\n"
    "MARKETPLACE_ESCROW_CANARY_ACCOUNT_NAME=oteryn_bazaar_escrow\n"
    "MARKETPLACE_ESCROW_CANARY_ACCOUNT_CREATION_EPOCH=1785456000\n"
    "CANARY_CHARACTER_TRANSFER_DB_USER=oteryn_character_transfer\n"
    "CANARY_CHARACTER_TRANSFER_DB_PASSWORD=not-a-secret\n"
)

FAKE_DOCKER = """#!/usr/bin/env bash
set -euo pipefail
printf '%s\\n' "$*" >>"$OTERYN_TEST_DOCKER_LOG"
case "$*" in
    *information_schema.tables*) printf '%s\\n' "$OTERYN_TEST_TABLES" ;;
    *character_auctions*) printf '%s\\n' "$OTERYN_TEST_PENDING" ;;
    *) exit 9 ;;
esac
"""


def run_loader(state: str, *, pending: str = "0", tables: str = "1") -> tuple[subprocess.CompletedProcess[str], str, str]:
    with tempfile.TemporaryDirectory() as td:
        root = pathlib.Path(td)
        state_dir = root / "state"
        state_dir.mkdir()
        state_file = state_dir / "marketplace.env"
        state_file.write_text(state)
        docker = root / "docker"
        docker.write_text(FAKE_DOCKER)
        docker.chmod(0o755)
        log = root / "docker.log"
        log.write_text("")
        env_file = root / ".env"
        env_file.write_text("APP_ENV=staging\n")
        env = os.environ.copy()
        env.update(
            {
                "PATH": f"{root}:{env['PATH']}",
                "OTERYN_STATE_DIR": str(state_dir),
                "OTERYN_TEST_DOCKER_LOG": str(log),
                "OTERYN_TEST_PENDING": pending,
                "OTERYN_TEST_TABLES": tables,
                "ENV_FILE": str(env_file),
                "COMPOSE_FILE": str(root / "compose.yml"),
                "MARIADB_ROOT_PASSWORD": "root-not-a-secret",
                "PLATFORM_DB_NAME": "oteryn_platform",
            }
        )
        result = subprocess.run(
            [
                "bash",
                "-c",
                'source "$1"; _oteryn_load_marketplace_runtime_state || exit $?; printf "enabled=%s\\n" "$MARKETPLACE_ENABLED"',
                "bash",
                str(LIB),
            ],
            text=True,
            capture_output=True,
            check=False,
            env=env,
        )
        return result, state_file.read_text(), log.read_text()


def test_legacy_disabled_drained_state_is_migrated() -> None:
    result, state, log = run_loader("MARKETPLACE_ENABLED=false\n" + LEGACY_KEYS)
    assert result.returncode == 0, result.stderr
    assert "enabled=false" in result.stdout
    assert state == "MARKETPLACE_ENABLED=false\n"
    assert "character_auctions" in log


def test_legacy_state_without_auction_table_is_migrated() -> None:
    result, state, _ = run_loader("MARKETPLACE_ENABLED=false\n" + LEGACY_KEYS, tables="0")
    assert result.returncode == 0, result.stderr
    assert state == "MARKETPLACE_ENABLED=false\n"


def test_legacy_enabled_state_fails_closed_unchanged() -> None:
    original = "MARKETPLACE_ENABLED=true\n" + LEGACY_KEYS
    result, state, log = run_loader(original)
    assert result.returncode != 0
    assert "prepare-rollback" in result.stderr
    assert state == original
    assert log == ""


def test_legacy_state_with_pending_auctions_fails_closed_unchanged() -> None:
    original = "MARKETPLACE_ENABLED=false\n" + LEGACY_KEYS
    result, state, _ = run_loader(original, pending="3")
    assert result.returncode != 0
    assert "3 non-terminal auctions" in result.stderr
    assert state == original


def test_unknown_durable_key_still_fails_closed() -> None:
    original = "MARKETPLACE_ENABLED=false\n" + LEGACY_KEYS + "CANARY_DB_PASSWORD=x\n"
    result, state, log = run_loader(original)
    assert result.returncode != 0
    assert "Unexpected durable Marketplace state key: CANARY_DB_PASSWORD" in result.stderr
    assert state == original
    assert log == ""


def test_native_only_state_needs_no_database_proof() -> None:
    result, state, log = run_loader("MARKETPLACE_ENABLED=true\n")
    assert result.returncode == 0, result.stderr
    assert "enabled=true" in result.stdout
    assert state == "MARKETPLACE_ENABLED=true\n"
    assert log == ""


def test_deploy_migrates_legacy_state_before_schema_change() -> None:
    lib = LIB.read_text(encoding="utf-8")
    before = lib.index("_oteryn_before_platform_migrate() {")
    migrate = lib.index("_oteryn_migrate_legacy_marketplace_state || return 1", before)
    candidate = lib.index('release-state.sh" write "$state_dir/candidate-release.env"', before)
    assert migrate < candidate
