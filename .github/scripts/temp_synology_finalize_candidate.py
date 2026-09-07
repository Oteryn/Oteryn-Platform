#!/usr/bin/env python3
from pathlib import Path

DEPLOY = Path('deploy/synology/scripts/deploy.sh')
RECOVERY_TEST = Path('tests/ci/test_synology_rollback_recovery_contract.py')
AUTO_TEST = Path('tests/ci/test_synology_auto_staging_deploy.py')

text = DEPLOY.read_text(encoding='utf-8')
insert_marker = '''apply_sql_template() {
'''
helper = r'''finalize_previous_candidate_if_healthy() {
    local candidate_file="$state_dir/candidate-release.env"
    local schema_file="$state_dir/schema-state.env"
    local current_file="$state_dir/current-release.env"
    local last_good_file="$state_dir/last-good-release.env"
    local requested_sha candidate_sha candidate_schema candidate_accepts
    local schema_state schema_id schema_target current_sha
    local candidate_env="$state_dir/candidate-health.env"
    local service container_id image_id resolved_image expected_image
    local index
    local -a candidate_state services expected_images candidate_compose

    [[ -f "$candidate_file" ]] || return 0
    bash "$SCRIPT_DIR/release-state.sh" validate "$candidate_file" || return 1
    requested_sha="$(_oteryn_release_sha)" || return 1

    mapfile -t candidate_state < <(bash -c '
        set -euo pipefail
        source "$1"
        printf "%s\n" \
            "$RELEASE_SHA" "$PLATFORM_IMAGE" "$GATEWAY_IMAGE" "$CANARY_IMAGE" \
            "${GAME_WORLD_ID:-}" "${GAME_WORLD_SLUG:-}" "${GAME_WORLD_NAME:-}" \
            "${GAME_WORLD_REGION:-}" "${GAME_WORLD_HOST:-}" "${GAME_WORLD_PORT:-}" \
            "$SCHEMA_COMPATIBILITY_ID" "$APP_ACCEPTS_SCHEMA_IDS"
    ' bash "$candidate_file")

    candidate_sha="${candidate_state[0]:-}"
    [[ "$candidate_sha" != "$requested_sha" ]] || return 0

    [[ -f "$schema_file" ]] || {
        echo "Previous candidate finalization rejected: managed schema state is missing." >&2
        return 1
    }
    schema_state="$(_oteryn_read_state_key "$schema_file" SCHEMA_STATE)" || return 1
    schema_id="$(_oteryn_read_state_key "$schema_file" SCHEMA_COMPATIBILITY_ID)" || return 1
    schema_target="$(_oteryn_read_state_key "$schema_file" MIGRATION_TARGET_RELEASE_SHA)" || return 1
    candidate_schema="${candidate_state[10]:-}"
    candidate_accepts="${candidate_state[11]:-}"
    [[ "$schema_state" == known && "$schema_target" == "$candidate_sha" && "$schema_id" == "$candidate_schema" ]] || {
        echo "Previous candidate finalization rejected: its migration is not proven complete." >&2
        return 1
    }
    _oteryn_schema_list_contains "$schema_id" "$candidate_accepts" || {
        echo "Previous candidate finalization rejected: candidate does not accept the proven schema." >&2
        return 1
    }

    [[ "${candidate_state[4]:-}" == "$GAME_WORLD_ID" \
        && "${candidate_state[5]:-}" == "$GAME_WORLD_SLUG" \
        && "${candidate_state[6]:-}" == "$GAME_WORLD_NAME" \
        && "${candidate_state[7]:-}" == "$GAME_WORLD_REGION" \
        && "${candidate_state[8]:-}" == "$GAME_WORLD_HOST" \
        && "${candidate_state[9]:-}" == "$GAME_WORLD_PORT" ]] || {
        echo "Previous candidate finalization rejected: staging world identity drifted." >&2
        return 1
    }

    services=(platform gateway canary)
    expected_images=("${candidate_state[1]}" "${candidate_state[2]}" "${candidate_state[3]}")
    for index in "${!services[@]}"; do
        service="${services[$index]}"
        expected_image="${expected_images[$index]}"
        container_id="$("${compose[@]}" ps -q "$service" 2>/dev/null || true)"
        [[ -n "$container_id" && "$(docker inspect --format '{{.State.Running}}' "$container_id")" == true ]] || {
            echo "Previous candidate finalization rejected: $service is not running." >&2
            return 1
        }
        image_id="$(docker inspect --format '{{.Image}}' "$container_id")" || return 1
        resolved_image="$(bash "$SCRIPT_DIR/release-state.sh" resolve-image "$image_id")" || return 1
        [[ "$resolved_image" == "$expected_image" ]] || {
            echo "Previous candidate finalization rejected: running $service image does not match candidate recovery identity." >&2
            return 1
        }
    done

    awk \
        -v platform_image="${candidate_state[1]}" \
        -v gateway_image="${candidate_state[2]}" \
        -v canary_image="${candidate_state[3]}" \
        -v gateway_version="sha-${candidate_sha}" '
        BEGIN { p=0; g=0; c=0; v=0 }
        /^PLATFORM_IMAGE=/ { print "PLATFORM_IMAGE=" platform_image; p=1; next }
        /^GATEWAY_IMAGE=/ { print "GATEWAY_IMAGE=" gateway_image; g=1; next }
        /^CANARY_IMAGE=/ { print "CANARY_IMAGE=" canary_image; c=1; next }
        /^GATEWAY_VERSION=/ { print "GATEWAY_VERSION=" gateway_version; v=1; next }
        { print }
        END { if (!(p && g && c && v)) exit 42 }
    ' "$ENV_FILE" >"$candidate_env.tmp" || {
        rm -f "$candidate_env.tmp"
        echo "Previous candidate finalization rejected: unable to construct bounded candidate health environment." >&2
        return 1
    }
    chmod 600 "$candidate_env.tmp"
    mv "$candidate_env.tmp" "$candidate_env"

    candidate_compose=(docker compose --env-file "$candidate_env" -f "$COMPOSE_FILE")
    if ! "${candidate_compose[@]}" config --quiet; then
        rm -f "$candidate_env"
        return 1
    fi
    if ! "${candidate_compose[@]}" up -d --no-deps --force-recreate internal-proxy gateway; then
        rm -f "$candidate_env"
        return 1
    fi
    if ! OTERYN_ENV_FILE="$candidate_env" bash "$SCRIPT_DIR/health-check.sh"; then
        rm -f "$candidate_env"
        echo "Previous candidate remains unresolved because its full staging health contract failed." >&2
        return 1
    fi

    if ! "${candidate_compose[@]}" exec -T platform php artisan game-auth:world:ensure \
        --id="$GAME_WORLD_ID" \
        --slug="$GAME_WORLD_SLUG" \
        --name="$GAME_WORLD_NAME" \
        --region="$GAME_WORLD_REGION" \
        --host="$GAME_WORLD_HOST" \
        --port="$GAME_WORLD_PORT" \
        --status=online \
        --login-enabled=1; then
        rm -f "$candidate_env"
        return 1
    fi
    rm -f "$candidate_env"

    if [[ -f "$current_file" ]]; then
        bash "$SCRIPT_DIR/release-state.sh" validate "$current_file" || return 1
        current_sha="$(_oteryn_read_state_key "$current_file" RELEASE_SHA)" || return 1
        if [[ "$current_sha" != "$candidate_sha" ]]; then
            cp "$current_file" "$last_good_file.tmp"
            chmod 600 "$last_good_file.tmp"
            mv "$last_good_file.tmp" "$last_good_file"
        fi
    fi

    cp "$candidate_file" "$current_file.tmp"
    chmod 600 "$current_file.tmp"
    mv "$current_file.tmp" "$current_file"
    rm -f "$candidate_file"
    echo "Finalized previously migrated candidate $candidate_sha after the full staging health contract passed."
}

'''
if 'finalize_previous_candidate_if_healthy() {' not in text:
    if insert_marker not in text:
        raise SystemExit('deploy helper insertion marker missing')
    text = text.replace(insert_marker, helper + insert_marker, 1)

