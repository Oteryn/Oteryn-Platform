<?php

namespace Tests\Feature\Identity\Mfa;

use App\Audit\SecurityEventRecorder;
use App\Identity\Actions\RevokeIdentityWebSessions;
use App\Identity\Mfa\MfaRecoveryCodes;
use App\Identity\Mfa\PendingMfaLogin;
use App\Identity\Models\Identity;
use App\Identity\Sessions\WebSessionState;
use Illuminate\Foundation\Testing\RefreshDatabase;
use Illuminate\Support\Facades\Hash;
use Illuminate\Testing\TestResponse;
use PragmaRX\Google2FA\Google2FA;
use Symfony\Component\HttpFoundation\Response as SymfonyResponse;
use Tests\TestCase;

final class MfaWebFlowTest extends TestCase
{
    use RefreshDatabase;

    private const PASSWORD = 'Correct-Horse-9!Battery';

    public function test_enrollment_requires_confirmation_hashes_recovery_codes_and_preserves_current_session_after_revocation(): void
    {
        $identity = $this->createIdentity();
        $this->passwordLogin($identity)->assertRedirect(route('home'));
        $this->assertAuthenticatedAs($identity, 'web');

        $this->post('/mfa/enroll')->assertRedirect(route('identity.mfa.settings'));

        $identity->refresh();
        self::assertIsString($identity->two_factor_secret);
        self::assertNull($identity->two_factor_confirmed_at);
        self::assertFalse($identity->hasConfirmedMfa());

        $this->get('/mfa')
            ->assertOk()
            ->assertHeaderContains('Cache-Control', 'no-store')
            ->assertSee('Scan with your authenticator app')
            ->assertSee('data:image/svg+xml;base64,', false)
            ->assertSee($identity->two_factor_secret)
            ->assertDontSee('otpauth://totp', false);

        $google2fa = new Google2FA;
        $timestamp = $google2fa->getTimestamp();
        $code = $google2fa->oathTotp($identity->two_factor_secret, $timestamp);

        $response = $this->post('/mfa/confirm', [
            'current_password' => self::PASSWORD,
            'code' => $code,
        ]);

        $response->assertOk();
        $response->assertViewIs('identity.mfa.recovery-codes');
        $response->assertHeaderContains('Cache-Control', 'no-store');
        $response->assertSessionHas(WebSessionState::GENERATION_KEY, 1);
        $this->assertAuthenticatedAs($identity, 'web');

        $plainRecoveryCodes = $response->viewData('recoveryCodes');
        self::assertIsArray($plainRecoveryCodes);
        self::assertCount(8, $plainRecoveryCodes);

        $identity->refresh();
        self::assertTrue($identity->hasConfirmedMfa());
        self::assertSame($timestamp, $identity->two_factor_last_used_timestep);
        self::assertSame(1, $identity->web_session_generation);

        $storedRecoveryCodes = $identity->two_factor_recovery_codes;
        self::assertIsArray($storedRecoveryCodes);
        self::assertCount(8, $storedRecoveryCodes);
        $normalizer = new MfaRecoveryCodes;

        foreach (array_values($plainRecoveryCodes) as $index => $plainRecoveryCode) {
            self::assertIsString($plainRecoveryCode);
            $storedHash = $storedRecoveryCodes[$index] ?? null;
            self::assertIsString($storedHash);
            self::assertNotSame($normalizer->normalize($plainRecoveryCode), $storedHash);
            self::assertTrue(Hash::check($normalizer->normalize($plainRecoveryCode), $storedHash));
        }

        $this->assertDatabaseHas('identity_security_events', [
            'identity_id' => $identity->id,
            'event_type' => SecurityEventRecorder::IDENTITY_MFA_ENROLLED,
        ]);
        $this->assertDatabaseHas('identity_security_events', [
            'identity_id' => $identity->id,
            'event_type' => SecurityEventRecorder::IDENTITY_WEB_SESSIONS_REVOKED,
        ]);
    }

    public function test_enrollment_confirmation_rejects_wrong_current_password_and_remains_unconfirmed(): void
    {
        $identity = $this->createIdentity();
        $this->passwordLogin($identity)->assertRedirect(route('home'));
        $this->post('/mfa/enroll')->assertRedirect(route('identity.mfa.settings'));

        $identity->refresh();
        self::assertIsString($identity->two_factor_secret);
        $google2fa = new Google2FA;
        $code = $google2fa->getCurrentOtp($identity->two_factor_secret);

        $this->post('/mfa/confirm', [
            'current_password' => 'Wrong-Horse-9!Battery',
            'code' => $code,
        ])->assertSessionHasErrors('code');

        $identity->refresh();
        self::assertFalse($identity->hasConfirmedMfa());
        self::assertNull($identity->two_factor_recovery_codes);
    }

