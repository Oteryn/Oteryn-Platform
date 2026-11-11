<?php

namespace Tests\Feature\GameAuth\Concurrency;

use App\GameAuth\NativeRuntimeStatus\NativeRuntimeStatusIngestion;
use App\GameAuth\NativeRuntimeStatus\NativeRuntimeStatusReadModel;
use App\GameAuth\NativeRuntimeStatus\NativeRuntimeStatusRefused;
use App\GameAuth\NativeRuntimeStatus\NativeRuntimeStatusReport;
use App\GameAuth\NativeRuntimeStatus\NativeRuntimeStatusSettings;
use App\GameAuth\NativeRuntimeStatus\NativeScopeAssignmentIngestion;
use App\GameAuth\NativeRuntimeStatus\NativeScopeAssignmentReport;
use App\GameAuth\NativeRuntimeStatus\NativeScopeAssignmentSettings;
use Illuminate\Database\Events\QueryExecuted;
use Illuminate\Foundation\Testing\DatabaseMigrations;
use Illuminate\Support\Facades\DB;
use Tests\TestCase;
use Throwable;

/**
 * Lock order of native runtime-status ingestion on real InnoDB with independent processes (login
 * contract §7.2 restore reset). The first operation pauses while it holds the epoch lock, so the second
 * deterministically waits for it: a runtime report can never be accepted against an epoch that a
 * concurrent assignment is raising, and an epoch raise waits for an in-flight report without deadlock.
 * The class name registers it in the existing `--filter=GameTicketConcurrencyTest` MariaDB step.
 */
final class NativeRuntimeStatusGameTicketConcurrencyTest extends TestCase
{
    use DatabaseMigrations;

    private const OPS = 'CN=oteryn-game-ops.ownership-authority';

    private const NODE = 'CN=node-a.runtime-status';

    private const NODE_B = 'CN=node-b.runtime-status';

    private const WORLD = '01934f10-7c02-7001-805b-3b1122334401';

    private const CHANNEL = '01934f10-7c03-7001-805b-3b1122334401';

    private const OTHER_CHANNEL = '01934f10-7c03-7001-805b-3b1122334402';

    private string $directory;

    protected function setUp(): void
    {
        parent::setUp();

        if (getenv('GAME_AUTH_CONCURRENCY_TEST') !== '1' || ! function_exists('pcntl_fork')) {
            $this->markTestSkipped('Requires the dedicated MariaDB concurrency workflow with pcntl.');
        }

        $this->directory = sys_get_temp_dir().'/oteryn-native-runtime-status-'.bin2hex(random_bytes(8));
        self::assertTrue(mkdir($this->directory, 0700, true));
        $scopes = [self::WORLD.'/'.self::CHANNEL, self::WORLD.'/'.self::OTHER_CHANNEL];
        config([
            'game-auth.native_runtime_status.enabled' => true,
            'game-auth.native_runtime_status.identities' => [self::NODE => $scopes, self::NODE_B => $scopes],
            'game-auth.native_runtime_status.freshness_seconds' => 15,
            'game-auth.native_runtime_status.clock_uncertainty_seconds' => 1,
            'game-auth.native_runtime_status.requests_per_minute' => 600,
            'game-auth.native_scope_assignment.enabled' => true,
            'game-auth.native_scope_assignment.identities' => [self::OPS => $scopes],
            'game-auth.native_scope_assignment.requests_per_minute' => 60,
        ]);
        self::assertSame('accepted', $this->assign(self::CHANNEL, '1', '3', self::NODE));
    }

    protected function tearDown(): void
    {
        if (isset($this->directory)) {
            // The read-model migrations refuse rollback while rows exist.
            DB::table('native_runtime_status_reports')->delete();
            DB::table('native_scope_assignments')->delete();
            array_map('unlink', glob($this->directory.'/*') ?: []);
            @rmdir($this->directory);
        }

        parent::tearDown();
    }

    public function test_a_runtime_report_waits_for_an_in_flight_epoch_raise_and_is_then_a_conflict(): void
    {
        $results = $this->race(
            fn (): string => $this->pausingAfterEpochLock(fn (): string => $this->assign(self::OTHER_CHANNEL, '2', '1', self::NODE_B)),
            fn (): string => $this->afterEpochLocked(fn (): string => $this->report('1', '3')),
        );

        self::assertSame(['accepted', 'refused:409'], $results);
        self::assertFalse(DB::table('native_runtime_status_reports')->exists());
    }

