<?php

namespace Tests\Feature\GameAuth\NativeTopology;

use App\GameAuth\Worlds\DatabaseWorldRegistry;
use App\GameAuth\Worlds\GameChannel;
use App\GameAuth\Worlds\GameWorld;
use App\GameAuth\Worlds\GameWorldStatus;
use App\GameAuth\Worlds\NativeTopologyReceipt;
use App\GameAuth\Worlds\NativeTopologyRegistry;
use Illuminate\Foundation\Testing\DatabaseMigrations;
use Illuminate\Support\Facades\Artisan;
use Illuminate\Support\Facades\DB;
use Illuminate\Support\Facades\Schema;
use Illuminate\Support\Str;
use LogicException;
use Ramsey\Uuid\Uuid;
use Tests\TestCase;

final class NativeTopologyRegistryTest extends TestCase
{
    use DatabaseMigrations;

    protected function tearDown(): void
    {
        Str::createUuidsNormally();
        if ($this->app !== null) {
            $this->app->detectEnvironment(fn (): string => 'testing');
            // Explicit disposal of this isolated test fixture, not a product
            // identity-deletion API or permission to weaken migration down.
            if (Schema::hasTable('game_channels')) {
                DB::table('game_channels')->delete();
            }
            if (Schema::hasColumn('game_worlds', 'world_id')) {
                DB::table('game_worlds')->update(['world_id' => null]);
            }
        }
        parent::tearDown();
    }

    public function test_additive_schema_leaves_legacy_worlds_unissued_and_integer_routes_unchanged(): void
    {
        $world = $this->world('legacy', true);
        self::assertNull($world->world_id);
        self::assertSame(0, GameChannel::query()->count());
        $before = serialize((new DatabaseWorldRegistry)->forAccount(1001));
        $attributes = $world->getAttributes();

        $receipt = (new NativeTopologyRegistry)->issueForPreproduction($world->id, 'entry-room');

        foreach ([$receipt->worldId, $receipt->channelId] as $id) {
            self::assertTrue(Uuid::isValid($id));
            $decoded = Uuid::fromString($id);
            self::assertSame(7, $decoded->getVersion());
            self::assertSame(16, strlen($decoded->getBytes()));
            self::assertSame(strtolower($id), $id);
        }
        self::assertNotSame($receipt->worldId, $receipt->channelId);
        self::assertSame($before, serialize((new DatabaseWorldRegistry)->forAccount(1001)));
        $attributes['world_id'] = $receipt->worldId;
        self::assertSame($attributes, $world->fresh()?->getAttributes());
        self::assertSame($world->id, GameChannel::query()->sole()->game_world_id);
    }

    public function test_cli_uses_real_provisioning_and_emits_only_committed_scoped_identity_readback(): void
    {
        self::assertSame(0, Artisan::call('game-auth:world:ensure', [
            '--id' => '17',
            '--slug' => 'disposable-entry',
            '--name' => 'Disposable entry',
            '--region' => 'TEST',
            '--host' => '127.0.0.1',
            '--port' => '7172',
            '--status' => 'maintenance',
            '--login-enabled' => '0',
        ]));
        $options = ['--world-row-id' => '17', '--channel-key' => 'entry-room'];
        self::assertSame(0, Artisan::call('game-auth:native-topology:issue', $options));
        $first = json_decode(Artisan::output(), true, 512, JSON_THROW_ON_ERROR);
        self::assertIsArray($first);
        $retained = (new NativeTopologyRegistry)->readbackForPreproduction(17, 'entry-room');
        self::assertSame($retained->toArray(), $first);
        self::assertSame([
            'version', 'purpose', 'issuer', 'world_id', 'channel_id',
        ], array_keys($first));
        self::assertSame('disposable-preproduction-native-topology', $first['purpose']);
        self::assertSame('oteryn-platform-world-registry', $first['issuer']);

        self::assertSame(0, Artisan::call('game-auth:native-topology:issue', $options));
        self::assertSame($first, json_decode(Artisan::output(), true, 512, JSON_THROW_ON_ERROR));
        self::assertSame(1, GameChannel::query()->count());
        self::assertSame([], (new DatabaseWorldRegistry)->forAccount(1001));
        self::assertFalse(GameWorld::query()->findOrFail(17)->login_enabled);
    }

