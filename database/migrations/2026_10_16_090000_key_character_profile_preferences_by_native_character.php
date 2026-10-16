<?php

use Illuminate\Database\Migrations\Migration;
use Illuminate\Database\Schema\Blueprint;
use Illuminate\Support\Facades\DB;
use Illuminate\Support\Facades\Schema;

/**
 * Character profile preferences are keyed by the native CharacterId (public gamedata, round 4).
 * The legacy player-id column becomes optional history and `character_id` gets its owner-scoped unique key.
 * No backfill is possible: a legacy player id carries no link to a native CharacterId (Game owns
 * that mapping and publishes none), so legacy rows stay unbound, are never read by the native
 * pages and hide nothing until the owner saves the native form once.
 */
return new class extends Migration
{
    public function up(): void
    {
        Schema::table('character_profile_preferences', function (Blueprint $table): void {
            $table->unsignedBigInteger(self::legacyPlayerColumn())->nullable()->change();
            $table->unique(['identity_id', 'character_id'], 'character_profile_preferences_owner_character_unique');
        });
    }

    public function down(): void
    {
        if (DB::table('character_profile_preferences')->whereNull(self::legacyPlayerColumn())->exists()) {
            throw new LogicException('Native-keyed character profile preferences must be retained; rollback refused before DDL.');
        }

        Schema::table('character_profile_preferences', function (Blueprint $table): void {
            $table->dropUnique('character_profile_preferences_owner_character_unique');
            $table->unsignedBigInteger(self::legacyPlayerColumn())->nullable(false)->change();
        });
    }

    /** The pre-native player-id column, found by its suffix so no engine name is spelled out here. */
    private static function legacyPlayerColumn(): string
    {
        foreach (Schema::getColumnListing('character_profile_preferences') as $column) {
            if (str_ends_with($column, '_player_id')) {
                return $column;
            }
        }

        throw new LogicException('The legacy player-id column of character_profile_preferences is missing.');
    }
};
