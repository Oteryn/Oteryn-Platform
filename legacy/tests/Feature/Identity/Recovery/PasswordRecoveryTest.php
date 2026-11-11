<?php

namespace Tests\Feature\Identity\Recovery;

use App\Audit\SecurityEventRecorder;
use App\Identity\Actions\RevokeIdentityGameAuthorizations;
use App\Identity\Actions\RevokeIdentityWebSessions;
use App\Identity\Credentials\IdentityCredentialUpdater;
use App\Identity\Credentials\PasswordResetCompleter;
use App\Identity\Models\Identity;
use Illuminate\Auth\Notifications\ResetPassword as ResetPasswordNotification;
use Illuminate\Foundation\Testing\RefreshDatabase;
use Illuminate\Support\Facades\DB;
use Illuminate\Support\Facades\Hash;
use Illuminate\Support\Facades\Notification;
use Illuminate\Support\Facades\Password;
use RuntimeException;
use Tests\TestCase;

final class PasswordRecoveryTest extends TestCase
{
    use RefreshDatabase;

    private const GENERIC_RECOVERY_STATUS = 'If an account exists for that email, a password reset link has been sent.';

    public function test_forgot_password_uses_generic_response_and_stores_only_hashed_token(): void
    {
        Notification::fake();
        $identity = $this->createIdentity();
        $token = null;
        $knownResponse = $this->post('/forgot-password', [
            'email' => '  PERSON@EXAMPLE.COM  ',
        ]);

        $knownResponse->assertSessionHas('status', self::GENERIC_RECOVERY_STATUS);

        Notification::assertSentTo(
            $identity,
            ResetPasswordNotification::class,
            function (ResetPasswordNotification $notification) use (&$token): bool {
                $token = $notification->token;

                return true;
            },
        );

        self::assertIsString($token);
        $storedToken = DB::table('password_reset_tokens')
            ->where('email', 'person@example.com')
            ->value('token');
        self::assertIsString($storedToken);
        self::assertNotSame($token, $storedToken);
        self::assertTrue(Hash::check($token, $storedToken));

        $unknownResponse = $this->post('/forgot-password', [
            'email' => 'unknown@example.com',
        ]);

        $unknownResponse->assertSessionHas('status', self::GENERIC_RECOVERY_STATUS);
    }

    public function test_forgot_password_is_rate_limited_per_canonical_identity(): void
    {
        Notification::fake();

        for ($attempt = 1; $attempt <= 3; $attempt++) {
            $this->post('/forgot-password', [
                'email' => ' PERSON@example.com ',
            ])->assertSessionHas('status', self::GENERIC_RECOVERY_STATUS);
        }

        $this->post('/forgot-password', [
            'email' => 'person@EXAMPLE.com',
        ])->assertStatus(429);
    }

    public function test_forgot_password_is_rate_limited_across_distinct_identity_keys(): void
    {
        Notification::fake();

        for ($attempt = 1; $attempt <= 10; $attempt++) {
            $this->post('/forgot-password', [
                'email' => "unknown{$attempt}@example.com",
            ])->assertSessionHas('status', self::GENERIC_RECOVERY_STATUS);
        }

        $this->post('/forgot-password', [
            'email' => 'unknown11@example.com',
        ])->assertStatus(429);
    }

    public function test_password_reset_link_sender_refuses_log_mail_transport(): void
    {
        config(['mail.default' => 'log']);
        $this->withoutExceptionHandling();
        $this->expectException(RuntimeException::class);

        $this->post('/forgot-password', [
            'email' => 'person@example.com',
        ]);
    }

    public function test_expired_reset_token_is_rejected_without_changing_password_or_sessions(): void
    {
        $identity = $this->createIdentity();
        $token = Password::createToken($identity);
        DB::table('password_reset_tokens')
            ->where('email', $identity->email)
            ->update(['created_at' => now()->subMinutes(61)]);

        $this->post('/reset-password', [
            'email' => $identity->email,
            'token' => $token,
            'password' => 'New-Correct-8!Password',
            'password_confirmation' => 'New-Correct-8!Password',
        ])->assertSessionHasErrors('email');

        $fresh = Identity::query()->findOrFail($identity->id);
        self::assertTrue(Hash::check('Correct-Horse-9!Battery', $fresh->password));
        self::assertSame(0, $fresh->web_session_generation);
        self::assertSame(0, $fresh->game_auth_generation);
    }

