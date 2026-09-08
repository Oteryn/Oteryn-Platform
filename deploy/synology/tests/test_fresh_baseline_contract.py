from __future__ import annotations

import pathlib
import subprocess
import tempfile

ROOT = pathlib.Path(__file__).resolve().parents[3]
SCRIPTS = ROOT / "deploy" / "synology" / "scripts"
ENTRYPOINT = SCRIPTS / "deploy-impact.sh"
DEPLOY = SCRIPTS / "deploy.sh"
FRESH_BASELINE = SCRIPTS / "prepare-fresh-schema-baseline.sh"
RECOVERY = SCRIPTS / "recover-schema.sh"
RELEASE_STATE = SCRIPTS / "release-state.sh"
RECOVERY_WORKFLOW = ROOT / ".github" / "workflows" / "recover-synology-staging-schema.yml"


def test_first_empty_database_gets_recoverable_baseline_before_platform_start() -> None:
    deploy = DEPLOY.read_text()
    helper_call = 'bash "$SCRIPT_DIR/prepare-fresh-schema-baseline.sh"'
    platform_start = '"${compose[@]}" up -d platform'
    assert helper_call in deploy
    assert deploy.index(helper_call) < deploy.index(platform_start)

    helper = FRESH_BASELINE.read_text()
    assert "Fresh baseline rejected: Platform DB is non-empty" in helper
    assert "mariadb-dump" in helper
    assert "BACKUP_BASELINE_KIND=fresh-empty" in helper
    assert "fresh-empty-before-${release_sha}" in helper
    assert '_oteryn_write_schema_state_known "$state_dir" fresh-empty "$release_sha"' in helper
    assert helper.index("mariadb-dump") < helper.index("BACKUP_BASELINE_KIND=fresh-empty")


def test_fresh_empty_recovery_is_bounded_and_verified() -> None:
    recovery = RECOVERY.read_text()
    assert 'baseline_kind="${BACKUP_BASELINE_KIND:-release}"' in recovery
    assert "Fresh-empty recovery evidence must not claim a source application release." in recovery
    assert '"fresh-empty-before-${candidate_sha}"' in recovery
    assert '"$state_dir/current-release.env" "$state_dir/last-good-release.env" "$state_dir/last-good.env"' in recovery
    destructive = recovery.index("DROP " + "DATABASE IF EXISTS")
    verify_empty = recovery.index("restored_table_count=", destructive)
    known = recovery.index('_oteryn_write_schema_state_known "$state_dir" fresh-empty "$candidate_sha"', verify_empty)
    assert destructive < verify_empty < known
    assert "No last-good runtime exists for image rollback" in recovery


def test_recovery_workflow_accepts_release_or_fresh_empty_evidence() -> None:
    workflow = RECOVERY_WORKFLOW.read_text()
    assert "(<old-40-sha>|fresh-empty)-before-<candidate-40-sha>/evidence.env" in workflow
    assert "|fresh-empty)-before-[0-9a-f]{40}/evidence" in workflow
    assert 'ref: ${{ github.sha }}' in workflow


def test_release_state_requires_candidate_to_accept_its_primary_schema() -> None:
    with tempfile.TemporaryDirectory() as td:
        state = pathlib.Path(td) / "candidate.env"
        sha = "2" * 40
        digest = "a" * 64
        result = subprocess.run(
            [
                "bash",
                str(RELEASE_STATE),
                "write",
                str(state),
                sha,
                sha,
                sha,
                "schema-v2",
                "schema-v1",
                f"example/platform@sha256:{digest}",
                f"example/gateway@sha256:{digest}",
                f"example/canary@sha256:{digest}",
                "1",
            ],
            text=True,
            capture_output=True,
            check=False,
        )
        assert result.returncode != 0
        assert "must accept its own primary schema identity 'schema-v2'" in result.stderr
        assert not state.exists()


def test_fast_deploy_uses_accumulated_impact_and_a_narrow_presentation_allowlist() -> None:
    deploy = DEPLOY.read_text()
    presentation = deploy[
        deploy.index("_oteryn_fast_presentation_path()") : deploy.index("_oteryn_fast_non_runtime_path()")
    ]
    assert "resources/*|lang/*|public/css/*|public/js/*|public/images/*" in presentation
    assert "public/index.php" not in presentation
    assert "public/*)" not in presentation
    assert 'git -C "$REPO_ROOT" merge-base --is-ancestor "$FAST_OLD_RELEASE_SHA" "$OTERYN_RELEASE_SHA"' in deploy
    assert "git -C \"$REPO_ROOT\" diff --name-only --no-renames --diff-filter=ACDMRTUXB -z" in deploy
    assert '"$FAST_OLD_RELEASE_SHA" "$OTERYN_RELEASE_SHA" --' in deploy
    assert "accumulated runtime impact includes $path" in deploy
    assert "docs/testing/WIKI_EXPECTED_CONTENT_INVENTORY.json" in deploy
    assert "deploy/synology/BUILD_ROUTING.md" in deploy


