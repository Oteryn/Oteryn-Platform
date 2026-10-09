<?php

namespace Tests\Feature\GameAuth\DeviceSessions;

use App\GameAuth\DeviceSessions\DeviceSessionCredential;
use App\GameAuth\DeviceSessions\DeviceSessionDenied;
use App\GameAuth\DeviceSessions\DeviceSessionFamily;
use App\GameAuth\DeviceSessions\DeviceSessionPurpose;
use App\GameAuth\DeviceSessions\DeviceSessionSecrets;
use App\GameAuth\DeviceSessions\IssuedDeviceSessionCredential;
use App\GameAuth\DeviceSessions\NativeRememberedDeviceSessions;
use App\GameAuth\DeviceSessions\VerifiedRememberedDeviceAuthorization;
use App\GameAuth\NativeLogin\NativeGameLoginTickets;
use App\GameAuth\OAuth\IssueGameLoginTicketFromOAuth;
use App\GameAuth\OAuth\NativeOAuthClientManager;
use App\Identity\Mfa\ConfirmIdentityMfaEnrollment;
use App\Identity\Mfa\StartIdentityMfaEnrollment;
use App\Identity\Models\Identity;
use Carbon\CarbonImmutable;
use Closure;
use Illuminate\Foundation\Testing\DatabaseMigrations;
use Illuminate\Support\Carbon;
use Illuminate\Support\Facades\DB;
use Illuminate\Support\Facades\Hash;
use Illuminate\Support\Facades\Schema;
use Laravel\Passport\Client;
use Laravel\Passport\Token;
use LogicException;
use PHPUnit\Framework\Attributes\DataProvider;
use PragmaRX\Google2FA\Google2FA;
use RuntimeException;
use Tests\Feature\GameAuth\OAuth\Concerns\ConfiguresEphemeralPassportKeys;
use Tests\Feature\GameAuth\OAuth\Concerns\CreatesNativeOAuthBootstrapToken;
use Tests\TestCase;

final class NativeRememberedDeviceSessionsTest extends TestCase
{
    use ConfiguresEphemeralPassportKeys;
    use CreatesNativeOAuthBootstrapToken;
    use DatabaseMigrations {
        runDatabaseMigrations as private runEphemeralDatabaseMigrations;
    }

    private mixed $passportConnectionBeforeTest = null;

    private string $environmentBeforeTest = 'testing';

    public function runDatabaseMigrations(): void
    {
        // This suite needs committed transactions; never migrate/reset a runtime database.
        if (DB::getDefaultConnection() !== 'sqlite' || config('database.connections.sqlite.database') !== ':memory:') {
            throw new LogicException('Remembered-device service tests require an isolated in-memory SQLite database.');
        }
        $this->runEphemeralDatabaseMigrations();
    }

    protected function setUp(): void
    {
        parent::setUp();
        $this->passportConnectionBeforeTest = config('passport.connection');
        $this->environmentBeforeTest = app()->environment();
        $this->configureEphemeralPassportKeys();
        config(['game-auth.device_sessions.enabled' => true, 'game-auth.native_admission.enabled' => true]);
    }

    protected function tearDown(): void
    {
        Carbon::setTestNow();
        CarbonImmutable::setTestNow();
        config(['passport.connection' => $this->passportConnectionBeforeTest]);
        $this->app['env'] = $this->environmentBeforeTest;

        if (DB::getDefaultConnection() !== 'sqlite' || config('database.connections.sqlite.database') !== ':memory:') {
            throw new LogicException('Remembered-device fixture cleanup requires an isolated in-memory SQLite database.');
        }

        // Only discard this suite's ephemeral fixtures, letting the migration test retain
        // its real refuse-populated-rollback behavior without failing framework teardown.
        if (Schema::hasTable('native_device_session_credentials')) {
            DB::table('native_device_session_credentials')->delete();
        }
        if (Schema::hasTable('native_device_session_families')) {
            DB::table('native_device_session_families')->delete();
        }
        if (Schema::hasTable('native_admission_attempts')) {
            DB::table('native_admission_attempts')->delete();
        }
        if (Schema::hasTable('game_login_tickets')) {
            DB::table('game_login_tickets')->whereNull('canary_account_id')->delete();
        }
        parent::tearDown();
    }

