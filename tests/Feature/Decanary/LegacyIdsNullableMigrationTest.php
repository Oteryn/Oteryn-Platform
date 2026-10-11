<?php

namespace Tests\Feature\Decanary;

use App\Identity\Models\Identity;
use App\Marketplace\Models\CharacterAuction;
use Closure;
use Illuminate\Database\Events\QueryExecuted;
use Illuminate\Database\Migrations\Migration;
use Illuminate\Database\UniqueConstraintViolationException;
use Illuminate\Foundation\Testing\DatabaseMigrations;
use Illuminate\Support\Facades\DB;
use Illuminate\Support\Facades\Event;
use Illuminate\Support\Facades\Hash;
use Illuminate\Support\Facades\Schema;
use Illuminate\Support\Str;
use LogicException;
use PHPUnit\Framework\Attributes\DataProvider;
use ReflectionMethod;
use Tests\TestCase;
use Throwable;

final class LegacyIdsNullableMigrationTest extends TestCase
{
    use DatabaseMigrations;

    /** Legacy id columns relaxed to nullable by the migration under test, per table. */
    private const RELAXED = ['character_auctions' => ['seller_canary_account_id', 'escrow_canary_account_id', 'player_id'], 'character_profile_preferences' => ['canary_player_id']];

    private const MIGRATION = 'migrations/2026_10_11_120000_relax_legacy_ids_and_add_native_guards.php';

    private const ACTIVE_GUARD = 'character_auctions_active_character_unique';

    private const PREFERENCE_GUARD = 'character_profile_preferences_owner_character_unique';

    private const CHARACTER = '0192b3c4-5d6e-7f80-8a1b-2c3d4e5f6a7d';

    private const OTHER_CHARACTER = '0192b3c4-5d6e-7f80-8a1b-2c3d4e5f6a7e';

    private const SELLER_ACCOUNT = '0192b3c4-5d6e-7f80-8a1b-2c3d4e5f6a70';

    private const ESCROW_ACCOUNT = '0192b3c4-5d6e-7f80-8a1b-2c3d4e5f6a71';

    protected function tearDown(): void
    {
        // Rows without a legacy id refuse the framework's migrate-down teardown by design; remove this fixture first.
        if ($this->app !== null && Schema::hasTable('character_auctions')) {
            DB::table('character_auction_bids')->delete();
            DB::table('character_auctions')->delete();
            DB::table('character_profile_preferences')->delete();
        }

        parent::tearDown();
    }

    public function test_up_keeps_existing_rows_and_copies_the_native_character_of_active_listings(): void
    {
        $seller = $this->identityId('existing-seller@example.test');
        $bidder = $this->identityId('existing-bidder@example.test');
        $this->applyMigration('down');

        try {
            $active = $this->auction($seller, ['active_player_id' => 103, 'character_id' => self::CHARACTER]);
            $ended = $this->auction($seller, ['status' => CharacterAuction::STATUS_COMPLETED, 'character_id' => self::OTHER_CHARACTER]);
            $legacyOnly = $this->auction($seller, ['active_player_id' => 104]);
            DB::table('character_auction_bids')->insert([
                'request_id' => (string) Str::uuid(),
                'auction_id' => $active,
                'bidder_identity_id' => $bidder,
                'amount' => 150,
                'status' => 'accepted',
                'placed_at' => now(),
            ]);
            $this->preference($seller, ['character_id' => self::CHARACTER]);
            $this->preference($seller, [self::RELAXED['character_profile_preferences'][0] => 202]);
            $before = $this->snapshot();

            $this->applyMigration('up');
        } finally {
            if (! Schema::hasColumn('character_auctions', 'active_character_id')) {
                $this->applyMigration('up');
            }
        }

        $after = $this->snapshot();
        foreach (array_keys($after['character_auctions']) as $id) {
            unset($after['character_auctions'][$id]['active_character_id']);
        }
        self::assertSame($before, $after);
        self::assertSame(1, DB::table('character_auction_bids')->where('auction_id', $active)->count());
        self::assertSame(self::CHARACTER, DB::table('character_auctions')->where('id', $active)->value('active_character_id'));
        self::assertNull(DB::table('character_auctions')->where('id', $ended)->value('active_character_id'));
        self::assertNull(DB::table('character_auctions')->where('id', $legacyOnly)->value('active_character_id'));
        $this->assertRelaxed(true);
        $this->assertNativeGuards(true);
        self::assertTrue(Schema::hasIndex('character_auctions', ['active_player_id'], 'unique'));
        self::assertTrue(Schema::hasIndex('character_profile_preferences', ['identity_id', self::RELAXED['character_profile_preferences'][0]], 'unique'));
    }

