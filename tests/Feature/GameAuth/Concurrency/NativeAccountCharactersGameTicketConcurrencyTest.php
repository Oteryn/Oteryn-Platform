<?php

namespace Tests\Feature\GameAuth\Concurrency;

use App\GameAuth\NativeAccountCharacters\NativeAccountCharactersAccountView;
use App\GameAuth\NativeAccountCharacters\NativeAccountCharactersIngestion;
use App\GameAuth\NativeAccountCharacters\NativeAccountCharactersReadModel;
use App\GameAuth\NativeAccountCharacters\NativeAccountCharactersRefused;
use App\GameAuth\NativeAccountCharacters\NativeAccountCharactersSettings;
use App\GameAuth\NativeAccountCharacters\NativeAccountCharactersSnapshot;
use App\GameAuth\NativeAccountCharacters\NativeAccountCharacterSummary;
use App\GameAuth\NativeAccountCharacters\NativeAccountCharactersWatermark;
use Carbon\CarbonImmutable;
use Illuminate\Database\Events\QueryExecuted;
use Illuminate\Database\QueryException;
use Illuminate\Foundation\Testing\DatabaseMigrations;
use Illuminate\Support\Facades\DB;
use Tests\TestCase;
use Throwable;

/**
 * LCFA ingestion and the issuer's locking read on real InnoDB with independent processes (login
 * contract §2.3). The first operation pauses while it holds the projection-state lock, so the second
 * deterministically waits for it: concurrent publications of one (epoch, revision) pair either agree or
 * invalidate the account, an epoch raise supersedes an in-flight lower-epoch snapshot, and the issuer
 * never reads a half-replaced character set. The class name registers it in the existing
 * `--filter=GameTicketConcurrencyTest` MariaDB step.
 */
final class NativeAccountCharactersGameTicketConcurrencyTest extends TestCase
{
    use DatabaseMigrations;

    private const IDENTITY = 'CN=character-authority-projection';

    private const AUTHORITY = 'oteryn:character-authority:primary';

    private const ACCOUNT = '0190f2a1-3b4c-7d5e-8f60-718293a4b5c6';

    private const WORLD = '01934f10-7c02-7001-805b-3b1122334401';

    private const CHARACTER = '01934f10-7c04-7001-805b-3b1122334401';

    private const OTHER_CHARACTER = '01934f10-7c04-7001-805b-3b1122334402';

    private string $directory;

    protected function setUp(): void
    {
        parent::setUp();

        if (getenv('GAME_AUTH_CONCURRENCY_TEST') !== '1' || ! function_exists('pcntl_fork')) {
            $this->markTestSkipped('Requires the dedicated MariaDB concurrency workflow with pcntl.');
        }

        $this->directory = sys_get_temp_dir().'/oteryn-native-account-characters-'.bin2hex(random_bytes(8));
        self::assertTrue(mkdir($this->directory, 0700, true));
        $this->travelTo(CarbonImmutable::createFromTimestamp(1_790_000_020));
        config([
            'game-auth.native_account_characters.enabled' => true,
            'game-auth.native_account_characters.identities' => [self::IDENTITY],
            'game-auth.native_account_characters.source_authority' => self::AUTHORITY,
            'game-auth.native_account_characters.freshness_seconds' => 30,
            'game-auth.native_account_characters.clock_uncertainty_seconds' => 1,
            'game-auth.native_account_characters.requests_per_minute' => 600,
        ]);
        self::assertSame('accepted', $this->watermark('1'));
    }

    protected function tearDown(): void
    {
        if (isset($this->directory)) {
            // The read-model migration refuses rollback while rows exist.
            DB::table('native_account_character_rows')->delete();
            DB::table('native_account_character_snapshots')->delete();
            array_map('unlink', glob($this->directory.'/*') ?: []);
            @rmdir($this->directory);
        }

        parent::tearDown();
    }

