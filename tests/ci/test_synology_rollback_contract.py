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
                f"PLATFORM_SOURCE_SHA={sha}",
                f"GATEWAY_SOURCE_SHA={sha}",
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


def run_release_sha(
    platform_revision: str,
    gateway_revision: str,
    *,
    release_sha: str = NEW_SHA,
    platform_source_sha: str | None = None,
    gateway_source_sha: str | None = None,
) -> subprocess.CompletedProcess[str]:
    platform_source_sha = platform_source_sha or platform_revision
    gateway_source_sha = gateway_source_sha or gateway_revision
    with tempfile.TemporaryDirectory() as td:
        root = pathlib.Path(td)
        docker = root / "docker"
        docker.write_text(
            "#!/usr/bin/env bash\n"
            "set -euo pipefail\n"
            "ref=${@: -1}\n"
            f"case \"$ref\" in *platform*) printf '%s\\n' '{platform_revision}' ;; *gateway*) printf '%s\\n' '{gateway_revision}' ;; *) exit 9 ;; esac\n"
        )
        docker.chmod(0o755)
        env = os.environ.copy()
        env.update(
            {
                "PATH": f"{root}:{env['PATH']}",
                "PLATFORM_IMAGE": f"example/platform@sha256:{DIGEST_A}",
                "GATEWAY_IMAGE": f"example/gateway@sha256:{DIGEST_B}",
                "OTERYN_RELEASE_SHA": release_sha,
                "PLATFORM_SOURCE_SHA": platform_source_sha,
                "GATEWAY_SOURCE_SHA": gateway_source_sha,
                "GATEWAY_VERSION": f"sha-{gateway_source_sha}",
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
                OLD_SHA,
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
        state_text = state.read_text()
        assert f"PLATFORM_SOURCE_SHA={OLD_SHA}" in state_text
        assert f"GATEWAY_SOURCE_SHA={NEW_SHA}" in state_text
        assert "GAME_WORLD_NAME=Oteryn\\ Staging" in state_text
        read = subprocess.run(
            ["bash", "-c", 'source "$1"; printf "%s" "$GAME_WORLD_NAME"', "bash", str(state)],
            text=True,
            capture_output=True,
            check=False,
        )
        assert read.returncode == 0, read.stderr
        assert read.stdout == "Oteryn Staging"


def test_release_sha_accepts_independent_component_provenance() -> None:
    result = run_release_sha(OLD_SHA, NEW_SHA, release_sha=NEW_SHA)
    assert result.returncode == 0, result.stderr
    assert result.stdout.strip() == NEW_SHA


def test_release_sha_rejects_component_revision_mismatch() -> None:
    result = run_release_sha(OLD_SHA, NEW_SHA, platform_source_sha=NEW_SHA)
    assert result.returncode != 0
    assert "Platform OCI application revision does not match its persisted component source SHA" in result.stderr


def test_release_sha_requires_explicit_overall_release_identity() -> None:
    result = run_release_sha(NEW_SHA, NEW_SHA, release_sha="")
    assert result.returncode != 0
    assert "OTERYN_RELEASE_SHA must be the exact protected-main release identity" in result.stderr


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
    snapshot_call = deploy.index("\nsnapshot_current_images\n")
    stage_call = deploy.index("\nstage_bootstrap_files\n", snapshot_call)
    assert snapshot_call < stage_call
    assert '"${compose[@]}" pull' not in deploy
    assert "observed_schema=\"observed-${old_sha}\"" in lib
    assert "_oteryn_write_schema_state_known \"$state_dir\" \"$observed_schema\" \"$old_sha\"" in lib
    assert "Legacy running-release snapshot is incomplete; refusing migration." in lib


def test_failed_exact_candidate_resume_requires_known_matching_schema_and_identity() -> None:
    lib = LIB.read_text()
    baseline = (SCRIPTS / "prepare-fresh-schema-baseline.sh").read_text()
    assert "_oteryn_resume_candidate_if_safe()" in lib
    assert '[[ "${candidate_runtime[0]:-}" == "$release_sha" ]]' in lib
    assert '"${candidate_runtime[1]:-}" == "$PLATFORM_SOURCE_SHA"' in lib
    assert '"${candidate_runtime[2]:-}" == "$GATEWAY_SOURCE_SHA"' in lib
    assert '"${candidate_runtime[3]:-}" == "$PLATFORM_IMAGE"' in lib
    assert '"${candidate_runtime[4]:-}" == "$GATEWAY_IMAGE"' in lib
    assert '"${candidate_runtime[5]:-}" == "$CANARY_IMAGE"' in lib
    assert '[[ "$schema_state" == known && "$schema_target" == "$release_sha" && "$schema_id" == "$candidate_schema" ]]' in lib
    assert "prior migration is not proven complete" in lib
    assert "preserving recovery evidence until health checks pass" in lib
    assert '_oteryn_resume_candidate_if_safe "$state_dir" "$release_sha"' in baseline
    assert "unresolved candidate release $candidate_sha still owns recovery evidence" not in baseline


def test_deploy_does_not_pull_all_images_twice_and_forces_tls_bootstrap_refresh() -> None:
    deploy = (SCRIPTS / "deploy.sh").read_text()
    assert '"${compose[@]}" pull' not in deploy
    assert 'docker image inspect "$runtime_image"' in deploy
    assert 'up -d --force-recreate tls-init' in deploy


def test_health_check_proves_each_gateway_dependency_before_aggregate_ready() -> None:
    health = (SCRIPTS / "health-check.sh").read_text()
    canary = health.index('probe_url canary 7180 /health "Canary session issuer /health"')
    platform_tls = health.index('probe_internal_tls "https://platform-internal:8443/health"')
    canary_tls = health.index('probe_internal_tls "https://canary-session-internal:8444/health"')
    gateway_ready = health.index('probe_url gateway 8080 /ready "Gateway /ready"')
    assert canary < platform_tls < canary_tls < gateway_ready
    assert "ssl.create_default_context(cafile='/etc/oteryn/tls/ca.crt')" in health
    assert 'docker logs --tail 80 "${container_ids[internal-proxy]}"' in health
    assert 'docker logs --tail 80 "${container_ids[gateway]}"' in health


def test_legacy_bootstrap_does_not_replace_candidate_image_variables() -> None:
    lib = LIB.read_text()
    start = lib.index("_oteryn_bootstrap_legacy_current_release()")
    end = lib.index("_oteryn_marketplace_state_file()", start)
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


def test_pre_migration_backup_uses_actual_known_schema_identity() -> None:
    lib = LIB.read_text()
    before = lib[lib.index("_oteryn_before_platform_migrate()") : lib.index("_oteryn_after_platform_migrate()")]
    assert 'old_schema="$(_oteryn_known_schema_identity "$state_dir")"' in before
    assert 'old_schema="$(_oteryn_read_state_key "$current_file" SCHEMA_COMPATIBILITY_ID)"' not in before
    assert "BACKUP_SCHEMA_COMPATIBILITY_ID=%s" in before


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
    assert 'command docker stop "$scheduler_id"' in quiesce
    after = lib[lib.index("_oteryn_after_platform_migrate()") : lib.index("_oteryn_finalize_release_on_exit()")]
    assert "_oteryn_restore_quiesced_consumers_after_migrate" in after
    assert 'up -d --force-recreate marketplace-scheduler' in lib


def test_marketplace_scheduler_recreation_uses_effective_or_durable_state() -> None:
    lib = LIB.read_text()
    assert "printf '%s/marketplace.env\\n'" in lib
    assert "grep -q '^MARKETPLACE_ENABLED=' \"$ENV_FILE\"" in lib
    assert "Unexpected durable Marketplace state key" in lib
    assert "_oteryn_load_marketplace_runtime_state" in lib
    assert 'scheduler_image="$(command docker inspect --format \'{{.Config.Image}}\' "$scheduler_id")"' in lib
    assert '[[ "$scheduler_image" == "$PLATFORM_IMAGE" ]]' in lib


def test_health_probe_helpers_are_repository_pinned_by_digest() -> None:
    lib = LIB.read_text()
    health = (SCRIPTS / "health-check.sh").read_text()
    assert "OTERYN_HEALTH_ALPINE_IMAGE='alpine@sha256:" in lib
    assert "OTERYN_HEALTH_PYTHON_IMAGE='python@sha256:" in lib
    assert "alpine:3.22) args[$i]=\"$OTERYN_HEALTH_ALPINE_IMAGE\"" in lib
    assert "python:3.12-alpine) args[$i]=\"$OTERYN_HEALTH_PYTHON_IMAGE\"" in lib
    assert health.count("alpine:3.22") >= 1
    assert health.count("python:3.12-alpine") >= 2


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


def test_recovery_evidence_is_allowlisted_not_sourced() -> None:
    recovery = (SCRIPTS / "recover-schema.sh").read_text()
    assert 'source "$evidence_file"' not in recovery
    assert "Recovery evidence contains unexpected key" in recovery
    assert "Recovery evidence contains duplicate key" in recovery
    for key in (
        "BACKUP_FROM_RELEASE_SHA",
        "BACKUP_BEFORE_RELEASE_SHA",
        "BACKUP_SCHEMA_COMPATIBILITY_ID",
        "BACKUP_SHA256",
    ):
        assert key in recovery


def test_recovery_accepts_actual_backup_schema_via_last_good_contract() -> None:
    recovery = (SCRIPTS / "recover-schema.sh").read_text()
    assert 'compatible-schema \\\n    "$BACKUP_SCHEMA_COMPATIBILITY_ID" "$last_good_file" "$candidate_sha"' in recovery
    assert '[[ "$SCHEMA_COMPATIBILITY_ID" == "$BACKUP_SCHEMA_COMPATIBILITY_ID" ]]' not in recovery


def test_recovery_marks_schema_unknown_before_destructive_restore() -> None:
    recovery = (SCRIPTS / "recover-schema.sh").read_text()
    unknown = "printf 'SCHEMA_STATE=unknown\\n'"
    drop = "DROP DATABASE IF EXISTS"
    known = '_oteryn_write_schema_state_known "$state_dir"'
    assert unknown in recovery and drop in recovery and known in recovery
    assert recovery.index(unknown) < recovery.index(drop) < recovery.rindex(known)


def test_recovery_stops_optional_marketplace_scheduler_before_drop() -> None:
    recovery = (SCRIPTS / "recover-schema.sh").read_text()
    scheduler_stop = 'command docker stop "$marketplace_scheduler_id"'
    drop = "DROP DATABASE IF EXISTS"
    assert "_oteryn_marketplace_scheduler_id" in recovery
    assert scheduler_stop in recovery
    assert "Recovery rejected: marketplace-scheduler is still running." in recovery
    assert recovery.index(scheduler_stop) < recovery.index(drop)


def test_runtime_rollback_reconciles_marketplace_before_release_promotion() -> None:
    rollback = (SCRIPTS / "rollback.sh").read_text()
    reconcile = rollback.index("_oteryn_reconcile_marketplace_scheduler_after_runtime_change")
    promote = rollback.index('cp "$last_good_file" "$state_dir/current-release.env.tmp"')
    assert reconcile < promote
    lib = LIB.read_text()
    reconcile_body = lib[lib.index("_oteryn_reconcile_marketplace_scheduler_after_runtime_change()") : lib.index("_oteryn_before_platform_migrate()")]
    assert "_oteryn_recreate_marketplace_scheduler" in reconcile_body
    assert 'command docker rm -f "$scheduler_id"' in reconcile_body


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
