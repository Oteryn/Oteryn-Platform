<?php

namespace Tests\Feature\GameAuth\DeviceSessions;

use App\GameAuth\DeviceSessions\DeviceSessionFamily;
use App\GameAuth\DeviceSessions\DeviceSessionSecrets;
use App\GameAuth\DeviceSessions\NativeRememberedDeviceSessions;
use App\GameAuth\NativeAccountCharacters\NativeAccountCharactersIngestion;
use App\GameAuth\NativeAccountCharacters\NativeAccountCharactersSettings;
use App\GameAuth\NativeAccountCharacters\NativeAccountCharactersSnapshot;
use App\GameAuth\NativeAccountCharacters\NativeAccountCharactersWatermark;
use App\GameAuth\NativeLogin\NativeGameLoginTickets;
use App\GameAuth\OAuth\NativeOAuthClientManager;
use App\Identity\Models\Identity;
use Illuminate\Foundation\Testing\DatabaseMigrations;
use Illuminate\Support\Facades\DB;
use Illuminate\Support\Facades\Hash;
use Illuminate\Support\Facades\Log;
use Illuminate\Support\Facades\Schema;
use Illuminate\Testing\TestResponse;
use Laravel\Passport\Client;
use Laravel\Passport\Token;
use LogicException;
use RuntimeException;
use Symfony\Component\HttpFoundation\Response;
use Tests\Feature\GameAuth\OAuth\Concerns\ConfiguresEphemeralPassportKeys;
use Tests\Feature\GameAuth\OAuth\Concerns\CreatesNativeOAuthBootstrapToken;
use Tests\TestCase;

final class NativeDeviceSessionHttpTest extends TestCase
{
    use ConfiguresEphemeralPassportKeys;
    use CreatesNativeOAuthBootstrapToken;
    use DatabaseMigrations {
        runDatabaseMigrations as private runEphemeralDatabaseMigrations;
    }

    private const BASE = 'https://oteryn.example.test/api/v1/game-auth/device-sessions';

    private const PUBLISHER = 'CN=character-authority-projection';

    private const AUTHORITY = 'oteryn:character-authority:primary';

    private const CHARACTER = '01934f10-7c04-7001-805b-3b1122334401';

    private const OTHER_CHARACTER = '01934f10-7c04-7001-805b-3b1122334402';

    private const WORLD = '01934f10-7c02-7001-805b-3b1122334401';

    public function runDatabaseMigrations(): void
    {
        $this->assertIsolatedDatabase();
        $this->runEphemeralDatabaseMigrations();
    }

    protected function setUp(): void
    {
        parent::setUp();
        $this->configureEphemeralPassportKeys();
        config([
            'app.url' => 'https://oteryn.example.test',
            'game-auth.device_sessions.enabled' => true,
            'game-auth.device_sessions.allow_insecure_loopback' => false,
            'game-auth.device_sessions.requests_per_minute' => 60,
            'game-auth.native_admission.enabled' => true,
            'game-auth.native_account_characters.enabled' => true,
            'game-auth.native_account_characters.identities' => [self::PUBLISHER],
            'game-auth.native_account_characters.source_authority' => self::AUTHORITY,
            'game-auth.native_account_characters.freshness_seconds' => 30,
            'game-auth.native_account_characters.clock_uncertainty_seconds' => 1,
            'game-auth.native_account_characters.requests_per_minute' => 120,
        ]);
    }

    protected function tearDown(): void
    {
        $this->app['env'] = 'testing';
        $this->assertIsolatedDatabase();
        // These are only this test's in-memory fixtures. Production rollback safeguards
        // remain active and must not be disabled to make integration tests pass.
        foreach (['native_device_session_credentials', 'native_device_session_families', 'native_account_character_rows',
            'native_account_character_snapshots', 'native_admission_attempts'] as $table) {
            if (Schema::hasTable($table)) {
                DB::table($table)->delete();
            }
        }
        if (Schema::hasTable('game_login_tickets')) {
            DB::table('game_login_tickets')->whereNull('canary_account_id')->delete();
        }
        parent::tearDown();
    }

