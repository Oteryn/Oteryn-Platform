<?php

namespace Tests\Feature\GameAuth\NativeLogin;

use App\GameAuth\NativeEvidence\NativeEvidenceContract;
use App\GameAuth\NativeEvidence\NativeSigningTrustRegistry;
use App\GameAuth\NativeLogin\NativeAdmissionAttempt;
use App\GameAuth\NativeLogin\NativeGameLoginTickets;
use App\GameAuth\Tickets\GameLoginTicket;
use App\GameAuth\Worlds\GameWorld;
use App\GameAuth\Worlds\GameWorldStatus;
use App\GameAuth\Worlds\NativeRouteRecord;
use App\GameAuth\Worlds\NativeTopologyRegistry;
use App\Identity\Models\Identity;
use Illuminate\Foundation\Testing\DatabaseMigrations;
use Illuminate\Support\Carbon;
use Illuminate\Support\Facades\DB;
use Illuminate\Support\Facades\Hash;
use Illuminate\Support\Facades\Schema;
use Illuminate\Testing\TestResponse;
use InvalidArgumentException;
use LogicException;
use Symfony\Component\HttpFoundation\Response;
use Tests\TestCase;

/**
 * `POST /internal/v1/game-auth/native-admissions` end to end inside Platform (login contract §3.3,
 * §5.4 under D171, §7 with the D172 route record): Gateway credential, exact wire, Registry route
 * selection over fresh ownership-bound runtime status, signed grant and the §11 error body.
 */
final class NativeAdmissionIssuerTest extends TestCase
{
    use DatabaseMigrations;

    private const PATH = '/internal/v1/game-auth/native-admissions';

    private const SERVICE_TOKEN = 'gateway-service-token-for-tests';

    private const PURPOSE = 'fresh_admission';

    private const NODE = 'CN=node-a.runtime-status';

    private const ATTEMPT = '0192b3c4-5d6e-7f80-9a1b-2c3d4e5f6a7b';

    private const CHARACTER = '0192b3c4-5d6e-7f80-8a1b-2c3d4e5f6a7c';

    private const NOW = 1_790_000_000;

    private string $directory;

    private string $worldId;

    /** @var array<string, string> channel key => ChannelId */
    private array $channels = [];

    private int $worldRow;

    protected function setUp(): void
    {
        parent::setUp();
        Carbon::setTestNow(Carbon::createFromTimestamp(self::NOW));

        $this->directory = storage_path('framework/testing/native-issuer-'.bin2hex(random_bytes(6)));
        mkdir($this->directory.'/witness', 0700, true);
        $seed = random_bytes(SODIUM_CRYPTO_SIGN_SEEDBYTES);
        file_put_contents($this->directory.'/current.key', rtrim(strtr(base64_encode($seed), '+/', '-_'), '=')."\n");
        chmod($this->directory.'/current.key', 0600);

        $this->worldRow = GameWorld::query()->create([
            'slug' => 'native-entry',
            'name' => 'Native entry',
            'region' => 'TEST',
            'status' => GameWorldStatus::Maintenance,
            'login_enabled' => false,
            'game_host' => '127.0.0.1',
            'game_port' => 7172,
        ])->id;
        $registry = new NativeTopologyRegistry;
        foreach (['alpha', 'beta'] as $key) {
            $receipt = $registry->issueForPreproduction($this->worldRow, $key);
            $this->worldId = $receipt->worldId;
            $this->channels[$key] = $receipt->channelId;
        }
        $scopes = array_map(fn (string $channel): string => $this->worldId.'/'.$channel, array_values($this->channels));

        config([
            'game-auth.gateway.service_token_sha256' => hash('sha256', self::SERVICE_TOKEN),
            'game-auth.native_evidence.high_water_directory' => $this->directory.'/witness',
            'game-auth.native_evidence.fresh_key_purpose' => self::PURPOSE,
            'game-auth.native_evidence.clock_uncertainty_seconds' => 1,
            'game-auth.native_admission.enabled' => true,
            'game-auth.native_admission.grant_ttl_seconds' => 20,
            'game-auth.native_admission.signing_key_file' => $this->directory.'/current.key',
            'game-auth.native_admission.signing_key_id' => 'admission-1',
            'game-auth.native_admission.unverified_character_ownership' => true,
            'game-auth.native_admission.unverified_character_world_id' => $this->worldId,
            'game-auth.native_runtime_status.enabled' => true,
            'game-auth.native_runtime_status.identities' => json_encode([self::NODE => $scopes], JSON_THROW_ON_ERROR),
        ]);
        $this->app->make(NativeSigningTrustRegistry::class)->publishTrustedKey(
            NativeEvidenceContract::FRESH_ISSUER,
            NativeEvidenceContract::FRESH_PROFILE,
            self::PURPOSE,
            'admission-1',
            sodium_crypto_sign_publickey(sodium_crypto_sign_seed_keypair($seed)),
        );
    }

