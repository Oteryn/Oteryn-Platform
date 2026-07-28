<?php

namespace Tests\Feature\Identity;

use App\Identity\Email\ConfirmIdentityEmailChange;
use App\Identity\Email\EmailChangeRejected;
use App\Identity\Email\RecoverIdentityEmailChange;
use App\Identity\Models\Identity;
use App\Identity\Models\IdentityEmailChangeRequest;
use App\Identity\Models\IdentityWebSession;
use App\Identity\Recovery\IdentityRecoveryKeyService;
use App\Identity\Recovery\RecoveryKeyRejected;
use App\Identity\Sessions\WebSessionState;
use App\Identity\Support\IdentitySecret;
use App\Identity\Termination\CancelIdentityTermination;
use App\Identity\Termination\FinalizeIdentityTermination;
use App\Identity\Termination\RequestIdentityTermination;
use Illuminate\Foundation\Testing\RefreshDatabase;
use Illuminate\Support\Facades\Hash;
use Illuminate\Support\Str;
use Tests\TestCase;

final class AccountSecurityLifecycleTest extends TestCase
{
    use RefreshDatabase;

    public function test_login_registers_a_concrete_session_and_targeted_revocation_is_owner_scoped(): void
    {
        $identity = $this->identity('sessions@example.test');

        $this->post(route('identity.login.store'), [
            'email' => $identity->email,
            'password' => 'Correct-password-123!',
        ])->assertRedirect('/');

        $currentSessionId = session()->get(WebSessionState::REGISTRY_ID_KEY);
        self::assertIsString($currentSessionId);
        $this->assertDatabaseHas('identity_web_sessions', [
            'id' => $currentSessionId,
            'identity_id' => $identity->id,
            'revoked_at' => null,
        ]);

        $otherSession = IdentityWebSession::query()->create([
            'id' => (string) Str::uuid(),
            'identity_id' => $identity->id,
            'generation' => $identity->web_session_generation,
            'user_agent' => 'Other browser',
            'ip_hash' => hash('sha256', 'other'),
            'issued_at' => now(),
            'last_seen_at' => now(),
            'expires_at' => now()->addHour(),
        ]);
        $foreignIdentity = $this->identity('foreign@example.test');
        $foreignSession = IdentityWebSession::query()->create([
            'id' => (string) Str::uuid(),
            'identity_id' => $foreignIdentity->id,
            'generation' => $foreignIdentity->web_session_generation,
            'user_agent' => 'Foreign browser',
            'ip_hash' => hash('sha256', 'foreign'),
            'issued_at' => now(),
            'last_seen_at' => now(),
            'expires_at' => now()->addHour(),
        ]);

        $this->delete(route('identity.sessions.destroy', ['session' => $foreignSession->id]))
            ->assertRedirect(route('identity.account-security.show'))
            ->assertSessionHasErrors('session');
        self::assertNull($foreignSession->fresh()?->revoked_at);

        $this->delete(route('identity.sessions.destroy', ['session' => $otherSession->id]))
            ->assertRedirect(route('identity.account-security.show'));
        self::assertNotNull($otherSession->fresh()?->revoked_at);
        self::assertNull(IdentityWebSession::query()->findOrFail($currentSessionId)->revoked_at);
    }

    public function test_email_confirmation_and_old_address_recovery_are_single_use_and_revoke_sessions(): void
    {
        $identity = $this->identity('old@example.test');
        $verificationToken = 'verify-email-change-token';
        $recoveryToken = 'recover-email-change-token';
        $change = IdentityEmailChangeRequest::query()->create([
            'id' => (string) Str::uuid(),
            'identity_id' => $identity->id,
            'old_email' => $identity->email,
            'new_email' => 'new@example.test',
            'verification_token_hash' => IdentitySecret::hash($verificationToken),
            'recovery_token_hash' => IdentitySecret::hash($recoveryToken),
            'requested_at' => now(),
            'expires_at' => now()->addHour(),
        ]);

        app(ConfirmIdentityEmailChange::class)->execute($verificationToken);
        $identity->refresh();
        $change->refresh();
        self::assertSame('new@example.test', $identity->email);
        self::assertNotNull($change->confirmed_at);
        self::assertGreaterThan(0, $identity->web_session_generation);
        self::assertGreaterThan(0, $identity->game_auth_generation);

        $this->expectException(EmailChangeRejected::class);
        app(ConfirmIdentityEmailChange::class)->execute($verificationToken);
    }

