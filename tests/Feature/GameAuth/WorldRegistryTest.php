<?php

namespace Tests\Feature\GameAuth;

use App\GameAuth\Worlds\DatabaseWorldRegistry;
use App\GameAuth\Worlds\GameWorld;
use App\GameAuth\Worlds\GameWorldProtocolCandidate;
use App\GameAuth\Worlds\GameWorldStatus;
use Illuminate\Foundation\Testing\RefreshDatabase;
use Tests\TestCase;

final class WorldRegistryTest extends TestCase
{
    use RefreshDatabase;

    private const NATIVE_SCHEMA_SHA256 = '9c67f19525400fb9890d2a3541ceb6d02eb955061540ad39ca1c1d891c06eba9';

    private const NATIVE_CAPABILITIES = [
        'actions.command-result.v1',
        'chat.semantic.v1',
        'combat.server-authoritative.v1',
        'inventory.server-authoritative.v1',
        'ordering.server-sequence.v1',
        'reconciliation.movement.v1',
        'session.single-admission.v1',
        'state.revision.v1',
        'state.snapshot-delta.v1',
    ];

    public function test_registry_is_empty_by_default_and_does_not_invent_a_production_route(): void
    {
        self::assertSame([], (new DatabaseWorldRegistry)->forAccount(1001));
        self::assertSame(0, GameWorld::query()->count());
        self::assertSame(0, GameWorldProtocolCandidate::query()->count());
    }

    public function test_registry_returns_only_login_enabled_online_routable_worlds(): void
    {
        $eligible = $this->createWorld(
            slug: 'oteryn-test',
            status: GameWorldStatus::Online,
            loginEnabled: true,
            host: 'game.test',
            port: 7172,
        );
        $this->createWorld('maintenance', GameWorldStatus::Maintenance, true, 'maintenance.test', 7172);
        $this->createWorld('offline', GameWorldStatus::Offline, true, 'offline.test', 7172);
        $this->createWorld('disabled', GameWorldStatus::Online, false, 'disabled.test', 7172);
        $this->createWorld('invalid-host', GameWorldStatus::Online, true, 'not a host', 7172);
        $this->createWorld('invalid-port', GameWorldStatus::Online, true, 'port.test', 0);

        $routes = (new DatabaseWorldRegistry)->forAccount(1001);

        self::assertCount(1, $routes);
        self::assertSame($eligible->id, $routes[0]->id);
        self::assertSame('oteryn-test', $routes[0]->slug);
        self::assertSame('game.test', $routes[0]->host);
        self::assertSame(7172, $routes[0]->port);
        $policy = $routes[0]->gameplayPolicy;
        self::assertNotNull($policy);
        self::assertSame(1, $policy->revision);
        self::assertSame([], $policy->candidates);
    }

    public function test_protocol_candidates_are_disabled_by_default_and_ordered_by_authoritative_policy(): void
    {
        $world = $this->createWorld('oteryn-test', GameWorldStatus::Online, true, 'legacy.test', 7172, 17);
        $this->createCandidate($world, 'disabled', 0, false, 'canary', 'canary.disabled', 7172, []);
        $this->createCandidate($world, 'canary-primary', 2, true, 'canary', 'canary.current', 7172, ['session.single-admission.v1']);
        $this->createCandidate($world, 'native-primary', 1, true, 'oteryn', 1, 7173);

        $routes = (new DatabaseWorldRegistry)->forAccount(1001);
        $policy = $routes[0]->gameplayPolicy;

        self::assertNotNull($policy);
        self::assertSame(17, $policy->revision);
        self::assertSame(1, $policy->channelId);
        self::assertCount(2, $policy->candidates);
        self::assertSame('native-primary', $policy->candidates[0]->endpointId);
        self::assertSame('canary-primary', $policy->candidates[1]->endpointId);
        self::assertSame(self::NATIVE_CAPABILITIES, $policy->candidates[0]->requiredCapabilities);
        self::assertSame(self::NATIVE_SCHEMA_SHA256, $policy->candidates[0]->schemaSha256);
    }