    public function test_an_epoch_raise_waits_for_an_in_flight_runtime_report_without_deadlock(): void
    {
        $results = $this->race(
            fn (): string => $this->pausingAfterEpochLock(fn (): string => $this->report('1', '3')),
            fn (): string => $this->afterEpochLocked(fn (): string => $this->assign(self::CHANNEL, '2', '1', self::NODE)),
        );

        self::assertSame(['accepted', 'accepted'], $results);
        self::assertSame('1', DB::table('native_runtime_status_reports')->value('assignment_epoch'));
        self::assertSame('2', DB::table('native_scope_assignments')->value('assignment_epoch'));
        $readModel = $this->app->make(NativeRuntimeStatusReadModel::class);
        self::assertSame(NativeRuntimeStatusReadModel::INVALID, $readModel->evidence(self::WORLD, self::CHANNEL, now()->getTimestamp()));
    }

    private function assign(string $channelId, string $epoch, string $generation, string $node): string
    {
        $settings = NativeScopeAssignmentSettings::current();
        $runtime = NativeRuntimeStatusSettings::current();
        self::assertNotNull($settings);
        self::assertNotNull($runtime);

        return $this->refusal(fn (): string => $this->app->make(NativeScopeAssignmentIngestion::class)->ingest($settings, $runtime, self::OPS, NativeScopeAssignmentReport::fromWire(json_encode([
            'contract_version' => 1,
            'operation' => 'ReportScopeAssignmentV1',
            'assignment_epoch' => $epoch,
            'world_id' => self::WORLD,
            'channel_id' => $channelId,
            'ownership_generation' => $generation,
            'node_identity' => $node,
            'assigned_at' => '1789999990',
        ], JSON_THROW_ON_ERROR))));
    }

    private function report(string $epoch, string $generation): string
    {
        $raw = file_get_contents(base_path('tests/Fixtures/GameAuth/native-runtime-status-v1/report.json'));
        self::assertIsString($raw);
        $body = json_decode($raw, true, 2, JSON_THROW_ON_ERROR);
        self::assertIsArray($body);
        $now = now()->getTimestamp();
        $body = array_merge($body, [
            'assignment_epoch' => $epoch,
            'scope_ownership_generation' => $generation,
            'decision_identity' => 'runtime-readiness:0a:'.$generation.':7:true',
            'published_at' => (string) $now,
            'observed_at' => (string) $now,
        ]);
        $settings = NativeRuntimeStatusSettings::current();
        self::assertNotNull($settings);
        $report = NativeRuntimeStatusReport::fromWire(json_encode($body, JSON_THROW_ON_ERROR | JSON_UNESCAPED_SLASHES));

        return $this->refusal(fn (): string => $this->app->make(NativeRuntimeStatusIngestion::class)->ingest($settings, self::NODE, $report, $now));
    }

    /** @param callable(): string $operation */
    private function refusal(callable $operation): string
    {
        try {
            return $operation();
        } catch (NativeRuntimeStatusRefused $refused) {
            return 'refused:'.$refused->status;
        }
    }

    /**
     * Holds the epoch lock for 700 ms after taking it, inside the operation's transaction.
     *
     * @param  callable(): string  $operation
     */
    private function pausingAfterEpochLock(callable $operation): string
    {
        DB::listen(function (QueryExecuted $query): void {
            if (str_contains($query->sql, 'native_assignment_epoch_locks') && ! file_exists($this->directory.'/epoch-locked')) {
                file_put_contents($this->directory.'/epoch-locked', '1');
                usleep(700_000);
            }
        });

        return $operation();
    }

    /** @param callable(): string $operation */
    private function afterEpochLocked(callable $operation): string
    {
        $deadline = microtime(true) + 10;
        while (! file_exists($this->directory.'/epoch-locked')) {
            if (microtime(true) > $deadline) {
                return 'error:epoch-not-locked';
            }
            usleep(1000);
        }

        return $operation();
    }

    /**
     * Runs both operations in forked processes with their own connections, released together.
     *
     * @param  callable(): string  $first
     * @param  callable(): string  $second
     * @return list<string>
     */
    private function race(callable $first, callable $second): array
    {
        $children = [];
        foreach ([$first, $second] as $index => $operation) {
            $pid = pcntl_fork();
            if ($pid === -1) {
                self::fail('Unable to fork concurrency test process.');
            }
            if ($pid === 0) {
                DB::disconnect();
                DB::purge();
                try {
                    $result = $operation();
                } catch (Throwable $exception) {
                    $result = 'error:'.$exception::class;
                }
                file_put_contents($this->directory.'/result-'.$index, $result);
                exit(0);
            }
            $children[] = $pid;
        }

        foreach ($children as $pid) {
            $status = null;
            pcntl_waitpid($pid, $status);
            if (! is_int($status)) {
                self::fail('Child process status was not an integer.');
            }
            self::assertTrue(pcntl_wifexited($status) && pcntl_wexitstatus($status) === 0);
        }

        $results = [];
        foreach ([0, 1] as $index) {
            $result = file_get_contents($this->directory.'/result-'.$index);
            self::assertIsString($result);
            $results[] = $result;
        }
        DB::purge();
        DB::reconnect();

        return $results;
    }
}