    public function test_concurrent_identical_publications_of_one_pair_are_both_accepted(): void
    {
        $results = $this->race(
            fn (): string => $this->pausingAfterStateLock(fn (): string => $this->snapshot('1', '7', self::CHARACTER)),
            fn (): string => $this->afterStateLocked(fn (): string => $this->snapshot('1', '7', self::CHARACTER)),
        );

        self::assertSame(['accepted', 'accepted'], $results);
        self::assertFalse((bool) DB::table('native_account_character_snapshots')->where('account_id', self::ACCOUNT)->value('invalid'));
        self::assertSame(1, DB::table('native_account_character_rows')->where('account_id', self::ACCOUNT)->count());
    }

    public function test_concurrent_different_publications_of_one_pair_invalidate_the_account(): void
    {
        $results = $this->race(
            fn (): string => $this->pausingAfterStateLock(fn (): string => $this->snapshot('1', '7', self::CHARACTER)),
            fn (): string => $this->afterStateLocked(fn (): string => $this->snapshot('1', '7', self::OTHER_CHARACTER)),
        );

        self::assertSame(['accepted', 'refused:409'], $results);
        self::assertTrue((bool) DB::table('native_account_character_snapshots')->where('account_id', self::ACCOUNT)->value('invalid'));
        self::assertSame(NativeAccountCharactersAccountView::INVALID, $this->accountView()->state);
    }

    public function test_a_lower_epoch_snapshot_waits_for_an_in_flight_epoch_raise_and_is_superseded(): void
    {
        $results = $this->race(
            fn (): string => $this->pausingAfterStateLock(fn (): string => $this->watermark('2')),
            fn (): string => $this->afterStateLocked(fn (): string => $this->snapshot('1', '7', self::CHARACTER)),
        );

        self::assertSame(['accepted', 'superseded'], $results);
        self::assertFalse(DB::table('native_account_character_snapshots')->exists());
        self::assertSame('2', DB::table('native_account_character_projection_state')->value('highest_epoch'));
    }

    public function test_the_issuer_locking_read_waits_for_an_in_flight_snapshot_and_sees_it_whole(): void
    {
        $this->requireSnapshotIsolationSetting();
        self::assertSame('accepted', $this->snapshot('1', '7', self::CHARACTER));

        $results = $this->race(
            fn (): string => $this->pausingAfterStateLock(fn (): string => $this->snapshot('1', '8', self::OTHER_CHARACTER)),
            fn (): string => $this->issuerRead(snapshotIsolation: false),
        );

        self::assertSame(['accepted', NativeAccountCharactersAccountView::READY.':'.self::OTHER_CHARACTER], $results);
    }

    public function test_under_mariadb_snapshot_isolation_the_issuer_read_racing_a_snapshot_fails_closed(): void
    {
        $this->requireSnapshotIsolationSetting();
        self::assertSame('accepted', $this->snapshot('1', '7', self::CHARACTER));

        $results = $this->race(
            fn (): string => $this->pausingAfterStateLock(fn (): string => $this->snapshot('1', '8', self::OTHER_CHARACTER)),
            fn (): string => $this->issuerRead(snapshotIsolation: true),
        );

        // ER_CHECKREAD rolls the issuer transaction back; issuance maps it to NATIVE_LOGIN_UNAVAILABLE.
        self::assertSame(['accepted', 'query-error:1020'], $results);
    }

