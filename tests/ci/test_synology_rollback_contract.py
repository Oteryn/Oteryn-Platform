from __future__ import annotations

import os
import pathlib
import subprocess
import tempfile

ROOT = pathlib.Path(__file__).resolve().parents[2]
SCRIPTS = ROOT / "deploy" / "synology" / "scripts"
RELEASE_STATE = SCRIPTS / "release-state.sh"
LIB = SCRIPTS / "lib.sh"

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


def run_compatible(candidate: pathlib.Path, old: pathlib.Path, schema: str = "schema-v2") -> subprocess.CompletedProcess[str]:
    candidate_sha = next(line.split("=", 1)[1] for line in candidate.read_text().splitlines() if line.startswith("RELEASE_SHA="))
    return subprocess.run(
        ["bash", str(RELEASE_STATE), "compatible-schema", schema, str(old), candidate_sha],
        text=True,
        capture_output=True,
        check=False,
    )


def run_release_sha(platform_sha: str, gateway_sha: str, *, explicit_sha: str = "") -> subprocess.CompletedProcess[str]:
    with tempfile.TemporaryDirectory() as td:
        root = pathlib.Path(td)
        docker = root / "docker"
        docker.write_text(
            "#!/usr/bin/env bash\n"
            "set -euo pipefail\n"
            "ref=${@: -1}\n"
            f"case \"$ref\" in *platform*) printf '%s\\n' '{platform_sha}' ;; *gateway*) printf '%s\\n' '{gateway_sha}' ;; *) exit 9 ;; esac\n"
        )
        docker.chmod(0o755)
        env = os.environ.copy()
        env.update(
            {
                "PATH": f"{root}:{env['PATH']}",
                "PLATFORM_IMAGE": "example/platform:test",
                "GATEWAY_IMAGE": "example/gateway:test",
                "OTERYN_RELEASE_SHA": explicit_sha,
            }
        )
        return subprocess.run(
            ["bash", "-c", 'source "$1"; _oteryn_release_sha', "bash", str(LIB)],
            text=True,
            capture_output=True,
            check=False,
            env=env,
        )