    public function test_replay_fresh_owner_and_metadata_changes_preserve_both_identifiers(): void
    {
        $world = $this->world('retained');
        $issued = (new NativeTopologyRegistry)->issueForPreproduction($world->id, 'entry-room');
        $world->fill(['name' => 'Renamed', 'game_host' => 'replacement.test', 'game_port' => 7173])->save();

        self::assertEquals($issued, (new NativeTopologyRegistry)->readbackForPreproduction($world->id, 'entry-room'));
        // Discarding the first response and replaying cannot issue a new pair.
        self::assertEquals($issued, (new NativeTopologyRegistry)->issueForPreproduction($world->id, 'entry-room'));
        self::assertSame(1, GameChannel::query()->count());
    }

    public function test_channel_readback_preserves_world_scope_and_never_reassociates_another_world(): void
    {
        $worldA = $this->world('scope-a');
        $worldB = $this->world('scope-b');
        $registry = new NativeTopologyRegistry;
        $pairA = $registry->issueForPreproduction($worldA->id, 'only-a');
        $this->assertRefused(fn () => $registry->readbackForPreproduction($worldB->id, 'only-a'));
        $pairB = $registry->issueForPreproduction($worldB->id, 'entry-room');
        self::assertNotSame($pairA->worldId, $pairB->worldId);
        self::assertNotSame($pairA->channelId, $pairB->channelId);
        $channelA = GameChannel::query()->where('channel_id', $pairA->channelId)->sole();
        $this->assertRefused(fn () => $channelA->forceFill(['game_world_id' => $worldB->id])->save());
        self::assertSame($worldA->id, $channelA->fresh()?->game_world_id);
        self::assertEquals($pairA, $registry->readbackForPreproduction($worldA->id, 'only-a'));
    }

    public function test_ordinary_models_cannot_mint_replace_clear_reassociate_or_delete_issued_identity(): void
    {
        $world = $this->world('immutable');
        $pair = (new NativeTopologyRegistry)->issueForPreproduction($world->id, 'entry-room');
        $channel = GameChannel::query()->sole();

        foreach ([null, (string) Str::uuid7()] as $replacement) {
            $this->assertRefused(fn () => GameWorld::query()->findOrFail($world->id)->forceFill(['world_id' => $replacement])->save());
        }
        foreach ([
            ['channel_id' => (string) Str::uuid7()],
            ['channel_key' => 'new-logical-channel'],
        ] as $changed) {
            $this->assertRefused(fn () => GameChannel::query()->findOrFail($channel->id)->forceFill($changed)->save());
        }
        $this->assertRefused(fn () => GameWorld::query()->findOrFail($world->id)->delete());
        $this->assertRefused(fn () => GameChannel::query()->findOrFail($channel->id)->delete());
        $this->assertRefused(fn () => (new GameWorld)->forceFill([
            'world_id' => (string) Str::uuid7(), 'slug' => 'caller-created',
        ])->save());
        $this->assertRefused(fn () => (new GameChannel)->forceFill([
            'game_world_id' => $world->id, 'channel_id' => (string) Str::uuid7(), 'channel_key' => 'caller-created',
        ])->save());
        self::assertEquals($pair, (new NativeTopologyRegistry)->readbackForPreproduction($world->id, 'entry-room'));
    }

    public function test_stale_preissuance_model_cannot_erase_current_identity_including_world_only_retention(): void
    {
        $world = $this->world('stale');
        $stale = GameWorld::query()->findOrFail($world->id);
        $staleClear = GameWorld::query()->findOrFail($world->id);
        self::assertNull($stale->world_id);
        $pair = (new NativeTopologyRegistry)->issueForPreproduction($world->id, 'entry-room');
        $this->assertRefused(fn () => $stale->forceFill(['world_id' => (string) Str::uuid7()])->save());

        // Clearing a value that was null in the stale object is a no-op, not
        // a dirty SQL assignment. The current DB value must remain issued.
        $staleClear->forceFill(['world_id' => null])->save();
        self::assertSame($pair->worldId, GameWorld::query()->findOrFail($world->id)->world_id);
        $this->assertRefused(fn () => $staleClear->delete());

        // Raw fixture disposal isolates the World-only retention invariant;
        // this is not an available product Channel deletion route.
        DB::table('game_channels')->delete();
        $this->assertRefused(fn () => $staleClear->delete());
        self::assertSame($pair->worldId, GameWorld::query()->findOrFail($world->id)->world_id);
    }

