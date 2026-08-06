<?php

namespace Tests\Feature\GameAuth;

use App\GameAuth\Worlds\GameWorld;
use App\GameAuth\Worlds\GameWorldStatus;
use Illuminate\Foundation\Testing\RefreshDatabase;
use Illuminate\Support\Facades\DB;
use Tests\TestCase;

final class NativeProtocolIdentityMigrationTest extends TestCase
{
    use RefreshDatabase;

    private const OLD_PROFILE = 'oteryn.native.v1';

    private const OLD_SCHEMA_SHA256 = 'c7665223f09001e3294e9a03ab4784defed66b0ac04450e8679d4778421207f8';

    private const NEW_SCHEMA_SHA256 = '9c67f19525400fb9890d2a3541ceb6d02eb955061540ad39ca1c1d891c06eba9';

    public function test_migration_and_rollback_preserve_disabled_native_identity(): void
    {
        $migration = require database_path('migrations/2026_08_05_130000_migrate_native_protocol_identity_to_version.php');
        $migration->down();

        $world = GameWorld::query()->create([
            'slug' => 'migration-test',
            'name' => 'Migration Test',
            'region' => 'TEST',
            'status' => GameWorldStatus::Online,
            'login_enabled' => true,
            'game_host' => 'migration.example.test',
            'game_port' => 7172,
            'gameplay_policy_revision' => 1,
        ]);

        DB::table('game_world_protocol_candidates')->insert([
            'game_world_id' => $world->id,
            'channel_id' => 1,
            'sort_order' => 1,
            'family' => 'oteryn',
            'profile' => self::OLD_PROFILE,
            'transport' => 'tcp.tls13.protobuf.be32.v1',
            'schema_revision' => 1,
            'schema_sha256' => self::OLD_SCHEMA_SHA256,
            'required_capabilities' => json_encode([], JSON_THROW_ON_ERROR),
            'optional_capabilities' => json_encode([], JSON_THROW_ON_ERROR),
            'endpoint_id' => 'native-migration-test',
            'game_host' => 'migration.example.test',
            'game_port' => 7173,
            'tls_server_name' => 'migration.example.test',
            'enabled' => true,
            'created_at' => now(),
            'updated_at' => now(),
        ]);

        $migration->up();
        $migrated = DB::table('game_world_protocol_candidates')->first();
        self::assertNotNull($migrated);
        self::assertNull($migrated->profile);
        self::assertSame(1, (int) $migrated->native_protocol_version);
        self::assertSame(2, (int) $migrated->schema_revision);
        self::assertSame(self::NEW_SCHEMA_SHA256, $migrated->schema_sha256);
        self::assertFalse((bool) $migrated->enabled);

        $migration->down();
        $rolledBack = DB::table('game_world_protocol_candidates')->first();
        self::assertNotNull($rolledBack);
        self::assertSame(self::OLD_PROFILE, $rolledBack->profile);
        self::assertSame(1, (int) $rolledBack->schema_revision);
        self::assertSame(self::OLD_SCHEMA_SHA256, $rolledBack->schema_sha256);
        self::assertFalse((bool) $rolledBack->enabled);

        $migration->up();
    }
}