def test_fast_deploy_recreates_platform_but_not_full_stack_services() -> None:
    deploy = DEPLOY.read_text()
    fast = deploy[
        deploy.index("_oteryn_fast_deploy_platform()") : deploy.index("fast_candidate_rc=0")
    ]
    assert '"${compose[@]}" up -d --no-deps --force-recreate platform' in fast
    assert "_oteryn_reconcile_marketplace_scheduler_after_runtime_change" in fast
    assert "_oteryn_fast_platform_smoke" in fast
    for service in ("mariadb", "redis", "canary", "internal-proxy", "gateway"):
        assert f"_oteryn_fast_assert_preserved_service {service}" in fast
    for forbidden in (
        "php artisan migrate",
        '"${compose[@]}" up -d mariadb redis',
        '"${compose[@]}" up -d canary',
        'bash "$SCRIPT_DIR/health-check.sh"',
        '"${compose[@]}" up -d gateway',
        "tls-init",
    ):
        assert forbidden not in fast

    full = deploy[deploy.index("stage_bootstrap_files()") :]
    assert '"${compose[@]}" up -d mariadb redis' in full
    assert '"${compose[@]}" up -d canary' in full
    assert 'php artisan migrate --force --no-interaction' in full
    assert 'bash "$SCRIPT_DIR/health-check.sh"' in full
    assert '"${compose[@]}" up -d gateway' in full


def test_fast_deploy_fails_closed_and_restores_previous_release_on_failure() -> None:
    deploy = DEPLOY.read_text()
    preflight = deploy[deploy.index("_oteryn_fast_candidate()") : deploy.index("_oteryn_fast_restore_on_exit()")]
    restore = deploy[deploy.index("_oteryn_fast_restore_on_exit()") : deploy.index("_oteryn_fast_assert_preserved_service()")]
    fast = deploy[deploy.index("_oteryn_fast_deploy_platform()") : deploy.index("fast_candidate_rc=0")]

    assert '[[ ! -f "$state_dir/candidate-release.env" ]]' in preflight
    assert "candidate primary schema identity changed" in preflight
    assert "Gateway provenance changed" in preflight
    assert "Canary provenance changed" in preflight
    assert "staging world identity changed" in preflight
    assert "Fast deploy recovery marker already exists" in preflight

    assert "trap - EXIT" in restore
    assert 'export PLATFORM_IMAGE="$FAST_OLD_PLATFORM_IMAGE"' in restore
    assert "Fast deploy recovery could not prove the previous Platform runtime was restored." in restore
    assert 'mv "$FAST_PREVIOUS_LAST_GOOD" "$state_dir/last-good-release.env"' in restore

    smoke = fast.index("_oteryn_fast_platform_smoke")
    preserved = fast.index("_oteryn_fast_assert_preserved_service mariadb", smoke)
    write_state = fast.index('release-state.sh" write "$state_dir/current-release.env.fast"', preserved)
    promote = fast.index('mv "$state_dir/current-release.env.fast" "$current_file"', write_state)
    assert smoke < preserved < write_state < promote
    assert "trap _oteryn_fast_restore_on_exit EXIT" in fast
    assert "FAST_STATE_PROMOTED=1" in fast
    assert "Fast deploy preflight failed closed; refusing to continue with ambiguous state." in deploy


def test_fast_deploy_smoke_proves_exact_platform_image_binding_and_public_page() -> None:
    deploy = DEPLOY.read_text()
    smoke = deploy[deploy.index("_oteryn_fast_platform_smoke()") : deploy.index("_oteryn_fast_deploy_platform()")]
    assert "docker image inspect --format '{{.Id}}' \"$PLATFORM_IMAGE\"" in smoke
    assert "Fast deploy Platform image identity mismatch." in smoke
    assert "${PLATFORM_BIND_ADDRESS}:${PLATFORM_PORT}" in smoke
    assert '"http://127.0.0.1:${PLATFORM_PORT}/health"' in smoke
    assert '"http://127.0.0.1:${PLATFORM_PORT}/login?locale=en"' in smoke
    assert "Fast deploy public login smoke returned HTTP $status." in smoke


def test_platform_reconcile_keeps_presentation_on_existing_fast_path() -> None:
    entrypoint = ENTRYPOINT.read_text()
    presentation = entrypoint[
        entrypoint.index("_oteryn_presentation_path()") : entrypoint.index("_oteryn_platform_runtime_path()")
    ]
    assert "resources/*|lang/*|public/css/*|public/js/*|public/images/*" in presentation
    assert "presentation-only candidate delegated to platform-fast" in entrypoint
    assert 'CORE_DEPLOY="$SCRIPT_DIR/deploy.sh"' in entrypoint
    assert 'exec bash "$CORE_DEPLOY"' in entrypoint