    protected function tearDown(): void
    {
        Carbon::setTestNow();
        if ($this->app !== null) {
            $this->app->detectEnvironment(fn (): string => 'testing');
            // Explicit disposal of this isolated fixture so the framework's migrate-down can run.
            if (Schema::hasTable('game_channels')) {
                DB::table('game_channels')->delete();
                DB::table('game_worlds')->update(['world_id' => null]);
            }
            foreach (['native_admission_attempts', 'game_login_tickets', 'native_runtime_status_reports', 'native_scope_assignments'] as $table) {
                Schema::hasTable($table) && DB::table($table)->delete();
            }
        }
        foreach (glob($this->directory.'/{,witness/}*', GLOB_BRACE) ?: [] as $path) {
            (is_link($path) || is_file($path)) && @unlink($path);
        }
        @rmdir($this->directory.'/witness');
        @rmdir($this->directory);

        parent::tearDown();
    }

    public function test_route_revision_binds_version_scope_and_endpoint(): void
    {
        $record = new NativeRouteRecord($this->worldId, $this->channels['alpha'], 'game-eu1.example.invalid', 7172, 'game-eu1.example.invalid', 1);
        $descriptor = '{"alpn":"oteryn-game/1","channel_id":"'.$this->channels['alpha'].'","host":"game-eu1.example.invalid","port":7172,'
            .'"protocol_major":1,"tls_server_name":"game-eu1.example.invalid","transport_profile":1,"world_id":"'.$this->worldId.'"}';

        self::assertSame($descriptor, $record->canonicalDescriptor());
        self::assertSame('rt.1.'.substr(hash('sha256', $descriptor), 0, 32), $record->routeRevision);
        self::assertNotSame($record->routeRevision, (new NativeRouteRecord($this->worldId, $this->channels['beta'], 'game-eu1.example.invalid', 7172, 'game-eu1.example.invalid', 1))->routeRevision);
        self::assertNotSame($record->routeRevision, (new NativeRouteRecord($this->worldId, $this->channels['alpha'], 'game-eu1.example.invalid', 7173, 'game-eu1.example.invalid', 1))->routeRevision);
        self::assertSame('::1', (new NativeRouteRecord($this->worldId, $this->channels['alpha'], '::1', 7172, 'localhost', 1))->host);

        foreach ([['Game.example', 7172, 'game.example'], ['game.example', 0, 'game.example'], ['game.example', 7172, '127.0.0.1:1'], ['0:0::1', 7172, 'game.example']] as [$host, $port, $sni]) {
            try {
                new NativeRouteRecord($this->worldId, $this->channels['alpha'], $host, $port, $sni, 1);
                self::fail('Invalid route record accepted.');
            } catch (InvalidArgumentException) {
            }
        }
    }

    public function test_publishing_keeps_an_unchanged_revision_and_advances_on_any_endpoint_change(): void
    {
        $first = $this->publish('alpha');
        self::assertSame(1, $first->version);
        self::assertSame($first->routeRevision, $this->publish('alpha', enabled: false)->routeRevision);

        $moved = $this->publish('alpha', port: 7200);
        self::assertSame(2, $moved->version);
        self::assertSame($moved->routeRevision, DB::table('game_channels')->where('channel_id', $this->channels['alpha'])->value('native_route_revision'));

        $this->app->detectEnvironment(fn (): string => 'production');
        try {
            $this->publish('alpha');
            self::fail('A route record was published outside testing/preproduction.');
        } catch (LogicException) {
        }
    }

