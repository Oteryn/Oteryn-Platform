<?php

namespace Tests\Feature\GameAuth\NativeTopology;

use App\GameAuth\Worlds\GameChannel;
use App\GameAuth\Worlds\GameWorld;
use App\GameAuth\Worlds\GameWorldStatus;
use App\GameAuth\Worlds\NativeTopologyReceipt;
use App\GameAuth\Worlds\NativeTopologyRegistry;
use Illuminate\Foundation\Testing\DatabaseMigrations;
use Illuminate\Support\Facades\DB;
use Illuminate\Support\Facades\Schema;
use Tests\TestCase;
use Throwable;

final class NativeTopologyConcurrencyTest extends TestCase
{
    use DatabaseMigrations;

    protected function setUp(): void
    {
        parent::setUp();
        if (getenv('GAME_AUTH_CONCURRENCY_TEST') !== '1'
            || ! function_exists('pcntl_fork')
            || DB::connection()->getDriverName() !== 'mysql') {
            $this->markTestSkipped('Requires the registered disposable MariaDB GameAuth concurrency environment with pcntl.');
        }
    }

    protected function tearDown(): void
    {
        if ($this->app !== null && Schema::hasTable('game_channels')) {
            // Test database disposal is separate from supported product rollback.
            DB::table('game_channels')->delete();
            DB::table('game_worlds')->update(['world_id' => null]);
        }
        parent::tearDown();
    }

    public function test_two_actual_waiters_replay_one_committed_world_and_channel_identity(): void
    {
        $world = $this->world('same-pair');
        $results = $this->race([
            ['world_row_id' => $world->id, 'channel_key' => 'entry-room'],
            ['world_row_id' => $world->id, 'channel_key' => 'entry-room'],
        ]);
        self::assertSame($results[0], $results[1]);
        self::assertSame(1, DB::table('game_channels')->count());
        $readback = (new NativeTopologyRegistry)->readbackForPreproduction($world->id, 'entry-room');
        self::assertSame($readback->toArray(), $this->decoded($results[0]));
        self::assertSame($readback->worldId, DB::table('game_worlds')->where('id', $world->id)->value('world_id'));
    }

    public function test_concurrent_distinct_channels_share_one_world_without_merging_channel_identity(): void
    {
        $world = $this->world('two-channels');
        $results = $this->race([
            ['world_row_id' => $world->id, 'channel_key' => 'entry-a'],
            ['world_row_id' => $world->id, 'channel_key' => 'entry-b'],
        ]);
        $a = $this->decoded($results[0]);
        $b = $this->decoded($results[1]);
        self::assertSame($a['world_id'], $b['world_id']);
        self::assertNotSame($a['channel_id'], $b['channel_id']);
        self::assertSame(2, DB::table('game_channels')->where('game_world_id', $world->id)->count());
        self::assertSame($a, (new NativeTopologyRegistry)->readbackForPreproduction($world->id, 'entry-a')->toArray());
        self::assertSame($b, (new NativeTopologyRegistry)->readbackForPreproduction($world->id, 'entry-b')->toArray());
    }

    public function test_concurrent_different_worlds_never_share_channel_identity_or_owner_binding(): void
    {
        $worldA = $this->world('world-a');
        $worldB = $this->world('world-b');
        $results = $this->race([
            ['world_row_id' => $worldA->id, 'channel_key' => 'entry-room'],
            ['world_row_id' => $worldB->id, 'channel_key' => 'entry-room'],
        ]);
        $a = $this->decoded($results[0]);
        $b = $this->decoded($results[1]);
        self::assertNotSame($a['world_id'], $b['world_id']);
        self::assertNotSame($a['channel_id'], $b['channel_id']);
        self::assertSame($a, (new NativeTopologyRegistry)->readbackForPreproduction($worldA->id, 'entry-room')->toArray());
        self::assertSame($b, (new NativeTopologyRegistry)->readbackForPreproduction($worldB->id, 'entry-room')->toArray());
        self::assertSame($worldA->id, GameChannel::query()->where('channel_id', $a['channel_id'])->sole()->game_world_id);
        self::assertSame($worldB->id, GameChannel::query()->where('channel_id', $b['channel_id'])->sole()->game_world_id);
    }