    public function test_pkce_enrollment_rotated_owner_read_and_native_ticket_result_without_new_oauth_tokens(): void
    {
        [$identity, $client, $bearer] = $this->bootstrap();
        $other = Identity::query()->create(['email' => 'other@example.invalid', 'password' => Hash::make('Correct-Horse-9!Battery')]);
        $this->publish($identity, self::CHARACTER, 'Aldric');
        $this->publish($other, self::OTHER_CHARACTER, 'Other Character');

        $enrollment = $this->postJson(self::BASE, ['protocol_version' => 1, 'remember_device' => true], ['Authorization' => 'Bearer '.$bearer]);
        $enrollment->assertOk()->assertJsonPath('protocol_version', 1);
        $this->assertNoStore($enrollment);
        $credential = $this->credential($enrollment);
        self::assertFalse(Token::query()->where('user_id', $identity->id)->sole()->revoked);
        $familyId = $enrollment->json('family_id');
        self::assertIsString($familyId);

        $characters = $this->device('/native-characters', $credential, $client);
        $characters->assertOk()->assertJsonPath('protocol_version', 1)->assertJsonPath('family_id', $familyId)
            ->assertJsonCount(1, 'characters')->assertJsonPath('characters.0.character_id', self::CHARACTER)
            ->assertJsonPath('characters.0.world_id', self::WORLD)->assertJsonPath('characters.0.name', 'Aldric')
            ->assertJsonPath('characters.0.availability', 'AVAILABLE');
        $content = $characters->getContent();
        self::assertIsString($content);
        self::assertStringNotContainsString(self::OTHER_CHARACTER, $content);
        self::assertStringNotContainsString($other->account_id, $content);
        $this->assertNoStore($characters);
        $successor = $this->credential($characters);
        self::assertNotSame($credential, $successor);

        $ticket = $this->device('/tickets', $successor, $client);
        $ticket->assertOk()->assertJsonPath('protocol_version', 1);
        $this->assertNoStore($ticket);
        $stored = DB::table('game_login_tickets')->sole();
        self::assertSame(NativeGameLoginTickets::AUDIENCE, $stored->audience);
        self::assertSame($identity->account_id, $stored->account_id);
        self::assertNull($stored->canary_account_id);
        self::assertIsInt($ticket->json('expires_in'));
        self::assertGreaterThan(0, $ticket->json('expires_in'));
        self::assertLessThanOrEqual(60, $ticket->json('expires_in'));
        self::assertIsInt($ticket->json('expires_at'));
        self::assertSame(1, Token::query()->count());
        self::assertSame(1, DB::table('oauth_refresh_tokens')->count());
        $this->credential($ticket);
    }

    public function test_enrollment_preserves_initial_oauth_ticket_consumption_and_device_continuation(): void
    {
        [$identity, $client, $bearer] = $this->bootstrap();
        $this->publish($identity, self::CHARACTER, 'Aldric');
        $enrollment = $this->postJson(self::BASE, ['protocol_version' => 1, 'remember_device' => true], ['Authorization' => 'Bearer '.$bearer]);
        $enrollment->assertOk();
        $credential = $this->credential($enrollment);

        $this->postJson('/api/v1/game-auth/tickets', ['protocol_version' => 1], ['Authorization' => 'Bearer '.$bearer])->assertOk();
        self::assertSame(0, DB::table('oauth_access_tokens')->where('revoked', false)->count());
        self::assertSame(0, DB::table('oauth_refresh_tokens')->where('revoked', false)->count());
        $this->postJson('/api/v1/game-auth/tickets', ['protocol_version' => 1], ['Authorization' => 'Bearer '.$bearer])->assertUnauthorized();

        $characters = $this->device('/native-characters', $credential, $client);
        $characters->assertOk()->assertJsonPath('characters.0.character_id', self::CHARACTER);
        $this->device('/tickets', $this->credential($characters), $client)->assertOk();
        self::assertSame(0, DB::table('oauth_access_tokens')->where('revoked', false)->count());
        self::assertSame(0, DB::table('oauth_refresh_tokens')->where('revoked', false)->count());
        self::assertSame(2, DB::table('game_login_tickets')->count());
    }