    public function test_old_address_recovery_restores_the_email_and_cannot_be_replayed(): void
    {
        $identity = $this->identity('new-address@example.test');
        $recoveryToken = 'old-address-recovery-token';
        $change = IdentityEmailChangeRequest::query()->create([
            'id' => (string) Str::uuid(),
            'identity_id' => $identity->id,
            'old_email' => 'old-address@example.test',
            'new_email' => $identity->email,
            'verification_token_hash' => IdentitySecret::hash('already-used-verification'),
            'recovery_token_hash' => IdentitySecret::hash($recoveryToken),
            'requested_at' => now()->subMinute(),
            'expires_at' => now()->addHour(),
            'confirmed_at' => now(),
            'recoverable_until' => now()->addHour(),
        ]);

        self::assertSame('recovered', app(RecoverIdentityEmailChange::class)->execute($recoveryToken));
        self::assertSame('old-address@example.test', $identity->fresh()?->email);
        self::assertNotNull($change->fresh()?->recovered_at);

        $this->expectException(EmailChangeRejected::class);
        app(RecoverIdentityEmailChange::class)->execute($recoveryToken);
    }

    public function test_recovery_key_is_stored_only_as_a_verifier_and_is_consumed_once(): void
    {
        $identity = $this->identity('recovery-key@example.test');
        $identity->forceFill([
            'two_factor_secret' => 'encrypted-secret',
            'two_factor_recovery_codes' => ['code'],
            'two_factor_confirmed_at' => now(),
        ])->save();

        $service = app(IdentityRecoveryKeyService::class);
        $rawKey = $service->generate($identity);
        self::assertStringStartsWith('OTERYN-', $rawKey);
        $stored = $identity->recoveryKey()->firstOrFail();
        self::assertNotSame($rawKey, $stored->key_hash);

        $service->recover($identity->email, $rawKey, 'New-password-456!');
        $identity->refresh();
        self::assertTrue(Hash::check('New-password-456!', $identity->password));
        self::assertNull($identity->two_factor_secret);
        self::assertNull($identity->two_factor_recovery_codes);
        self::assertNotNull($stored->fresh()?->used_at);

        $this->expectException(RecoveryKeyRejected::class);
        $service->recover($identity->email, $rawKey, 'Another-password-789!');
    }

    public function test_termination_can_be_cancelled_during_grace_and_finalization_preserves_identity_record(): void
    {
        $identity = $this->identity('termination@example.test');

        $scheduled = app(RequestIdentityTermination::class)->execute($identity);
        self::assertTrue($scheduled->hasPendingTermination());
        self::assertNull($scheduled->terminated_at);

        $cancelled = app(CancelIdentityTermination::class)->execute($scheduled);
        self::assertFalse($cancelled->hasPendingTermination());

        $scheduled = app(RequestIdentityTermination::class)->execute($cancelled);
        $scheduled->forceFill(['termination_scheduled_for' => now()->subSecond()])->save();
        self::assertTrue(app(FinalizeIdentityTermination::class)->execute($scheduled->id));

        $terminated = Identity::query()->findOrFail($identity->id);
        self::assertNotNull($terminated->terminated_at);
        self::assertNotNull($terminated->disabled_at);
        self::assertNotNull($terminated->terminated_email_hash);
        self::assertSame("terminated+{$identity->id}@invalid.oteryn", $terminated->email);
        self::assertFalse($terminated->public_account_association);
        self::assertFalse($terminated->public_status_visible);
        self::assertFalse(app(FinalizeIdentityTermination::class)->execute($scheduled->id));
    }

    private function identity(string $email): Identity
    {
        return Identity::query()->create([
            'email' => $email,
            'password' => Hash::make('Correct-password-123!'),
        ])->refresh();
    }
}