    /**
     * @param  list<array{world_row_id:int,channel_key:string}>  $bindings
     * @return list<string>
     */
    private function race(array $bindings): array
    {
        $directory = sys_get_temp_dir().'/oteryn-native-topology-race-'.bin2hex(random_bytes(8));
        self::assertTrue(mkdir($directory, 0700));
        $children = [];
        try {
            try {
                foreach ($bindings as $index => $binding) {
                    $pid = pcntl_fork();
                    if ($pid === -1) {
                        self::fail('Unable to fork native topology test process.');
                    }
                    if ($pid === 0) {
                        DB::disconnect();
                        DB::purge();
                        try {
                            $backend = DB::selectOne('SELECT CONNECTION_ID() AS id');
                            if ($backend === null || (! is_int($backend->id) && ! is_string($backend->id))) {
                                throw new \LogicException('No child database backend.');
                            }
                            file_put_contents($directory.'/ready-'.$index, (string) $backend->id);
                            $deadline = microtime(true) + 10;
                            while (! file_exists($directory.'/start') && microtime(true) < $deadline) {
                                usleep(1000);
                            }
                            if (! file_exists($directory.'/start')) {
                                throw new \LogicException('Native topology test start barrier expired.');
                            }
                            $connection = DB::connection();
                            $currentBackend = DB::selectOne('SELECT CONNECTION_ID() AS id');
                            $short = static fn (mixed $value): ?string => is_string($value) ? substr($value, 0, 128) : null;
                            file_put_contents($directory.'/profile-'.$index, json_encode([
                                'backend_id' => $currentBackend?->id,
                                'driver' => $connection->getDriverName(),
                                'database' => substr($connection->getDatabaseName(), 0, 128),
                                'host' => $short($connection->getConfig('host')),
                                'socket' => $short($connection->getConfig('unix_socket')),
                                'environment' => $short(app('env')),
                                'transaction_level' => $connection->transactionLevel(),
                            ], JSON_THROW_ON_ERROR));
                            $result = json_encode(
                                (new NativeTopologyRegistry)->issueForPreproduction($binding['world_row_id'], $binding['channel_key'])->toArray(),
                                JSON_THROW_ON_ERROR,
                            );
                        } catch (Throwable $exception) {
                            $result = 'error:'.get_class($exception).':'.substr($exception->getMessage(), 0, 512);
                        }
                        file_put_contents($directory.'/result-'.$index, $result);
                        exit(0);
                    }
                    $children[] = $pid;
                }

                $deadline = microtime(true) + 10;
                while ((! file_exists($directory.'/ready-0') || ! file_exists($directory.'/ready-1')) && microtime(true) < $deadline) {
                    usleep(1000);
                }
                self::assertFileExists($directory.'/ready-0');
                self::assertFileExists($directory.'/ready-1');
                $backendIds = [
                    (int) file_get_contents($directory.'/ready-0'),
                    (int) file_get_contents($directory.'/ready-1'),
                ];
                self::assertGreaterThan(0, $backendIds[0]);
                self::assertGreaterThan(0, $backendIds[1]);
                self::assertNotSame($backendIds[0], $backendIds[1]);

                // Open the blocker only after both forked children have discarded
                // inherited PDOs, so closing them cannot release this transaction.
                DB::purge();
                DB::beginTransaction();
                try {
                    GameWorld::query()->whereIn('id', array_column($bindings, 'world_row_id'))
                        ->orderBy('id')->lockForUpdate()->get();
                    touch($directory.'/start');
                    $waiting = 0;
                    $deadline = microtime(true) + 10;
                    while ($waiting !== 2 && microtime(true) < $deadline) {
                        $row = DB::selectOne(
                            'SELECT COUNT(DISTINCT trx.trx_mysql_thread_id) AS waiting
                             FROM information_schema.INNODB_LOCK_WAITS AS waits
                             JOIN information_schema.INNODB_TRX AS trx ON trx.trx_id = waits.requesting_trx_id
                             WHERE trx.trx_mysql_thread_id IN (?, ?)',
                            $backendIds,
                        );
                        self::assertNotNull($row);
                        $count = $row->waiting;
                        self::assertTrue(is_int($count) || is_string($count));
                        $waiting = (int) $count;
                        if ($waiting !== 2) {
                            // MariaDB 11.8's shared I_S cache refresh needs more
                            // than 100 ms since its last read; every read resets
                            // that timestamp. Polling at 1 ms pins an empty cache.
                            usleep(150000);
                        }
                    }
                    // Preserve the actual two-waiter oracle; diagnostic facts
                    // cannot turn an alternative state into a passing result.
                    self::assertSame(2, $waiting, $waiting === 2 ? '' : $this->waiterDiagnostic($backendIds, $directory));
                    self::assertFileDoesNotExist($directory.'/result-0');
                    self::assertFileDoesNotExist($directory.'/result-1');
                } finally {
                    DB::rollBack();
                }
            } finally {
                touch($directory.'/start');
                $exitStatuses = [];
                foreach ($children as $pid) {
                    $status = 0;
                    pcntl_waitpid($pid, $status);
                    $exitStatuses[] = $status;
                }
                DB::purge();
                foreach ($exitStatuses as $status) {
                    self::assertTrue(pcntl_wifexited($status));
                    self::assertSame(0, pcntl_wexitstatus($status));
                }
            }

            $results = [
                (string) file_get_contents($directory.'/result-0'),
                (string) file_get_contents($directory.'/result-1'),
            ];
            foreach ($results as $result) {
                self::assertFalse(str_starts_with($result, 'error:'), $result);
            }

            return $results;
        } finally {
            // A failed waiter assertion must also dispose its owned fixture.
            foreach (glob($directory.'/*') ?: [] as $path) {
                unlink($path);
            }
            rmdir($directory);
        }
    }