    public function test_enrollment_rejects_joined_duplicate_and_whitespace_bearers_before_enrolling(): void
    {
        [, , $bearer] = $this->bootstrap();
        foreach (['Bearer '.$bearer.', Bearer '.$bearer, 'Bearer '.$bearer.' ', 'Bearer  '.$bearer, "Bearer\t".$bearer, 'Bearer opaque', 'bearer '.$bearer] as $header) {
            $response = $this->postJson(self::BASE, ['protocol_version' => 1, 'remember_device' => true], ['Authorization' => $header]);
            $response->assertUnauthorized()->assertExactJson(['error' => 'device_authorization_unavailable']);
            $this->assertNoStore($response);
        }
        self::assertSame(0, DB::table('native_device_session_families')->count());
        self::assertSame(1, DB::table('oauth_access_tokens')->where('revoked', false)->count());
    }

    public function test_replay_is_denied_and_revokes_latest_http_successor_without_issuing_another_ticket(): void
    {
        [, $client, $credential] = $this->enrolled();
        $ticket = $this->device('/tickets', $credential, $client);
        $ticket->assertOk();
        $successor = $this->credential($ticket);
        $replay = $this->device('/tickets', $credential, $client);
        $replay->assertUnauthorized()->assertExactJson(['error' => 'device_authorization_unavailable']);
        $this->assertNoStore($replay);
        self::assertSame('credential_replay', DeviceSessionFamily::query()->sole()->revocation_reason);
        $this->device('/tickets', $successor, $client)->assertUnauthorized();
        self::assertSame(1, DB::table('game_login_tickets')->count());
    }

    public function test_http_logout_is_idempotent_and_cannot_reauthorize_revoked_family(): void
    {
        [, $client, $credential] = $this->enrolled();
        $revoke = $this->device('/revoke', $credential, $client);
        $revoke->assertOk()->assertExactJson(['protocol_version' => 1, 'revoked' => true]);
        $this->assertNoStore($revoke);
        $this->device('/revoke', $credential, $client)->assertOk();
        $this->device('/tickets', $credential, $client)->assertUnauthorized();
        self::assertSame('client_revoked', DeviceSessionFamily::query()->sole()->revocation_reason);
        self::assertSame(0, DB::table('game_login_tickets')->count());
    }

    public function test_unknown_owner_authority_fields_and_wrong_client_are_refused_without_rotating(): void
    {
        [$identity, $client, $credential] = $this->enrolled();
        foreach (['account_id', 'identity_id', 'native_security_generation', 'purpose', 'device_credential'] as $field) {
            $this->postJson(self::BASE.'/tickets', ['protocol_version' => 1, 'client_id' => $client->getKey(), $field => $identity->account_id],
                ['Authorization' => 'OterynDevice '.$credential])->assertBadRequest();
        }
        $foreign = app(NativeOAuthClientManager::class)->ensure();
        $this->device('/tickets', $credential, $foreign)->assertUnauthorized();
        self::assertNull(DeviceSessionFamily::query()->sole()->revoked_at);
        self::assertSame(1, DeviceSessionFamily::query()->sole()->current_sequence);
        self::assertSame(0, DB::table('game_login_tickets')->count());
    }

