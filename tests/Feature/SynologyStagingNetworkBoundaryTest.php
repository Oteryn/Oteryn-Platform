<?php

namespace Tests\Feature;

use Tests\TestCase;

class SynologyStagingNetworkBoundaryTest extends TestCase
{
    public function test_host_loopback_protocol_checks_run_in_the_host_network_namespace(): void
    {
        $healthCheck = file_get_contents(base_path('deploy/synology/scripts/health-check.sh'));

        $this->assertIsString($healthCheck);
        $this->assertStringContainsString('--network host', $healthCheck);
        $this->assertStringContainsString('python:3.12-alpine', $healthCheck);
        $this->assertStringContainsString('for attempt in range(15):', $healthCheck);
        $this->assertStringContainsString(
            'Host-loopback request failed after bounded retries',
            $healthCheck,
        );
    }
}
