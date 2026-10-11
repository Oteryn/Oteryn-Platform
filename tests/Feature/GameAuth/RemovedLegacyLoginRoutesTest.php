<?php

namespace Tests\Feature\GameAuth;

use Illuminate\Foundation\Testing\RefreshDatabase;
use Tests\TestCase;

final class RemovedLegacyLoginRoutesTest extends TestCase
{
    use RefreshDatabase;

    private const SERVICE_CREDENTIAL = 'gateway-removed-legacy-login-credential';

    protected function setUp(): void
    {
        parent::setUp();

        config(['game-auth.gateway.service_token_sha256' => hash('sha256', self::SERVICE_CREDENTIAL)]);
    }

    public function test_v1_login_context_route_no_longer_exists(): void
    {
        $this->withToken(self::SERVICE_CREDENTIAL)
            ->getJson('/internal/v1/game-auth/accounts/1001/login-context')
            ->assertNotFound();
    }

    public function test_v1_ticket_redeem_route_no_longer_exists(): void
    {
        $this->withToken(self::SERVICE_CREDENTIAL)
            ->postJson('/internal/v1/game-auth/tickets/redeem', ['ticket' => 'x', 'audience' => 'oteryn-game-gateway'])
            ->assertNotFound();
    }
}