    /**
     * The issuer's character read inside a transaction whose read view is already open, as it is after
     * the issuer's earlier plain reads. With snapshot isolation off (MySQL, or MariaDB configured so),
     * only the locking reads keep the snapshot and its rows consistent.
     */
    private function issuerRead(bool $snapshotIsolation): string
    {
        DB::statement('SET SESSION innodb_snapshot_isolation = '.($snapshotIsolation ? 'ON' : 'OFF'));
        try {
            return DB::transaction(function (): string {
                DB::table('native_account_character_rows')->count();

                return $this->afterStateLocked(function (): string {
                    $view = $this->accountView(lock: true);

                    return $view->state.':'.implode(',', array_map(
                        fn (NativeAccountCharacterSummary $character): string => $character->characterId,
                        $view->characters,
                    ));
                });
            });
        } catch (QueryException $exception) {
            return 'query-error:'.(is_scalar($exception->errorInfo[1] ?? null) ? (string) $exception->errorInfo[1] : 'unknown');
        }
    }

    private function requireSnapshotIsolationSetting(): void
    {
        $row = DB::selectOne("SHOW VARIABLES LIKE 'innodb_snapshot_isolation'");
        if ($row === null) {
            $this->markTestSkipped('Requires a MariaDB server with innodb_snapshot_isolation.');
        }
    }

    private function accountView(bool $lock = false): NativeAccountCharactersAccountView
    {
        return $this->app->make(NativeAccountCharactersReadModel::class)->viewForAccount(self::ACCOUNT, now()->getTimestamp(), $lock);
    }

    private function snapshot(string $epoch, string $revision, string $characterId): string
    {
        $wire = json_encode([
            'contract_version' => 1,
            'operation' => 'PublishAccountCharactersV1',
            'source_authority' => self::AUTHORITY,
            'account_id' => self::ACCOUNT,
            'projection_epoch' => $epoch,
            'projection_revision' => $revision,
            'source_observed_at' => '1790000018',
            'characters' => [[
                'character_id' => $characterId,
                'world_id' => self::WORLD,
                'name' => 'Aldric',
                'availability' => 'AVAILABLE',
            ]],
        ], JSON_THROW_ON_ERROR | JSON_UNESCAPED_SLASHES);

        return $this->refusal(fn (): string => $this->app->make(NativeAccountCharactersIngestion::class)
            ->snapshot($this->settings(), self::IDENTITY, NativeAccountCharactersSnapshot::fromWire($wire), now()->getTimestamp()));
    }

    private function watermark(string $epoch): string
    {
        $wire = json_encode([
            'contract_version' => 1,
            'operation' => 'PublishProjectionWatermarkV1',
            'source_authority' => self::AUTHORITY,
            'projection_epoch' => $epoch,
            'complete_through' => '1790000015',
            'observed_at' => '1790000019',
        ], JSON_THROW_ON_ERROR | JSON_UNESCAPED_SLASHES);

        return $this->refusal(fn (): string => $this->app->make(NativeAccountCharactersIngestion::class)
            ->watermark($this->settings(), self::IDENTITY, NativeAccountCharactersWatermark::fromWire($wire), now()->getTimestamp()));
    }

    private function settings(): NativeAccountCharactersSettings
    {
        $settings = NativeAccountCharactersSettings::current();
        self::assertNotNull($settings);

        return $settings;
    }

    /** @param callable(): string $operation */
    private function refusal(callable $operation): string
    {
        try {
            return $operation();
        } catch (NativeAccountCharactersRefused $refused) {
            return 'refused:'.$refused->status;
        }
    }

    /**
     * Holds the projection-state lock for 700 ms after taking it, inside the operation's transaction.
     *
     * @param  callable(): string  $operation
     */
    private function pausingAfterStateLock(callable $operation): string
    {
        DB::listen(function (QueryExecuted $query): void {
            if (str_contains($query->sql, 'native_account_character_projection_state') && ! file_exists($this->directory.'/state-locked')) {
                file_put_contents($this->directory.'/state-locked', '1');
                usleep(700_000);
            }
        });

        return $operation();
    }

    /** @param callable(): string $operation */
    private function afterStateLocked(callable $operation): string
    {
        $deadline = microtime(true) + 10;
        while (! file_exists($this->directory.'/state-locked')) {
            if (microtime(true) > $deadline) {
                return 'error:state-not-locked';
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
