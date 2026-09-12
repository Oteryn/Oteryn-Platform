<?php

namespace Tests\Feature\GameAuth;

use App\GameAuth\NativeEvidence\NativeEvidenceContract;
use App\GameAuth\NativeEvidence\NativeEvidenceRecoveryReconciler;
use App\GameAuth\NativeEvidence\NativeEvidenceUnavailable;
use App\GameAuth\NativeEvidence\NativeSigningTrustRegistry;
use App\Identity\Actions\RevokeIdentityGameAuthorizations;
use App\Identity\Models\Identity;
use Illuminate\Database\Migrations\Migration;
use Illuminate\Foundation\Testing\RefreshDatabase;
use Illuminate\Support\Facades\Artisan;
use Illuminate\Support\Facades\DB;
use Illuminate\Support\Facades\Hash;
use Illuminate\Testing\TestResponse;
use LogicException;
use Symfony\Component\Console\Command\Command;
use Symfony\Component\HttpFoundation\Response;
use Tests\TestCase;

final class NativeEvidenceHardeningTest extends TestCase
{
    use RefreshDatabase;

    private const PEER_SUBJECT = 'CN=oteryn-game-native-evidence';

    private string $witnessDirectory;

    protected function setUp(): void
    {
        parent::setUp();

        self::assertTrue(function_exists('fsync'), 'Native evidence hardening requires PHP fsync support in the qualified runtime.');
        $this->witnessDirectory = storage_path('framework/testing/native-evidence-hardening-'.bin2hex(random_bytes(6)));
        mkdir($this->witnessDirectory, 0700, true);
        config([
            'game-auth.native_evidence.source_authority' => 'platform',
            'game-auth.native_evidence.activated' => true,
            'game-auth.native_evidence.mtls_client_identity' => self::PEER_SUBJECT,
            'game-auth.native_evidence.high_water_directory' => $this->witnessDirectory,
            'game-auth.native_evidence.fresh_account_purpose' => 'platform_security',
            'game-auth.native_evidence.fresh_account_scope' => 'fresh_admission',
            'game-auth.native_evidence.fresh_key_purpose' => 'fresh_admission',
            'game-auth.native_evidence.clock_uncertainty_seconds' => 0,
            'game-auth.native_evidence.requests_per_minute' => 120,
        ]);
    }

    protected function tearDown(): void
    {
        foreach (glob($this->witnessDirectory.'/*') ?: [] as $path) {
            @unlink($path);
        }
        @rmdir($this->witnessDirectory);

        parent::tearDown();
    }

    public function test_retained_store_identity_recovers_database_binding_but_rejects_an_empty_replacement_store(): void
    {
        $identity = $this->identity('native-store-provenance@example.test');
        $request = $this->freshAccountRequest($identity->account_id);

        $this->postEvidence($request)->assertOk()->assertJsonPath('source_revision', '1');
        $marker = $this->witnessDirectory.'/witness-store.id';
        self::assertFileExists($marker);
        $storeId = trim((string) file_get_contents($marker));
        self::assertMatchesRegularExpression('/^[0-9a-f]{64}$/', $storeId);
        self::assertSame($storeId, DB::table('native_game_evidence_witness_stores')->where('id', 1)->value('store_id'));

        DB::table('native_game_evidence_witness_stores')->delete();
        $this->postEvidence($request)->assertOk()->assertJsonPath('source_revision', '2');
        self::assertSame($storeId, DB::table('native_game_evidence_witness_stores')->where('id', 1)->value('store_id'));

        foreach (glob($this->witnessDirectory.'/*') ?: [] as $path) {
            @unlink($path);
        }

        $this->postEvidence($request)->assertOk()->assertExactJson([
            'version' => 1,
            'operation' => NativeEvidenceContract::FRESH_ACCOUNT,
            'result' => 'unavailable',
        ]);
    }

    public function test_account_generation_reconciliation_is_explicit_forward_only_after_outer_transaction_rollback(): void
    {
        $identity = $this->identity('native-account-reconcile@example.test');
        $request = $this->freshAccountRequest($identity->account_id);
        $this->postEvidence($request)->assertOk()->assertJsonPath('minimum_valid_generation', '1');

        DB::beginTransaction();
        try {
            app(RevokeIdentityGameAuthorizations::class)->execute($identity->refresh());
            self::assertSame(2, $identity->refresh()->native_security_generation);
        } finally {
            DB::rollBack();
        }
        self::assertSame(1, $identity->refresh()->native_security_generation);

        $this->postEvidence($request)->assertOk()->assertExactJson([
            'version' => 1,
            'operation' => NativeEvidenceContract::FRESH_ACCOUNT,
            'result' => 'unavailable',
        ]);

        self::assertSame(Command::SUCCESS, Artisan::call('game-auth:native-evidence:reconcile', [
            '--account-id' => $identity->account_id,
        ]));
        self::assertStringContainsString('generation to 2', Artisan::output());
        self::assertSame(2, $identity->refresh()->native_security_generation);
        $this->postEvidence($request)->assertOk()
            ->assertJsonPath('source_revision', '2')
            ->assertJsonPath('minimum_valid_generation', '2');

        DB::table('identities')->where('id', $identity->id)->update(['native_security_generation' => 3]);
        $this->expectException(NativeEvidenceUnavailable::class);
        app(NativeEvidenceRecoveryReconciler::class)->reconcileAccountGeneration($identity->account_id);
    }

