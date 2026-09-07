from pathlib import Path


def replace_once(text: str, old: str, new: str, label: str) -> str:
    count = text.count(old)
    if count != 1:
        raise SystemExit(f"{label}: expected exactly one match, found {count}")
    return text.replace(old, new, 1)


deploy_path = Path("deploy/synology/scripts/deploy.sh")
deploy = deploy_path.read_text()

deploy = replace_once(
    deploy,
    "    local service container_id image_id expected_image expected_image_id\n",
    "    local service container_id image_id expected_image expected_image_id\n"
    "    local gateway_container_id gateway_image_id expected_gateway_image expected_gateway_image_id\n",
    "candidate finalizer local variables",
)

old_block = '''    services=(platform gateway canary)
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
        expected_image_id="$(docker image inspect --format '{{.Id}}' "$expected_image" 2>/dev/null || true)"
        [[ -n "$expected_image_id" && "$image_id" == "$expected_image_id" ]] || {
            echo "Previous candidate finalization rejected: running $service image does not match candidate recovery identity." >&2
            return 1
        }
    done
'''
new_block = '''    # Platform and Canary are immutable anchors for recovery. Gateway is intentionally
    # reconstructible because finalization force-recreates it from the persisted candidate image
    # before the full health contract can promote the candidate.
    services=(platform canary)
    expected_images=("${candidate_state[1]}" "${candidate_state[3]}")
    for index in "${!services[@]}"; do
        service="${services[$index]}"
        expected_image="${expected_images[$index]}"
        container_id="$("${compose[@]}" ps -q "$service" 2>/dev/null || true)"
        [[ -n "$container_id" && "$(docker inspect --format '{{.State.Running}}' "$container_id")" == true ]] || {
            echo "Previous candidate finalization rejected: $service is not running." >&2
            return 1
        }
        image_id="$(docker inspect --format '{{.Image}}' "$container_id")" || return 1
        expected_image_id="$(docker image inspect --format '{{.Id}}' "$expected_image" 2>/dev/null || true)"
        [[ -n "$expected_image_id" && "$image_id" == "$expected_image_id" ]] || {
            echo "Previous candidate finalization rejected: running $service image does not match candidate recovery identity." >&2
            return 1
        }
    done

    gateway_container_id="$("${compose[@]}" ps -q gateway 2>/dev/null || true)"
    [[ -n "$gateway_container_id" && "$(docker inspect --format '{{.State.Running}}' "$gateway_container_id")" == true ]] || {
        echo "Previous candidate finalization rejected: gateway is not running." >&2
        return 1
    }
    gateway_image_id="$(docker inspect --format '{{.Image}}' "$gateway_container_id")" || return 1
    expected_gateway_image="${candidate_state[2]}"
    expected_gateway_image_id="$(docker image inspect --format '{{.Id}}' "$expected_gateway_image" 2>/dev/null || true)"
    [[ -n "$expected_gateway_image_id" ]] || {
        echo "Previous candidate finalization rejected: exact candidate Gateway image is unavailable." >&2
        return 1
    }
    if [[ "$gateway_image_id" != "$expected_gateway_image_id" ]]; then
        echo "Previous candidate Gateway runtime drift detected; reconstructing exact candidate Gateway before health proof."
    fi
'''
deploy = replace_once(deploy, old_block, new_block, "candidate runtime identity block")
deploy_path.write_text(deploy)

test_path = Path("tests/ci/test_synology_rollback_recovery_contract.py")
tests = test_path.read_text()
old_assertions = '''    assert "running $service image does not match candidate recovery identity" in body
    assert 'expected_image_id="$(docker image inspect --format' in body
    assert 'release-state.sh" resolve-image "$image_id"' not in body
    assert '"PLATFORM_IMAGE=${candidate_state[1]}"' in body
'''
new_assertions = '''    assert "running $service image does not match candidate recovery identity" in body
    assert 'services=(platform canary)' in body
    assert 'expected_images=("${candidate_state[1]}" "${candidate_state[3]}")' in body
    assert 'expected_gateway_image="${candidate_state[2]}"' in body
    assert "exact candidate Gateway image is unavailable" in body
    assert "Previous candidate Gateway runtime drift detected; reconstructing exact candidate Gateway before health proof." in body
    assert 'expected_image_id="$(docker image inspect --format' in body
    assert 'release-state.sh" resolve-image "$image_id"' not in body
    assert '"PLATFORM_IMAGE=${candidate_state[1]}"' in body
'''
tests = replace_once(tests, old_assertions, new_assertions, "candidate reconstruction contract assertions")
test_path.write_text(tests)
