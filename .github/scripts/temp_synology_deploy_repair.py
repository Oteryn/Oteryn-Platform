#!/usr/bin/env python3
from pathlib import Path

# 1) Safe exact-candidate resume semantics.
lib_path = Path('deploy/synology/scripts/lib.sh')
lib = lib_path.read_text(encoding='utf-8')
marker = '_oteryn_before_platform_migrate() {\n'
helper = r'''_oteryn_resume_candidate_if_safe() {
    local state_dir="$1" release_sha="$2"
    local candidate_file="$state_dir/candidate-release.env" schema_file="$state_dir/schema-state.env"
    local schema_state schema_id schema_target candidate_schema candidate_accepts
    local -a candidate_runtime

    [[ -f "$candidate_file" ]] || return 1
    bash "$SCRIPT_DIR/release-state.sh" validate "$candidate_file" || return 1
    [[ -f "$schema_file" ]] || {
        echo "Candidate resume rejected: managed schema state is missing." >&2
        return 1
    }

    mapfile -t candidate_runtime < <(bash -c '
        set -euo pipefail
        source "$1"
        printf "%s\n" \
            "$RELEASE_SHA" "$PLATFORM_IMAGE" "$GATEWAY_IMAGE" "$CANARY_IMAGE" \
            "${GAME_WORLD_ID:-}" "${GAME_WORLD_SLUG:-}" "${GAME_WORLD_NAME:-}" \
            "${GAME_WORLD_REGION:-}" "${GAME_WORLD_HOST:-}" "${GAME_WORLD_PORT:-}"
    ' bash "$candidate_file")

    [[ "${candidate_runtime[0]:-}" == "$release_sha" ]] || {
        echo "Deployment rejected: unresolved candidate release ${candidate_runtime[0]:-UNKNOWN} differs from requested release $release_sha." >&2
        return 1
    }
    [[ "${candidate_runtime[1]:-}" == "$PLATFORM_IMAGE" \
        && "${candidate_runtime[2]:-}" == "$GATEWAY_IMAGE" \
        && "${candidate_runtime[3]:-}" == "$CANARY_IMAGE" ]] || {
        echo "Candidate resume rejected: immutable runtime image identity drifted." >&2
        return 1
    }
    [[ "${candidate_runtime[4]:-}" == "${GAME_WORLD_ID:-}" \
        && "${candidate_runtime[5]:-}" == "${GAME_WORLD_SLUG:-}" \
        && "${candidate_runtime[6]:-}" == "${GAME_WORLD_NAME:-}" \
        && "${candidate_runtime[7]:-}" == "${GAME_WORLD_REGION:-}" \
        && "${candidate_runtime[8]:-}" == "${GAME_WORLD_HOST:-}" \
        && "${candidate_runtime[9]:-}" == "${GAME_WORLD_PORT:-}" ]] || {
        echo "Candidate resume rejected: staged world identity drifted." >&2
        return 1
    }

    schema_state="$(_oteryn_read_state_key "$schema_file" SCHEMA_STATE)" || return 1
    schema_id="$(_oteryn_read_state_key "$schema_file" SCHEMA_COMPATIBILITY_ID)" || return 1
    schema_target="$(_oteryn_read_state_key "$schema_file" MIGRATION_TARGET_RELEASE_SHA)" || return 1
    candidate_schema="$(_oteryn_read_state_key "$candidate_file" SCHEMA_COMPATIBILITY_ID)" || return 1
    candidate_accepts="$(_oteryn_read_state_key "$candidate_file" APP_ACCEPTS_SCHEMA_IDS)" || return 1

    [[ "$schema_state" == known && "$schema_target" == "$release_sha" && "$schema_id" == "$candidate_schema" ]] || {
        echo "Candidate resume rejected: prior migration is not proven complete for the exact candidate release." >&2
        return 1
    }
    _oteryn_schema_list_contains "$schema_id" "$candidate_accepts" || {
        echo "Candidate resume rejected: candidate application does not accept the proven schema." >&2
        return 1
    }

    OTERYN_SAME_RELEASE_REDEPLOY=1
    export OTERYN_SAME_RELEASE_REDEPLOY
    echo "Resuming exact candidate release $release_sha after a previously successful migration; preserving recovery evidence until health checks pass."
    return 0
}

'''
if '_oteryn_resume_candidate_if_safe() {' not in lib:
    if marker not in lib:
        raise SystemExit('lib insertion marker missing')
    lib = lib.replace(marker, helper + marker, 1)