    public function test_rows_with_native_ids_only_are_accepted(): void
    {
        $seller = $this->identityId('native-only@example.test');

        $auction = $this->auction($seller, $this->nativeListing(self::CHARACTER, true));
        $preference = $this->preference($seller, [...$this->nulls('character_profile_preferences'), 'character_id' => self::CHARACTER]);

        $listing = (array) DB::table('character_auctions')->where('id', $auction)->first();
        foreach (self::RELAXED['character_auctions'] as $column) {
            self::assertNull($listing[$column]);
        }
        self::assertNull($listing['active_player_id']);
        self::assertSame(self::SELLER_ACCOUNT, $listing['seller_account_id']);
        self::assertSame(self::ESCROW_ACCOUNT, $listing['escrow_account_id']);
        self::assertSame(self::CHARACTER, $listing['active_character_id']);

        $stored = (array) DB::table('character_profile_preferences')->where('id', $preference)->first();
        self::assertNull($stored[self::RELAXED['character_profile_preferences'][0]]);
        self::assertSame(self::CHARACTER, $stored['character_id']);
    }

    public function test_native_character_holds_at_most_one_active_listing(): void
    {
        $seller = $this->identityId('active-guard@example.test');
        $this->auction($seller, $this->nativeListing(self::CHARACTER, true));
        // Ended listings release the guard exactly like active_player_id does.
        $this->auction($seller, $this->nativeListing(self::CHARACTER, false));
        $this->auction($seller, $this->nativeListing(self::CHARACTER, false));
        $this->auction($seller, $this->nativeListing(self::OTHER_CHARACTER, true));

        $this->assertUniqueViolation(fn () => $this->auction($seller, $this->nativeListing(self::CHARACTER, true)));

        self::assertSame(1, DB::table('character_auctions')->where('active_character_id', self::CHARACTER)->count());
        self::assertSame(3, DB::table('character_auctions')->where('character_id', self::CHARACTER)->count());
    }

    public function test_identity_holds_at_most_one_preference_per_native_character(): void
    {
        $owner = $this->identityId('preference-owner@example.test');
        $other = $this->identityId('preference-other@example.test');
        $native = $this->nulls('character_profile_preferences');
        $legacyColumn = self::RELAXED['character_profile_preferences'][0];

        $this->preference($owner, [...$native, 'character_id' => self::CHARACTER]);
        $this->preference($owner, [...$native, 'character_id' => self::OTHER_CHARACTER]);
        $this->preference($other, [...$native, 'character_id' => self::CHARACTER]);
        // Legacy rows without a native id stay guarded by the legacy key only.
        $this->preference($owner, [$legacyColumn => 301]);
        $this->preference($owner, [$legacyColumn => 302]);

        $this->assertUniqueViolation(fn () => $this->preference($owner, [...$native, 'character_id' => self::CHARACTER]));
        $this->assertUniqueViolation(fn () => $this->preference($owner, [$legacyColumn => 303, 'character_id' => self::CHARACTER]));

        self::assertSame(1, DB::table('character_profile_preferences')->where('identity_id', $owner)->where('character_id', self::CHARACTER)->count());
    }

    /** @return iterable<string, array{string, string}> */
    public static function relaxedColumns(): iterable
    {
        foreach (self::RELAXED as $table => $columns) {
            foreach ($columns as $column) {
                yield "{$table}.{$column}" => [$table, $column];
            }
        }
    }