    /** @param list<int> $backendIds */
    private function waiterDiagnostic(array $backendIds, string $directory): string
    {
        $children = [];
        foreach ([0, 1] as $index) {
            foreach (['profile', 'result'] as $kind) {
                $path = $directory.'/'.$kind.'-'.$index;
                $contents = is_file($path) ? file_get_contents($path, false, null, 0, 1024) : false;
                $children[$kind.'-'.$index] = $contents === false ? null : $contents;
            }
        }
        $snapshot = ['expected_backend_ids' => $backendIds, 'children' => $children];
        try {
            $snapshot['blocker'] = DB::selectOne('SELECT CONNECTION_ID() AS id, DATABASE() AS database_name');
            $snapshot['world_engine'] = DB::selectOne(
                "SELECT ENGINE FROM information_schema.TABLES WHERE TABLE_SCHEMA = DATABASE() AND TABLE_NAME = 'game_worlds' LIMIT 1",
            );
            $snapshot['processes'] = DB::select(
                'SELECT ID, COMMAND, STATE FROM information_schema.PROCESSLIST WHERE ID IN (?, ?) LIMIT 2',
                $backendIds,
            );
            $snapshot['transactions'] = DB::select(
                'SELECT trx_mysql_thread_id, trx_state, trx_requested_lock_id
                 FROM information_schema.INNODB_TRX WHERE trx_mysql_thread_id IN (?, ?) LIMIT 2',
                $backendIds,
            );
            $snapshot['waits'] = DB::select(
                'SELECT waits.requesting_trx_id, waits.blocking_trx_id, trx.trx_mysql_thread_id
                 FROM information_schema.INNODB_LOCK_WAITS AS waits
                 JOIN information_schema.INNODB_TRX AS trx ON trx.trx_id = waits.requesting_trx_id
                 WHERE trx.trx_mysql_thread_id IN (?, ?) LIMIT 4',
                $backendIds,
            );
        } catch (Throwable $exception) {
            $snapshot['metadata_error_class'] = get_class($exception);
        }
        $json = json_encode($snapshot, JSON_THROW_ON_ERROR | JSON_INVALID_UTF8_SUBSTITUTE);
        self::assertIsString($json);

        return $json;
    }

    /** @return array{version:int,purpose:string,issuer:string,world_id:string,channel_id:string} */
    private function decoded(string $json): array
    {
        $value = json_decode($json, true, 512, JSON_THROW_ON_ERROR);
        self::assertIsArray($value);
        self::assertSame(1, $value['version'] ?? null);
        self::assertSame('disposable-preproduction-native-topology', $value['purpose'] ?? null);
        self::assertSame('oteryn-platform-world-registry', $value['issuer'] ?? null);
        $worldId = $value['world_id'] ?? null;
        $channelId = $value['channel_id'] ?? null;
        self::assertIsString($worldId);
        self::assertIsString($channelId);

        return (new NativeTopologyReceipt($worldId, $channelId))->toArray();
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
