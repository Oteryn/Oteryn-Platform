<?php

namespace Tests\Feature\GameAuth;

use App\Accounts\Models\IdentityCanaryAccount;
use App\GameAuth\Tickets\GameLoginTicket;
use Illuminate\Foundation\Testing\RefreshDatabase;
use Illuminate\Testing\TestResponse;
use Laravel\Passport\RefreshToken;
use Laravel\Passport\Token;
use Symfony\Component\HttpFoundation\Response;
use Tests\Feature\GameAuth\OAuth\Concerns\ConfiguresEphemeralPassportKeys;
use Tests\Feature\GameAuth\OAuth\Concerns\CreatesNativeOAuthBootstrapToken;
use Tests\TestCase;

final class GameLoginTicketApiTest extends TestCase
{
    use ConfiguresEphemeralPassportKeys;
    use CreatesNativeOAuthBootstrapToken;
    use RefreshDatabase;

    protected function setUp(): void
    {
        parent::setUp();
        $this->configureEphemeralPassportKeys();
    }

    public function test_scoped_native_oauth_token_is_exchanged_once_for_one_game_ticket_and_token_family_is_revoked(): void
    {
        $identity = $this->createOAuthIdentity();
        $this->createReadyBinding($identity->id, 1001);
        $bootstrap = $this->issueNativeOAuthBootstrapToken($identity);

        $response = $this->withToken($bootstrap['access_token'])
            ->postJson('/api/v1/game-auth/tickets', [
                'protocol_version' => 1,
            ]);

        if ($response->getStatusCode() !== 200) {
            self::fail('Game ticket issuance failed: '.$response->getContent());
        }

        $this->assertSensitiveResponseIsNotCacheable($response);
        $response->assertJsonPath('protocol_version', 1)
            ->assertJsonStructure(['ticket', 'expires_in']);

        $payload = $response->json();

        if (! is_array($payload)) {
            self::fail('Game ticket issuance response was not a JSON object.');
        }

        $ticket = $payload['ticket'] ?? null;
        $expiresIn = $payload['expires_in'] ?? null;

        if (! is_string($ticket) || ! is_int($expiresIn)) {
            self::fail('Game ticket issuance response did not contain typed ticket expiry data.');
        }

        self::assertGreaterThan(0, $expiresIn);
        self::assertLessThanOrEqual(60, $expiresIn);
        self::assertArrayNotHasKey('identity_id', $payload);
        self::assertArrayNotHasKey('canary_account_id', $payload);
        self::assertSame(1, GameLoginTicket::query()->count());
        self::assertSame(hash('sha256', $ticket), GameLoginTicket::query()->value('ticket_hash'));

        $accessToken = Token::query()->where('user_id', $identity->id)->firstOrFail();
        self::assertTrue($accessToken->revoked);
        self::assertTrue((bool) RefreshToken::query()
            ->where('access_token_id', $accessToken->getKey())
            ->value('revoked'));

        $revokedResponse = $this->withToken($bootstrap['access_token'])
            ->postJson('/api/v1/game-auth/tickets', ['protocol_version' => 1]);
        $revokedResponse->assertStatus(401);
        $this->assertSensitiveResponseIsNotCacheable($revokedResponse);
        self::assertSame(1, GameLoginTicket::query()->count());
    }

    public function test_legacy_issuance_is_denied_without_consuming_the_bearer_when_native_only_is_set(): void
    {
        config(['game-auth.legacy_login.enabled' => false]);
        $identity = $this->createOAuthIdentity();
        $this->createReadyBinding($identity->id, 1001);
        $bootstrap = $this->issueNativeOAuthBootstrapToken($identity);

        $this->withToken($bootstrap['access_token'])
            ->postJson('/api/v1/game-auth/tickets', ['protocol_version' => 1])
            ->assertStatus(401)
            ->assertJsonPath('error', 'game_login_unavailable');

        self::assertSame(0, GameLoginTicket::query()->count());
        self::assertFalse(Token::query()->where('user_id', $identity->id)->firstOrFail()->revoked);
    }

    public function test_client_cannot_override_identity_or_canary_account_binding(): void
    {
        $identity = $this->createOAuthIdentity();
        $this->createReadyBinding($identity->id, 1001);
        $bootstrap = $this->issueNativeOAuthBootstrapToken($identity);

        $response = $this->withToken($bootstrap['access_token'])
            ->postJson('/api/v1/game-auth/tickets', [
                'protocol_version' => 1,
                'identity_id' => 999,
                'canary_account_id' => 999,
            ]);

        $response->assertStatus(422);
        $this->assertSensitiveResponseIsNotCacheable($response);

        self::assertSame(0, GameLoginTicket::query()->count());
        self::assertFalse(Token::query()->where('user_id', $identity->id)->firstOrFail()->revoked);
    }

    public function test_unscoped_oauth_token_cannot_mint_game_ticket(): void
    {
        $identity = $this->createOAuthIdentity();
        $this->createReadyBinding($identity->id, 1001);
        $bootstrap = $this->issueNativeOAuthBootstrapToken($identity, []);

        $response = $this->withToken($bootstrap['access_token'])
            ->postJson('/api/v1/game-auth/tickets', ['protocol_version' => 1]);

        $response->assertStatus(401);
        $this->assertSensitiveResponseIsNotCacheable($response);

        self::assertSame(0, GameLoginTicket::query()->count());
    }

    public function test_ticket_issuance_requires_oauth_authentication(): void
    {
        $response = $this->postJson('/api/v1/game-auth/tickets', ['protocol_version' => 1]);

        $response->assertStatus(401);
        $this->assertSensitiveResponseIsNotCacheable($response);
    }

    /**
     * @param  TestResponse<Response>  $response
     */
    private function assertSensitiveResponseIsNotCacheable(TestResponse $response): void
    {
        $cacheControl = (string) $response->headers->get('Cache-Control');

        self::assertStringContainsString('no-store', $cacheControl);
        self::assertStringContainsString('no-cache', $cacheControl);
        self::assertStringContainsString('must-revalidate', $cacheControl);
        self::assertStringContainsString('private', $cacheControl);
        $response->assertHeader('Pragma', 'no-cache')
            ->assertHeader('Expires', '0');
    }

    private function createReadyBinding(int $identityId, int $canaryAccountId): IdentityCanaryAccount
    {
        return IdentityCanaryAccount::query()->create([
            'identity_id' => $identityId,
            'canary_account_id' => $canaryAccountId,
            'provisioning_name' => 'ready_'.$identityId,
            'canary_creation_epoch' => 1,
            'status' => IdentityCanaryAccount::STATUS_READY,
            'ready_at' => now(),
        ]);
    }
}
