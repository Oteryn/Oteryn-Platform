<?php

namespace Tests\Feature\GameAuth\NativeTopology;

use App\GameAuth\Worlds\GameWorld;
use App\GameAuth\Worlds\GameWorldStatus;
use App\GameAuth\Worlds\NativeTopologyRegistry;
use Illuminate\Database\Migrations\Migration;
use Illuminate\Foundation\Testing\DatabaseMigrations;
use Illuminate\Support\Facades\DB;
use Illuminate\Support\Facades\Schema;
use LogicException;
use ReflectionMethod;
use Tests\TestCase;

final class NativeTopologyMigrationRollbackTest extends TestCase
{
    use DatabaseMigrations;

    protected function tearDown(): void
    {
        if ($this->app !== null && Schema::hasTable('game_channels')) {
            // Destroy only this isolated fixture before the framework's
            // migrate-down teardown. Issued product identities remain retained.
            DB::table('game_channels')->delete();
            DB::table('game_worlds')->update(['world_id' => null]);
        }
        parent::tearDown();
    }

    public function test_additive_migration_preserves_existing_integer_worlds_without_backfill(): void
    {
        $this->applyMigration('down');
        try {
            $world = $this->world('pre-existing');
            $before = $world->getAttributes();
            $this->applyMigration('up');
            $before['world_id'] = null;
            self::assertSame($before, $world->fresh()?->getAttributes());
            self::assertSame(0, DB::table('game_channels')->count());
        } finally {
            if (! Schema::hasTable('game_channels')) {
                $this->applyMigration('up');
            }
        }
    }

    public function test_empty_native_additions_can_roll_back_without_losing_legacy_world_rows(): void
    {
        $world = $this->world('empty-native');
        $before = $world->getAttributes();
        unset($before['world_id']);
        try {
            $this->applyMigration('down');
            self::assertTrue(Schema::hasTable('game_worlds'));
            self::assertFalse(Schema::hasColumn('game_worlds', 'world_id'));
            self::assertFalse(Schema::hasTable('game_channels'));
            self::assertSame($before, $world->fresh()?->getAttributes());
        } finally {
            $this->applyMigration('up');
        }
    }

    public function test_inactive_issued_pair_refuses_rollback_before_any_schema_or_identity_change(): void
    {
        $world = $this->world('inactive-issued');
        $pair = (new NativeTopologyRegistry)->issueForPreproduction($world->id, 'entry-room');
        self::assertFalse($world->login_enabled);
        $beforeWorldIndexes = Schema::getIndexes('game_worlds');
        $beforeChannelIndexes = Schema::getIndexes('game_channels');

        $this->assertRollbackRefused();

        self::assertTrue(Schema::hasColumn('game_worlds', 'world_id'));
        self::assertTrue(Schema::hasTable('game_channels'));
        self::assertSame($beforeWorldIndexes, Schema::getIndexes('game_worlds'));
        self::assertSame($beforeChannelIndexes, Schema::getIndexes('game_channels'));
        self::assertEquals($pair, (new NativeTopologyRegistry)->readbackForPreproduction($world->id, 'entry-room'));
    }

    public function test_issued_world_without_channel_already_requires_retention(): void
    {
        $world = $this->world('world-only');
        $pair = (new NativeTopologyRegistry)->issueForPreproduction($world->id, 'entry-room');
        // Explicit fixture disposal leaves the retained World-only invariant.
        DB::table('game_channels')->delete();

        $this->assertRollbackRefused();

        self::assertTrue(Schema::hasTable('game_channels'));
        self::assertTrue(Schema::hasColumn('game_worlds', 'world_id'));
        self::assertSame($pair->worldId, GameWorld::query()->findOrFail($world->id)->world_id);
    }

    public function test_malformed_nonnull_retained_identity_cannot_enable_destructive_rollback(): void
    {
        $world = $this->world('ambiguous-retained');
        $invalid = "01890f4e-7c00-7000-8000-000000000001\n";
        DB::table('game_worlds')->where('id', $world->id)->update(['world_id' => $invalid]);

        $this->assertRollbackRefused();

        self::assertTrue(Schema::hasTable('game_channels'));
        self::assertTrue(Schema::hasColumn('game_worlds', 'world_id'));
        self::assertSame($invalid, DB::table('game_worlds')->where('id', $world->id)->value('world_id'));
    }

    private function assertRollbackRefused(): void
    {
        try {
            $this->applyMigration('down');
            self::fail('Even inactive issued topology must survive rollback.');
        } catch (LogicException $exception) {
            self::assertStringContainsString('before DDL', $exception->getMessage());
        }
    }

    /** @param 'up'|'down' $direction */
    private function applyMigration(string $direction): void
    {
        $migration = require database_path('migrations/2026_09_26_150000_add_native_world_topology.php');
        self::assertInstanceOf(Migration::class, $migration);

        (new ReflectionMethod($migration, $direction))->invoke($migration);
    }

    private function world(string $slug): GameWorld
    {
        return GameWorld::query()->create([
            'slug' => $slug, 'name' => 'Disposable '.$slug, 'region' => 'TEST',
            'status' => GameWorldStatus::Maintenance, 'login_enabled' => false,
            'game_host' => '127.0.0.1', 'game_port' => 7172,
        ])->refresh();
    }
}
