<?php

namespace Tests\Feature\GameAuth;

use App\Console\Commands\IssueCharacterBootstrapIntent;
use App\GameAuth\CharacterBootstrapIntent\CharacterBootstrapIntentConflict;
use App\GameAuth\CharacterBootstrapIntent\CharacterBootstrapIntentContract;
use App\GameAuth\CharacterBootstrapIntent\CharacterBootstrapIntentIssuer;
use App\GameAuth\NativeEvidence\NativeEvidenceContract;
use App\Identity\Models\Identity;
use Illuminate\Foundation\Testing\RefreshDatabase;
use Illuminate\Support\Carbon;
use Illuminate\Support\Facades\Artisan;
use Illuminate\Support\Facades\DB;
use Illuminate\Support\Facades\Hash;
use Illuminate\Support\Str;
use Illuminate\Testing\TestResponse;
use Symfony\Component\HttpFoundation\Response;
use Tests\TestCase;

final class CharacterBootstrapIntentProducerTest extends TestCase
{
    use RefreshDatabase;

    private const PEER = 'CN=oteryn-game-character-authority';

    protected function setUp(): void
    {
        parent::setUp();
        Carbon::setTestNow('2026-09-23T12:00:00Z');
        config([
            'hashing.driver' => 'bcrypt',
            'game-auth.character_bootstrap_intent.ttl_seconds' => '60',
            'game-auth.character_bootstrap_intent.mtls_client_identity' => self::PEER,
        ]);
    }

    protected function tearDown(): void
    {
        Carbon::setTestNow();
        parent::tearDown();
    }

    public function test_command_has_no_account_id_authority_and_returns_persisted_platform_account_id(): void
    {
        $identity = $this->identity('operator-subject@example.test');
        $options = $this->commandOptions($identity);

        $definition = (new IssueCharacterBootstrapIntent)->getDefinition();
        self::assertFalse($definition->hasOption('account-id'));
        self::assertSame(0, DB::table('character_bootstrap_intents')->count());

        $exit = Artisan::call('game-auth:character-bootstrap-intent:issue', $options);
        self::assertSame(0, $exit, Artisan::output());
        $stored = DB::table('character_bootstrap_intents')->value('intent_json');
        self::assertIsString($stored);
        $payload = json_decode($stored, true, 3, JSON_THROW_ON_ERROR);
        self::assertIsArray($payload);
        self::assertSame($identity->account_id, $payload['account_id']);
        self::assertNotSame((string) $identity->id, $payload['account_id']);
        self::assertStringNotContainsString('Correct-Horse', Artisan::output());
        self::assertStringNotContainsString(self::PEER, Artisan::output());
    }

    public function test_exact_retry_rereads_byte_equivalent_intent_and_changed_binding_conflicts(): void
    {
        $identity = $this->identity('retry@example.test');
        $issuer = app(CharacterBootstrapIntentIssuer::class);
        $options = $this->binding();
        $operationId = strtolower((string) Str::uuid());

        $first = $issuer->issue($identity->id, $operationId, $options);
        Carbon::setTestNow('2026-09-23T12:00:15Z');
        app()->forgetInstance(CharacterBootstrapIntentIssuer::class);
        $second = app(CharacterBootstrapIntentIssuer::class)->issue($identity->id, $operationId, $options);

        self::assertSame($first, $second);
        self::assertSame(1, DB::table('character_bootstrap_intents')->count());
        self::assertSame('1', $first['source_revision']);
        self::assertSame(CharacterBootstrapIntentContract::VARIANT, $first['variant']);
        self::assertSame(array_keys($first), [
            'contract_version', 'variant', 'issuer_authority', 'issuer_decision_id', 'source_revision',
            'operation_id', 'operation', 'account_id', 'target_world_id', 'interpretation_context',
            'issued_at_source', 'expires_at_source', 'audience',
        ]);

        $otherIdentity = $this->identity('changed-account@example.test');
        foreach ([
            [$identity->id, array_merge($options, ['target_world_id' => strtolower((string) Str::uuid())])],
            [$identity->id, array_merge($options, ['content_revision' => 'content-92'])],
            [$otherIdentity->id, $options],
        ] as [$candidateIdentityId, $candidateBinding]) {
            try {
                $issuer->issue($candidateIdentityId, $operationId, $candidateBinding);
                self::fail('Changed immutable operation binding must conflict.');
            } catch (CharacterBootstrapIntentConflict) {
                self::assertSame(1, DB::table('character_bootstrap_intents')->count());
            }
        }
    }