    public function test_enrollment_requires_literal_boolean_opt_in_and_exact_json_without_duplicate_members(): void
    {
        [, , $bearer] = $this->bootstrap();
        foreach ([false, 1, 'true', null] as $consent) {
            $this->postJson(self::BASE, ['protocol_version' => 1, 'remember_device' => $consent], ['Authorization' => 'Bearer '.$bearer])->assertBadRequest();
        }
        $this->postJson(self::BASE, ['protocol_version' => '1', 'remember_device' => true], ['Authorization' => 'Bearer '.$bearer])->assertBadRequest();
        foreach (['{"protocol_version":1,"remember_device":false,"remember_device":true}',
            '{"protocol_version":1,"remember_device":true,"remember_\\u0064evice":true}',
            '{"protocol_version":1,"remember_device":true,"account_id":"ignored"}'] as $body) {
            $this->call('POST', self::BASE, [], [], [], ['CONTENT_TYPE' => 'application/json', 'HTTP_AUTHORIZATION' => 'Bearer '.$bearer], $body)->assertBadRequest();
        }
        self::assertSame(0, DeviceSessionFamily::query()->count());
    }

    public function test_device_secret_is_required_only_in_its_authorization_header_not_body_or_query(): void
    {
        [, $client, $credential] = $this->enrolled();
        $body = ['protocol_version' => 1, 'client_id' => $client->getKey()];
        $this->postJson(self::BASE.'/tickets', $body)->assertUnauthorized();
        $this->postJson(self::BASE.'/tickets', $body, ['Authorization' => 'Bearer '.$credential])->assertUnauthorized();
        $this->postJson(self::BASE.'/tickets?device_credential='.$credential, $body, ['Authorization' => 'OterynDevice '.$credential])->assertBadRequest();
        self::assertSame(0, DB::table('game_login_tickets')->count());
    }

    public function test_http_bounds_and_source_throttle_are_enforced_before_authorization(): void
    {
        $oversized = $this->postJson(self::BASE.'/tickets', ['padding' => str_repeat('x', 1025)]);
        $oversized->assertStatus(413);
        $this->assertNoStore($oversized);
        $this->call('POST', self::BASE.'/tickets', [], [], [], ['CONTENT_TYPE' => 'text/plain'], '{}')->assertBadRequest();
        config(['game-auth.device_sessions.requests_per_minute' => 2]);
        // A fresh source key makes the throttle boundary independent of earlier bounds cases.
        $server = ['REMOTE_ADDR' => '192.0.2.44', 'CONTENT_TYPE' => 'application/json'];
        $this->call('POST', self::BASE.'/tickets', [], [], [], $server, '{}')->assertUnauthorized();
        $this->call('POST', self::BASE.'/tickets', [], [], [], $server, '{}')->assertUnauthorized();
        $limited = $this->call('POST', self::BASE.'/tickets', [], [], [], $server, '{}');
        $limited->assertStatus(429)->assertExactJson(['error' => 'too_many_requests']);
        self::assertTrue($limited->headers->has('Retry-After'));
        $this->assertNoStore($limited);
    }

    public function test_transport_exception_requires_exact_explicit_loopback_origin_and_local_source(): void
    {
        $body = ['protocol_version' => 1, 'client_id' => '01934f10-7c04-7001-805b-3b1122334401'];
        $header = ['Authorization' => 'OterynDevice '.app(DeviceSessionSecrets::class)->generate()];
        $local = 'http://127.0.0.1:18584/api/v1/game-auth/device-sessions/tickets';
        config(['app.url' => 'http://127.0.0.1:18584']);
        $this->postJson($local, $body, $header)->assertStatus(503);
        config(['game-auth.device_sessions.allow_insecure_loopback' => true]);
        $this->postJson($local, $body, $header)->assertUnauthorized();
        $this->postJson('http://127.0.0.1:18585/api/v1/game-auth/device-sessions/tickets', $body, $header)->assertStatus(503);
        $this->postJson('http://localhost:18584/api/v1/game-auth/device-sessions/tickets', $body, $header)->assertStatus(503);
        $this->postJson('http://oteryn.example.test/api/v1/game-auth/device-sessions/tickets', $body, $header)->assertStatus(503);
        $this->call('POST', $local, [], [], [], ['REMOTE_ADDR' => '192.0.2.44', 'CONTENT_TYPE' => 'application/json', 'HTTP_AUTHORIZATION' => $header['Authorization']], json_encode($body, JSON_THROW_ON_ERROR))->assertStatus(503);
    }