    #[DataProvider('relaxedColumns')]
    public function test_rollback_is_refused_before_ddl_while_a_relaxed_column_is_null(string $table, string $column): void
    {
        $seller = $this->identityId('null-legacy@example.test');
        $row = [...$this->nulls($table), $column => null];
        // Only the column under test is NULL; every other legacy id keeps a value.
        foreach (self::RELAXED[$table] ?? [] as $other) {
            if ($other !== $column) {
                $row[$other] = 501;
            }
        }
        if ($table === 'character_auctions') {
            $this->auction($seller, $row);
        } else {
            $this->preference($seller, $row);
        }
        $indexes = $this->indexes();

        try {
            $this->applyMigration('down');
            self::fail('Rollback must refuse while a relaxed column is NULL.');
        } catch (LogicException $exception) {
            self::assertStringContainsString("{$table}.{$column}", $exception->getMessage());
            self::assertStringContainsString('before DDL', $exception->getMessage());
        }

        $this->assertRelaxed(true);
        $this->assertNativeGuards(true);
        self::assertSame($indexes, $this->indexes());
    }

    public function test_rollback_outside_maintenance_mode_is_refused_before_ddl(): void
    {
        $this->app['env'] = 'local';
        try {
            $this->applyMigration('down');
            self::fail('Rollback must require maintenance mode.');
        } catch (LogicException $exception) {
            self::assertStringContainsString('php artisan down', $exception->getMessage());
        } finally {
            $this->app['env'] = 'testing';
        }

        $this->assertRelaxed(true);
        $this->assertNativeGuards(true);
    }

    public function test_rollback_in_maintenance_mode_restores_legacy_constraints_and_keeps_rows(): void
    {
        $seller = $this->identityId('restore@example.test');
        $this->auction($seller, ['active_player_id' => 103, 'character_id' => self::CHARACTER, 'active_character_id' => self::CHARACTER]);
        $this->preference($seller, ['character_id' => self::CHARACTER]);
        $before = $this->snapshot();

        $this->app['env'] = 'local';
        $this->app->maintenanceMode()->activate([]);
        try {
            $this->applyMigration('down');

            $after = $this->snapshot();
            foreach (array_keys($before['character_auctions']) as $id) {
                unset($before['character_auctions'][$id]['active_character_id']);
            }
            self::assertSame($before, $after);
            $this->assertRelaxed(false);
            self::assertFalse(Schema::hasColumn('character_auctions', 'active_character_id'));
            self::assertFalse(Schema::hasIndex('character_profile_preferences', self::PREFERENCE_GUARD));
            self::assertTrue(Schema::hasColumn('character_auctions', 'character_id'));
            self::assertTrue(Schema::hasColumn('character_profile_preferences', 'character_id'));
        } finally {
            $this->app->maintenanceMode()->deactivate();
            $this->app['env'] = 'testing';
            if (! Schema::hasColumn('character_auctions', 'active_character_id')) {
                $this->applyMigration('up');
            }
        }

        $this->assertRelaxed(true);
        $this->assertNativeGuards(true);
    }

    public function test_up_refuses_duplicate_native_preferences_before_ddl(): void
    {
        $owner = $this->identityId('duplicate-preference@example.test');
        $legacyColumn = self::RELAXED['character_profile_preferences'][0];
        $this->applyMigration('down');

        try {
            $this->preference($owner, [$legacyColumn => 401, 'character_id' => self::CHARACTER]);
            $this->preference($owner, [$legacyColumn => 402, 'character_id' => self::CHARACTER]);

            $this->assertUpRefused();
        } finally {
            DB::table('character_profile_preferences')->delete();
            $this->applyMigration('up');
        }
    }

    public function test_up_refuses_two_active_listings_of_one_native_character_before_ddl(): void
    {
        $seller = $this->identityId('duplicate-listing@example.test');
        $this->applyMigration('down');

        try {
            $this->auction($seller, ['active_player_id' => 103, 'character_id' => self::CHARACTER]);
            $this->auction($seller, ['active_player_id' => 104, 'character_id' => self::CHARACTER]);

            $this->assertUpRefused();
        } finally {
            DB::table('character_auctions')->delete();
            $this->applyMigration('up');
        }
    }