    public function test_failure_after_world_assignment_rolls_back_before_any_receipt_or_channel_escapes(): void
    {
        $world = $this->world('rollback');
        Str::createUuidsUsingSequence([Uuid::uuid7(), Uuid::uuid4()]);
        try {
            $this->assertRefused(fn () => (new NativeTopologyRegistry)->issueForPreproduction($world->id, 'entry-room'));
        } finally {
            Str::createUuidsNormally();
        }
        self::assertNull(GameWorld::query()->findOrFail($world->id)->world_id);
        self::assertSame(0, GameChannel::query()->count());
        $this->assertRefused(fn () => (new NativeTopologyRegistry)->readbackForPreproduction($world->id, 'entry-room'));
    }

    public function test_outer_transaction_never_returns_uncommitted_issuance_or_readback(): void
    {
        $world = $this->world('outer');
        $registry = new NativeTopologyRegistry;
        DB::beginTransaction();
        try {
            $this->assertRefused(fn () => $registry->issueForPreproduction($world->id, 'entry-room'));
            self::assertNull(GameWorld::query()->findOrFail($world->id)->world_id);
            self::assertSame(0, GameChannel::query()->count());
        } finally {
            DB::rollBack();
        }
        $pair = $registry->issueForPreproduction($world->id, 'entry-room');
        DB::beginTransaction();
        try {
            $this->assertRefused(fn () => $registry->readbackForPreproduction($world->id, 'entry-room'));
        } finally {
            DB::rollBack();
        }
        self::assertEquals($pair, $registry->readbackForPreproduction($world->id, 'entry-room'));
    }

    public function test_exact_inputs_and_retained_ids_reject_trailing_lf_cr_space_nil_and_wrong_version(): void
    {
        $world = $this->world('strict');
        $registry = new NativeTopologyRegistry;
        foreach (["entry-room\n", "entry-room\r", 'entry-room ', '', str_repeat('a', 65)] as $key) {
            $this->assertRefused(fn () => $registry->issueForPreproduction($world->id, $key));
        }
        self::assertNull(GameWorld::query()->findOrFail($world->id)->world_id);
        self::assertSame(0, GameChannel::query()->count());
        foreach (['17'."\n", '17'."\r", '17 ', '0', (string) Str::uuid7(), (string) PHP_INT_MAX.'0'] as $rowId) {
            self::assertSame(1, Artisan::call('game-auth:native-topology:issue', [
                '--world-row-id' => $rowId, '--channel-key' => 'entry-room',
            ]));
        }
        $pair = $registry->issueForPreproduction($world->id, 'entry-room');
        foreach ([
            $pair->worldId."\n", $pair->worldId."\r", $pair->worldId.' ',
            '00000000-0000-0000-0000-000000000000', (string) Str::uuid(),
            '01890F4E-7C00-7000-8000-000000000001',
        ] as $invalid) {
            DB::table('game_worlds')->where('id', $world->id)->update(['world_id' => $invalid]);
            $this->assertRefused(fn () => $registry->readbackForPreproduction($world->id, 'entry-room'));
            $this->assertRefused(fn () => $registry->issueForPreproduction($world->id, 'entry-room'));
        }
        DB::table('game_worlds')->where('id', $world->id)->update(['world_id' => $pair->worldId]);
        foreach ([$pair->channelId."\n", $pair->channelId."\r", $pair->channelId.' '] as $invalid) {
            DB::table('game_channels')->update(['channel_id' => $invalid]);
            $this->assertRefused(fn () => $registry->readbackForPreproduction($world->id, 'entry-room'));
            $this->assertRefused(fn () => $registry->issueForPreproduction($world->id, 'entry-room'));
        }
        DB::table('game_channels')->update(['channel_id' => $pair->channelId]);
        self::assertEquals($pair, $registry->readbackForPreproduction($world->id, 'entry-room'));
    }

    public function test_missing_world_production_environment_and_unsafe_database_profile_fail_without_issuance(): void
    {
        $world = $this->world('guard');
        $registry = new NativeTopologyRegistry;
        $this->assertRefused(fn () => $registry->issueForPreproduction($world->id + 100, 'entry-room'));
        $this->app->detectEnvironment(fn (): string => 'production');
        try {
            $this->assertRefused(fn () => $registry->issueForPreproduction($world->id, 'entry-room'));
            $this->assertRefused(fn () => $registry->readbackForPreproduction($world->id, 'entry-room'));
            self::assertSame(1, Artisan::call('game-auth:native-topology:issue', [
                '--world-row-id' => (string) $world->id, '--channel-key' => 'entry-room',
            ]));
        } finally {
            $this->app->detectEnvironment(fn (): string => 'testing');
        }
        $connection = DB::connection();
        $database = $connection->getDatabaseName();
        $connection->setDatabaseName('/not-a-disposable-fixture/production.sqlite');
        try {
            $this->assertRefused(fn () => $registry->issueForPreproduction($world->id, 'entry-room'));
            $this->assertRefused(fn () => $registry->readbackForPreproduction($world->id, 'entry-room'));
        } finally {
            $connection->setDatabaseName($database);
        }
        self::assertNull(GameWorld::query()->findOrFail($world->id)->world_id);
        self::assertSame(0, GameChannel::query()->count());
    }

