<?php

namespace Tests\Feature\GameAuth;

use App\GameAuth\Worlds\GameWorld;
use App\GameAuth\Worlds\GameWorldStatus;
use App\GameAuth\Worlds\NativeRouteRecord;
use Illuminate\Foundation\Testing\RefreshDatabase;
use Illuminate\Support\Facades\DB;
use Tests\TestCase;

final class ClientDirectoryTest extends TestCase
{
    use RefreshDatabase;

    private const WORLD = '018f0f1e-7b2c-7a31-8d4e-1234567890ab';

    private const CHANNEL_A = '018f0f1e-7b2c-7a32-8d4e-1234567890ac';

    private const NODE_IDENTITY = 'CN=acceptance-runtime-node';

    protected function setUp(): void
    {
        parent::setUp();
        config([
            'game-auth.native_runtime_status.enabled' => true,
            'game-auth.native_runtime_status.identities' => [self::NODE_IDENTITY => [self::WORLD.'/'.self::CHANNEL_A]],
            'game-auth.native_runtime_status.freshness_seconds' => 15,
            'game-auth.native_runtime_status.clock_uncertainty_seconds' => 1,
            'game-auth.native_runtime_status.requests_per_minute' => 600,
        ]);
    }

    public function test_anonymous_directory_has_only_public_availability_and_no_characters_or_route(): void
    {
        $this->available();
        $response = $this->getJson('/v1/client/directory')->assertOk();
        self::assertSame([
            ['id' => self::WORLD, 'name' => 'Acceptance', 'channels' => [
                ['id' => self::CHANNEL_A, 'name' => 'channel-1'],
            ], 'characters' => []],
        ], $response->json('worlds'));
        self::assertGreaterThan(0, $response->json('epoch'));
        self::assertStringContainsString('no-store', $response->headers->get('Cache-Control'));
        self::assertSame(['epoch', 'worlds'], array_keys($response->json()));
    }

    public function test_missing_stale_not_ready_and_mismatched_route_evidence_fail_closed(): void
    {
        $this->available();
        foreach ([['ready' => false], ['ready' => true, 'observed_at' => now()->getTimestamp() - 30],
            ['observed_at' => now()->getTimestamp(), 'route_revision' => 'wrong']] as $change) {
            DB::table('native_runtime_status_reports')->update($change);
            $this->getJson('/v1/client/directory')->assertOk()->assertJsonPath('worlds', []);
        }
        DB::table('native_runtime_status_reports')->delete();
        $this->getJson('/v1/client/directory')->assertOk()->assertJsonPath('worlds', []);
    }

    public function test_disabled_world_and_production_activation_are_refused(): void
    {
        $this->available();
        DB::table('game_worlds')->update(['login_enabled' => false]);
        $this->getJson('/v1/client/directory')->assertOk()->assertJsonPath('worlds', []);
        app()->instance('env', 'production');
        $this->getJson('/v1/client/directory')->assertStatus(503);
    }

    public function test_invalid_display_name_is_unavailable_instead_of_an_invalid_client_payload(): void
    {
        $this->available();
        DB::table('game_worlds')->update(['name' => str_repeat('x', 97)]);
        $this->getJson('/v1/client/directory')->assertStatus(503);
    }

    private function available(): void
    {
        $this->world(GameWorldStatus::Online, true, [self::CHANNEL_A]);
        $route = new NativeRouteRecord(self::WORLD, self::CHANNEL_A, '127.0.0.1', 7172, 'localhost', 1);
        DB::table('game_channels')->update([
            'native_route_host' => $route->host, 'native_route_port' => $route->port,
            'native_route_tls_server_name' => $route->tlsServerName, 'native_route_version' => 1,
            'native_route_revision' => $route->routeRevision, 'native_login_enabled' => true,
        ]);
        $this->runtime(self::CHANNEL_A, true, now()->getTimestamp() - 2);
    }

    /** @param list<string> $channels */
    private function world(GameWorldStatus $status, bool $loginEnabled, array $channels): GameWorld
    {
        $world = GameWorld::query()->create([
            'slug' => 'acceptance',
            'name' => 'Acceptance',
            'region' => 'TEST',
            'status' => $status,
            'login_enabled' => $loginEnabled,
            'game_host' => '127.0.0.1',
            'game_port' => 7172,
        ]);
        DB::table('game_worlds')->where('id', $world->id)->update(['world_id' => self::WORLD]);
        foreach ($channels as $index => $channelId) {
            DB::table('game_channels')->insert([
                'game_world_id' => $world->id,
                'channel_id' => $channelId,
                'channel_key' => 'channel-'.($index + 1),
                'created_at' => now(),
                'updated_at' => now(),
            ]);
        }
        $world->refresh();

        return $world;
    }

    private function runtime(string $channelId, bool $ready, int $observedAt, bool $invalid = false): void
    {
        DB::table('native_scope_assignments')->updateOrInsert(
            ['world_id' => self::WORLD, 'channel_id' => $channelId],
            [
                'assignment_epoch' => '1',
                'ownership_generation' => '1',
                'node_identity' => self::NODE_IDENTITY,
                'assigned_at' => $observedAt - 1,
                'created_at' => now(),
                'updated_at' => now(),
            ],
        );
        DB::table('native_runtime_status_reports')->insert([
            'world_id' => self::WORLD,
            'channel_id' => $channelId,
            'node_identity' => self::NODE_IDENTITY,
            'source_authority' => 'oteryn-game',
            'node_id' => '11111111-2222-3333-8444-555555555555',
            'assignment_epoch' => '1',
            'scope_ownership_generation' => '1',
            'source_revision' => '1',
            'decision_identity' => 'acceptance-runtime',
            'ready' => $ready,
            'published_at' => $observedAt - 1,
            'observed_at' => $observedAt,
            'protocol_major' => 1,
            'transport_profile' => 1,
            'route_revision' => (new NativeRouteRecord(self::WORLD, $channelId, '127.0.0.1', 7172, 'localhost', 1))->routeRevision,
            'runtime_observation_revision' => 'runtime-1',
            'ruleset_revision' => 'ruleset-1',
            'content_revision' => 'content-1',
            'map_revision' => 'map-1',
            'world_policy_revision' => 'policy-1',
            'offer_revision' => 'offer-1',
            'content_digest' => str_repeat('a', 64),
            'invalid' => $invalid,
            'created_at' => now(),
            'updated_at' => now(),
        ]);
    }
}