    public function test_issues_a_grant_for_the_lowest_ready_channel_with_the_registry_endpoint(): void
    {
        $alpha = $this->publish('alpha');
        $beta = $this->publish('beta', host: 'game-b.example.invalid');
        $this->report('alpha', $alpha->routeRevision);
        $this->report('beta', $beta->routeRevision);
        $expected = strcmp($alpha->channelId, $beta->channelId) < 0 ? $alpha : $beta;
        $ticket = $this->ticket();

        $response = $this->admit($this->body($ticket));

        $response->assertOk();
        self::assertStringContainsString('no-store', (string) $response->headers->get('Cache-Control'));
        $json = $response->json();
        self::assertIsArray($json);
        self::assertSame(['protocol_version', 'attempt_ref', 'world_id', 'channel_id', 'endpoint', 'grant'], array_keys($json));
        self::assertSame($expected->channelId, $json['channel_id']);
        self::assertSame($expected->endpoint(), $json['endpoint']);
        self::assertSame(20, $response->json('grant.valid_for_seconds'));
        $token = $response->json('grant.token');
        self::assertIsString($token);
        $claims = json_decode($this->decode(explode('.', $token)[1] ?? ''), true, 2, JSON_THROW_ON_ERROR);
        self::assertIsArray($claims);
        self::assertSame($expected->routeRevision, $claims['route_revision']);
        self::assertSame(self::CHARACTER, $claims['character_id']);
        self::assertSame($this->worldId, $claims['world_id']);
        self::assertSame('3', $claims['scope_ownership_generation']);
        self::assertSame('offer-1', $claims['offer_revision']);

        $retry = $this->admit($this->body($ticket));
        self::assertSame($token, $retry->json('grant.token'));
        self::assertSame(1, NativeAdmissionAttempt::query()->count());
    }

    public function test_requested_channel_must_itself_be_a_candidate(): void
    {
        $alpha = $this->publish('alpha');
        $beta = $this->publish('beta');
        $this->report('alpha', $alpha->routeRevision);
        $this->report('beta', $beta->routeRevision, ready: false);
        $ticket = $this->ticket();

        $this->assertError($this->admit($this->body($ticket, channelId: $beta->channelId)), 503, 'NATIVE_LOGIN_ROUTE_UNAVAILABLE');
        self::assertNull(GameLoginTicket::query()->sole()->used_at);

        $this->admit($this->body($ticket, channelId: $alpha->channelId))->assertOk()->assertJsonPath('channel_id', $alpha->channelId);
    }

    public function test_stale_mismatched_disabled_or_unbound_runtime_status_routes_nowhere(): void
    {
        $alpha = $this->publish('alpha');
        $ticket = $this->ticket();

        $this->assertError($this->admit($this->body($ticket)), 503, 'NATIVE_LOGIN_ROUTE_UNAVAILABLE');

        $this->report('alpha', $alpha->routeRevision, observedAt: self::NOW - 15);
        $this->assertError($this->admit($this->body($ticket)), 503, 'NATIVE_LOGIN_ROUTE_UNAVAILABLE');

        $this->report('alpha', 'rt.1.00000000000000000000000000000000');
        $this->assertError($this->admit($this->body($ticket)), 503, 'NATIVE_LOGIN_ROUTE_UNAVAILABLE');

        $this->report('alpha', $alpha->routeRevision);
        DB::table('native_scope_assignments')->update(['ownership_generation' => '4']);
        $this->assertError($this->admit($this->body($ticket)), 503, 'NATIVE_LOGIN_ROUTE_UNAVAILABLE');

        $this->report('alpha', $alpha->routeRevision);
        $this->publish('alpha', enabled: false);
        $this->assertError($this->admit($this->body($ticket)), 503, 'NATIVE_LOGIN_ROUTE_UNAVAILABLE');

        $this->publish('alpha');
        DB::table('game_channels')->update(['native_route_port' => 7999]);
        $this->assertError($this->admit($this->body($ticket)), 503, 'NATIVE_LOGIN_ROUTE_UNAVAILABLE');

        $this->publish('alpha', port: 7172);
        self::assertNull(GameLoginTicket::query()->sole()->used_at);
        self::assertSame(0, NativeAdmissionAttempt::query()->count());
    }

