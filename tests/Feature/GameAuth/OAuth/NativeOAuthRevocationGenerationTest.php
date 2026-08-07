<?php

namespace Tests\Feature\GameAuth\OAuth;

use App\Accounts\Models\IdentityCanaryAccount;
use App\GameAuth\Tickets\GameLoginTicket;
use App\Identity\Actions\RevokeIdentityGameAuthorizations;
use App\Identity\Models\Identity;
use Illuminate\Foundation\Testing\RefreshDatabase;
use Illuminate\Support\Facades\DB;
use Laravel\Passport\RefreshToken;
use Laravel\Passport\Token;
use Tests\Feature\GameAuth\OAuth\Concerns\ConfiguresEphemeralPassportKeys;
use Tests\Feature\GameAuth\OAuth\Concerns\CreatesNativeOAuthBootstrapToken;
use Tests\TestCase;

final class NativeOAuthRevocationGenerationTest extends TestCase
{
    use ConfiguresEphemeralPassportKeys;
    use CreatesNativeOAuthBootstrapToken;
    use RefreshDatabase;

    protected function setUp(): void
    {
        parent::setUp();
        $this->configureEphemeralPassportKeys();
    }

    public function test_pre_revocation_access_token_cannot_bootstrap_after_generation_change(): void
    {
        $identity = $this->createOAuthIdentity();
        $this->createReadyBinding($identity);
        $bootstrap = $this->issueNativeOAuthBootstrapToken($identity);
        $accessToken = Token::query()->where('user_id', $identity->id)->firstOrFail();
        $authCodeGeneration = DB::table('oauth_auth_codes')->value('game_auth_generation');

        self::assertSame(0, $authCodeGeneration);
        self::assertSame(0, $accessToken->getAttribute('game_auth_generation'));

        $this->app->make(RevokeIdentityGameAuthorizations::class)->execute($identity);

        self::assertSame(1, $identity->fresh()?->game_auth_generation);
        $revokedAccessToken = Token::query()->whereKey($accessToken->getKey())->firstOrFail();
        self::assertTrue((bool) $revokedAccessToken->getAttribute('revoked'));
        self::assertTrue((bool) RefreshToken::query()
            ->where('access_token_id', $accessToken->getKey())
            ->value('revoked'));

        Token::query()->whereKey($accessToken->getKey())->update(['revoked' => false]);

        $this->withToken($bootstrap['access_token'])
            ->postJson('/api/v1/game-auth/tickets', ['protocol_version' => 1])
            ->assertStatus(401);

        self::assertSame(0, GameLoginTicket::query()->count());
    }

    public function test_pre_revocation_refresh_token_cannot_mint_descendant_after_generation_change(): void
    {
        $identity = $this->createOAuthIdentity();
        $bootstrap = $this->issueNativeOAuthBootstrapToken($identity);
        $accessToken = Token::query()->where('user_id', $identity->id)->firstOrFail();

        $this->app->make(RevokeIdentityGameAuthorizations::class)->execute($identity);

        $this->post('/oauth/token', [
            'grant_type' => 'refresh_token',
            'client_id' => $bootstrap['client']->getKey(),
            'refresh_token' => $bootstrap['refresh_token'],
        ])->assertStatus(400);

        self::assertSame(1, Token::query()->where('user_id', $identity->id)->count());
        $revokedAccessToken = Token::query()->whereKey($accessToken->getKey())->firstOrFail();
        self::assertTrue((bool) $revokedAccessToken->getAttribute('revoked'));
    }

    public function test_refresh_descendant_inherits_authorization_generation(): void
    {
        $identity = $this->createOAuthIdentity();
        $bootstrap = $this->issueNativeOAuthBootstrapToken($identity);

        $refresh = $this->post('/oauth/token', [
            'grant_type' => 'refresh_token',
            'client_id' => $bootstrap['client']->getKey(),
            'refresh_token' => $bootstrap['refresh_token'],
        ]);

        $refresh->assertOk();
        $newAccessToken = Token::query()
            ->where('user_id', $identity->id)
            ->where('revoked', false)
            ->firstOrFail();

        self::assertSame(0, $newAccessToken->getAttribute('game_auth_generation'));

        $newRefreshToken = $refresh->json('refresh_token');

        if (! is_string($newRefreshToken)) {
            self::fail('OAuth refresh response did not contain a refresh token.');
        }

        $this->app->make(RevokeIdentityGameAuthorizations::class)->execute($identity);

        $this->post('/oauth/token', [
            'grant_type' => 'refresh_token',
            'client_id' => $bootstrap['client']->getKey(),
            'refresh_token' => $newRefreshToken,
        ])->assertStatus(400);
    }

    public function test_new_authorization_after_revocation_uses_current_generation_and_can_bootstrap(): void
    {
        $identity = $this->createOAuthIdentity();
        $this->createReadyBinding($identity);
        $this->app->make(RevokeIdentityGameAuthorizations::class)->execute($identity);
        $bootstrap = $this->issueNativeOAuthBootstrapToken($identity->fresh() ?? $identity);
        $accessToken = Token::query()->where('user_id', $identity->id)->firstOrFail();

        self::assertSame(1, $accessToken->getAttribute('game_auth_generation'));

        $this->withToken($bootstrap['access_token'])
            ->postJson('/api/v1/game-auth/tickets', ['protocol_version' => 1])
            ->assertOk();

        self::assertSame(1, GameLoginTicket::query()->value('security_generation'));
    }

    public function test_disabled_and_terminated_identities_fail_closed_before_bootstrap(): void
    {
        $identity = $this->createOAuthIdentity();
        $this->createReadyBinding($identity);
        $bootstrap = $this->issueNativeOAuthBootstrapToken($identity);

        $identity->forceFill(['disabled_at' => now()])->save();

        $this->withToken($bootstrap['access_token'])
            ->postJson('/api/v1/game-auth/tickets', ['protocol_version' => 1])
            ->assertStatus(401);

        $identity->forceFill([
            'disabled_at' => null,
            'terminated_at' => now(),
        ])->save();

        $this->withToken($bootstrap['access_token'])
            ->postJson('/api/v1/game-auth/tickets', ['protocol_version' => 1])
            ->assertStatus(401);

        self::assertSame(0, GameLoginTicket::query()->count());
    }

    private function createReadyBinding(Identity $identity): void
    {
        IdentityCanaryAccount::query()->create([
            'identity_id' => $identity->id,
            'canary_account_id' => 1001,
            'provisioning_name' => 'ready_'.$identity->id,
            'canary_creation_epoch' => 1,
            'status' => IdentityCanaryAccount::STATUS_READY,
            'ready_at' => now(),
        ]);
    }
}
