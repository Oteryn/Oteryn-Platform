<?php

use Illuminate\Database\Migrations\Migration;
use Illuminate\Database\Schema\Blueprint;
use Illuminate\Support\Facades\DB;
use Illuminate\Support\Facades\Schema;

/**
 * Backward-compatible relaxation of the legacy engine ids (removal plan packet C0). Rows may now carry
 * only the native AccountId/CharacterId columns added by 2026_10_08_090000; no read or write changes
 * here. The legacy unique keys do not guard rows without a legacy id, so the native guards ship in the
 * same migration:
 *  - character_profile_preferences: unique (identity_id, character_id);
 *  - character_auctions.active_character_id: unique, the native twin of active_player_id (set while a
 *    listing holds the character, cleared when it ends), copied from character_id where active_player_id is set.
 * Rollback restores NOT NULL and therefore refuses, before any DDL, while a relaxed column holds NULL.
 */
return new class extends Migration
{
    /** Legacy id columns relaxed to nullable, per table. */
    private const RELAXED = ['character_auctions' => ['seller_canary_account_id', 'escrow_canary_account_id', 'player_id'], 'character_profile_preferences' => ['canary_player_id']];

    public function up(): void
    {
        // Refuse before any DDL when existing native ids would violate the new guards.
        if (DB::table('character_profile_preferences')
            ->select('identity_id', 'character_id')
            ->whereNotNull('character_id')
            ->groupBy('identity_id', 'character_id')
            ->havingRaw('count(*) > 1')
            ->exists()
            || DB::table('character_auctions')
                ->select('character_id')
                ->whereNotNull('active_player_id')
                ->whereNotNull('character_id')
                ->groupBy('character_id')
                ->havingRaw('count(*) > 1')
                ->exists()) {
            throw new LogicException('Duplicate native character ids violate the native guards; migration refused before DDL.');
        }

        $this->setRelaxed(true);

        Schema::table('character_auctions', function (Blueprint $table): void {
            $table->char('active_character_id', 36)->nullable()->after('character_id');

            $table->unique('active_character_id', 'character_auctions_active_character_unique');
        });

        DB::table('character_auctions')
            ->whereNotNull('active_player_id')
            ->whereNotNull('character_id')
            ->update(['active_character_id' => DB::raw('character_id')]);

        Schema::table('character_profile_preferences', function (Blueprint $table): void {
            $table->unique(['identity_id', 'character_id'], 'character_profile_preferences_owner_character_unique');
        });
    }

    public function down(): void
    {
        // Rollback runs only with the application down so no request or worker writes a row without a legacy id mid-rollback.
        // The test runner is exempt because DatabaseMigrations rolls every migration back after each test.
        if (! app()->isDownForMaintenance() && ! app()->runningUnitTests()) {
            throw new LogicException('Run "php artisan down" before restoring the legacy id constraints; rollback refused before DDL.');
        }

        $locking = in_array(DB::connection()->getDriverName(), ['mysql', 'mariadb'], true);

        if ($locking) {
            // The checks and the DDL share one table lock so no writer can add a row without a legacy id between them.
            DB::unprepared('LOCK TABLES character_auctions WRITE, character_profile_preferences WRITE');
        }

        try {
            $this->restoreLegacyConstraints();
        } finally {
            if ($locking) {
                DB::unprepared('UNLOCK TABLES');
            }
        }
    }

    private function restoreLegacyConstraints(): void
    {
        // NOT NULL cannot hold a row without a legacy id: refuse before any DDL while one exists.
        foreach (self::RELAXED as $tableName => $columns) {
            foreach ($columns as $column) {
                if (DB::table($tableName)->whereNull($column)->exists()) {
                    throw new LogicException("Rows without a legacy id exist in {$tableName}.{$column}; rollback refused before DDL.");
                }
            }
        }

        Schema::table('character_profile_preferences', function (Blueprint $table): void {
            $table->dropUnique('character_profile_preferences_owner_character_unique');
        });

        Schema::table('character_auctions', function (Blueprint $table): void {
            $table->dropUnique('character_auctions_active_character_unique');
            $table->dropColumn('active_character_id');
        });

        $this->setRelaxed(false);
    }

    private function setRelaxed(bool $nullable): void
    {
        foreach (self::RELAXED as $tableName => $columns) {
            Schema::table($tableName, function (Blueprint $table) use ($columns, $nullable): void {
                foreach ($columns as $column) {
                    $table->unsignedBigInteger($column)->nullable($nullable)->change();
                }
            });
        }
    }
};