old_before = '''_oteryn_before_platform_migrate() {
    local state_dir release_sha backup_dir backup_file backup_meta current_file old_sha old_schema
    state_dir="$(_oteryn_deploy_state_dir)"
    _oteryn_load_candidate_contract || return 1
    release_sha="$(_oteryn_release_sha)" || return 1
    mkdir -p "$state_dir/backups"
'''
new_before = '''_oteryn_before_platform_migrate() {
    local state_dir release_sha backup_dir backup_file backup_meta current_file old_sha old_schema
    state_dir="$(_oteryn_deploy_state_dir)"
    release_sha="$(_oteryn_release_sha)" || return 1

    if [[ -f "$state_dir/candidate-release.env" ]]; then
        _oteryn_resume_candidate_if_safe "$state_dir" "$release_sha" || return 1
        return 0
    fi

    _oteryn_load_candidate_contract || return 1
    mkdir -p "$state_dir/backups"
'''
if old_before not in lib:
    raise SystemExit('before-migrate marker missing')
lib = lib.replace(old_before, new_before, 1)
lib_path.write_text(lib, encoding='utf-8')

baseline_path = Path('deploy/synology/scripts/prepare-fresh-schema-baseline.sh')
baseline = baseline_path.read_text(encoding='utf-8')
old_candidate = '''# A surviving candidate marks an incomplete prior transition. Refuse before the
# lib.sh migration hook can rewrite candidate metadata or reuse its backup path.
if [[ -f "$candidate_file" ]]; then
    bash "$SCRIPT_DIR/release-state.sh" validate "$candidate_file"
    candidate_sha="$(_oteryn_read_state_key "$candidate_file" RELEASE_SHA)"
    echo "Deployment rejected: unresolved candidate release $candidate_sha still owns recovery evidence." >&2
    exit 1
fi
'''
new_candidate = '''# A surviving candidate normally marks an incomplete prior transition. The only
# resumable case is the same exact immutable candidate after its migration has
# already been durably proven known. lib.sh re-validates the same boundary before
# Platform starts and preserves candidate recovery evidence until health passes.
if [[ -f "$candidate_file" ]]; then
    release_sha="$(_oteryn_release_sha)" || exit 1
    _oteryn_resume_candidate_if_safe "$state_dir" "$release_sha" || exit 1
    exit 0
fi
'''
if old_candidate not in baseline:
    raise SystemExit('baseline candidate block missing')
baseline = baseline.replace(old_candidate, new_candidate, 1)
baseline_path.write_text(baseline, encoding='utf-8')

# 2) Eliminate the redundant second full pull. Runtime images are already pulled
# and digest-verified by the guarded workflow; Compose will lazily pull a missing
# infrastructure image at create/up time.
deploy_path = Path('deploy/synology/scripts/deploy.sh')
deploy = deploy_path.read_text(encoding='utf-8')
old_pull = '''snapshot_current_images

"${compose[@]}" config --quiet
"${compose[@]}" pull
stage_bootstrap_files
"${compose[@]}" up -d mariadb redis tls-init
'''
new_pull = '''snapshot_current_images

"${compose[@]}" config --quiet
for runtime_image in "$PLATFORM_IMAGE" "$GATEWAY_IMAGE" "$CANARY_IMAGE"; do
    docker image inspect "$runtime_image" >/dev/null 2>&1 || {
        echo "Guarded workflow did not leave a resolved runtime image locally: $runtime_image" >&2
        exit 1
    }
done
stage_bootstrap_files
"${compose[@]}" up -d mariadb redis
"${compose[@]}" up -d --force-recreate tls-init
'''
if old_pull not in deploy:
    raise SystemExit('deploy full-pull block missing')
deploy = deploy.replace(old_pull, new_pull, 1)
deploy_path.write_text(deploy, encoding='utf-8')