    public function test_feature_is_off_by_default_and_refuses_production_even_when_flag_is_true(): void
    {
        [$identity, $token] = $this->fixture();
        config(['game-auth.device_sessions.enabled' => false]);
        $this->denied(fn () => $this->service()->enroll($identity, $token, true));
        config(['game-auth.device_sessions.enabled' => true]);
        $this->app['env'] = 'production';
        $this->denied(fn () => $this->service()->enroll($identity, $token, true));
        self::assertSame(0, DeviceSessionFamily::query()->count());
    }

    public function test_enrollment_requires_explicit_consent_and_can_occur_only_once_per_oauth_token(): void
    {
        [$identity, $token] = $this->fixture();
        $this->denied(fn () => $this->service()->enroll($identity, $token));
        $issued = $this->service()->enroll($identity, $token, true);
        $this->denied(fn () => $this->service()->enroll($identity, $token, true));
        self::assertSame(1, DeviceSessionFamily::query()->count());
        self::assertSame(1, DeviceSessionCredential::query()->count());
        self::assertFalse(Token::query()->findOrFail($token)->revoked);
        $this->assertStoredSecretIsOnlyHash($issued);
    }

    public function test_real_pkce_enrollment_precedes_unchanged_single_use_oauth_ticket_revocation(): void
    {
        $identity = $this->createOAuthIdentity();
        $client = app(NativeOAuthClientManager::class)->ensureRust();
        $this->issueNativeOAuthBootstrapToken($identity, client: $client);
        $token = Token::query()->where('user_id', $identity->id)->sole();
        $tokenId = (string) $token->getKey();
        $issued = $this->service()->enroll($identity, $tokenId, true);

        app(IssueGameLoginTicketFromOAuth::class)->execute($identity, $tokenId);

        self::assertTrue($token->fresh()->revoked);
        self::assertSame(0, DB::table('oauth_refresh_tokens')->where('revoked', false)->count());
        self::assertNull(DeviceSessionFamily::query()->findOrFail($issued->familyId)->revoked_at);
        $rotated = $this->service()->rotateAndUse($issued->secret(), (string) $client->getKey(), DeviceSessionPurpose::NativeTicketIssue,
            fn (VerifiedRememberedDeviceAuthorization $authorization) => app(NativeGameLoginTickets::class)->issue($authorization->identity()));
        self::assertSame(2, DB::table('game_login_tickets')->count());
        self::assertNotSame($issued->secret(), $rotated->credential()->secret());
        self::assertSame(0, DB::table('oauth_access_tokens')->where('revoked', false)->count());
    }

    public function test_legacy_client_and_foreign_identity_cannot_enroll(): void
    {
        $legacy = app(NativeOAuthClientManager::class)->ensure();
        [$identity, $token] = $this->fixture($legacy);
        $this->denied(fn () => $this->service()->enroll($identity, $token, true));
        [$identity, $token] = $this->fixture();
        [$other] = $this->fixture();
        $this->denied(fn () => $this->service()->enroll($other, $token, true));
        self::assertSame(0, DeviceSessionFamily::query()->count());
    }

    #[DataProvider('oauthDefects')]
    /** @param array<string, mixed> $mutation */
    public function test_invalid_oauth_authority_cannot_enroll(array $mutation): void
    {
        [$identity, $token] = $this->fixture();
        Token::query()->findOrFail($token)->forceFill($mutation)->save();
        $this->denied(fn () => $this->service()->enroll($identity, $token, true));
        self::assertSame(0, DeviceSessionFamily::query()->count());
    }

    /** @return array<string, array{array<string, mixed>}> */
    public static function oauthDefects(): array
    {
        return [
            'revoked' => [['revoked' => true]],
            'expired' => [['expires_at' => '2000-01-01 00:00:00']],
            'missing scope' => [['scopes' => []]],
            'old generation' => [['game_auth_generation' => 999]],
        ];
    }