    public function test_unconfirmed_mfa_secret_does_not_create_a_half_enforced_login_challenge(): void
    {
        $identity = $this->createIdentity();
        $identity->forceFill([
            'two_factor_secret' => (new Google2FA)->generateSecretKey(),
            'two_factor_confirmed_at' => null,
        ])->save();

        $this->passwordLogin($identity)->assertRedirect(route('home'));
        $this->assertAuthenticatedAs($identity, 'web');
    }

    public function test_confirmed_mfa_identity_remains_guest_until_fresh_totp_is_consumed(): void
    {
        $identity = $this->createIdentity();
        $secret = $this->enableMfa($identity);

        $response = $this->passwordLogin($identity);
        $response->assertRedirect(route('identity.mfa.challenge.create'));
        $response->assertSessionHas(PendingMfaLogin::IDENTITY_ID_KEY, $identity->id);
        $this->assertGuest('web');
        $this->assertDatabaseMissing('identity_security_events', [
            'identity_id' => $identity->id,
            'event_type' => SecurityEventRecorder::IDENTITY_LOGIN_SUCCEEDED,
        ]);

        $google2fa = new Google2FA;
        $timestamp = $google2fa->getTimestamp();
        $code = $google2fa->oathTotp($secret, $timestamp);

        $this->post('/mfa/challenge', ['code' => $code])
            ->assertRedirect(route('home'));

        $this->assertAuthenticatedAs($identity, 'web');
        $this->assertDatabaseHas('identity_security_events', [
            'identity_id' => $identity->id,
            'event_type' => SecurityEventRecorder::IDENTITY_LOGIN_SUCCEEDED,
        ]);
        self::assertSame(
            $timestamp,
            Identity::query()->findOrFail($identity->id)->two_factor_last_used_timestep,
        );
    }

    public function test_same_totp_timestep_cannot_be_replayed_across_login_attempts(): void
    {
        $identity = $this->createIdentity();
        $secret = $this->enableMfa($identity);
        $google2fa = new Google2FA;
        $timestamp = $google2fa->getTimestamp();
        $code = $google2fa->oathTotp($secret, $timestamp);

        $this->passwordLogin($identity)->assertRedirect(route('identity.mfa.challenge.create'));
        $this->post('/mfa/challenge', ['code' => $code])->assertRedirect(route('home'));
        $this->post('/logout')->assertRedirect(route('home'));

        $this->passwordLogin($identity)->assertRedirect(route('identity.mfa.challenge.create'));
        $this->post('/mfa/challenge', ['code' => $code])
            ->assertSessionHasErrors('code');

        $this->assertGuest('web');
        self::assertSame(
            $timestamp,
            Identity::query()->findOrFail($identity->id)->two_factor_last_used_timestep,
        );
    }

    public function test_recovery_code_is_consumed_once_and_cannot_be_reused(): void
    {
        $identity = $this->createIdentity();
        $recoveryCode = 'ABCDE-12345';
        $this->enableMfa($identity, [$recoveryCode]);

        $this->passwordLogin($identity)->assertRedirect(route('identity.mfa.challenge.create'));
        $this->post('/mfa/challenge', ['code' => $recoveryCode])->assertRedirect(route('home'));
        $this->assertAuthenticatedAs($identity, 'web');

        $identity->refresh();
        self::assertSame([], $identity->two_factor_recovery_codes);
        $this->assertDatabaseHas('identity_security_events', [
            'identity_id' => $identity->id,
            'event_type' => SecurityEventRecorder::IDENTITY_MFA_RECOVERY_CODE_USED,
        ]);

        $this->post('/logout')->assertRedirect(route('home'));
        $this->passwordLogin($identity)->assertRedirect(route('identity.mfa.challenge.create'));
        $this->post('/mfa/challenge', ['code' => $recoveryCode])
            ->assertSessionHasErrors('code');
        $this->assertGuest('web');
    }

    public function test_pending_mfa_login_expires_and_is_cleared(): void
    {
        $identity = $this->createIdentity();
        $this->enableMfa($identity);

        $this->passwordLogin($identity)->assertRedirect(route('identity.mfa.challenge.create'));
        $this->travel(6)->minutes();

        $response = $this->get('/mfa/challenge');
        $response->assertRedirect(route('identity.login.create'));
        $response->assertSessionMissing(PendingMfaLogin::IDENTITY_ID_KEY);
        $this->assertGuest('web');
    }

