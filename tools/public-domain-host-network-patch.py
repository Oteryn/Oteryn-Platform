from pathlib import Path


def replace_exact(path: Path, old: str, new: str, *, count: int = 1) -> None:
    text = path.read_text()
    actual = text.count(old)
    if actual != count:
        raise SystemExit(f"{path}: expected {count} exact matches, found {actual}")
    path.write_text(text.replace(old, new))


health = Path("deploy/synology/scripts/health-check.sh")
replace_exact(
    health,
    'python3 - "$PLATFORM_PORT" "$GATEWAY_PORT" "$expected_gateway_version" <<\'PY\'\n',
    'docker run --rm \\\n    --network host \\\n    -i \\\n    python:3.12-alpine \\\n    python3 - "$PLATFORM_PORT" "$GATEWAY_PORT" "$expected_gateway_version" <<\'PY\'\n',
)
replace_exact(
    health,
    "import sys\n\nplatform_port = int(sys.argv[1])\n",
    "import sys\nimport time\n\nplatform_port = int(sys.argv[1])\n",
)
replace_exact(
    health,
    """def request(port, method, path, *, body=None, headers=None):
    connection = http.client.HTTPConnection('127.0.0.1', port, timeout=5)
    try:
        connection.request(method, path, body=body, headers=headers or {})
        response = connection.getresponse()
        payload = response.read(8192)
        return response.status, {key.lower(): value for key, value in response.getheaders()}, payload
    finally:
        connection.close()
""",
    """def request(port, method, path, *, body=None, headers=None):
    last_error = None
    for attempt in range(15):
        connection = http.client.HTTPConnection('127.0.0.1', port, timeout=5)
        try:
            connection.request(method, path, body=body, headers=headers or {})
            response = connection.getresponse()
            payload = response.read(8192)
            return response.status, {key.lower(): value for key, value in response.getheaders()}, payload
        except OSError as exc:
            last_error = exc
            if attempt == 14:
                break
            time.sleep(2)
        finally:
            connection.close()

    raise ConnectionError(
        f'Host-loopback request failed after bounded retries: {method} {path}'
    ) from last_error
""",
)

control = Path(".github/workflows/character-bazaar-staging-control.yml")
replace_exact(
    control,
    """      - 'deploy/synology/compose.marketplace.yml'
      - 'deploy/synology/scripts/marketplace-staging.sh'
""",
    """      - 'deploy/synology/compose.marketplace.yml'
      - 'deploy/synology/scripts/lib.sh'
      - 'deploy/synology/scripts/health-check.sh'
      - 'deploy/synology/scripts/marketplace-staging.sh'
""",
)

validation = Path(".github/workflows/character-bazaar-staging-validation.yml")
replace_exact(
    validation,
    """      - 'deploy/synology/compose.marketplace.yml'
      - 'deploy/synology/scripts/marketplace-staging.sh'
""",
    """      - 'deploy/synology/compose.marketplace.yml'
      - 'deploy/synology/scripts/lib.sh'
      - 'deploy/synology/scripts/health-check.sh'
      - 'deploy/synology/scripts/marketplace-staging.sh'
""",
    count=2,
)

Path("tests/Feature/SynologyStagingNetworkBoundaryTest.php").write_text(
    r'''<?php

namespace Tests\Feature;

use Tests\TestCase;

class SynologyStagingNetworkBoundaryTest extends TestCase
{
    public function test_marketplace_workflows_track_shared_deployment_dependencies(): void
    {
        $control = file_get_contents(base_path('.github/workflows/character-bazaar-staging-control.yml'));
        $validation = file_get_contents(base_path('.github/workflows/character-bazaar-staging-validation.yml'));

        $this->assertIsString($control);
        $this->assertIsString($validation);

        foreach ([
            'deploy/synology/scripts/lib.sh',
            'deploy/synology/scripts/health-check.sh',
        ] as $path) {
            $this->assertStringContainsString($path, $control);
            $this->assertSame(2, substr_count($validation, $path));
        }
    }

    public function test_host_loopback_protocol_checks_run_in_the_host_network_namespace(): void
    {
        $healthCheck = file_get_contents(base_path('deploy/synology/scripts/health-check.sh'));

        $this->assertIsString($healthCheck);
        $this->assertStringContainsString('--network host', $healthCheck);
        $this->assertStringContainsString('python:3.12-alpine', $healthCheck);
        $this->assertStringContainsString('for attempt in range(15):', $healthCheck);
        $this->assertStringContainsString('Host-loopback request failed after bounded retries', $healthCheck);
    }
}
'''
)
