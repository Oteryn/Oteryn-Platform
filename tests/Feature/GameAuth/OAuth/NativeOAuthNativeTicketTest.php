<?php

namespace Tests\Feature\GameAuth\OAuth;

use App\GameAuth\NativeLogin\NativeGameLoginTickets;
use App\GameAuth\OAuth\NativeOAuthClientManager;
use Illuminate\Foundation\Testing\RefreshDatabase;
use Illuminate\Support\Facades\DB;
use Tests\Feature\GameAuth\OAuth\Concerns\ConfiguresEphemeralPassportKeys;
use Tests\Feature\GameAuth\OAuth\Concerns\CreatesNativeOAuthBootstrapToken;
use Tests\TestCase;

final class NativeOAuthNativeTicketTest extends TestCase
{
    use ConfiguresEphemeralPassportKeys;
    use CreatesNativeOAuthBootstrapToken;
    use RefreshDatabase;

    protected function setUp(): void
    {
        parent::setUp();
        $this->configureEphemeralPassportKeys();
        config(['game-auth.native_admission.enabled' => true]);
    }

    public function test_rust_oauth_issues_native_ticket_without_canary_binding_and_consumes_bearer_once(): void
    {
        $identity = $this->createOAuthIdentity();
        $manager = app(NativeOAuthClientManager::class);
        $canary = $manager->ensure();
        $rust = $manager->ensureRust();
        self::assertNotSame($canary->getKey(), $rust->getKey());
        self::assertSame($rust->getKey(), $manager->ensureRust()->getKey());
        self::assertFalse($rust->confidential());
        self::assertFalse($manager->usesNativeTicket($canary));
        self::assertTrue($manager->usesNativeTicket($rust));
        $token = $this->issueNativeOAuthBootstrapToken($identity, client: $rust);

        $response = $this->withToken($token['access_token'])->postJson('/api/v1/game-auth/tickets', ['protocol_version' => 1]);
        $response->assertOk()->assertJsonPath('protocol_version', 1);
        $ttl = $response->json('expires_in');
        self::assertIsInt($ttl);
        self::assertGreaterThan(0, $ttl);
        self::assertLessThanOrEqual(60, $ttl);
        $stored = DB::table('game_login_tickets')->sole();
        self::assertSame(NativeGameLoginTickets::AUDIENCE, $stored->audience);
        self::assertSame($identity->account_id, $stored->account_id);
        self::assertNull($stored->canary_account_id);
        self::assertSame($identity->native_security_generation, $stored->native_security_generation);
        self::assertNotSame($response->json('ticket'), $stored->ticket_hash);
        self::assertSame(0, DB::table('oauth_access_tokens')->where('revoked', false)->count());
        self::assertSame(0, DB::table('oauth_refresh_tokens')->where('revoked', false)->count());
        $this->withToken($token['access_token'])->postJson('/api/v1/game-auth/tickets', ['protocol_version' => 1])->assertUnauthorized();
        self::assertSame(1, DB::table('game_login_tickets')->count());
    }

    public function test_disabled_native_switch_refuses_issuance_without_consuming_bearer(): void
    {
        $token = $this->issueNativeOAuthBootstrapToken($this->createOAuthIdentity(), client: app(NativeOAuthClientManager::class)->ensureRust());
        config(['game-auth.native_admission.enabled' => false]);

        $this->withToken($token['access_token'])->postJson('/api/v1/game-auth/tickets', ['protocol_version' => 1])->assertUnauthorized();
        self::assertSame(0, DB::table('game_login_tickets')->count());
        self::assertSame(1, DB::table('oauth_access_tokens')->where('revoked', false)->count());
    }

    public function test_native_issuance_stays_refused_in_production(): void
    {
        $token = $this->issueNativeOAuthBootstrapToken($this->createOAuthIdentity(), client: app(NativeOAuthClientManager::class)->ensureRust());
        $this->app['env'] = 'production';

        $this->withToken($token['access_token'])->postJson('/api/v1/game-auth/tickets', ['protocol_version' => 1])->assertUnauthorized();
        self::assertSame(0, DB::table('game_login_tickets')->count());
    }

    public function test_account_disabled_after_oauth_refuses_native_ticket(): void
    {
        $identity = $this->createOAuthIdentity();
        $token = $this->issueNativeOAuthBootstrapToken($identity, client: app(NativeOAuthClientManager::class)->ensureRust());
        $identity->forceFill(['disabled_at' => now()])->save();

        $this->withToken($token['access_token'])->postJson('/api/v1/game-auth/tickets', ['protocol_version' => 1])->assertUnauthorized();
        self::assertSame(0, DB::table('game_login_tickets')->count());
    }
}
