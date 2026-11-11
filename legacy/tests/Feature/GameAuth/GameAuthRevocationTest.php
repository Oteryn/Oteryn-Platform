<?php

namespace Tests\Feature\GameAuth;

use App\Audit\SecurityEventRecorder;
use App\Identity\Mfa\DisableIdentityMfa;
use App\Identity\Mfa\ResetIdentityMfa;
use App\Identity\Models\Identity;
use Illuminate\Foundation\Testing\RefreshDatabase;
use Illuminate\Support\Facades\Hash;
use PragmaRX\Google2FA\Google2FA;
use Tests\TestCase;

final class GameAuthRevocationTest extends TestCase
{
    use RefreshDatabase;

    private const PASSWORD = 'Correct-Horse-9!Battery';

    public function test_mfa_reset_revokes_pending_game_authorizations(): void
    {
        $identity = $this->createIdentityWithMfa();

        $this->app->make(ResetIdentityMfa::class)->execute($identity);

        $identity->refresh();
        self::assertSame(1, $identity->web_session_generation);
        self::assertSame(1, $identity->game_auth_generation);
        self::assertFalse($identity->hasConfirmedMfa());
        $this->assertDatabaseHas('identity_security_events', [
            'identity_id' => $identity->id,
            'event_type' => SecurityEventRecorder::IDENTITY_GAME_AUTHORIZATIONS_REVOKED,
        ]);
    }

    public function test_mfa_disable_revokes_pending_game_authorizations(): void
    {
        $identity = $this->createIdentityWithMfa();
        $secret = $identity->two_factor_secret;

        if (! is_string($secret)) {
            self::fail('MFA test identity does not contain a decrypted TOTP secret.');
        }

        $code = (new Google2FA)->getCurrentOtp($secret);

        $this->app->make(DisableIdentityMfa::class)->execute($identity, self::PASSWORD, $code);

        $identity->refresh();
        self::assertSame(1, $identity->web_session_generation);
        self::assertSame(1, $identity->game_auth_generation);
        self::assertFalse($identity->hasConfirmedMfa());
        $this->assertDatabaseHas('identity_security_events', [
            'identity_id' => $identity->id,
            'event_type' => SecurityEventRecorder::IDENTITY_GAME_AUTHORIZATIONS_REVOKED,
        ]);
    }

    private function createIdentityWithMfa(): Identity
    {
        $identity = Identity::query()->create([
            'email' => 'person@example.com',
            'password' => Hash::make(self::PASSWORD),
        ]);

        $identity->forceFill([
            'two_factor_secret' => (new Google2FA)->generateSecretKey(),
            'two_factor_recovery_codes' => [],
            'two_factor_confirmed_at' => now(),
            'two_factor_last_used_timestep' => null,
        ])->save();

        return $identity;
    }
}
