<?php

namespace Tests\Feature\GameAuth;

use Illuminate\Foundation\Testing\RefreshDatabase;
use Tests\TestCase;

final class RetiredLoginContextTest extends TestCase
{
    use RefreshDatabase;

    private const SERVICE_CREDENTIAL = 'gateway-retired-login-context-credential';

    protected function setUp(): void
    {
        parent::setUp();

        config(['game-auth.gateway.service_token_sha256' => hash('sha256', self::SERVICE_CREDENTIAL)]);
    }

    public function test_v1_login_context_answers_gone_for_an_authorized_gateway(): void
    {
        $this->withToken(self::SERVICE_CREDENTIAL)
            ->getJson('/internal/v1/game-auth/accounts/1001/login-context')
            ->assertStatus(410)
            ->assertExactJson(['error' => 'login_context_retired']);
    }

    public function test_v1_login_context_still_requires_the_gateway_credential(): void
    {
        $this->getJson('/internal/v1/game-auth/accounts/1001/login-context')
            ->assertUnauthorized();
        $this->withToken('wrong-credential')
            ->getJson('/internal/v1/game-auth/accounts/1001/login-context')
            ->assertUnauthorized();
    }
}
