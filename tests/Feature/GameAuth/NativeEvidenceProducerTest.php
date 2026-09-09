<?php

namespace Tests\Feature\GameAuth;

use App\GameAuth\NativeEvidence\NativeEvidenceContract;
use App\GameAuth\NativeEvidence\NativeEvidenceNamespace;
use App\GameAuth\NativeEvidence\NativeSigningTrustRegistry;
use App\Identity\Actions\RevokeIdentityGameAuthorizations;
use App\Identity\Models\Identity;
use App\Identity\Support\CanonicalAccountId;
use Illuminate\Foundation\Testing\RefreshDatabase;
use Illuminate\Support\Facades\DB;
use Illuminate\Support\Facades\Hash;
use Illuminate\Testing\TestResponse;
use LogicException;
use Symfony\Component\HttpFoundation\Response;
use Tests\TestCase;

final class NativeEvidenceProducerTest extends TestCase
{
    use RefreshDatabase;

    private const PEER_SUBJECT = 'CN=oteryn-game-native-evidence';

    private string $witnessDirectory;

    protected function setUp(): void
    {
        parent::setUp();

        $this->witnessDirectory = storage_path('framework/testing/native-evidence-'.bin2hex(random_bytes(6)));
        mkdir($this->witnessDirectory, 0700, true);
        config([
            'game-auth.native_evidence.source_authority' => 'platform',
            'game-auth.native_evidence.mtls_client_identity' => self::PEER_SUBJECT,
            'game-auth.native_evidence.high_water_directory' => $this->witnessDirectory,
            'game-auth.native_evidence.fresh_account_purpose' => 'platform_security',
            'game-auth.native_evidence.fresh_account_scope' => 'fresh_admission',
            'game-auth.native_evidence.fresh_key_purpose' => 'fresh_admission',
            'game-auth.native_evidence.clock_uncertainty_seconds' => 0,
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

    public function test_account_id_is_platform_issued_uuidv7_and_immutable(): void
    {
        $identity = $this->identity('account-id@example.test');

        self::assertTrue(CanonicalAccountId::isValid($identity->account_id));
        self::assertSame(1, $identity->native_security_generation);
        self::assertSame(0, $identity->refresh()->game_auth_generation);

        $identity->forceFill(['account_id' => CanonicalAccountId::generate()]);
        $this->expectException(LogicException::class);
        $identity->save();
    }

    public function test_fresh_and_recovery_share_ordering_and_native_generation_without_hidden_translation(): void
    {
        $identity = $this->identity('ordering@example.test');

        DB::table('identities')->where('id', $identity->id)->update([
            'game_auth_generation' => 41,
            'native_security_generation' => 7,
        ]);
        $identity->refresh();

        $fresh = $this->postEvidence([
            'version' => 1,
            'operation' => NativeEvidenceContract::FRESH_ACCOUNT,
            'account_id' => $identity->account_id,
            'purpose' => 'platform_security',
            'scope' => 'fresh_admission',
        ])->assertOk();
        $this->assertSensitiveResponseIsNotCacheable($fresh);

        $fresh->assertExactJson([
            'version' => 1,
            'operation' => NativeEvidenceContract::FRESH_ACCOUNT,
            'result' => 'observed',
            'source_authority' => 'platform',
            'source_revision' => '1',
            'decision_identity' => '1',
            'source_observed_at' => $fresh->json('source_observed_at'),
            'clock_uncertainty_seconds' => '0',
            'account_id' => $identity->account_id,
            'purpose' => 'platform_security',
            'scope' => 'fresh_admission',
            'allowed' => true,
            'minimum_valid_generation' => '7',
        ]);
        self::assertNotSame('42', $fresh->json('minimum_valid_generation'));

        $stored = DB::table('native_game_evidence_observations')
            ->where('namespace_hash', NativeEvidenceNamespace::accountSource($identity->account_id))
            ->where('source_revision', 1)
            ->value('response_json');
        self::assertIsString($stored);
        self::assertSame($fresh->json(), json_decode($stored, true, 16, JSON_THROW_ON_ERROR));

        $recovery = $this->postEvidence([
            'version' => 2,
            'operation' => NativeEvidenceContract::RECOVERY_ACCOUNT,
            'account_id' => $identity->account_id,
            'purpose' => NativeEvidenceContract::RECOVERY_PURPOSE,
            'scope' => NativeEvidenceContract::RECOVERY_SCOPE,
        ])->assertOk();
        self::assertSame('2', $recovery->json('source_revision'));
        self::assertSame('7', $recovery->json('minimum_valid_generation'));

        $issuedNativeGeneration = $this->integerValue($fresh->json('minimum_valid_generation'));
        app(RevokeIdentityGameAuthorizations::class)->execute($identity->refresh());
        $afterRevocation = $this->postEvidence([
            'version' => 2,
            'operation' => NativeEvidenceContract::RECOVERY_ACCOUNT,
            'account_id' => $identity->account_id,
            'purpose' => NativeEvidenceContract::RECOVERY_PURPOSE,
            'scope' => NativeEvidenceContract::RECOVERY_SCOPE,
        ])->assertOk();
        self::assertSame('3', $afterRevocation->json('source_revision'));
        self::assertSame('8', $afterRevocation->json('minimum_valid_generation'));
        self::assertGreaterThan($issuedNativeGeneration, $this->integerValue($afterRevocation->json('minimum_valid_generation')));
    }

    public function test_security_generation_witness_detects_post_revocation_database_rollback(): void
    {
        $identity = $this->identity('generation-rollback@example.test');
        $request = $this->freshAccountRequest($identity->account_id);

        $this->postEvidence($request)->assertOk()->assertJsonPath('minimum_valid_generation', '1');
        app(RevokeIdentityGameAuthorizations::class)->execute($identity->refresh());
        self::assertSame(2, $identity->refresh()->native_security_generation);

        DB::table('identities')->where('id', $identity->id)->update(['native_security_generation' => 1]);

        $this->postEvidence($request)->assertOk()->assertExactJson([
            'version' => 1,
            'operation' => NativeEvidenceContract::FRESH_ACCOUNT,
            'result' => 'unavailable',
        ]);
    }

    public function test_signing_trust_has_key_revision_revocation_and_set_wide_source_ordering(): void
    {
        $registry = app(NativeSigningTrustRegistry::class);
        $keyOne = str_repeat("\x01", 32);
        $keyTwo = str_repeat("\x02", 32);

        $versionOne = $registry->publishTrustedKey(
            NativeEvidenceContract::FRESH_ISSUER,
            NativeEvidenceContract::FRESH_PROFILE,
            'fresh_admission',
            'key-1',
            $keyOne,
        );
        self::assertSame(1, $this->integerValue($versionOne->key_revision ?? null));
        self::assertSame(1, $this->integerValue(DB::table('native_game_signing_trust_profiles')->value('issuer_revision')));

        $first = $this->postEvidence($this->freshTrustRequest('key-1'))->assertOk();
        self::assertSame('1', $first->json('source_revision'));
        self::assertTrue($first->json('trusted'));
        self::assertSame(NativeEvidenceContract::encodePublicKey($keyOne), $first->json('public_key'));

        $registry->publishTrustedKey(
            NativeEvidenceContract::FRESH_ISSUER,
            NativeEvidenceContract::FRESH_PROFILE,
            'fresh_admission',
            'key-2',
            $keyTwo,
        );
        $second = $this->postEvidence($this->freshTrustRequest('key-2'))->assertOk();
        self::assertSame('2', $second->json('source_revision'));
        self::assertSame(2, $this->integerValue(DB::table('native_game_signing_trust_profiles')->value('issuer_revision')));

        $revoked = $registry->revokeKey(
            NativeEvidenceContract::FRESH_ISSUER,
            NativeEvidenceContract::FRESH_PROFILE,
            'fresh_admission',
            'key-1',
        );
        self::assertSame(2, $this->integerValue($revoked->key_revision ?? null));
        self::assertFalse((bool) $revoked->trusted);
        self::assertSame(3, $this->integerValue(DB::table('native_game_signing_trust_profiles')->value('issuer_revision')));

        $third = $this->postEvidence($this->freshTrustRequest('key-1'))->assertOk();
        self::assertSame('3', $third->json('source_revision'));
        self::assertFalse($third->json('trusted'));

        $registry->revokeProfile(
            NativeEvidenceContract::FRESH_ISSUER,
            NativeEvidenceContract::FRESH_PROFILE,
            'fresh_admission',
        );
        $fourth = $this->postEvidence($this->freshTrustRequest('key-2'))->assertOk();
        self::assertSame('4', $fourth->json('source_revision'));
        self::assertFalse($fourth->json('trusted'));
        self::assertSame(4, $this->integerValue(DB::table('native_game_signing_trust_profiles')
            ->where('issuer', NativeEvidenceContract::FRESH_ISSUER)
            ->value('issuer_revision')));

        $recoveryKey = str_repeat("\x03", 32);
        $registry->publishTrustedKey(
            NativeEvidenceContract::RECOVERY_ISSUER,
            NativeEvidenceContract::RECOVERY_PROFILE,
            NativeEvidenceContract::RECOVERY_KEY_PURPOSE,
            'recovery-key-1',
            $recoveryKey,
        );
        $recovery = $this->postEvidence([
            'version' => 2,
            'operation' => NativeEvidenceContract::RECOVERY_TRUST,
            'issuer' => NativeEvidenceContract::RECOVERY_ISSUER,
            'profile' => NativeEvidenceContract::RECOVERY_PROFILE,
            'key_purpose' => NativeEvidenceContract::RECOVERY_KEY_PURPOSE,
            'key_id' => 'recovery-key-1',
        ])->assertOk();
        self::assertSame('1', $recovery->json('source_revision'));
        self::assertTrue($recovery->json('trusted'));
        self::assertSame(NativeEvidenceContract::encodePublicKey($recoveryKey), $recovery->json('public_key'));
    }

    public function test_signing_trust_witness_detects_state_rollback_before_a_new_observation(): void
    {
        $registry = app(NativeSigningTrustRegistry::class);
        $registry->publishTrustedKey(
            NativeEvidenceContract::FRESH_ISSUER,
            NativeEvidenceContract::FRESH_PROFILE,
            'fresh_admission',
            'key-1',
            str_repeat("\x01", 32),
        );
        $registry->publishTrustedKey(
            NativeEvidenceContract::FRESH_ISSUER,
            NativeEvidenceContract::FRESH_PROFILE,
            'fresh_admission',
            'key-2',
            str_repeat("\x02", 32),
        );

        $profileId = DB::table('native_game_signing_trust_profiles')->value('id');
        self::assertNotNull($profileId);
        DB::table('native_game_signing_trust_key_versions')
            ->where('profile_id', $profileId)
            ->where('key_id', 'key-2')
            ->delete();
        DB::table('native_game_signing_trust_profiles')
            ->where('id', $profileId)
            ->update(['issuer_revision' => 1]);

        $this->postEvidence($this->freshTrustRequest('key-1'))->assertOk()->assertExactJson([
            'version' => 1,
            'operation' => NativeEvidenceContract::FRESH_TRUST,
            'result' => 'unavailable',
        ]);
    }

    public function test_retained_source_witness_detects_observation_history_rollback_and_fails_closed(): void
    {
        $identity = $this->identity('source-rollback@example.test');
        $request = $this->freshAccountRequest($identity->account_id);

        $this->postEvidence($request)->assertOk()->assertJsonPath('source_revision', '1');
        $this->postEvidence($request)->assertOk()->assertJsonPath('source_revision', '2');

        DB::table('native_game_evidence_observations')
            ->where('namespace_hash', NativeEvidenceNamespace::accountSource($identity->account_id))
            ->where('source_revision', 2)
            ->delete();

        $this->postEvidence($request)->assertOk()->assertExactJson([
            'version' => 1,
            'operation' => NativeEvidenceContract::FRESH_ACCOUNT,
            'result' => 'unavailable',
        ]);
    }

    public function test_unknown_subjects_do_not_allocate_durable_history_and_transport_failures_are_closed(): void
    {
        $unknown = CanonicalAccountId::generate();
        self::assertSame([], glob($this->witnessDirectory.'/*.floor') ?: []);

        $this->postEvidence($this->freshAccountRequest($unknown))->assertOk()->assertExactJson([
            'version' => 1,
            'operation' => NativeEvidenceContract::FRESH_ACCOUNT,
            'result' => 'not_found',
        ]);
        self::assertSame([], glob($this->witnessDirectory.'/*.floor') ?: []);

        $this->withServerVariables([
            'SSL_CLIENT_VERIFY' => '',
            'SSL_PROTOCOL' => '',
            'SSL_CLIENT_S_DN' => '',
        ])->postJson('/internal/v1/game-auth/native-evidence', $this->freshAccountRequest($unknown))
            ->assertUnauthorized()
            ->assertContent('');

        $raw = '{"version":1,"operation":"ReadAccountSecurityV1","operation":"ReadAccountSecurityV1","account_id":"'.$unknown.'","purpose":"platform_security","scope":"fresh_admission"}';
        $this->rawEvidence($raw)->assertStatus(400)->assertContent('');
    }

    public function test_missing_source_witness_and_invalid_private_boundary_inputs_fail_closed(): void
    {
        $identity = $this->identity('boundary@example.test');
        $request = $this->freshAccountRequest($identity->account_id);

        $this->postEvidence($request)->assertOk()->assertJsonPath('source_revision', '1');
        $sourceFloor = $this->witnessDirectory.'/'.NativeEvidenceNamespace::accountSource($identity->account_id).'.floor';
        self::assertFileExists($sourceFloor);
        unlink($sourceFloor);

        $this->postEvidence($request)->assertOk()->assertExactJson([
            'version' => 1,
            'operation' => NativeEvidenceContract::FRESH_ACCOUNT,
            'result' => 'unavailable',
        ]);

        $wrongPeer = $this->peerServer();
        $wrongPeer['SSL_CLIENT_S_DN'] = 'CN=wrong-peer';
        $this->withServerVariables($wrongPeer)
            ->postJson('/internal/v1/game-auth/native-evidence', $request)
            ->assertUnauthorized()
            ->assertContent('');

        $this->rawEvidence(str_repeat(' ', NativeEvidenceContract::MAX_REQUEST_BYTES + 1))
            ->assertStatus(413)
            ->assertContent('');

        $extra = $request;
        $extra['unexpected'] = 'value';
        $this->postEvidence($extra)->assertStatus(400)->assertContent('');

        $wrongPurpose = $request;
        $wrongPurpose['purpose'] = 'wrong_security_purpose';
        $this->postEvidence($wrongPurpose)->assertOk()->assertExactJson([
            'version' => 1,
            'operation' => NativeEvidenceContract::FRESH_ACCOUNT,
            'result' => 'unsupported',
        ]);
    }

    public function test_protected_game_golden_requests_are_exactly_compatible_for_all_four_operations(): void
    {
        // Controlled counterpart fixtures mirror Oteryn-Game protected #470:
        // apps/game-server/tests/admission_evidence_wire.rs@ae22132fd9ddb6c83f5bf386fdc267cb670d16aa.
        config([
            'game-auth.native_evidence.fresh_account_purpose' => 'fixture_security',
            'game-auth.native_evidence.fresh_account_scope' => 'fixture_fresh',
            'game-auth.native_evidence.fresh_key_purpose' => 'fixture_key',
        ]);

        $accountId = '01890f4e-7c00-7000-8000-000000000001';
        $identity = $this->identity('game-golden@example.test');
        DB::table('identities')->where('id', $identity->id)->update([
            'account_id' => $accountId,
            'disabled_at' => now(),
        ]);

        $freshAccount = $this->rawEvidence(
            '{"version":1,"operation":"ReadAccountSecurityV1","account_id":"01890f4e-7c00-7000-8000-000000000001","purpose":"fixture_security","scope":"fixture_fresh"}',
        )->assertOk();
        $freshAccount->assertExactJson([
            'version' => 1,
            'operation' => NativeEvidenceContract::FRESH_ACCOUNT,
            'result' => 'observed',
            'source_authority' => 'platform',
            'source_revision' => '1',
            'decision_identity' => '1',
            'source_observed_at' => $freshAccount->json('source_observed_at'),
            'clock_uncertainty_seconds' => '0',
            'account_id' => $accountId,
            'purpose' => 'fixture_security',
            'scope' => 'fixture_fresh',
            'allowed' => false,
            'minimum_valid_generation' => '1',
        ]);

        $registry = app(NativeSigningTrustRegistry::class);
        $zeroKey = str_repeat("\0", 32);
        $registry->publishTrustedKey(
            NativeEvidenceContract::FRESH_ISSUER,
            NativeEvidenceContract::FRESH_PROFILE,
            'fixture_key',
            'key-1',
            $zeroKey,
        );
        $freshTrust = $this->rawEvidence(
            '{"version":1,"operation":"ReadFreshSigningTrustV1","issuer":"urn:oteryn:platform:game-admission","profile":"oteryn-pre-admission-v1","key_purpose":"fixture_key","key_id":"key-1"}',
        )->assertOk();
        self::assertSame('1', $freshTrust->json('source_revision'));
        self::assertSame('AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA', $freshTrust->json('public_key'));

        $recoveryAccount = $this->rawEvidence(
            '{"version":2,"operation":"ReadRecoveryAccountSecurityV2","account_id":"01890f4e-7c00-7000-8000-000000000001","purpose":"platform_security","scope":"existing_actor_recovery"}',
        )->assertOk();
        self::assertSame('2', $recoveryAccount->json('source_revision'));
        self::assertSame('1', $recoveryAccount->json('minimum_valid_generation'));

        $registry->publishTrustedKey(
            NativeEvidenceContract::RECOVERY_ISSUER,
            NativeEvidenceContract::RECOVERY_PROFILE,
            NativeEvidenceContract::RECOVERY_KEY_PURPOSE,
            'key-1',
            $zeroKey,
        );
        $recoveryTrust = $this->rawEvidence(
            '{"version":2,"operation":"ReadRecoverySigningTrustV2","issuer":"urn:oteryn:platform:game-recovery","profile":"oteryn-reauth-recovery-v1","key_purpose":"existing_actor_recovery","key_id":"key-1"}',
        )->assertOk();
        self::assertSame('1', $recoveryTrust->json('source_revision'));
        self::assertSame('AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA', $recoveryTrust->json('public_key'));
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

    /** @return array<string, string> */
    private function peerServer(): array
    {
        return [
            'CONTENT_TYPE' => 'application/json',
            'SSL_CLIENT_VERIFY' => 'SUCCESS',
            'SSL_PROTOCOL' => 'TLSv1.3',
            'SSL_CLIENT_S_DN' => self::PEER_SUBJECT,
        ];
    }

    private function withPeer(): self
    {
        return $this->withServerVariables($this->peerServer());
    }

    /**
     * @param  array<string, int|string>  $payload
     * @return TestResponse<Response>
     */
    private function postEvidence(array $payload): TestResponse
    {
        return $this->withPeer()->postJson('/internal/v1/game-auth/native-evidence', $payload);
    }

    /**
     * @return TestResponse<Response>
     */
    private function rawEvidence(string $raw): TestResponse
    {
        return $this->call(
            'POST',
            '/internal/v1/game-auth/native-evidence',
            [],
            [],
            [],
            $this->peerServer(),
            $raw,
        );
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

    private function integerValue(mixed $value): int
    {
        if (is_int($value) && $value >= 0) {
            return $value;
        }
        if (is_string($value) && preg_match('/^(0|[1-9][0-9]{0,18})$/', $value) === 1) {
            $parsed = filter_var($value, FILTER_VALIDATE_INT, ['options' => ['min_range' => 0]]);
            if (is_int($parsed)) {
                return $parsed;
            }
        }

        self::fail('Expected a non-negative bounded integer value.');
    }
}