    public function test_rotation_returns_only_current_owner_facts_and_preserves_original_generations(): void
    {
        [$identity, $token, $client] = $this->fixture();
        $issued = $this->service()->enroll($identity, $token, true);
        $rotated = $this->service()->rotateAndUse($issued->secret(), (string) $client->getKey(), DeviceSessionPurpose::NativeCharactersRead,
            function (VerifiedRememberedDeviceAuthorization $authorization) use ($identity, $client, $issued): string {
                self::assertSame($identity->id, $authorization->identityId);
                self::assertSame($identity->account_id, $authorization->accountId);
                self::assertSame((string) $client->getKey(), $authorization->oauthClientId);
                self::assertSame($identity->game_auth_generation, $authorization->gameAuthGeneration);
                self::assertSame($identity->native_security_generation, $authorization->nativeSecurityGeneration);
                self::assertSame($issued->familyId, $authorization->familyId);
                self::assertSame(DeviceSessionPurpose::NativeCharactersRead, $authorization->purpose);
                self::assertSame(1, DB::transactionLevel());

                return 'owner-scoped-result';
            });

        self::assertSame('owner-scoped-result', $rotated->result());
        self::assertSame($issued->absoluteExpiresAt->getTimestamp(), $rotated->credential()->absoluteExpiresAt->getTimestamp());
        self::assertSame(2, DeviceSessionCredential::query()->count());
        self::assertSame(2, DeviceSessionFamily::query()->findOrFail($issued->familyId)->current_sequence);
        $this->assertStoredSecretIsOnlyHash($rotated->credential());
    }

    public function test_replay_revokes_whole_family_and_durable_revocation_survives_thrown_denial(): void
    {
        [$identity, $token, $client] = $this->fixture();
        $issued = $this->service()->enroll($identity, $token, true);
        $current = $issued;
        for ($index = 0; $index < 3; $index++) {
            $current = $this->service()->rotateAndUse($current->secret(), (string) $client->getKey(), DeviceSessionPurpose::NativeCharactersRead,
                fn () => 'result')->credential();
        }
        $this->denied(fn () => $this->service()->rotateAndUse($issued->secret(), (string) $client->getKey(), DeviceSessionPurpose::NativeCharactersRead,
            fn () => self::fail('Replayed credentials must not reach native application code.')));
        self::assertSame('credential_replay', DeviceSessionFamily::query()->findOrFail($issued->familyId)->revocation_reason);
        $this->denied(fn () => $this->service()->rotateAndUse($current->secret(), (string) $client->getKey(), DeviceSessionPurpose::NativeTicketIssue,
            fn () => self::fail('A replay must revoke the current successor too.')));
        self::assertSame(0, DB::table('game_login_tickets')->count());
    }

    public function test_wrong_client_unknown_and_malformed_credentials_do_not_revoke_someone_elses_family(): void
    {
        [$identity, $token, $client] = $this->fixture();
        $issued = $this->service()->enroll($identity, $token, true);
        $this->denied(fn () => $this->service()->rotateAndUse($issued->secret(), 'wrong-client', DeviceSessionPurpose::NativeCharactersRead, fn () => null));
        foreach (['', str_repeat('x', 4096), app(DeviceSessionSecrets::class)->generate()] as $secret) {
            $this->denied(fn () => $this->service()->rotateAndUse($secret, (string) $client->getKey(), DeviceSessionPurpose::NativeCharactersRead, fn () => null));
        }
        self::assertNull(DeviceSessionFamily::query()->findOrFail($issued->familyId)->revoked_at);
        self::assertSame(1, DeviceSessionCredential::query()->count());
    }

    #[DataProvider('identityDefects')]
    /** @param array<string, mixed> $mutation */
    public function test_security_change_rejects_authority_and_commits_family_revocation(array $mutation): void
    {
        [$identity, $token, $client] = $this->fixture();
        $issued = $this->service()->enroll($identity, $token, true);
        $identity->forceFill($mutation)->save();
        $this->denied(fn () => $this->service()->rotateAndUse($issued->secret(), (string) $client->getKey(), DeviceSessionPurpose::NativeTicketIssue,
            fn () => self::fail('Changed security context cannot issue a ticket.')));
        self::assertSame('authorization_changed', DeviceSessionFamily::query()->findOrFail($issued->familyId)->revocation_reason);
        self::assertSame(1, DeviceSessionCredential::query()->count());
    }