    public function test_successful_reset_is_single_use_and_replay_is_rejected(): void
    {
        $identity = $this->createIdentity();
        $token = Password::createToken($identity);

        $this->post('/reset-password', [
            'email' => $identity->email,
            'token' => $token,
            'password' => 'New-Correct-8!Password',
            'password_confirmation' => 'New-Correct-8!Password',
        ])->assertRedirect(route('identity.login.create'));

        $fresh = Identity::query()->findOrFail($identity->id);
        self::assertTrue(Hash::check('New-Correct-8!Password', $fresh->password));
        self::assertFalse(Hash::check('Correct-Horse-9!Battery', $fresh->password));
        self::assertSame(1, $fresh->web_session_generation);
        self::assertSame(1, $fresh->game_auth_generation);
        $this->assertDatabaseMissing('password_reset_tokens', [
            'email' => $identity->email,
        ]);
        $this->assertDatabaseHas('identity_security_events', [
            'identity_id' => $identity->id,
            'event_type' => SecurityEventRecorder::IDENTITY_PASSWORD_RESET_COMPLETED,
        ]);
        $this->assertDatabaseHas('identity_security_events', [
            'identity_id' => $identity->id,
            'event_type' => SecurityEventRecorder::IDENTITY_WEB_SESSIONS_REVOKED,
        ]);
        $this->assertDatabaseHas('identity_security_events', [
            'identity_id' => $identity->id,
            'event_type' => SecurityEventRecorder::IDENTITY_GAME_AUTHORIZATIONS_REVOKED,
        ]);

        $this->post('/reset-password', [
            'email' => $identity->email,
            'token' => $token,
            'password' => 'Replay-Correct-7!Password',
            'password_confirmation' => 'Replay-Correct-7!Password',
        ])->assertSessionHasErrors('email');

        $afterReplay = Identity::query()->findOrFail($identity->id);
        self::assertTrue(Hash::check('New-Correct-8!Password', $afterReplay->password));
        self::assertSame(1, $afterReplay->web_session_generation);
        self::assertSame(1, $afterReplay->game_auth_generation);
    }

    public function test_reset_revokes_an_existing_platform_web_session(): void
    {
        $identity = $this->createIdentity();
        $this->login($identity);
        $token = Password::createToken($identity);
        $securityEvents = new SecurityEventRecorder;
        $completer = new PasswordResetCompleter(
            new IdentityCredentialUpdater(
                new RevokeIdentityWebSessions($securityEvents),
                new RevokeIdentityGameAuthorizations($securityEvents),
                $securityEvents,
            ),
        );

        $status = $completer->complete([
            'email' => $identity->email,
            'token' => $token,
            'password' => 'New-Correct-8!Password',
            'password_confirmation' => 'New-Correct-8!Password',
        ]);

        self::assertSame(Password::PASSWORD_RESET, $status);

        $this->get('/')->assertOk();
        $this->assertGuest('web');
        self::assertSame(1, Identity::query()->findOrFail($identity->id)->game_auth_generation);
    }

    public function test_reset_rejects_weak_password_and_confirmation_mismatch_before_token_consumption(): void
    {
        $identity = $this->createIdentity();
        $token = Password::createToken($identity);

        $this->post('/reset-password', [
            'email' => $identity->email,
            'token' => $token,
            'password' => 'weak',
            'password_confirmation' => 'weak',
        ])->assertSessionHasErrors('password');

        $this->post('/reset-password', [
            'email' => $identity->email,
            'token' => $token,
            'password' => 'New-Correct-8!Password',
            'password_confirmation' => 'Different-Correct-8!Password',
        ])->assertSessionHasErrors('password');

        self::assertTrue(Password::tokenExists($identity, $token));
    }

    private function createIdentity(): Identity
    {
        return Identity::query()->create([
            'email' => 'person@example.com',
            'password' => Hash::make('Correct-Horse-9!Battery'),
        ]);
    }

    private function login(Identity $identity): void
    {
        $this->post('/login', [
            'email' => $identity->email,
            'password' => 'Correct-Horse-9!Battery',
        ])->assertRedirect(route('home'));

        $this->assertAuthenticatedAs($identity, 'web');
    }
}
