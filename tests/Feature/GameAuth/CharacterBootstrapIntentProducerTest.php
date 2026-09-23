<?php

namespace Tests\Feature\GameAuth;

use App\GameAuth\CharacterBootstrapIntent\CharacterBootstrapIntentConflict;
use App\GameAuth\CharacterBootstrapIntent\CharacterBootstrapIntentService;
use App\Identity\Models\Identity;
use Illuminate\Foundation\Testing\RefreshDatabase;
use Illuminate\Support\Facades\Artisan;
use Illuminate\Support\Facades\DB;
use Illuminate\Support\Str;
use Illuminate\Testing\TestResponse;
use Symfony\Component\Console\Command\Command;
use Symfony\Component\HttpFoundation\Response;
use Tests\TestCase;

final class CharacterBootstrapIntentProducerTest extends TestCase
{
    use RefreshDatabase;

    private const PEER = 'CN=oteryn-game-character-bootstrap';

    private string $highWater;

    protected function setUp(): void
    {
        parent::setUp();
        $this->highWater = storage_path('framework/testing/character-intent-'.bin2hex(random_bytes(6)));
        mkdir($this->highWater, 0700, true);
        config([
            'game-auth.character_bootstrap_intent.activated' => true,
            'game-auth.character_bootstrap_intent.issuer_authority' => 'OTERYN_PLATFORM_CHARACTER_AUTHORITY',
            'game-auth.character_bootstrap_intent.mtls_client_identity' => self::PEER,
            'game-auth.character_bootstrap_intent.ttl_seconds' => 300,
            'game-auth.character_bootstrap_intent.high_water_directory' => $this->highWater,
        ]);
    }

    protected function tearDown(): void
    {
        foreach (glob($this->highWater.'/*') ?: [] as $path) {
            @unlink($path);
        }
        @rmdir($this->highWater);
        parent::tearDown();
    }

    public function test_operator_issuance_resolves_persisted_account_and_exact_retry(): void
    {
        $identity = $this->identity();
        $operation = strtolower((string) Str::uuid());
        $first = $this->issue($identity, $operation);
        $second = $this->issue($identity, $operation);

        self::assertSame($first, $second);
        self::assertSame($identity->account_id, $first['account_id']);
        self::assertSame('1', $first['source_revision']);
        self::assertSame(1, DB::table('character_bootstrap_intents')->count());
        self::assertArrayNotHasKey('password', $first);
        self::assertArrayNotHasKey('mtls_client_identity', $first);
    }

    public function test_operation_reuse_with_changed_binding_fails_closed_without_advancing(): void
    {
        $identity = $this->identity();
        $operation = strtolower((string) Str::uuid());
        $this->issue($identity, $operation);

        $this->expectException(CharacterBootstrapIntentConflict::class);
        try {
            app(CharacterBootstrapIntentService::class)->issue($identity->id, $operation, 'world-2', $this->context());
        } finally {
            self::assertSame(1, DB::table('character_bootstrap_intents')->count());
            self::assertSame(1, DB::table('character_bootstrap_intent_authority_states')->value('source_revision'));
        }
    }

    public function test_command_accepts_identity_lookup_not_caller_account_authority(): void
    {
        $identity = $this->identity();
        self::assertSame(Command::SUCCESS, Artisan::call('game-auth:character-bootstrap-intent:issue', [
            'identity-id' => (string) $identity->id,
            '--operation-id' => strtolower((string) Str::uuid()),
            '--target-world-id' => 'world-1',
            '--profile-revision' => '1',
            '--ruleset-revision' => '2',
            '--content-revision' => '3',
            '--starter-template-revision' => '4',
        ]));
        $payload = json_decode(trim(Artisan::output()), true, 4, JSON_THROW_ON_ERROR);
        self::assertIsArray($payload);
        self::assertSame($identity->account_id, $payload['account_id'] ?? null);
        self::assertStringNotContainsString($identity->password, Artisan::output());
    }

    public function test_private_endpoint_requires_exact_tls_peer_and_strict_wire_shape(): void
    {
        $identity = $this->identity();
        $payload = $this->issue($identity, strtolower((string) Str::uuid()));
        $operationId = $this->payloadString($payload, 'operation_id');
        $request = json_encode([
            'contract_version' => 1,
            'operation_id' => $operationId,
            'audience' => 'OTERYN_GAME_CHARACTER_AUTHORITY',
        ], JSON_THROW_ON_ERROR);

        $this->call('POST', '/internal/v1/game-auth/character-bootstrap-intents/reconcile', [], [], [], [
            'CONTENT_TYPE' => 'application/json',
        ], $request)->assertUnauthorized()->assertHeaderContains('Cache-Control', 'no-store');

        $response = $this->postRaw($request)->assertOk()->assertExactJson($payload);
        $response->assertHeaderContains('Cache-Control', 'no-store');

        $duplicate = '{"contract_version":1,"operation_id":"'.$operationId.'","operation_id":"'.$operationId.'","audience":"OTERYN_GAME_CHARACTER_AUTHORITY"}';
        $this->postRaw($duplicate)->assertBadRequest();
        $this->postRaw(json_encode(['contract_version' => 1, 'operation_id' => $operationId, 'audience' => 'OTERYN_GAME_CHARACTER_AUTHORITY', 'unknown' => 'x'], JSON_THROW_ON_ERROR))->assertBadRequest();
    }

    public function test_expired_future_and_rollback_ambiguous_intents_are_not_served(): void
    {
        $identity = $this->identity();
        $payload = $this->issue($identity, strtolower((string) Str::uuid()));
        $request = json_encode(['contract_version' => 1, 'operation_id' => $payload['operation_id'], 'audience' => 'OTERYN_GAME_CHARACTER_AUTHORITY'], JSON_THROW_ON_ERROR);
        DB::table('character_bootstrap_intents')->where('operation_id', $payload['operation_id'])->update(['expires_at_source' => now()->timestamp]);
        $this->postRaw($request)->assertNotFound();
        DB::table('character_bootstrap_intent_authority_states')->where('id', 1)->update(['source_revision' => 0]);
        $this->postRaw($request)->assertNotFound();
    }

    /** @return array<string, int|string|array<string, string>> */
    private function issue(Identity $identity, string $operation): array
    {
        return app(CharacterBootstrapIntentService::class)->issue($identity->id, $operation, 'world-1', $this->context());
    }

    /** @return array{profile_revision: string, ruleset_revision: string, content_revision: string, starter_template_revision: string} */
    private function context(): array
    {
        return ['profile_revision' => '1', 'ruleset_revision' => '2', 'content_revision' => '3', 'starter_template_revision' => '4'];
    }

    private function identity(): Identity
    {
        return Identity::query()->create(['email' => Str::random(10).'@example.test', 'password' => '$2y$12$abcdefghijklmnopqrstuu5T9ycw9pB1NK/JDhYVKP.Mx7KVVIYV2']);
    }

    /** @param array<string, int|string|array<string, string>> $payload */
    private function payloadString(array $payload, string $field): string
    {
        $value = $payload[$field] ?? null;
        self::assertIsString($value);

        return $value;
    }

    /** @return TestResponse<Response> */
    private function postRaw(string $body): TestResponse
    {
        return $this->call('POST', '/internal/v1/game-auth/character-bootstrap-intents/reconcile', [], [], [], [
            'CONTENT_TYPE' => 'application/json',
            'SSL_CLIENT_VERIFY' => 'SUCCESS',
            'SSL_PROTOCOL' => 'TLSv1.3',
            'SSL_CLIENT_S_DN' => self::PEER,
        ], $body);
    }
}
