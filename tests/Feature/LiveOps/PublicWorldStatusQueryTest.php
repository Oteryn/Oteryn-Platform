<?php

namespace Tests\Feature\LiveOps;

use App\GameAuth\Worlds\GameWorld;
use App\GameAuth\Worlds\GameWorldStatus;
use App\LiveOps\WorldStatus\PublicWorldStatus;
use App\LiveOps\WorldStatus\PublicWorldStatusQuery;
use Illuminate\Foundation\Testing\RefreshDatabase;
use Illuminate\Support\Facades\DB;
use Tests\TestCase;

final class PublicWorldStatusQueryTest extends TestCase
{
    use RefreshDatabase;

    private const WORLD = '018f0f1e-7b2c-7a31-8d4e-1234567890ab';

    private const CHANNEL_A = '018f0f1e-7b2c-7a32-8d4e-1234567890ac';

    private const CHANNEL_B = '018f0f1e-7b2c-7a33-8d4e-1234567890ad';

    private const NODE_IDENTITY = 'CN=acceptance-runtime-node';

    private const ASSIGNMENT_IDENTITY = 'CN=acceptance-runtime-owner';

    protected function setUp(): void
    {
        parent::setUp();

        config()->set('game-auth.native_runtime_status.enabled', true);
        config()->set('game-auth.native_runtime_status.identities', [self::NODE_IDENTITY => [
            self::WORLD.'/'.self::CHANNEL_A,
            self::WORLD.'/'.self::CHANNEL_B,
        ]]);
        config()->set('game-auth.native_runtime_status.freshness_seconds', 15);
        config()->set('game-auth.native_runtime_status.clock_uncertainty_seconds', 1);
        config()->set('game-auth.native_runtime_status.requests_per_minute', 600);
        config()->set('game-auth.native_scope_assignment.identities', [self::ASSIGNMENT_IDENTITY => [
            self::WORLD.'/'.self::CHANNEL_A,
            self::WORLD.'/'.self::CHANNEL_B,
        ]]);
    }

    public function test_fresh_ready_evidence_projects_ready_without_private_runtime_fields(): void
    {
        $now = 1_800_000_000;
        $this->world(GameWorldStatus::Online, true, [self::CHANNEL_A]);
        $this->runtime(self::CHANNEL_A, true, $now - 2);

        $worlds = app(PublicWorldStatusQuery::class)->get($now);

        self::assertCount(1, $worlds);
        self::assertSame(PublicWorldStatus::RUNTIME_READY, $worlds[0]->runtimeState);
        self::assertSame('ready', $worlds[0]->publicState());
        self::assertSame($now - 2, $worlds[0]->observedAt);
        self::assertSame('Acceptance', $worlds[0]->name);
    }

    public function test_configured_maintenance_remains_separate_and_overrides_public_runtime_label(): void
    {
        $now = 1_800_000_000;
        $this->world(GameWorldStatus::Maintenance, false, [self::CHANNEL_A]);
        $this->runtime(self::CHANNEL_A, true, $now - 2);

        $world = app(PublicWorldStatusQuery::class)->get($now)[0];

        self::assertSame(PublicWorldStatus::RUNTIME_READY, $world->runtimeState);
        self::assertSame('maintenance', $world->publicState());
        self::assertFalse($world->isPartial());
    }

    public function test_stale_unavailable_and_invalid_evidence_never_become_offline(): void
    {
        $now = 1_800_000_000;
        $this->world(GameWorldStatus::Online, true, [self::CHANNEL_A]);
        $this->runtime(self::CHANNEL_A, true, $now - 30);

        $query = app(PublicWorldStatusQuery::class);
        self::assertSame('stale', $query->get($now)[0]->publicState());

        DB::table('native_runtime_status_reports')->delete();
        self::assertSame('unavailable', $query->get($now)[0]->publicState());

        $this->runtime(self::CHANNEL_A, true, $now - 2, invalid: true);
        self::assertSame('invalid', $query->get($now)[0]->publicState());
    }

    public function test_mixed_channel_evidence_is_degraded_and_recovery_requires_fresh_authoritative_evidence(): void
    {
        $now = 1_800_000_000;
        $this->world(GameWorldStatus::Online, true, [self::CHANNEL_A, self::CHANNEL_B]);
        $this->runtime(self::CHANNEL_A, true, $now - 2);
        $this->runtime(self::CHANNEL_B, false, $now - 30);

        $query = app(PublicWorldStatusQuery::class);
        self::assertSame('degraded', $query->get($now)[0]->publicState());

        DB::table('native_runtime_status_reports')->where('channel_id', self::CHANNEL_B)->update([
            'ready' => true,
            'observed_at' => $now - 1,
            'updated_at' => now(),
        ]);

        self::assertSame('ready', $query->get($now)[0]->publicState());
    }

    public function test_public_today_consumes_liveops_query_and_redacts_runtime_owner_details(): void
    {
        $now = now()->getTimestamp();
        $this->world(GameWorldStatus::Online, true, [self::CHANNEL_A]);
        $this->runtime(self::CHANNEL_A, true, $now - 2);

        $response = $this->get('/en/today')
            ->assertOk()
            ->assertSee('data-today-card="liveops"', false)
            ->assertSee('data-content-state="present"', false)
            ->assertSeeText('Acceptance')
            ->assertSeeText('Ready');

        $body = (string) $response->getContent();
        self::assertStringNotContainsString(self::NODE_IDENTITY, $body);
        self::assertStringNotContainsString('ownership_generation', $body);
        self::assertStringNotContainsString('route_revision', $body);
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
            'route_revision' => 'route-1',
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