def test_platform_reconcile_accepts_known_platform_inputs_but_not_schema_or_control_inputs() -> None:
    entrypoint = ENTRYPOINT.read_text()
    runtime = entrypoint[
        entrypoint.index("_oteryn_platform_runtime_path()") : entrypoint.index("_oteryn_non_runtime_path()")
    ]
    for allowed in (
        "composer.json",
        "deploy/synology/docker/platform.Dockerfile",
        "app/*",
        "bootstrap/*",
        "config/*",
        "public/*",
        "resources/*",
        "routes/*",
        "lang/*",
        "storage/*",
    ):
        assert allowed in runtime
    for forbidden in (
        "database/*",
        "deploy/synology/release-contract.env",
        "deploy/synology/docker/platform.Dockerfile.dockerignore",
        "services/game-gateway/",
        "deploy/synology/scripts/deploy.sh",
    ):
        assert forbidden not in runtime

    non_runtime = entrypoint[
        entrypoint.index("_oteryn_non_runtime_path()") : entrypoint.index("_oteryn_running_service_id()")
    ]
    assert "scripts/ci/classify_synology_builds.py" in non_runtime
    assert "deploy/synology/scripts/repository-ghcr-image.sh" in non_runtime
    assert "deploy/synology/scripts/deploy-impact.sh" in non_runtime
    assert "scripts/acceptance/*" in non_runtime


def test_platform_reconcile_uses_accumulated_exact_release_impact() -> None:
    entrypoint = ENTRYPOINT.read_text()
    preflight = entrypoint[
        entrypoint.index("_oteryn_platform_reconcile_candidate()") : entrypoint.index("_oteryn_reconcile_restore_on_exit()")
    ]
    assert 'git -C "$REPO_ROOT" merge-base --is-ancestor "$RECONCILE_OLD_RELEASE_SHA" "$OTERYN_RELEASE_SHA"' in preflight
    assert "git -C \"$REPO_ROOT\" diff --name-only --no-renames --diff-filter=ACDMRTUXB -z" in preflight
    assert '"$RECONCILE_OLD_RELEASE_SHA" "$OTERYN_RELEASE_SHA" --' in preflight
    assert "accumulated runtime/control impact includes $path" in preflight
    assert "candidate primary schema identity changed" in preflight
    assert "candidate does not accept the current schema" in preflight
    assert "Gateway provenance changed" in preflight
    assert "Canary provenance changed" in preflight
    assert "staging world identity changed" in preflight
    assert "Platform reconcile recovery marker already exists" in preflight


def test_platform_reconcile_restarts_only_platform_and_runs_full_health_before_promotion() -> None:
    entrypoint = ENTRYPOINT.read_text()
    reconcile = entrypoint[
        entrypoint.index("_oteryn_platform_reconcile()") : entrypoint.index("reconcile_rc=0")
    ]
    assert '"${compose[@]}" up -d --no-deps --force-recreate platform' in reconcile
    assert "_oteryn_reconcile_marketplace_scheduler_after_runtime_change" in reconcile
    assert "_oteryn_platform_smoke" in reconcile
    health = reconcile.index('bash "$SCRIPT_DIR/health-check.sh"')
    for service in ("mariadb", "redis", "canary", "internal-proxy", "gateway"):
        marker = f"_oteryn_assert_preserved_service {service}"
        assert marker in reconcile
        assert reconcile.index(marker) > health
    write_state = reconcile.index('release-state.sh" write "$state_dir/current-release.env.reconcile"')
    promote = reconcile.index('mv "$state_dir/current-release.env.reconcile" "$current_file"')
    assert health < write_state < promote
    for forbidden in (
        "php artisan migrate",
        '"${compose[@]}" up -d mariadb redis',
        '"${compose[@]}" up -d canary',
        '"${compose[@]}" up -d gateway',
        "tls-init",
    ):
        assert forbidden not in reconcile


def test_platform_reconcile_failure_restores_previous_platform_and_release_state() -> None:
    entrypoint = ENTRYPOINT.read_text()
    restore = entrypoint[
        entrypoint.index("_oteryn_reconcile_restore_on_exit()") : entrypoint.index("_oteryn_assert_preserved_service()")
    ]
    assert "trap - EXIT" in restore
    assert 'export PLATFORM_IMAGE="$RECONCILE_OLD_PLATFORM_IMAGE"' in restore
    assert "Platform reconcile recovery could not prove the previous Platform runtime was restored." in restore
    assert 'mv "$RECONCILE_PREVIOUS_LAST_GOOD" "$state_dir/last-good-release.env"' in restore
    assert "Platform reconcile preflight failed closed; refusing to continue with ambiguous state." in entrypoint


def main() -> None:
    tests = [
        value
        for name, value in sorted(globals().items())
        if name.startswith("test_") and callable(value)
    ]
    for test in tests:
        test()
    print(f"synology fresh baseline contract: PASS ({len(tests)} tests)")


if __name__ == "__main__":
    main()