    public function test_disabled_and_production_gates_refuse_even_well_formed_requests(): void
    {
        config(['game-auth.device_sessions.enabled' => false]);
        foreach (['', '/native-characters', '/tickets', '/revoke'] as $suffix) {
            $response = $this->postJson(self::BASE.$suffix, ['protocol_version' => 1]);
            $response->assertStatus(503)->assertExactJson(['error' => 'device_authorization_unavailable']);
            $this->assertNoStore($response);
        }
        config(['game-auth.device_sessions.enabled' => true]);
        $this->app['env'] = 'production';
        $this->postJson(self::BASE.'/tickets', ['protocol_version' => 1])->assertStatus(503);
        self::assertSame(0, DeviceSessionFamily::query()->count());
    }

    public function test_forwarded_loopback_address_cannot_enable_plain_http_for_remote_peer(): void
    {
        config(['app.url' => 'http://127.0.0.1:18584', 'game-auth.device_sessions.allow_insecure_loopback' => true]);
        $body = json_encode(['protocol_version' => 1, 'client_id' => '01934f10-7c04-7001-805b-3b1122334401'], JSON_THROW_ON_ERROR);
        $server = [
            'REMOTE_ADDR' => '192.0.2.44',
            'CONTENT_TYPE' => 'application/json',
            'HTTP_AUTHORIZATION' => 'OterynDevice '.app(DeviceSessionSecrets::class)->generate(),
            'HTTP_X_FORWARDED_FOR' => '127.0.0.1',
            'HTTP_X_FORWARDED_PROTO' => 'http',
        ];
        foreach ([[], ['192.0.2.44']] as $proxies) {
            config(['http.trusted_proxies' => $proxies]);
            $response = $this->call('POST', 'http://127.0.0.1:18584/api/v1/game-auth/device-sessions/tickets', [], [], [], $server, $body);
            $response->assertStatus(503)->assertExactJson(['error' => 'device_authorization_unavailable']);
            $this->assertNoStore($response);
        }

        // Raw loopback alone is insufficient if a trusted local proxy describes a remote client.
        config(['http.trusted_proxies' => ['127.0.0.1']]);
        $server['REMOTE_ADDR'] = '127.0.0.1';
        $server['HTTP_X_FORWARDED_FOR'] = '192.0.2.44';
        $this->call('POST', 'http://127.0.0.1:18584/api/v1/game-auth/device-sessions/tickets', [], [], [], $server, $body)
            ->assertStatus(503);
    }

    public function test_unavailable_projection_rolls_back_rotation_and_returns_generic_noncacheable_failure(): void
    {
        [$identity, $client, $credential] = $this->enrolled();
        $this->publish($identity, self::CHARACTER, 'Aldric');
        DB::table('native_account_character_snapshots')->where('account_id', $identity->account_id)->update(['invalid' => true]);
        $response = $this->device('/native-characters', $credential, $client);
        $response->assertStatus(503)->assertExactJson(['error' => 'device_authorization_unavailable']);
        $this->assertNoStore($response);
        self::assertSame(1, DeviceSessionFamily::query()->sole()->current_sequence);
        self::assertSame(1, DB::table('native_device_session_credentials')->count());
    }