    public function test_ambiguous_signing_trust_rollback_is_reconciled_revoked_before_explicit_successor_version(): void
    {
        $registry = app(NativeSigningTrustRegistry::class);
        $keyOne = str_repeat("\x01", 32);
        $keyTwo = str_repeat("\x02", 32);
        $registry->publishTrustedKey(
            NativeEvidenceContract::FRESH_ISSUER,
            NativeEvidenceContract::FRESH_PROFILE,
            'fresh_admission',
            'key-1',
            $keyOne,
        );
        $this->postEvidence($this->freshTrustRequest('key-1'))->assertOk()->assertJsonPath('trusted', true);

        DB::beginTransaction();
        try {
            $registry->revokeProfile(
                NativeEvidenceContract::FRESH_ISSUER,
                NativeEvidenceContract::FRESH_PROFILE,
                'fresh_admission',
            );
            self::assertNotNull(DB::table('native_game_signing_trust_profiles')->where('profile_version', 1)->value('revoked_at'));
        } finally {
            DB::rollBack();
        }
        self::assertNull(DB::table('native_game_signing_trust_profiles')->where('profile_version', 1)->value('revoked_at'));

        $this->postEvidence($this->freshTrustRequest('key-1'))->assertOk()->assertExactJson([
            'version' => 1,
            'operation' => NativeEvidenceContract::FRESH_TRUST,
            'result' => 'unavailable',
        ]);

        self::assertSame(Command::SUCCESS, Artisan::call('game-auth:native-evidence:reconcile', ['--trust' => 'fresh']));
        self::assertStringContainsString('profile version remains revoked', Artisan::output());
        self::assertEquals(2, DB::table('native_game_signing_trust_profiles')->where('profile_version', 1)->value('issuer_revision'));
        self::assertNotNull(DB::table('native_game_signing_trust_profiles')->where('profile_version', 1)->value('revoked_at'));
        $this->postEvidence($this->freshTrustRequest('key-1'))->assertOk()->assertJsonPath('trusted', false);

        try {
            $registry->publishTrustedKey(
                NativeEvidenceContract::FRESH_ISSUER,
                NativeEvidenceContract::FRESH_PROFILE,
                'fresh_admission',
                'key-2',
                $keyTwo,
            );
            self::fail('A revoked trust profile version must not be reopened by ordinary key publication.');
        } catch (LogicException $exception) {
            self::assertStringContainsString('cannot accept a trusted key', $exception->getMessage());
        }

        $next = $registry->publishNextProfileVersion(
            NativeEvidenceContract::FRESH_ISSUER,
            NativeEvidenceContract::FRESH_PROFILE,
            'fresh_admission',
            'key-2',
            $keyTwo,
        );
        self::assertEquals(1, $next->key_revision);
        self::assertSame(2, DB::table('native_game_signing_trust_profiles')->count());
        self::assertEquals(3, DB::table('native_game_signing_trust_profiles')->where('profile_version', 2)->value('issuer_revision'));
        self::assertNull(DB::table('native_game_signing_trust_profiles')->where('profile_version', 2)->value('revoked_at'));
        $this->postEvidence($this->freshTrustRequest('key-2'))->assertOk()->assertJsonPath('trusted', true);

        $registry->revokeProfile(
            NativeEvidenceContract::FRESH_ISSUER,
            NativeEvidenceContract::FRESH_PROFILE,
            'fresh_admission',
        );
        $this->expectException(LogicException::class);
        $registry->publishNextProfileVersion(
            NativeEvidenceContract::FRESH_ISSUER,
            NativeEvidenceContract::FRESH_PROFILE,
            'fresh_admission',
            'key-1',
            $keyOne,
        );
    }

    public function test_hardening_migration_refuses_destructive_rollback_while_native_authority_is_active(): void
    {
        /** @var Migration $migration */
        $migration = require database_path('migrations/2026_09_12_204800_harden_native_game_evidence_authority.php');
        self::assertIsObject($migration);
        config(['game-auth.native_evidence.activated' => true]);

        $this->expectException(LogicException::class);
        $this->expectExceptionMessage('unless activation is explicitly false');
        $migration->down();
    }

    private function identity(string $email): Identity
    {
        return Identity::query()->create([
            'email' => $email,
            'password' => Hash::make('correct horse battery staple'),
        ]);
    }

    /** @return array<string, int|string> */
    private function freshAccountRequest(string $accountId): array
    {
        return [
            'version' => 1,
            'operation' => NativeEvidenceContract::FRESH_ACCOUNT,
            'account_id' => $accountId,
            'purpose' => 'platform_security',
            'scope' => 'fresh_admission',
        ];
    }

    /** @return array<string, int|string> */
    private function freshTrustRequest(string $keyId): array
    {
        return [
            'version' => 1,
            'operation' => NativeEvidenceContract::FRESH_TRUST,
            'issuer' => NativeEvidenceContract::FRESH_ISSUER,
            'profile' => NativeEvidenceContract::FRESH_PROFILE,
            'key_purpose' => 'fresh_admission',
            'key_id' => $keyId,
        ];
    }

    /**
     * @param  array<string, int|string>  $payload
     * @return TestResponse<Response>
     */
    private function postEvidence(array $payload): TestResponse
    {
        return $this->withServerVariables([
            'CONTENT_TYPE' => 'application/json',
            'SSL_CLIENT_VERIFY' => 'SUCCESS',
            'SSL_PROTOCOL' => 'TLSv1.3',
            'SSL_CLIENT_S_DN' => self::PEER_SUBJECT,
        ])->postJson('/internal/v1/game-auth/native-evidence', $payload);
    }
}