    public function test_enabled_noncanonical_candidate_invalidates_policy_without_breaking_legacy_world_route(): void
    {
        $world = $this->createWorld('oteryn-test', GameWorldStatus::Online, true, 'legacy.test', 7172);
        $candidate = $this->createCandidate($world, 'native-invalid', 1, true, 'oteryn', 1, 7173);
        $candidate->forceFill(['required_capabilities' => array_reverse(self::NATIVE_CAPABILITIES)])->save();

        $routes = (new DatabaseWorldRegistry)->forAccount(1001);
        $policy = $routes[0]->gameplayPolicy;

        self::assertCount(1, $routes);
        self::assertSame('legacy.test', $routes[0]->host);
        self::assertNotNull($policy);
        self::assertSame(0, $policy->revision);
        self::assertSame([], $policy->candidates);
    }

    public function test_wrong_native_schema_hash_invalidates_policy(): void
    {
        $world = $this->createWorld('oteryn-test', GameWorldStatus::Online, true, 'legacy.test', 7172);
        $candidate = $this->createCandidate($world, 'native-invalid-hash', 1, true, 'oteryn', 1, 7173);
        $candidate->forceFill(['schema_sha256' => str_repeat('a', 64)])->save();

        $routes = (new DatabaseWorldRegistry)->forAccount(1001);
        $policy = $routes[0]->gameplayPolicy;

        self::assertNotNull($policy);
        self::assertSame(0, $policy->revision);
        self::assertSame([], $policy->candidates);
    }

    public function test_registry_fails_closed_for_invalid_account_identifier(): void
    {
        $this->createWorld('oteryn-test', GameWorldStatus::Online, true, 'game.test', 7172);

        self::assertSame([], (new DatabaseWorldRegistry)->forAccount(0));
    }

    private function createWorld(
        string $slug,
        GameWorldStatus $status,
        bool $loginEnabled,
        string $host,
        int $port,
        int $policyRevision = 1,
    ): GameWorld {
        return GameWorld::query()->create([
            'slug' => $slug,
            'name' => ucfirst($slug),
            'region' => 'TEST',
            'status' => $status,
            'login_enabled' => $loginEnabled,
            'game_host' => $host,
            'game_port' => $port,
            'gameplay_policy_revision' => $policyRevision,
        ]);
    }

    /**
     * @param  list<string>|null  $requiredCapabilities
     */
    private function createCandidate(
        GameWorld $world,
        string $endpointId,
        int $sortOrder,
        bool $enabled,
        string $family,
        string|int $identity,
        int $port,
        ?array $requiredCapabilities = null,
    ): GameWorldProtocolCandidate {
        return GameWorldProtocolCandidate::query()->create([
            'game_world_id' => $world->id,
            'channel_id' => 1,
            'sort_order' => $sortOrder,
            'family' => $family,
            'profile' => $family === 'oteryn' ? null : (string) $identity,
            'native_protocol_version' => $family === 'oteryn' ? (int) $identity : null,
            'transport' => $family === 'oteryn' ? 'tcp.tls13.protobuf.be32.v1' : 'canary.sequence.v1',
            'schema_revision' => $family === 'oteryn' ? 2 : 1,
            'schema_sha256' => $family === 'oteryn' && $identity === 1
                ? self::NATIVE_SCHEMA_SHA256
                : str_repeat('a', 64),
            'required_capabilities' => $requiredCapabilities ?? self::NATIVE_CAPABILITIES,
            'optional_capabilities' => [],
            'endpoint_id' => $endpointId,
            'game_host' => 'game.example.test',
            'game_port' => $port,
            'tls_server_name' => 'game.example.test',
            'enabled' => $enabled,
        ]);
    }
}