    /** @return array<string, array{array<string, mixed>}> */
    public static function identityDefects(): array
    {
        return [
            'game authorization revoked' => [['game_auth_generation' => 999]],
            'native security revoked' => [['native_security_generation' => 999]],
            'disabled' => [['disabled_at' => '2026-10-09 00:00:00']],
            'terminated' => [['terminated_at' => '2026-10-09 00:00:00']],
        ];
    }

    public function test_oauth_client_revocation_blocks_existing_remembered_device(): void
    {
        [$identity, $token, $client] = $this->fixture();
        $issued = $this->service()->enroll($identity, $token, true);
        $client->forceFill(['revoked' => true])->save();
        $this->denied(fn () => $this->service()->rotateAndUse($issued->secret(), (string) $client->getKey(), DeviceSessionPurpose::NativeCharactersRead, fn () => null));
        self::assertSame('authorization_changed', DeviceSessionFamily::query()->findOrFail($issued->familyId)->revocation_reason);
    }

    public function test_real_mfa_confirmation_invalidates_devices_and_oauth_authority_enrolled_before_mfa(): void
    {
        [$identity, $token, $client] = $this->fixture();
        $issued = $this->service()->enroll($identity, $token, true);
        $originalGameGeneration = $identity->game_auth_generation;
        $originalNativeGeneration = $identity->native_security_generation;

        $pendingIdentity = app(StartIdentityMfaEnrollment::class)->execute($identity);
        self::assertFalse($pendingIdentity->hasConfirmedMfa());
        self::assertSame($originalGameGeneration, $pendingIdentity->game_auth_generation);
        self::assertSame($originalNativeGeneration, $pendingIdentity->native_security_generation);
        $secret = $pendingIdentity->two_factor_secret;
        self::assertIsString($secret);
        $code = app(Google2FA::class)->getCurrentOtp($secret);

        $confirmation = app(ConfirmIdentityMfaEnrollment::class)->execute(
            $pendingIdentity,
            'Correct-Horse-9!Battery',
            $code,
        );

        self::assertTrue($confirmation->identity->hasConfirmedMfa());
        self::assertSame($originalGameGeneration + 1, $confirmation->identity->game_auth_generation);
        self::assertSame($originalNativeGeneration + 1, $confirmation->identity->native_security_generation);
        self::assertTrue(Token::query()->findOrFail($token)->revoked);
        $this->denied(fn () => $this->service()->rotateAndUse(
            $issued->secret(),
            (string) $client->getKey(),
            DeviceSessionPurpose::NativeTicketIssue,
            fn () => self::fail('A remembered device authenticated before MFA confirmation cannot bypass the new factor.'),
        ));
        self::assertSame('authorization_changed', DeviceSessionFamily::query()->findOrFail($issued->familyId)->revocation_reason);
        self::assertSame(0, DB::table('game_login_tickets')->count());
    }

    public function test_idle_expiry_is_terminal_and_success_cannot_extend_absolute_expiry(): void
    {
        $this->freezeClock('2026-10-09 10:00:00');
        config(['game-auth.device_sessions.absolute_ttl_seconds' => 120, 'game-auth.device_sessions.idle_ttl_seconds' => 60]);
        [$identity, $token, $client] = $this->fixture();
        $issued = $this->service()->enroll($identity, $token, true);
        $this->freezeClock('2026-10-09 10:00:59');
        $rotated = $this->service()->rotateAndUse($issued->secret(), (string) $client->getKey(), DeviceSessionPurpose::NativeCharactersRead, fn () => null)->credential();
        self::assertSame('2026-10-09 10:01:59', $rotated->idleExpiresAt->format('Y-m-d H:i:s'));
        $this->freezeClock('2026-10-09 10:01:58');
        $rotated = $this->service()->rotateAndUse($rotated->secret(), (string) $client->getKey(), DeviceSessionPurpose::NativeCharactersRead, fn () => null)->credential();
        self::assertSame('2026-10-09 10:02:00', $rotated->idleExpiresAt->format('Y-m-d H:i:s'));
        self::assertSame('2026-10-09 10:02:00', $rotated->absoluteExpiresAt->format('Y-m-d H:i:s'));
        $this->freezeClock('2026-10-09 10:02:00');
        $this->denied(fn () => $this->service()->rotateAndUse($rotated->secret(), (string) $client->getKey(), DeviceSessionPurpose::NativeCharactersRead, fn () => null));
        self::assertSame('expired', DeviceSessionFamily::query()->findOrFail($issued->familyId)->revocation_reason);
    }