    public function test_unverified_character_mode_is_config_gated_and_refused_outside_testing_and_preproduction(): void
    {
        $alpha = $this->publish('alpha');
        $this->report('alpha', $alpha->routeRevision);
        $ticket = $this->ticket();

        config(['game-auth.native_admission.unverified_character_ownership' => false]);
        $this->assertError($this->admit($this->body($ticket)), 503, 'NATIVE_LOGIN_ROUTE_UNAVAILABLE');

        config(['game-auth.native_admission.unverified_character_ownership' => true]);
        $this->app->detectEnvironment(fn (): string => 'production');
        $this->assertError($this->admit($this->body($ticket)), 503, 'NATIVE_LOGIN_UNAVAILABLE');
        $this->app->detectEnvironment(fn (): string => 'testing');

        config(['game-auth.native_admission.unverified_character_world_id' => 'not-a-world']);
        $this->assertError($this->admit($this->body($ticket)), 503, 'NATIVE_LOGIN_UNAVAILABLE');
        self::assertNull(GameLoginTicket::query()->sole()->used_at);
    }

    public function test_wire_errors_use_the_exact_error_body_and_echo_only_a_canonical_attempt_ref(): void
    {
        $ticket = $this->ticket();

        $this->assertError($this->admit(json_encode(['protocol_version' => 1] + $this->payload($ticket), JSON_THROW_ON_ERROR)), 400, 'NATIVE_LOGIN_UNSUPPORTED_VERSION', self::ATTEMPT);
        $duplicate = substr($this->body($ticket), 0, -1).',"attempt_ref":"'.self::ATTEMPT.'"}';
        $this->assertError($this->admit($duplicate), 400, 'NATIVE_LOGIN_REQUEST_MALFORMED', self::ATTEMPT);
        $unknown = json_encode($this->payload($ticket) + ['extra' => 1], JSON_THROW_ON_ERROR);
        $this->assertError($this->admit($unknown), 400, 'NATIVE_LOGIN_REQUEST_MALFORMED', self::ATTEMPT);
        $missing = $this->payload($ticket);
        unset($missing['channel_id']);
        $this->assertError($this->admit(json_encode($missing, JSON_THROW_ON_ERROR)), 400, 'NATIVE_LOGIN_REQUEST_MALFORMED', self::ATTEMPT);
        $upper = json_encode(['attempt_ref' => strtoupper(self::ATTEMPT)] + $this->payload($ticket), JSON_THROW_ON_ERROR);
        $this->assertError($this->admit($upper), 400, 'NATIVE_LOGIN_REQUEST_MALFORMED', null);
        $this->assertError($this->admit(str_repeat(' ', 2049)), 400, 'NATIVE_LOGIN_REQUEST_MALFORMED', null);

        $this->assertError($this->admit($this->body(str_repeat('a', 43))), 401, 'NATIVE_LOGIN_AUTHENTICATION_REQUIRED', self::ATTEMPT);
    }

    public function test_gateway_credential_and_per_credential_rate_limit_guard_the_issuer(): void
    {
        $body = $this->body($this->ticket());
        $this->call('POST', self::PATH, [], [], [], ['CONTENT_TYPE' => 'application/json'], $body)->assertUnauthorized();
        $this->call('POST', self::PATH, [], [], [], ['CONTENT_TYPE' => 'application/json', 'HTTP_AUTHORIZATION' => 'Bearer wrong'], $body)->assertUnauthorized();

        config(['game-auth.native_admission.requests_per_minute' => 2]);
        $this->admit($body);
        $this->admit($body);
        $limited = $this->admit($body);
        $this->assertError($limited, 429, 'NATIVE_LOGIN_RATE_LIMITED', self::ATTEMPT);
        self::assertGreaterThanOrEqual(1, (int) $limited->headers->get('Retry-After'));
        $config = require base_path('config/game-auth.php');
        self::assertIsArray($config);
        self::assertIsArray($config['native_scope_assignment']);
        self::assertSame(120, $config['native_scope_assignment']['requests_per_minute']);
    }

