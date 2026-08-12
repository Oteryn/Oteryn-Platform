from __future__ import annotations

import pathlib

ROOT = pathlib.Path(__file__).resolve().parents[3]
SCRIPTS = ROOT / "deploy" / "synology" / "scripts"
DEPLOY = SCRIPTS / "deploy.sh"
FRESH_BASELINE = SCRIPTS / "prepare-fresh-schema-baseline.sh"
RECOVERY = SCRIPTS / "recover-schema.sh"
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