    public function test_idle_boundary_expires_before_absolute_boundary(): void
    {
        $this->freezeClock('2026-10-09 10:00:00');
        config(['game-auth.device_sessions.absolute_ttl_seconds' => 120, 'game-auth.device_sessions.idle_ttl_seconds' => 60]);
        [$identity, $token, $client] = $this->fixture();
        $issued = $this->service()->enroll($identity, $token, true);
        $this->freezeClock('2026-10-09 10:01:00');
        $this->denied(fn () => $this->service()->rotateAndUse($issued->secret(), (string) $client->getKey(), DeviceSessionPurpose::NativeCharactersRead, fn () => null));
        self::assertSame('expired', DeviceSessionFamily::query()->findOrFail($issued->familyId)->revocation_reason);
    }

    public function test_callback_failure_rolls_back_rotation_and_native_mutation(): void
    {
        [$identity, $token, $client] = $this->fixture();
        $issued = $this->service()->enroll($identity, $token, true);
        try {
            $this->service()->rotateAndUse($issued->secret(), (string) $client->getKey(), DeviceSessionPurpose::NativeTicketIssue,
                function (VerifiedRememberedDeviceAuthorization $authorization): never {
                    app(NativeGameLoginTickets::class)->issue($authorization->identity());
                    throw new RuntimeException('Test operation failed before commit.');
                });
            self::fail('Expected callback failure.');
        } catch (RuntimeException $exception) {
            self::assertSame('Test operation failed before commit.', $exception->getMessage());
        }
        self::assertSame(0, DB::table('game_login_tickets')->count());
        self::assertSame(1, DeviceSessionCredential::query()->count());
        self::assertNull(DeviceSessionCredential::query()->sole()->consumed_at);
        self::assertSame(1, DeviceSessionFamily::query()->sole()->current_sequence);
    }

    public function test_explicit_revoke_is_idempotent_and_wrong_owner_cannot_revoke(): void
    {
        [$identity, $token, $client] = $this->fixture();
        [$other] = $this->fixture();
        $issued = $this->service()->enroll($identity, $token, true);
        $this->denied(fn () => $this->service()->revokeForOwner($other, $issued->familyId));
        $this->denied(fn () => $this->service()->revokeCredential($issued->secret(), 'wrong-client'));
        self::assertNull(DeviceSessionFamily::query()->findOrFail($issued->familyId)->revoked_at);
        $this->service()->revokeForOwner($identity, $issued->familyId);
        $this->service()->revokeForOwner($identity, $issued->familyId);
        $this->denied(fn () => $this->service()->rotateAndUse($issued->secret(), (string) $client->getKey(), DeviceSessionPurpose::NativeCharactersRead, fn () => null));
        self::assertSame('owner_revoked', DeviceSessionFamily::query()->findOrFail($issued->familyId)->revocation_reason);
    }

    public function test_logout_can_revoke_with_predecessor_after_an_uncertain_rotation_response(): void
    {
        [$identity, $token, $client] = $this->fixture();
        $issued = $this->service()->enroll($identity, $token, true);
        $successor = $this->service()->rotateAndUse($issued->secret(), (string) $client->getKey(), DeviceSessionPurpose::NativeCharactersRead, fn () => null)->credential();
        $this->service()->revokeCredential($issued->secret(), (string) $client->getKey());
        $this->service()->revokeCredential($issued->secret(), (string) $client->getKey());
        $this->denied(fn () => $this->service()->rotateAndUse($successor->secret(), (string) $client->getKey(), DeviceSessionPurpose::NativeTicketIssue, fn () => null));
        self::assertSame('client_revoked', DeviceSessionFamily::query()->findOrFail($issued->familyId)->revocation_reason);
    }