    public function test_mariadb_checks_and_ddl_run_under_one_table_lock(): void
    {
        if (! in_array(DB::connection()->getDriverName(), ['mysql', 'mariadb'], true)) {
            self::markTestSkipped('LOCK TABLES applies to MariaDB/MySQL only.');
        }

        $statements = [];
        $recording = true;
        Event::listen(QueryExecuted::class, function (QueryExecuted $query) use (&$statements, &$recording): void {
            if ($recording) {
                $statements[] = $query->sql;
            }
        });

        try {
            $this->applyMigration('down');
        } finally {
            $recording = false;
            $this->applyMigration('up');
        }

        // Every rollback statement, the NULL checks first, runs between LOCK TABLES and UNLOCK TABLES.
        self::assertSame('LOCK TABLES character_auctions WRITE, character_profile_preferences WRITE', $statements[0] ?? null);
        self::assertSame('UNLOCK TABLES', $statements[count($statements) - 1] ?? null);
        $underLock = array_values(array_slice($statements, 1, -1));
        self::assertCount(4, array_filter($underLock, fn (string $sql): bool => str_starts_with($sql, 'select exists')));
        self::assertSame(array_fill(0, 4, true), array_map(fn (string $sql): bool => str_starts_with($sql, 'select exists'), array_slice($underLock, 0, 4)));
        self::assertNotEmpty(array_filter($underLock, fn (string $sql): bool => str_starts_with($sql, 'alter table')));
    }

    public function test_mariadb_native_guards_hold_under_concurrent_writers(): void
    {
        if (! function_exists('pcntl_fork') || ! in_array(DB::connection()->getDriverName(), ['mysql', 'mariadb'], true)) {
            self::markTestSkipped('Independent-process race requires MariaDB/MySQL and pcntl.');
        }

        $seller = $this->identityId('race@example.test');
        $listing = fn (): int => $this->auction($seller, $this->nativeListing(self::CHARACTER, true));
        $listings = $this->race($listing, $listing);
        sort($listings);

        $preference = fn (): int => $this->preference($seller, [...$this->nulls('character_profile_preferences'), 'character_id' => self::CHARACTER]);
        $preferences = $this->race($preference, $preference);
        sort($preferences);

        self::assertSame(['duplicate', 'inserted'], $listings);
        self::assertSame(['duplicate', 'inserted'], $preferences);
        self::assertSame(1, DB::table('character_auctions')->where('active_character_id', self::CHARACTER)->count());
        self::assertSame(1, DB::table('character_profile_preferences')->where('character_id', self::CHARACTER)->count());
    }

