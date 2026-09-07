from pathlib import Path


def replace_once(text: str, old: str, new: str, label: str) -> str:
    count = text.count(old)
    if count != 1:
        raise SystemExit(f"{label}: expected exactly one match, found {count}")
    return text.replace(old, new, 1)


root = Path(".")
health_path = root / "deploy/synology/scripts/health-check.sh"
deploy_path = root / "deploy/synology/scripts/deploy.sh"
auto_test_path = root / "tests/ci/test_synology_auto_staging_deploy.py"
recovery_test_path = root / "tests/ci/test_synology_rollback_recovery_contract.py"

health = health_path.read_text()
health = replace_once(
    health,
    'load_oteryn_env_file "$ENV_FILE"\n\ncompose=(docker compose --env-file "$ENV_FILE" -f "$COMPOSE_FILE")\n',
    'load_oteryn_env_file "$ENV_FILE"\n\n'
    'health_profile="${OTERYN_HEALTH_PROFILE:-full}"\n'
    'case "$health_profile" in\n'
    '    full|recovery) ;;\n'
    '    *)\n'
    '        echo "OTERYN_HEALTH_PROFILE must be full or recovery." >&2\n'
    '        exit 1\n'
    '        ;;\n'
    'esac\n\n'
    'compose=(docker compose --env-file "$ENV_FILE" -f "$COMPOSE_FILE")\n',
    "health profile validation",
)

canary_block = '''if ! docker run --rm \\
    --network "container:${container_ids[canary]}" \\
    alpine:3.22 \\
    /bin/sh -ec \\
    "nc -z -w 3 127.0.0.1 '${CANARY_GAME_PORT}'"; then
    echo "Canary game TCP port is not reachable inside the Canary network namespace." >&2
    exit 1
fi

if [[ "$CANARY_GAME_BIND_ADDRESS" != "127.0.0.1" ]]; then
    if ! timeout 5 bash -c "exec 3<>/dev/tcp/${CANARY_GAME_BIND_ADDRESS}/${CANARY_GAME_PORT}"; then
        echo "Canary game TCP port is not reachable through the configured Synology LAN address." >&2
        exit 1
    fi
    echo "Verified LAN game endpoint: ${CANARY_GAME_BIND_ADDRESS}:${CANARY_GAME_PORT}"
fi

'''
health = replace_once(health, canary_block, "", "existing Canary TCP block")
health = replace_once(
    health,
    'PY\n\nplatform_container="${container_ids[platform]}"\n',
    'PY\n\n'
    + canary_block
    + 'if [[ "$health_profile" == recovery ]]; then\n'
      '    echo "Platform, Gateway, Canary stable recovery probes passed."\n'
      '    exit 0\n'
      'fi\n\n'
      'platform_container="${container_ids[platform]}"\n',
    "recovery gate placement",
)
health_path.write_text(health)

deploy = deploy_path.read_text()
deploy = replace_once(
    deploy,
    '    if ! OTERYN_ENV_FILE="$candidate_env" bash "$SCRIPT_DIR/health-check.sh"; then\n'
    '        rm -f "$candidate_env"\n'
    '        echo "Previous candidate remains unresolved because its full staging health contract failed." >&2\n',
    '    if ! OTERYN_HEALTH_PROFILE=recovery OTERYN_ENV_FILE="$candidate_env" bash "$SCRIPT_DIR/health-check.sh"; then\n'
    '        rm -f "$candidate_env"\n'
    '        echo "Previous candidate remains unresolved because its stable recovery health contract failed." >&2\n',
    "candidate recovery health invocation",
)
deploy_path.write_text(deploy)

auto_tests = auto_test_path.read_text()
auto_tests = replace_once(
    auto_tests,
    '        self.assertIn(\'OTERYN_ENV_FILE="$candidate_env" bash "$SCRIPT_DIR/health-check.sh"\', self.deploy_script)\n'
    '        self.assertIn(\'Finalized previously migrated candidate\', self.deploy_script)\n',
    '        self.assertIn(\'OTERYN_HEALTH_PROFILE=recovery OTERYN_ENV_FILE="$candidate_env" bash "$SCRIPT_DIR/health-check.sh"\', self.deploy_script)\n'
    '        self.assertIn(\'Finalized previously migrated candidate\', self.deploy_script)\n'
    '\n'
    '    def test_previous_candidate_recovery_profile_stays_stable_across_feature_checks(self) -> None:\n'
    '        health = (ROOT / "deploy/synology/scripts/health-check.sh").read_text(encoding="utf-8")\n'
    '        self.assertIn(\'health_profile="${OTERYN_HEALTH_PROFILE:-full}"\', health)\n'
    '        recovery_gate = health.index(\'if [[ "$health_profile" == recovery ]]\')\n'
    '        canary_tcp = health.index("Canary game TCP port is not reachable inside the Canary network namespace.")\n'
    '        mfa_feature = health.index("MfaQrCode")\n'
    '        self.assertLess(canary_tcp, recovery_gate)\n'
    '        self.assertLess(recovery_gate, mfa_feature)\n'
    '        self.assertIn("Platform, Gateway, Canary stable recovery probes passed.", health)\n',
    "auto staging recovery profile contract",
)
auto_test_path.write_text(auto_tests)

recovery_tests = recovery_test_path.read_text()
recovery_tests = replace_once(
    recovery_tests,
    '    assert \'OTERYN_ENV_FILE="$candidate_env" bash "$SCRIPT_DIR/health-check.sh"\' in body\n'
    '    assert \'game-auth:world:ensure\' in body\n\n'
    '    health = body.index(\'OTERYN_ENV_FILE="$candidate_env" bash "$SCRIPT_DIR/health-check.sh"\')\n',
    '    assert \'OTERYN_HEALTH_PROFILE=recovery OTERYN_ENV_FILE="$candidate_env" bash "$SCRIPT_DIR/health-check.sh"\' in body\n'
    '    assert "stable recovery health contract failed" in body\n'
    '    assert \'game-auth:world:ensure\' in body\n\n'
    '    health = body.index(\'OTERYN_HEALTH_PROFILE=recovery OTERYN_ENV_FILE="$candidate_env" bash "$SCRIPT_DIR/health-check.sh"\')\n',
    "rollback recovery profile assertions",
)
recovery_test_path.write_text(recovery_tests)