call_marker = '''# The first managed migration of a truly empty Platform DB receives its own
# verified pre-migration dump. This runs immediately before lib.sh's migration
# preparation hook and before the candidate Platform container can start.
OTERYN_ENV_FILE="$ENV_FILE" bash "$SCRIPT_DIR/prepare-fresh-schema-baseline.sh"
'''
call_replacement = '''# A previous candidate whose migration succeeded but whose post-migration health
# gate failed must be resolved before a newer release can own recovery state.
# Finalization is allowed only after exact running-image/world/schema identity and
# the complete current staging health contract pass.
finalize_previous_candidate_if_healthy

# The first managed migration of a truly empty Platform DB receives its own
# verified pre-migration dump. This runs immediately before lib.sh's migration
# preparation hook and before the candidate Platform container can start.
OTERYN_ENV_FILE="$ENV_FILE" bash "$SCRIPT_DIR/prepare-fresh-schema-baseline.sh"
'''
if 'finalize_previous_candidate_if_healthy\n\n# The first managed migration' not in text:
    if call_marker not in text:
        raise SystemExit('deploy finalizer call marker missing')
    text = text.replace(call_marker, call_replacement, 1)
DEPLOY.write_text(text, encoding='utf-8')

recovery = RECOVERY_TEST.read_text(encoding='utf-8')
recovery_marker = '''\ndef test_health_probe_helper_pins_are_full_immutable_digests() -> None:\n'''
recovery_tests = r'''

def test_new_release_finalizes_only_a_proven_healthy_previous_candidate() -> None:
    deploy = (SCRIPTS / "deploy.sh").read_text()
    start = deploy.index("finalize_previous_candidate_if_healthy()")
    end = deploy.index("apply_sql_template()", start)
    body = deploy[start:end]

    assert '[[ "$candidate_sha" != "$requested_sha" ]] || return 0' in body
    assert '[[ "$schema_state" == known && "$schema_target" == "$candidate_sha" && "$schema_id" == "$candidate_schema" ]]' in body
    assert "candidate does not accept the proven schema" in body
    assert "staging world identity drifted" in body
    assert "running $service image does not match candidate recovery identity" in body
    assert 'OTERYN_ENV_FILE="$candidate_env" bash "$SCRIPT_DIR/health-check.sh"' in body
    assert 'game-auth:world:ensure' in body

    health = body.index('OTERYN_ENV_FILE="$candidate_env" bash "$SCRIPT_DIR/health-check.sh"')
    world = body.index('game-auth:world:ensure', health)
    promote = body.index('cp "$candidate_file" "$current_file.tmp"', world)
    clear = body.index('rm -f "$candidate_file"', promote)
    assert health < world < promote < clear

    call = deploy.index("\nfinalize_previous_candidate_if_healthy\n", end)
    baseline = deploy.index('bash "$SCRIPT_DIR/prepare-fresh-schema-baseline.sh"', call)
    assert call < baseline


def test_previous_candidate_finalizer_never_blindly_discards_recovery_state() -> None:
    deploy = (SCRIPTS / "deploy.sh").read_text()
    start = deploy.index("finalize_previous_candidate_if_healthy()")
    end = deploy.index("apply_sql_template()", start)
    body = deploy[start:end]
    clear = body.index('rm -f "$candidate_file"')
    assert body.index('health-check.sh') < clear
    assert body.index('game-auth:world:ensure') < clear
    assert body.index('running $service image does not match candidate recovery identity') < clear
'''
if 'def test_new_release_finalizes_only_a_proven_healthy_previous_candidate' not in recovery:
    if recovery_marker not in recovery:
        raise SystemExit('recovery test insertion marker missing')
    recovery = recovery.replace(recovery_marker, recovery_tests + recovery_marker, 1)