    public function test_private_read_is_strict_purpose_separated_non_cacheable_and_does_not_mint(): void
    {
        $identity = $this->identity('read@example.test');
        $operationId = strtolower((string) Str::uuid());
        $issued = app(CharacterBootstrapIntentIssuer::class)->issue($identity->id, $operationId, $this->binding());

        $missingPeer = $this->postJson('/internal/v1/game-auth/character-bootstrap-intents/read', [
            'contract_version' => 1,
            'operation_id' => $operationId,
        ])->assertUnauthorized()->assertContent('');
        $this->assertPrivateNoStore($missingPeer);

        $wrong = $this->peer();
        $wrong['SSL_CLIENT_S_DN'] = 'CN=native-evidence-only';
        $this->withServerVariables($wrong)->postJson('/internal/v1/game-auth/character-bootstrap-intents/read', [
            'contract_version' => 1,
            'operation_id' => $operationId,
        ])->assertUnauthorized()->assertContent('');

        $response = $this->read($operationId)->assertOk()->assertExactJson($issued);
        $this->assertPrivateNoStore($response);
        self::assertSame(1, DB::table('character_bootstrap_intents')->count());

        foreach ([
            '{}',
            '{"contract_version":1,"operation_id":"'.$operationId.'","operation_id":"'.$operationId.'"}',
            '{"contract_version":1,"operation_id":"'.$operationId.'","extra":"x"}',
            '{"contract_version":1,"operation_id":{"nested":"'.$operationId.'"}}',
            '{"contract_version":2,"operation_id":"'.$operationId.'"}',
            str_repeat(' ', CharacterBootstrapIntentContract::MAX_REQUEST_BYTES + 1),
        ] as $raw) {
            $this->rawRead($raw)->assertStatus(400)->assertContent('');
        }

        $this->read(strtolower((string) Str::uuid()))->assertNotFound()->assertContent('');
        self::assertSame(1, DB::table('character_bootstrap_intents')->count());
    }

    public function test_expired_or_rollback_conflicted_intent_is_not_served_as_current(): void
    {
        $identity = $this->identity('expiry@example.test');
        $operationId = strtolower((string) Str::uuid());
        app(CharacterBootstrapIntentIssuer::class)->issue($identity->id, $operationId, $this->binding());

        Carbon::setTestNow('2026-09-23T12:01:00Z');
        $this->read($operationId)->assertNotFound()->assertContent('');

        Carbon::setTestNow('2026-09-23T12:00:30Z');
        DB::table('character_bootstrap_intent_authority')->where('id', 1)->update([
            'last_issuer_decision_id' => strtolower((string) Str::uuid()),
        ]);
        $this->read($operationId)->assertStatus(503)->assertContent('');
    }

    public function test_native_evidence_contract_remains_exactly_four_operations(): void
    {
        self::assertSame([
            'ReadAccountSecurityV1',
            'ReadFreshSigningTrustV1',
            'ReadRecoveryAccountSecurityV2',
            'ReadRecoverySigningTrustV2',
        ], [
            NativeEvidenceContract::FRESH_ACCOUNT,
            NativeEvidenceContract::FRESH_TRUST,
            NativeEvidenceContract::RECOVERY_ACCOUNT,
            NativeEvidenceContract::RECOVERY_TRUST,
        ]);
    }

    private function identity(string $email): Identity
    {
        return Identity::query()->create([
            'email' => $email,
            'password' => Hash::make('Correct-Horse-9!Battery'),
        ]);
    }

    /** @return array{target_world_id:string,profile_revision:string,ruleset_revision:string,content_revision:string,starter_template_revision:string} */
    private function binding(): array
    {
        return [
            'target_world_id' => '01890f4e-7c00-7000-8000-000000000002',
            'profile_revision' => 'profile-17',
            'ruleset_revision' => 'ruleset-22',
            'content_revision' => 'content-91',
            'starter_template_revision' => 'starter-4',
        ];
    }

    /** @return array<string, int|string> */
    private function commandOptions(Identity $identity): array
    {
        return [
            '--identity-id' => (string) $identity->id,
            '--operation-id' => '01890f4e-7c00-7000-8000-000000000001',
            '--target-world-id' => '01890f4e-7c00-7000-8000-000000000002',
            '--profile-revision' => 'profile-17',
            '--ruleset-revision' => 'ruleset-22',
            '--content-revision' => 'content-91',
            '--starter-template-revision' => 'starter-4',
        ];
    }

    /** @return array<string, string> */
    private function peer(): array
    {
        return [
            'CONTENT_TYPE' => 'application/json',
            'SSL_CLIENT_VERIFY' => 'SUCCESS',
            'SSL_PROTOCOL' => 'TLSv1.3',
            'SSL_CLIENT_S_DN' => self::PEER,
        ];
    }

    /** @return TestResponse<Response> */
    private function read(string $operationId): TestResponse
    {
        return $this->withServerVariables($this->peer())->postJson('/internal/v1/game-auth/character-bootstrap-intents/read', [
            'contract_version' => 1,
            'operation_id' => $operationId,
        ]);
    }

    /** @return TestResponse<Response> */
    private function rawRead(string $raw): TestResponse
    {
        return $this->call('POST', '/internal/v1/game-auth/character-bootstrap-intents/read', [], [], [], $this->peer(), $raw);
    }

    /** @param TestResponse<Response> $response */
    private function assertPrivateNoStore(TestResponse $response): void
    {
        $value = (string) $response->headers->get('Cache-Control');
        self::assertStringContainsString('no-store', $value);
        self::assertStringContainsString('private', $value);
    }
}