    private function publish(string $key, string $host = 'game-a.example.invalid', int $port = 7172, bool $enabled = true): NativeRouteRecord
    {
        return (new NativeTopologyRegistry)->publishRouteForPreproduction($this->worldRow, $key, $host, $port, $host, $enabled);
    }

    private function report(string $key, string $routeRevision, bool $ready = true, int $observedAt = self::NOW): void
    {
        $scope = ['world_id' => $this->worldId, 'channel_id' => $this->channels[$key]];
        DB::table('native_scope_assignments')->updateOrInsert($scope, [
            'assignment_epoch' => '1', 'ownership_generation' => '3', 'node_identity' => self::NODE, 'assigned_at' => self::NOW - 60,
        ]);
        $revision = fn (string $name): string => $name.'-1';
        DB::table('native_runtime_status_reports')->updateOrInsert($scope, [
            'node_identity' => self::NODE,
            'source_authority' => 'oteryn-game',
            'node_id' => '01934f10-7c04-7001-805b-3b1122334401',
            'assignment_epoch' => '1',
            'scope_ownership_generation' => '3',
            'source_revision' => '7',
            'decision_identity' => 'runtime-readiness:0a:3:7:true',
            'ready' => $ready,
            'published_at' => $observedAt,
            'observed_at' => $observedAt,
            'protocol_major' => 1,
            'transport_profile' => 1,
            'route_revision' => $routeRevision,
            'runtime_observation_revision' => $revision('observation'),
            'ruleset_revision' => $revision('ruleset'),
            'content_revision' => $revision('content'),
            'map_revision' => $revision('map'),
            'world_policy_revision' => $revision('policy'),
            'offer_revision' => $revision('offer'),
            'content_digest' => str_repeat('0', 64),
            'invalid' => false,
        ]);
    }

    private function ticket(): string
    {
        $identity = Identity::query()->create(['email' => 'native-issuer@example.test', 'password' => Hash::make('Correct-Horse-9!Battery')])->refresh();
        $ticket = bin2hex(random_bytes(32));
        GameLoginTicket::query()->create([
            'ticket_hash' => hash('sha256', $ticket),
            'identity_id' => $identity->id,
            'canary_account_id' => null,
            'account_id' => $identity->account_id,
            'audience' => NativeGameLoginTickets::AUDIENCE,
            'security_generation' => $identity->game_auth_generation,
            'native_security_generation' => $identity->native_security_generation,
            'expires_at' => now()->addSeconds(60),
        ]);

        return $ticket;
    }

    /** @return array<string, mixed> */
    private function payload(string $ticket, ?string $channelId = null): array
    {
        return [
            'protocol_version' => 2,
            'game_login_ticket' => $ticket,
            'attempt_ref' => self::ATTEMPT,
            'character_id' => self::CHARACTER,
            'channel_id' => $channelId,
            'offer' => [
                'client_build' => '0.1.0+abc123',
                'client_platform' => 'windows',
                'transports' => [['protocol_major' => 1, 'transport_profile' => 1, 'alpn' => 'oteryn-game/1']],
            ],
        ];
    }

    private function body(string $ticket, ?string $channelId = null): string
    {
        return json_encode($this->payload($ticket, $channelId), JSON_THROW_ON_ERROR | JSON_UNESCAPED_SLASHES);
    }

    /** @return TestResponse<Response> */
    private function admit(string $body): TestResponse
    {
        return $this->call('POST', self::PATH, [], [], [], [
            'CONTENT_TYPE' => 'application/json',
            'HTTP_AUTHORIZATION' => 'Bearer '.self::SERVICE_TOKEN,
        ], $body);
    }

    /** @param TestResponse<Response> $response */
    private function assertError(TestResponse $response, int $status, string $code, ?string $attemptRef = self::ATTEMPT): void
    {
        $response->assertStatus($status);
        self::assertStringContainsString('no-store', (string) $response->headers->get('Cache-Control'));
        $json = $response->json();
        self::assertIsArray($json);
        self::assertSame(['protocol_version', 'error', 'attempt_ref'], array_keys($json));
        self::assertSame($code, $response->json('error.code'));
        self::assertSame($attemptRef, $json['attempt_ref']);
    }

    private function decode(string $segment): string
    {
        return (string) base64_decode(strtr($segment, '-_', '+/'), true);
    }
}