    public function test_outer_transaction_and_cross_database_passport_are_rejected(): void
    {
        [$identity, $token] = $this->fixture();
        try {
            DB::transaction(fn () => $this->service()->enroll($identity, $token, true));
            self::fail('An outer transaction could roll back terminal revocation.');
        } catch (LogicException $exception) {
            self::assertStringContainsString('own their transaction', $exception->getMessage());
        }
        config(['passport.connection' => 'other-database']);
        try {
            $this->service()->enroll($identity, $token, true);
            self::fail('Cross-database authority is not atomic.');
        } catch (LogicException $exception) {
            self::assertStringContainsString('one database connection', $exception->getMessage());
        }
        self::assertSame(0, DeviceSessionFamily::query()->count());
    }

    public function test_plaintext_is_redacted_in_debug_json_and_forbidden_in_serialization(): void
    {
        [$identity, $token] = $this->fixture();
        $issued = $this->service()->enroll($identity, $token, true);
        self::assertStringNotContainsString($issued->secret(), var_export($issued->__debugInfo(), true));
        self::assertStringNotContainsString($issued->secret(), (string) json_encode($issued));
        $this->expectException(LogicException::class);
        serialize($issued);
    }

    public function test_migration_rollback_preserves_any_retained_authentication_history(): void
    {
        [$identity, $token] = $this->fixture();
        $this->service()->enroll($identity, $token, true);
        $migration = require database_path('migrations/2026_10_09_220000_create_native_device_sessions.php');
        try {
            $migration->down();
            self::fail('Rollback must not erase credential replay history.');
        } catch (RuntimeException $exception) {
            self::assertStringContainsString('authorized retirement', $exception->getMessage());
        }
        self::assertSame(1, DeviceSessionFamily::query()->count());
        self::assertSame(1, DeviceSessionCredential::query()->count());
    }

    private function service(): NativeRememberedDeviceSessions
    {
        return app(NativeRememberedDeviceSessions::class);
    }

    /** @return array{Identity, string, Client} */
    private function fixture(?Client $client = null): array
    {
        $identity = Identity::query()->create([
            'email' => 'device-'.bin2hex(random_bytes(8)).'@example.invalid',
            'password' => Hash::make('Correct-Horse-9!Battery'),
        ])->fresh();
        $client ??= app(NativeOAuthClientManager::class)->ensureRust();
        $tokenId = bin2hex(random_bytes(40));
        $token = new Token;
        $token->forceFill([
            'id' => $tokenId,
            'user_id' => $identity->id,
            'client_id' => $client->getKey(),
            'scopes' => ['game:ticket'],
            'revoked' => false,
            'expires_at' => now()->addMinutes(5),
            'game_auth_generation' => $identity->game_auth_generation,
        ])->save();

        return [$identity, $tokenId, $client];
    }

    private function assertStoredSecretIsOnlyHash(IssuedDeviceSessionCredential $issued): void
    {
        $hash = app(DeviceSessionSecrets::class)->hash($issued->secret());
        self::assertSame(64, strlen($hash));
        self::assertNotSame($issued->secret(), $hash);
        self::assertTrue(DeviceSessionCredential::query()->whereKey($hash)->exists());
        self::assertStringNotContainsString($issued->secret(), (string) json_encode(DB::table('native_device_session_credentials')->get()));
        self::assertStringNotContainsString($issued->secret(), (string) json_encode(DB::table('native_device_session_families')->get()));
    }

    private function denied(Closure $operation): void
    {
        try {
            $operation();
            self::fail('Expected remembered device authorization refusal.');
        } catch (DeviceSessionDenied $exception) {
            self::assertSame('Remembered device authorization is unavailable. Sign in again.', $exception->getMessage());
        }
    }

    private function freezeClock(string $at): void
    {
        Carbon::setTestNow($at);
        CarbonImmutable::setTestNow($at);
    }
}