def run_image_contract(payload: str) -> subprocess.CompletedProcess[str]:
    with tempfile.TemporaryDirectory() as td:
        root = pathlib.Path(td)
        docker = root / "docker"
        docker.write_text(
            "#!/usr/bin/env bash\n"
            "set -euo pipefail\n"
            "if [[ \"${1:-}\" == run ]]; then cat \"$OTERYN_TEST_CONTRACT\"; exit 0; fi\n"
            "exit 9\n"
        )
        docker.chmod(0o755)
        contract = root / "contract.env"
        contract.write_text(payload)
        env = os.environ.copy()
        env.update({"PATH": f"{root}:{env['PATH']}", "OTERYN_TEST_CONTRACT": str(contract)})
        return subprocess.run(
            ["bash", "-c", 'source "$1"; _oteryn_contract_from_platform_image example/platform:test', "bash", str(LIB)],
            text=True,
            capture_output=True,
            check=False,
            env=env,
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


def test_release_state_round_trips_world_name_with_spaces() -> None:
    with tempfile.TemporaryDirectory() as td:
        state = pathlib.Path(td) / "release.env"
        env = os.environ.copy()
        env.update(
            {
                "GAME_WORLD_ID": "1",
                "GAME_WORLD_SLUG": "oteryn-staging",
                "GAME_WORLD_NAME": "Oteryn Staging",
                "GAME_WORLD_REGION": "EU",
                "GAME_WORLD_HOST": "127.0.0.1",
                "GAME_WORLD_PORT": "7172",
            }
        )
        write = subprocess.run(
            [
                "bash",
                str(RELEASE_STATE),
                "write",
                str(state),
                NEW_SHA,
                "schema-v2",
                "schema-v1,schema-v2",
                f"example/platform@sha256:{DIGEST_A}",
                f"example/gateway@sha256:{DIGEST_B}",
                f"example/canary@sha256:{DIGEST_C}",
                "1",
            ],
            text=True,
            capture_output=True,
            check=False,
            env=env,
        )
        assert write.returncode == 0, write.stderr
        assert "GAME_WORLD_NAME=Oteryn\\ Staging" in state.read_text()
        read = subprocess.run(
            ["bash", "-c", 'source "$1"; printf "%s" "$GAME_WORLD_NAME"', "bash", str(state)],
            text=True,
            capture_output=True,
            check=False,
        )
        assert read.returncode == 0, read.stderr
        assert read.stdout == "Oteryn Staging"


def test_release_sha_is_derived_from_matching_runtime_oci_revisions() -> None:
    result = run_release_sha(NEW_SHA, NEW_SHA)
    assert result.returncode == 0, result.stderr
    assert result.stdout.strip() == NEW_SHA


def test_release_sha_rejects_runtime_revision_mismatch() -> None:
    result = run_release_sha(NEW_SHA, OLD_SHA)
    assert result.returncode != 0
    assert "OCI application revisions disagree" in result.stderr


def test_release_sha_rejects_conflicting_explicit_identity() -> None:
    result = run_release_sha(NEW_SHA, NEW_SHA, explicit_sha=OLD_SHA)
    assert result.returncode != 0
    assert "OTERYN_RELEASE_SHA disagrees" in result.stderr


def test_candidate_contract_is_loaded_from_platform_image() -> None:
    result = run_image_contract(
        "OTERYN_MIGRATION_POLICY=expand-contract\n"
        "OTERYN_SCHEMA_COMPATIBILITY_ID=schema-v2\n"
        "OTERYN_APP_ACCEPTS_SCHEMA_IDS=schema-v1,schema-v2\n"
    )
    assert result.returncode == 0, result.stderr
    assert result.stdout.strip() == "expand-contract\tschema-v2\tschema-v1,schema-v2"


def test_candidate_contract_rejects_unexpected_image_metadata() -> None:
    result = run_image_contract(
        "OTERYN_MIGRATION_POLICY=expand-contract\n"
        "OTERYN_SCHEMA_COMPATIBILITY_ID=schema-v2\n"
        "OTERYN_APP_ACCEPTS_SCHEMA_IDS=schema-v2\n"
        "UNTRUSTED_OVERRIDE=yes\n"
    )
    assert result.returncode != 0
    assert "Unexpected release contract key" in result.stderr


def test_legacy_bootstrap_uses_immutable_running_image_snapshot() -> None:
    lib = LIB.read_text()
    deploy = (SCRIPTS / "deploy.sh").read_text()
    assert "image_id=\"$(docker inspect --format '{{.Image}}' \"$container_id\")\"" in deploy
    assert "release-state.sh\" resolve-image \"$image_id\"" in deploy
    assert deploy.index("snapshot_current_images") < deploy.index('"${compose[@]}" pull')
    assert "observed_schema=\"observed-${old_sha}\"" in lib
    assert "Legacy running-release snapshot is incomplete; refusing migration." in lib


def test_legacy_bootstrap_does_not_replace_candidate_image_variables() -> None:
    lib = LIB.read_text()
    start = lib.index("_oteryn_bootstrap_legacy_current_release()")
    end = lib.index("_oteryn_quiesce_platform_db_consumers()", start)
    bootstrap = lib[start:end]
    assert "mapfile -t legacy_images" in bootstrap
    assert "unset PLATFORM_IMAGE GATEWAY_IMAGE CANARY_IMAGE" not in bootstrap
    assert "source \"$legacy_file\"" not in bootstrap


def test_existing_database_without_baseline_fails_closed() -> None:
    lib = LIB.read_text()
    assert "Existing Platform DB has no managed application baseline; refusing migration before backup-capable baseline is proven." in lib
    bootstrap = lib.index("_oteryn_bootstrap_legacy_current_release")
    candidate = lib.index('release-state.sh\" write \"$state_dir/candidate-release.env', bootstrap)
    assert bootstrap < candidate


def test_candidate_platform_is_not_started_before_migration_preparation() -> None:
    lib = LIB.read_text()
    wrapper = lib.index("docker()")
    up_branch = lib.index('"$joined" == *" up -d platform "*', wrapper)
    prepare = lib.index("_oteryn_before_platform_migrate", up_branch)
    start_candidate = lib.index('command docker "${args[@]}"', prepare)
    assert up_branch < prepare < start_candidate


def test_pre_migration_backup_quiesces_db_consumers_and_restores_scheduler() -> None:
    lib = LIB.read_text()
    before = lib.index("_oteryn_before_platform_migrate()")
    quiesce_call = lib.index("_oteryn_quiesce_platform_db_consumers", before)
    dump = lib.index("mariadb-dump", quiesce_call)
    unknown = lib.index("printf 'SCHEMA_STATE=unknown\\n'", dump)
    assert before < quiesce_call < dump < unknown
    quiesce = lib[lib.index("_oteryn_quiesce_platform_db_consumers()") : before]
    assert 'stop platform gateway internal-proxy' in quiesce
    assert "OTERYN_MARKETPLACE_SCHEDULER_WAS_RUNNING=1" in quiesce
    after = lib[lib.index("_oteryn_after_platform_migrate()") : lib.index("_oteryn_finalize_release_on_exit()")]
    assert "_oteryn_restore_quiesced_consumers_after_migrate" in after
    assert 'up -d marketplace-scheduler' in lib


def test_health_probe_helpers_are_repository_pinned_by_digest() -> None:
    lib = LIB.read_text()
    health = (SCRIPTS / "health-check.sh").read_text()
    assert "OTERYN_HEALTH_ALPINE_IMAGE='alpine@sha256:" in lib
    assert "OTERYN_HEALTH_PYTHON_IMAGE='python@sha256:" in lib
    assert "alpine:3.22) args[$i]=\"$OTERYN_HEALTH_ALPINE_IMAGE\"" in lib
    assert "python:3.12-alpine) args[$i]=\"$OTERYN_HEALTH_PYTHON_IMAGE\"" in lib
    assert health.count("alpine:3.22") >= 1
    assert health.count("python:3.12-alpine") == 1


def test_migration_ambiguity_fails_closed_before_migrate() -> None:
    lib = LIB.read_text()
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


def test_recovery_marks_schema_unknown_before_destructive_restore() -> None:
    recovery = (SCRIPTS / "recover-schema.sh").read_text()
    unknown = "printf 'SCHEMA_STATE=unknown\\n'"
    drop = "DROP DATABASE IF EXISTS"
    known = "printf 'SCHEMA_STATE=known\\n'"
    assert unknown in recovery and drop in recovery and known in recovery
    assert recovery.index(unknown) < recovery.index(drop) < recovery.rindex(known)


def test_recovery_stops_optional_marketplace_scheduler_before_drop() -> None:
    recovery = (SCRIPTS / "recover-schema.sh").read_text()
    scheduler_stop = '"${marketplace_compose[@]}" stop marketplace-scheduler'
    drop = "DROP DATABASE IF EXISTS"
    assert "compose.marketplace.yml" in recovery
    assert scheduler_stop in recovery
    assert "Recovery rejected: marketplace-scheduler is still running." in recovery
    assert recovery.index(scheduler_stop) < recovery.index(drop)


def test_image_rollback_never_claims_database_rollback() -> None:
    rollback = (SCRIPTS / "rollback.sh").read_text()
    assert "Database schema was NOT rolled back or changed" in rollback
    assert "migrate:rollback" not in rollback
    assert "migrate:rollback" not in (SCRIPTS / "deploy.sh").read_text()


def main() -> None:
    tests = [
        value
        for name, value in sorted(globals().items())
        if name.startswith("test_") and callable(value)
    ]
    for test in tests:
        test()
    print(f"synology rollback contract: PASS ({len(tests)} tests)")


if __name__ == "__main__":
    main()
