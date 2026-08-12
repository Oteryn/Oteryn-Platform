from __future__ import annotations

import pathlib
import re

ROOT = pathlib.Path(__file__).resolve().parents[2]
SCRIPTS = ROOT / "deploy" / "synology" / "scripts"
LIB = SCRIPTS / "lib.sh"
RECOVERY = SCRIPTS / "recover-schema.sh"
ROLLBACK = SCRIPTS / "rollback.sh"
RECOVERY_WORKFLOW = ROOT / ".github" / "workflows" / "recover-synology-staging-schema.yml"
CONTRACT_WORKFLOW = ROOT / ".github" / "workflows" / "synology-rollback-contract.yml"


def test_backup_evidence_binds_destructive_target() -> None:
    lib = LIB.read_text()
    recovery = RECOVERY.read_text()
    for key in ("BACKUP_COMPOSE_PROJECT_NAME", "BACKUP_PLATFORM_DB_NAME"):
        assert f"printf '{key}=%s" in lib
        assert key in recovery
    target_check = recovery.index('[[ "$BACKUP_COMPOSE_PROJECT_NAME" == "$effective_compose_project" ]]')
    db_check = recovery.index('[[ "$BACKUP_PLATFORM_DB_NAME" == "$PLATFORM_DB_NAME" ]]')
    unknown = recovery.index("printf 'SCHEMA_STATE=unknown")
    drop = recovery.index("DROP DATABASE IF EXISTS")
    assert target_check < unknown < drop
    assert db_check < unknown < drop


def test_recovery_uses_state_dir_loaded_from_reconstructed_environment() -> None:
    recovery = RECOVERY.read_text()
    load = recovery.index('load_oteryn_env_file "$ENV_FILE"')
    state = recovery.index('state_dir="${OTERYN_STATE_DIR:-/var/lib/oteryn-staging-state}"')
    evidence = recovery.index('evidence_file="$1"')
    assert load < state < evidence


def test_recovery_workflow_reconstructs_ephemeral_environment() -> None:
    workflow = RECOVERY_WORKFLOW.read_text()
    assert "workflow_dispatch:" in workflow
    assert "environment: synology-staging" in workflow
    assert "group: synology-staging-deployment" in workflow
    assert "backup_evidence:" in workflow
    assert "backup_evidence must be exactly <old-sha>-before-<candidate-sha>/evidence.env." in workflow
    assert "cat > deploy/synology/.env <<EOF" in workflow
    assert "bash deploy/synology/scripts/recover-schema.sh" in workflow
    assert '"$state_dir/backups/$BACKUP_EVIDENCE_INPUT"' in workflow
    assert "if: always()" in workflow
    assert "rm -f deploy/synology/.env" in workflow
    assert "environment: production" not in workflow
    assert 'ref: ${{ github.sha }}' in workflow


def test_recovery_workflow_is_in_focused_ci_trigger() -> None:
    workflow = CONTRACT_WORKFLOW.read_text()
    needle = "- .github/workflows/recover-synology-staging-schema.yml"
    assert workflow.count(needle) == 2
    assert "test_synology_rollback_recovery_contract.py" in workflow
    assert "python3 tests/ci/test_synology_rollback_contract.py" in workflow
    assert "actions/checkout@3d3c42e5aac5ba805825da76410c181273ba90b1" in workflow


def test_marketplace_reconciliation_recreates_platform_and_scheduler() -> None:
    lib = LIB.read_text()
    start = lib.index("_oteryn_recreate_marketplace_runtime()")
    end = lib.index("_oteryn_recreate_marketplace_scheduler()", start)
    body = lib[start:end]
    assert '"${marketplace_compose[@]}" up -d --no-deps --force-recreate platform' in body
    assert '"${marketplace_compose[@]}" up -d --force-recreate marketplace-scheduler' in body
    assert '[[ "$platform_image" == "$PLATFORM_IMAGE" ]]' in body
    assert '[[ "$scheduler_image" == "$PLATFORM_IMAGE" ]]' in body
    assert '[[ "$platform_enabled" == true && "$scheduler_enabled" == true ]]' in body

    rollback = ROLLBACK.read_text()
    reconcile = rollback.index("_oteryn_reconcile_marketplace_scheduler_after_runtime_change")
    health = rollback.index('bash "$SCRIPT_DIR/health-check.sh"')
    promote = rollback.index('cp "$last_good_file" "$state_dir/current-release.env.tmp"')
    assert reconcile < health < promote


def test_same_release_redeploy_preserves_distinct_last_good() -> None:
    lib = LIB.read_text()
    start = lib.index("_oteryn_before_platform_migrate()")
    end = lib.index("_oteryn_after_platform_migrate()", start)
    body = lib[start:end]
    same_release = body.index('if [[ "$old_sha" == "$release_sha" ]]')
    last_good_copy = body.index('cp "$current_file" "$state_dir/last-good-release.env.tmp"')
    assert same_release < last_good_copy
    assert "OTERYN_SAME_RELEASE_REDEPLOY=1" in body
    assert "Same-release redeploy rejected" in body

    wrapper = lib[lib.index("docker()") :]
    assert 'if [[ "${OTERYN_SAME_RELEASE_REDEPLOY:-0}" == 1 ]]' in wrapper
    assert "Same-release redeploy: skipping migration" in wrapper
    skip_block = wrapper.index('if [[ "${OTERYN_SAME_RELEASE_REDEPLOY:-0}" == 1 ]]')
    actual_migrate = wrapper.index('command docker "${args[@]}"', skip_block)
    assert skip_block < actual_migrate


def test_health_probe_helper_pins_are_full_immutable_digests() -> None:
    lib = LIB.read_text()
    alpine = re.search(r"^OTERYN_HEALTH_ALPINE_IMAGE='([^']+)'$", lib, re.MULTILINE)
    python = re.search(r"^OTERYN_HEALTH_PYTHON_IMAGE='([^']+)'$", lib, re.MULTILINE)
    assert alpine is not None
    assert python is not None
    assert re.fullmatch(r"alpine@sha256:[0-9a-f]{64}", alpine.group(1))
    assert re.fullmatch(r"python@sha256:[0-9a-f]{64}", python.group(1))
    assert 'alpine:3.22) args[$i]="$OTERYN_HEALTH_ALPINE_IMAGE"' in lib
    assert 'python:3.12-alpine) args[$i]="$OTERYN_HEALTH_PYTHON_IMAGE"' in lib


def test_rollback_revalidates_last_good_image_revision_before_start() -> None:
    rollback = ROLLBACK.read_text()
    pull = rollback.index('"${compose[@]}" pull platform gateway canary')
    revision = rollback.index('last_good_revision="$(_oteryn_release_sha_for_images "$PLATFORM_IMAGE" "$GATEWAY_IMAGE")"')
    compare = rollback.index('[[ "$last_good_revision" == "$RELEASE_SHA" ]]', revision)
    start = rollback.index('"${compose[@]}" up -d canary platform internal-proxy gateway')
    assert pull < revision < compare < start
    assert "last-good runtime image revision does not match persisted release identity" in rollback


def main() -> None:
    tests = [
        value
        for name, value in sorted(globals().items())
        if name.startswith("test_") and callable(value)
    ]
    for test in tests:
        test()
    print(f"synology rollback recovery contract: PASS ({len(tests)} tests)")


if __name__ == "__main__":
    main()