    public function test_unexpected_dependency_failure_is_not_reported_or_exposed_even_with_debug_enabled(): void
    {
        config(['app.debug' => true]);
        $logger = Log::spy();
        app()->bind(NativeRememberedDeviceSessions::class, function (): never {
            throw new RuntimeException('PRIVATE-DEVICE-DIAGNOSTIC-MARKER');
        });
        $client = app(NativeOAuthClientManager::class)->ensureRust();
        $response = $this->device('/tickets', app(DeviceSessionSecrets::class)->generate(), $client);
        $response->assertStatus(503)->assertExactJson(['error' => 'device_authorization_unavailable']);
        $this->assertNoStore($response);
        $logger->shouldNotHaveReceived('error');
    }

    /** @return array{Identity, Client, string} */
    private function bootstrap(): array
    {
        $identity = $this->createOAuthIdentity();
        $client = app(NativeOAuthClientManager::class)->ensureRust();
        $bootstrap = $this->issueNativeOAuthBootstrapToken($identity, client: $client);

        return [$identity, $client, $bootstrap['access_token']];
    }

    /** @return array{Identity, Client, string} */
    private function enrolled(): array
    {
        [$identity, $client, $bearer] = $this->bootstrap();
        $response = $this->postJson(self::BASE, ['protocol_version' => 1, 'remember_device' => true], ['Authorization' => 'Bearer '.$bearer]);
        $response->assertOk();

        return [$identity, $client, $this->credential($response)];
    }

    /** @return TestResponse<Response> */
    private function device(string $suffix, string $secret, Client $client): TestResponse
    {
        return $this->postJson(self::BASE.$suffix, ['protocol_version' => 1, 'client_id' => $client->getKey()], ['Authorization' => 'OterynDevice '.$secret]);
    }

    /** @param TestResponse<Response> $response */
    private function credential(TestResponse $response): string
    {
        $secret = $response->json('device_credential');
        self::assertIsString($secret);
        self::assertTrue(app(DeviceSessionSecrets::class)->valid($secret));
        self::assertIsInt($response->json('absolute_expires_at'));
        self::assertIsInt($response->json('idle_expires_at'));

        return $secret;
    }

    private function publish(Identity $identity, string $characterId, string $name): void
    {
        $settings = NativeAccountCharactersSettings::current() ?? throw new LogicException('Fixture projection settings are unavailable.');
        $now = now()->getTimestamp();
        $ingestion = app(NativeAccountCharactersIngestion::class);
        $ingestion->snapshot($settings, self::PUBLISHER, NativeAccountCharactersSnapshot::fromWire(json_encode([
            'contract_version' => 1, 'operation' => 'PublishAccountCharactersV1', 'source_authority' => self::AUTHORITY,
            'account_id' => $identity->account_id, 'projection_epoch' => '1', 'projection_revision' => '1', 'source_observed_at' => (string) $now,
            'characters' => [['character_id' => $characterId, 'world_id' => self::WORLD, 'name' => $name, 'availability' => 'AVAILABLE']],
        ], JSON_THROW_ON_ERROR)), $now);
        $ingestion->watermark($settings, self::PUBLISHER, NativeAccountCharactersWatermark::fromWire(json_encode([
            'contract_version' => 1, 'operation' => 'PublishProjectionWatermarkV1', 'source_authority' => self::AUTHORITY,
            'projection_epoch' => '1', 'complete_through' => (string) ($now - 1), 'observed_at' => (string) $now,
        ], JSON_THROW_ON_ERROR)), $now);
    }

    /** @param TestResponse<Response> $response */
    private function assertNoStore(TestResponse $response): void
    {
        self::assertStringContainsString('no-store', (string) $response->headers->get('Cache-Control'));
        self::assertStringContainsString('no-cache', (string) $response->headers->get('Cache-Control'));
        self::assertSame('no-cache', $response->headers->get('Pragma'));
        self::assertSame('0', $response->headers->get('Expires'));
    }

    private function assertIsolatedDatabase(): void
    {
        if (DB::getDefaultConnection() !== 'sqlite' || config('database.connections.sqlite.database') !== ':memory:') {
            throw new LogicException('Device HTTP tests require an isolated in-memory SQLite database.');
        }
    }
}
