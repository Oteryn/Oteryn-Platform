<?php

namespace Tests\Feature;

use Tests\TestCase;

class HealthTest extends TestCase
{
    public function test_health_endpoint_reports_application_availability(): void
    {
        $response = $this->get('/health');

        $response->assertOk();
        $response->assertDontSee('APP_KEY');
        $response->assertDontSee('APP_ENV');
    }
}