    public function test_controlled_regular_file_reconnect_retains_issuer_readback_and_symlink_profile_is_refused(): void
    {
        self::assertSame('sqlite', DB::connection()->getDriverName());
        $original = config('database.connections.sqlite.database');
        self::assertIsString($original);
        $temporaryRoot = realpath(sys_get_temp_dir());
        self::assertIsString($temporaryRoot);
        $directory = $temporaryRoot.'/oteryn-native-topology-'.bin2hex(random_bytes(8));
        $linkedDirectory = $temporaryRoot.'/oteryn-native-topology-'.bin2hex(random_bytes(8));
        self::assertTrue(mkdir($directory, 0700));
        $database = $directory.'/oteryn-native-topology.sqlite';
        self::assertNotFalse(file_put_contents($database, ''));

        try {
            config(['database.connections.sqlite.database' => $database]);
            DB::purge();
            self::assertSame(0, Artisan::call('migrate:fresh', ['--force' => true]));
            $world = $this->world('persistent');
            $this->app->detectEnvironment(fn (): string => 'preproduction');
            $pair = (new NativeTopologyRegistry)->issueForPreproduction($world->id, 'entry-room');
            DB::purge();
            self::assertEquals($pair, (new NativeTopologyRegistry)->readbackForPreproduction($world->id, 'entry-room'));
            self::assertEquals($pair, (new NativeTopologyRegistry)->issueForPreproduction($world->id, 'entry-room'));

            self::assertTrue(symlink($directory, $linkedDirectory));
            config(['database.connections.sqlite.database' => $linkedDirectory.'/oteryn-native-topology.sqlite']);
            DB::purge();
            $this->assertRefused(fn () => (new NativeTopologyRegistry)->readbackForPreproduction($world->id, 'entry-room'));
            $this->assertRefused(fn () => (new NativeTopologyRegistry)->issueForPreproduction($world->id, 'entry-room'));
            unlink($linkedDirectory);
            self::assertTrue(mkdir($linkedDirectory, 0700));
            $linkedDatabase = $linkedDirectory.'/oteryn-native-topology.sqlite';
            self::assertTrue(symlink($database, $linkedDatabase));
            DB::purge();
            $this->assertRefused(fn () => (new NativeTopologyRegistry)->readbackForPreproduction($world->id, 'entry-room'));
            $this->assertRefused(fn () => (new NativeTopologyRegistry)->issueForPreproduction($world->id, 'entry-room'));
            unlink($linkedDatabase);
            self::assertTrue(mkdir($linkedDatabase, 0700));
            DB::purge();
            $this->assertRefused(fn () => (new NativeTopologyRegistry)->readbackForPreproduction($world->id, 'entry-room'));
        } finally {
            DB::purge();
            $this->app->detectEnvironment(fn (): string => 'testing');
            config(['database.connections.sqlite.database' => $original]);
            self::assertSame(0, Artisan::call('migrate:fresh', ['--force' => true]));
            if (is_link($linkedDirectory)) {
                unlink($linkedDirectory);
            } elseif (is_dir($linkedDirectory)) {
                foreach (glob($linkedDirectory.'/*') ?: [] as $path) {
                    if (is_dir($path) && ! is_link($path)) {
                        rmdir($path);
                    } else {
                        unlink($path);
                    }
                }
                rmdir($linkedDirectory);
            }
            foreach (glob($directory.'/*') ?: [] as $path) {
                unlink($path);
            }
            rmdir($directory);
        }
    }

    private function world(string $slug, bool $online = false): GameWorld
    {
        return GameWorld::query()->create([
            'slug' => $slug,
            'name' => 'Disposable '.$slug,
            'region' => 'TEST',
            'status' => $online ? GameWorldStatus::Online : GameWorldStatus::Maintenance,
            'login_enabled' => $online,
            'game_host' => '127.0.0.1',
            'game_port' => 7172,
        ])->refresh();
    }

    /** @param callable():mixed $operation */
    private function assertRefused(callable $operation): void
    {
        try {
            $operation();
            self::fail('The native topology operation must fail closed.');
        } catch (LogicException $exception) {
            self::assertNotSame('', $exception->getMessage());
        }
    }
}