    public function test_pending_mfa_login_fails_closed_after_web_session_generation_changes(): void
    {
        $identity = $this->createIdentity();
        $secret = $this->enableMfa($identity);
        $this->passwordLogin($identity)->assertRedirect(route('identity.mfa.challenge.create'));

        (new RevokeIdentityWebSessions(new SecurityEventRecorder))->execute($identity);
        $code = (new Google2FA)->getCurrentOtp($secret);

        $this->post('/mfa/challenge', ['code' => $code])
            ->assertRedirect(route('identity.login.create'));
        $this->assertGuest('web');
    }

    public function test_mfa_challenge_is_rate_limited(): void
    {
        $identity = $this->createIdentity();
        $this->enableMfa($identity);
        $this->passwordLogin($identity)->assertRedirect(route('identity.mfa.challenge.create'));

        for ($attempt = 1; $attempt <= 5; $attempt++) {
            $this->post('/mfa/challenge', ['code' => 'not-a-code'])
                ->assertSessionHasErrors('code');
        }

        $this->post('/mfa/challenge', ['code' => 'not-a-code'])
            ->assertStatus(429);
        $this->assertGuest('web');
    }

    public function test_mfa_disable_requires_current_password_and_valid_factor_then_revokes_all_sessions_and_logs_out(): void
    {
        $identity = $this->createIdentity();
        $recoveryCode = 'ABCDE-12345';
        $secret = $this->enableMfa($identity, [$recoveryCode]);

        $this->passwordLogin($identity)->assertRedirect(route('identity.mfa.challenge.create'));
        $this->post('/mfa/challenge', ['code' => $recoveryCode])->assertRedirect(route('home'));
        $this->assertAuthenticatedAs($identity, 'web');

        $google2fa = new Google2FA;
        $timestamp = $google2fa->getTimestamp();
        $code = $google2fa->oathTotp($secret, $timestamp);

        $this->delete('/mfa', [
            'current_password' => self::PASSWORD,
            'code' => $code,
        ])->assertRedirect(route('home'));

        $this->assertGuest('web');
        $identity->refresh();
        self::assertNull($identity->two_factor_secret);
        self::assertNull($identity->two_factor_recovery_codes);
        self::assertNull($identity->two_factor_confirmed_at);
        self::assertNull($identity->two_factor_last_used_timestep);
        self::assertSame(1, $identity->web_session_generation);
        $this->assertDatabaseHas('identity_security_events', [
            'identity_id' => $identity->id,
            'event_type' => SecurityEventRecorder::IDENTITY_MFA_DISABLED,
        ]);
        $this->assertDatabaseHas('identity_security_events', [
            'identity_id' => $identity->id,
            'event_type' => SecurityEventRecorder::IDENTITY_WEB_SESSIONS_REVOKED,
        ]);
    }

    public function test_wrong_current_password_cannot_disable_mfa_or_consume_totp(): void
    {
        $identity = $this->createIdentity();
        $recoveryCode = 'ABCDE-12345';
        $secret = $this->enableMfa($identity, [$recoveryCode]);

        $this->passwordLogin($identity)->assertRedirect(route('identity.mfa.challenge.create'));
        $this->post('/mfa/challenge', ['code' => $recoveryCode])->assertRedirect(route('home'));

        $google2fa = new Google2FA;
        $timestamp = $google2fa->getTimestamp();
        $code = $google2fa->oathTotp($secret, $timestamp);

        $this->delete('/mfa', [
            'current_password' => 'Wrong-Horse-9!Battery',
            'code' => $code,
        ])->assertSessionHasErrors('code');

        $identity->refresh();
        self::assertTrue($identity->hasConfirmedMfa());
        self::assertNull($identity->two_factor_last_used_timestep);
        $this->assertAuthenticatedAs($identity, 'web');
    }

    private function createIdentity(): Identity
    {
        return Identity::query()->create([
            'email' => 'person@example.com',
            'password' => Hash::make(self::PASSWORD),
        ]);
    }

    /**
     * @param  list<string>  $plainRecoveryCodes
     */
    private function enableMfa(Identity $identity, array $plainRecoveryCodes = []): string
    {
        $google2fa = new Google2FA;
        $secret = $google2fa->generateSecretKey();
        $normalizer = new MfaRecoveryCodes;
        $hashes = [];

        foreach ($plainRecoveryCodes as $plainRecoveryCode) {
            $hashes[] = Hash::make($normalizer->normalize($plainRecoveryCode));
        }

        $identity->forceFill([
            'two_factor_secret' => $secret,
            'two_factor_recovery_codes' => $hashes,
            'two_factor_confirmed_at' => now(),
            'two_factor_last_used_timestep' => null,
        ])->save();

        return $secret;
    }

    /**
     * @return TestResponse<SymfonyResponse>
     */
    private function passwordLogin(Identity $identity): TestResponse
    {
        return $this->post('/login', [
            'email' => $identity->email,
            'password' => self::PASSWORD,
        ]);
    }
}