    /**
     * Runs both writers in separate processes and connections, released together.
     *
     * @param  Closure(): int  $first
     * @param  Closure(): int  $second
     * @return list<string>
     */
    private function race(Closure $first, Closure $second): array
    {
        $directory = sys_get_temp_dir().'/oteryn-native-guard-'.bin2hex(random_bytes(8));
        self::assertTrue(mkdir($directory, 0700));
        $children = [];

        foreach ([$first, $second] as $index => $writer) {
            $pid = pcntl_fork();
            if ($pid === -1) {
                self::fail('Unable to fork native guard test process.');
            }
            if ($pid === 0) {
                DB::disconnect();
                DB::purge();
                touch($directory.'/ready-'.$index);
                while (! file_exists($directory.'/start')) {
                    usleep(1000);
                }
                try {
                    $writer();
                    $result = 'inserted';
                } catch (UniqueConstraintViolationException) {
                    $result = 'duplicate';
                } catch (Throwable $exception) {
                    $result = 'error:'.$exception->getMessage();
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
        touch($directory.'/start');
        foreach ($children as $pid) {
            $status = 0;
            pcntl_waitpid($pid, $status);
            self::assertTrue(pcntl_wifexited($status));
            self::assertSame(0, pcntl_wexitstatus($status));
        }

        DB::purge();
        DB::reconnect();
        $results = [
            (string) file_get_contents($directory.'/result-0'),
            (string) file_get_contents($directory.'/result-1'),
        ];
        foreach (glob($directory.'/*') ?: [] as $path) {
            unlink($path);
        }
        rmdir($directory);

        return $results;
    }

    private function assertUpRefused(): void
    {
        try {
            $this->applyMigration('up');
            self::fail('Duplicate native character ids must refuse the migration.');
        } catch (LogicException $exception) {
            self::assertStringContainsString('before DDL', $exception->getMessage());
        }

        $this->assertRelaxed(false);
        self::assertFalse(Schema::hasColumn('character_auctions', 'active_character_id'));
        self::assertFalse(Schema::hasIndex('character_profile_preferences', self::PREFERENCE_GUARD));
    }

    private function assertUniqueViolation(Closure $write): void
    {
        try {
            $write();
            self::fail('The native guard must reject the duplicate row.');
        } catch (UniqueConstraintViolationException) {
            $this->addToAssertionCount(1);
        }
    }

    private function assertRelaxed(bool $nullable): void
    {
        foreach (self::RELAXED as $table => $columns) {
            $definitions = [];
            foreach (Schema::getColumns($table) as $definition) {
                $definitions[$definition['name']] = $definition['nullable'];
            }
            foreach ($columns as $column) {
                self::assertSame($nullable, $definitions[$column] ?? null, "{$table}.{$column}");
            }
        }
    }

    private function assertNativeGuards(bool $present): void
    {
        self::assertSame($present, Schema::hasIndex('character_auctions', self::ACTIVE_GUARD, 'unique'));
        self::assertSame($present, Schema::hasIndex('character_profile_preferences', self::PREFERENCE_GUARD, 'unique'));
        self::assertSame($present, Schema::hasIndex('character_profile_preferences', ['identity_id', 'character_id'], 'unique'));
    }

    /** @return array<string, mixed> */
    private function indexes(): array
    {
        return [
            'character_auctions' => Schema::getIndexes('character_auctions'),
            'character_profile_preferences' => Schema::getIndexes('character_profile_preferences'),
        ];
    }

    /** @return array<string, array<int, array<string, mixed>>> */
    private function snapshot(): array
    {
        $snapshot = [];
        foreach (['character_auctions', 'character_auction_bids', 'character_profile_preferences'] as $table) {
            $snapshot[$table] = [];
            foreach (DB::table($table)->orderBy('id')->get() as $row) {
                $values = (array) $row;
                ksort($values);
                $id = $values['id'] ?? null;
                self::assertIsInt($id);
                $snapshot[$table][$id] = $values;
            }
        }

        return $snapshot;
    }

    /**
     * A listing that carries native ids only; an active one holds its character like active_player_id.
     *
     * @return array<string, mixed>
     */
    private function nativeListing(string $character, bool $active): array
    {
        return [
            ...$this->nulls('character_auctions'),
            'seller_account_id' => self::SELLER_ACCOUNT,
            'escrow_account_id' => self::ESCROW_ACCOUNT,
            'character_id' => $character,
            'active_character_id' => $active ? $character : null,
            'status' => $active ? CharacterAuction::STATUS_ACTIVE : CharacterAuction::STATUS_COMPLETED,
        ];
    }

    /** @return array<string, null> */
    private function nulls(string $table): array
    {
        return array_fill_keys(self::RELAXED[$table] ?? [], null);
    }

    /** @param  array<string, mixed>  $overrides */
    private function auction(int $sellerId, array $overrides = []): int
    {
        return DB::table('character_auctions')->insertGetId([
            'listing_request_id' => (string) Str::uuid(),
            'seller_identity_id' => $sellerId,
            ...array_combine(self::RELAXED['character_auctions'], [101, 102, 103]),
            'player_name' => 'Guard Knight',
            'level' => 50,
            'vocation' => 4,
            'sex' => 1,
            'character_snapshot' => '{}',
            'status' => CharacterAuction::STATUS_ACTIVE,
            'saga_state' => CharacterAuction::SAGA_ACTIVE,
            'duration_days' => 3,
            'starting_bid' => 100,
            'created_at' => now(),
            'updated_at' => now(),
            ...$overrides,
        ]);
    }

    /** @param  array<string, mixed>  $overrides */
    private function preference(int $identityId, array $overrides = []): int
    {
        return DB::table('character_profile_preferences')->insertGetId([
            'identity_id' => $identityId,
            ...array_fill_keys(self::RELAXED['character_profile_preferences'], 201),
            'created_at' => now(),
            'updated_at' => now(),
            ...$overrides,
        ]);
    }

    private function identityId(string $email): int
    {
        return Identity::query()->create(['email' => $email, 'password' => Hash::make('Correct-Horse-9!Battery')])->id;
    }

    /** @param 'up'|'down' $direction */
    private function applyMigration(string $direction): void
    {
        $migration = require database_path(self::MIGRATION);
        self::assertInstanceOf(Migration::class, $migration);

        (new ReflectionMethod($migration, $direction))->invoke($migration);
    }
}