RECOVERY_TEST.write_text(recovery, encoding='utf-8')

auto = AUTO_TEST.read_text(encoding='utf-8')
auto_marker = '''\n\nif __name__ == "__main__":\n'''
auto_method = r'''
    def test_new_main_can_finalize_a_proven_previous_candidate_before_transition(self) -> None:
        self.assertIn("finalize_previous_candidate_if_healthy()", self.deploy_script)
        finalizer_call = self.deploy_script.index("\nfinalize_previous_candidate_if_healthy\n")
        baseline = self.deploy_script.index('bash "$SCRIPT_DIR/prepare-fresh-schema-baseline.sh"', finalizer_call)
        self.assertLess(finalizer_call, baseline)
        self.assertIn('OTERYN_ENV_FILE="$candidate_env" bash "$SCRIPT_DIR/health-check.sh"', self.deploy_script)
        self.assertIn('Finalized previously migrated candidate', self.deploy_script)
'''
if 'def test_new_main_can_finalize_a_proven_previous_candidate_before_transition' not in auto:
    if auto_marker not in auto:
        raise SystemExit('auto test insertion marker missing')
    auto = auto.replace(auto_marker, auto_method + auto_marker, 1)
AUTO_TEST.write_text(auto, encoding='utf-8')

Path('.github/scripts/temp_synology_finalize_candidate.py').unlink()
