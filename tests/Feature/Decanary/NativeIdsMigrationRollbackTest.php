<?php

namespace Tests\Feature\Decanary;

use App\Identity\Models\Identity;
use Illuminate\Database\Events\QueryExecuted;
use Illuminate\Database\Migrations\Migration;
use Illuminate\Foundation\Testing\DatabaseMigrations;
use Illuminate\Support\Facades\DB;
use Illuminate\Support\Facades\Event;
use Illuminate\Support\Facades\Hash;
use Illuminate\Support\Facades\Schema;
use LogicException;
use ReflectionMethod;
use Tests\TestCase;

final class NativeIdsMigrationRollbackTest extends TestCase
{
    use DatabaseMigrations;

    private const CHARACTER = '0192b3c4-5d6e-7f80-8a1b-2c3d4e5f6a7c';

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

        $this->assertNativeColumnsPresent();
    }

    public function test_empty_native_ids_roll_back_in_maintenance_mode(): void
    {
        $this->app['env'] = 'local';
        $this->app->maintenanceMode()->activate([]);
        $this->rollBackKeying();
        try {
            $this->applyMigration('down');

            self::assertFalse(Schema::hasColumn('character_auctions', 'seller_account_id'));
            self::assertFalse(Schema::hasColumn('character_auctions', 'escrow_account_id'));
            self::assertFalse(Schema::hasColumn('character_auctions', 'character_id'));
            self::assertFalse(Schema::hasColumn('character_profile_preferences', 'character_id'));
            self::assertTrue(Schema::hasColumn('character_profile_preferences', 'canary_player_id'));
        } finally {
            $this->app->maintenanceMode()->deactivate();
            $this->app['env'] = 'testing';
            $this->applyMigration('up');
            $this->restoreKeying();
        }
    }

    public function test_written_native_id_refuses_rollback_before_ddl(): void
    {
        DB::table('character_profile_preferences')->insert([
            'identity_id' => $this->identityId(),
            'canary_player_id' => 7,
            'character_id' => self::CHARACTER,
        ]);

        try {
            $this->applyMigration('down');
            self::fail('Native ids must survive rollback.');
        } catch (LogicException $exception) {
            self::assertStringContainsString('before DDL', $exception->getMessage());
        } finally {
            DB::table('character_profile_preferences')->delete();
        }

        $this->assertNativeColumnsPresent();
    }

    public function test_mariadb_checks_and_ddl_run_under_one_table_lock(): void
    {
        if (! in_array(DB::connection()->getDriverName(), ['mysql', 'mariadb'], true)) {
            self::markTestSkipped('LOCK TABLES applies to MariaDB/MySQL only.');
        }

        $this->rollBackKeying();
        $statements = [];
        Event::listen(QueryExecuted::class, function (QueryExecuted $query) use (&$statements): void {
            $statements[] = $query->sql;
        });

        try {
            $this->applyMigration('down');
        } finally {
            $this->applyMigration('up');
            $this->restoreKeying();
        }

        $unlock = array_search('UNLOCK TABLES', $statements, true);
        self::assertIsInt($unlock);
        self::assertSame('LOCK TABLES character_auctions WRITE, character_profile_preferences WRITE', $statements[0]);
        $underLock = array_slice($statements, 1, $unlock - 1);
        self::assertCount(4, array_filter($underLock, fn (string $sql): bool => str_starts_with($sql, 'select exists')));
        self::assertCount(6, array_filter($underLock, fn (string $sql): bool => str_starts_with($sql, 'alter table')));
        self::assertCount(10, $underLock);
    }

    public function test_pgsql_checks_and_ddl_run_in_one_transaction_under_exclusive_table_locks(): void
    {
        if (DB::connection()->getDriverName() !== 'pgsql') {
            self::markTestSkipped('LOCK TABLE ... IN ACCESS EXCLUSIVE MODE applies to PostgreSQL only.');
        }

        $this->rollBackKeying();
        /** @var list<array{sql: string, level: int}> $statements */
        $statements = [];
        Event::listen(QueryExecuted::class, function (QueryExecuted $query) use (&$statements): void {
            $statements[] = ['sql' => $query->sql, 'level' => $query->connection->transactionLevel()];
        });

        try {
            $this->applyMigration('down');
        } finally {
            $this->applyMigration('up');
            $this->restoreKeying();
        }

        $down = array_slice($statements, 0, 11);
        self::assertSame('LOCK TABLE character_auctions, character_profile_preferences IN ACCESS EXCLUSIVE MODE', $down[0]['sql']);
        foreach ($down as $statement) {
            self::assertGreaterThanOrEqual(1, $statement['level'], $statement['sql']);
        }
        $sql = array_column(array_slice($down, 1), 'sql');
        self::assertCount(4, array_filter($sql, fn (string $statement): bool => str_starts_with($statement, 'select exists')));
        self::assertCount(6, array_filter($sql, fn (string $statement): bool => str_starts_with($statement, 'alter table')
            || str_starts_with($statement, 'drop index')));
        self::assertSame(0, DB::transactionLevel());
        $this->assertNativeColumnsPresent();
    }

    private function assertNativeColumnsPresent(): void
    {
        self::assertTrue(Schema::hasColumn('character_auctions', 'seller_account_id'));
        self::assertTrue(Schema::hasColumn('character_auctions', 'escrow_account_id'));
        self::assertTrue(Schema::hasColumn('character_auctions', 'character_id'));
        self::assertTrue(Schema::hasColumn('character_profile_preferences', 'character_id'));
    }

    private function identityId(): int
    {
        return Identity::query()->create(['email' => 'native-ids@example.test', 'password' => Hash::make('Correct-Horse-9!Battery')])->id;
    }

    /** The later native-keying migration is rolled back first, as `migrate:rollback` orders it. */
    private function rollBackKeying(): void
    {
        $this->applyKeying('down');
    }

    private function restoreKeying(): void
    {
        $this->applyKeying('up');
    }

    /** @param 'up'|'down' $direction */
    private function applyKeying(string $direction): void
    {
        $migration = require database_path('migrations/2026_10_16_090000_key_character_profile_preferences_by_native_character.php');
        self::assertInstanceOf(Migration::class, $migration);

        (new ReflectionMethod($migration, $direction))->invoke($migration);
    }

    /** @param 'up'|'down' $direction */
    private function applyMigration(string $direction): void
    {
        $migration = require database_path('migrations/2026_10_08_090000_add_native_ids_to_character_bazaar_and_profile_preferences.php');
        self::assertInstanceOf(Migration::class, $migration);

        (new ReflectionMethod($migration, $direction))->invoke($migration);
    }
}