# 3) Make readiness dependency legs explicit before aggregate Gateway /ready.
health_path = Path('deploy/synology/scripts/health-check.sh')
health = health_path.read_text(encoding='utf-8')
old_probes = '''probe_url platform 8000 /health "Platform /health"
probe_url gateway 8080 /health "Gateway /health"
probe_url gateway 8080 /ready "Gateway /ready"
probe_url gateway 8080 /version "Gateway /version"
'''
new_probes = r'''probe_internal_tls() {
    local url="$1"
    local label="$2"
    local project="${COMPOSE_PROJECT_NAME:-oteryn-staging}"
    local network="${project}_private"
    local tls_volume="${project}_internal_tls"

    for _ in $(seq 1 12); do
        if docker run --rm \
            --network "$network" \
            -v "${tls_volume}:/etc/oteryn/tls:ro" \
            -e SSL_CERT_FILE=/etc/oteryn/tls/ca.crt \
            python:3.12-alpine \
            python3 - "$url" <<'PY'
import ssl
import sys
import urllib.request

url = sys.argv[1]
context = ssl.create_default_context(cafile='/etc/oteryn/tls/ca.crt')
try:
    with urllib.request.urlopen(url, timeout=5, context=context) as response:
        if response.status != 200:
            raise SystemExit(f'unexpected HTTP status: {response.status}')
        response.read(8192)
except Exception as exc:
    print(f'internal TLS probe failed: {exc}', file=sys.stderr)
    raise SystemExit(1) from exc
PY
        then
            echo "Verified internal TLS dependency: $label"
            return 0
        fi
        sleep 2
    done

    echo "Internal TLS dependency failed: $label" >&2
    docker logs --tail 80 "${container_ids[internal-proxy]}" >&2 || true
    return 1
}

probe_url platform 8000 /health "Platform /health"
probe_url canary 7180 /health "Canary session issuer /health"
probe_internal_tls "https://platform-internal:8443/health" "Gateway -> Platform"
probe_internal_tls "https://canary-session-internal:8444/health" "Gateway -> Canary session issuer"
probe_url gateway 8080 /health "Gateway /health"
if ! probe_url gateway 8080 /ready "Gateway /ready"; then
    echo "Gateway aggregate readiness failed after both internal dependencies passed." >&2
    docker logs --tail 80 "${container_ids[gateway]}" >&2 || true
    exit 1
fi
probe_url gateway 8080 /version "Gateway /version"
'''
if old_probes not in health:
    raise SystemExit('health probe block missing')
health = health.replace(old_probes, new_probes, 1)
health_path.write_text(health, encoding='utf-8')

# 4) Update focused contract tests for retry safety, no duplicate pull, and
# explicit dependency diagnostics.
test_path = Path('tests/ci/test_synology_rollback_contract.py')
test = test_path.read_text(encoding='utf-8')
test = test.replace(
    '    assert deploy.index("snapshot_current_images") < deploy.index(\'"${compose[@]}" pull\')\n',
    '    assert deploy.index("snapshot_current_images") < deploy.index("stage_bootstrap_files")\n    assert \'"${compose[@]}" pull\' not in deploy\n',
    1,
)
append_marker = '\n\ndef test_legacy_bootstrap_does_not_replace_candidate_image_variables() -> None:\n'
new_tests = r'''

def test_failed_exact_candidate_resume_requires_known_matching_schema_and_identity() -> None:
    lib = LIB.read_text()
    baseline = (SCRIPTS / "prepare-fresh-schema-baseline.sh").read_text()
    assert "_oteryn_resume_candidate_if_safe()" in lib
    assert '[[ "${candidate_runtime[0]:-}" == "$release_sha" ]]' in lib
    assert '"${candidate_runtime[1]:-}" == "$PLATFORM_IMAGE"' in lib
    assert '"${candidate_runtime[2]:-}" == "$GATEWAY_IMAGE"' in lib
    assert '"${candidate_runtime[3]:-}" == "$CANARY_IMAGE"' in lib
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
'''
if 'def test_failed_exact_candidate_resume_requires_known_matching_schema_and_identity' not in test:
    if append_marker not in test:
        raise SystemExit('test insertion marker missing')
    test = test.replace(append_marker, new_tests + append_marker, 1)
test_path.write_text(test, encoding='utf-8')

# Extend the auto-deploy focused test with performance/readiness invariants.
auto_path = Path('tests/ci/test_synology_auto_staging_deploy.py')
auto = auto_path.read_text(encoding='utf-8')
insert = '\n\nif __name__ == "__main__":\n'
methods = r'''
    def test_runtime_deploy_avoids_duplicate_full_compose_pull(self) -> None:
        self.assertNotIn('"${compose[@]}" pull', self.deploy_script)
        self.assertIn('docker image inspect "$runtime_image"', self.deploy_script)

    def test_runtime_health_names_gateway_dependency_failures_before_ready(self) -> None:
        health = (ROOT / "deploy/synology/scripts/health-check.sh").read_text(encoding="utf-8")
        self.assertIn('Canary session issuer /health', health)
        self.assertIn('Gateway -> Platform', health)
        self.assertIn('Gateway -> Canary session issuer', health)
        self.assertIn('Gateway aggregate readiness failed after both internal dependencies passed.', health)
'''
if 'def test_runtime_deploy_avoids_duplicate_full_compose_pull' not in auto:
    if insert not in auto:
        raise SystemExit('auto test insertion marker missing')
    auto = auto.replace(insert, methods + insert, 1)
auto_path.write_text(auto, encoding='utf-8')

Path('.github/scripts/temp_synology_deploy_repair.py').unlink()
