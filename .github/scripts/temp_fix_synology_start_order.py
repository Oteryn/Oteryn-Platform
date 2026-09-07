#!/usr/bin/env python3
from pathlib import Path

# 1. Make the normal current-release startup use the same proven no-dependency
# recreation sequence as recovery candidate finalization. This prevents Compose
# from needlessly restarting Platform/tls-init after migrations are complete.
deploy_path = Path('deploy/synology/scripts/deploy.sh')
deploy = deploy_path.read_text(encoding='utf-8')
old_start = '''"${compose[@]}" up -d --no-deps --force-recreate internal-proxy
"${compose[@]}" up -d gateway
'''
new_start = '''"${compose[@]}" up -d --no-deps --force-recreate internal-proxy gateway
'''
if old_start not in deploy:
    raise SystemExit('expected dependency-churning Gateway startup sequence not found')
deploy = deploy.replace(old_start, new_start, 1)
deploy_path.write_text(deploy, encoding='utf-8')

# 2. Probe published Gateway endpoints directly through their already-verified
# loopback binding. Spawning a helper container for every 503 made the bounded
# /ready loop consume most of the 30-minute deployment budget on Synology.
health_path = Path('deploy/synology/scripts/health-check.sh')
health = health_path.read_text(encoding='utf-8')
insert_after = '''probe_url() {
    local service="$1"
    local port="$2"
    local path="$3"
    local label="$4"
    local container_id="${container_ids[$service]}"

    for _ in $(seq 1 30); do
        if docker run --rm \\
            --network "container:$container_id" \\
            alpine:3.22 \\
            /bin/sh -ec \\
            "wget -qO- -T 5 'http://127.0.0.1:${port}${path}' >/dev/null"; then
            return 0
        fi
        sleep 2
    done

    echo "Health probe failed: $label" >&2
    return 1
}
'''
helper = insert_after + '''
probe_published_url() {
    local address="$1"
    local port="$2"
    local path="$3"
    local label="$4"
    local attempt status

    for attempt in $(seq 1 12); do
        status="$(curl --silent --show-error --output /dev/null \\
            --write-out '%{http_code}' --max-time 5 \\
            "http://${address}:${port}${path}" 2>/dev/null || true)"
        if [[ "$status" == "200" ]]; then
            return 0
        fi
        echo "${label} attempt ${attempt}/12 returned HTTP ${status:-transport-error}" >&2
        sleep 2
    done

    echo "Health probe failed: $label" >&2
    return 1
}
'''
if 'probe_published_url() {' not in health:
    if insert_after not in health:
        raise SystemExit('probe_url insertion marker not found')
    health = health.replace(insert_after, helper, 1)
old_gateway = '''probe_url gateway 8080 /health "Gateway /health"
if ! probe_url gateway 8080 /ready "Gateway /ready"; then
    echo "Gateway aggregate readiness failed after both internal dependencies passed." >&2
    docker logs --tail 80 "${container_ids[gateway]}" >&2 || true
    exit 1
fi
probe_url gateway 8080 /version "Gateway /version"
'''
new_gateway = '''probe_published_url "$GATEWAY_BIND_ADDRESS" "$GATEWAY_PORT" /health "Gateway /health"
if ! probe_published_url "$GATEWAY_BIND_ADDRESS" "$GATEWAY_PORT" /ready "Gateway /ready"; then
    echo "Gateway aggregate readiness failed after both internal dependencies passed." >&2
    docker logs --tail 80 "${container_ids[gateway]}" >&2 || true
    exit 1
fi
probe_published_url "$GATEWAY_BIND_ADDRESS" "$GATEWAY_PORT" /version "Gateway /version"
'''
if old_gateway not in health:
    raise SystemExit('expected container-per-attempt Gateway probe block not found')
health = health.replace(old_gateway, new_gateway, 1)
health_path.write_text(health, encoding='utf-8')

# 3. Contract the exact orchestration and fast-failure behavior.
test_path = Path('tests/ci/test_synology_auto_staging_deploy.py')
test = test_path.read_text(encoding='utf-8')
method = '''
    def test_current_runtime_start_does_not_rewalk_compose_dependencies(self) -> None:
        self.assertIn(
            '"${compose[@]}" up -d --no-deps --force-recreate internal-proxy gateway',
            self.deploy_script,
        )
        self.assertNotIn('"${compose[@]}" up -d gateway\\n', self.deploy_script)

    def test_gateway_readiness_uses_bounded_host_probe_not_container_per_attempt(self) -> None:
        health = (ROOT / "deploy/synology/scripts/health-check.sh").read_text(encoding="utf-8")
        self.assertIn('probe_published_url "$GATEWAY_BIND_ADDRESS" "$GATEWAY_PORT" /ready', health)
        self.assertIn('for attempt in $(seq 1 12)', health)
        self.assertIn('Gateway aggregate readiness failed after both internal dependencies passed.', health)

'''
if 'def test_current_runtime_start_does_not_rewalk_compose_dependencies' not in test:
    marker = '\n\nif __name__ == "__main__":\n'
    if marker not in test:
        raise SystemExit('test insertion marker missing')
    test = test.replace(marker, method + marker, 1)
test_path.write_text(test, encoding='utf-8')
