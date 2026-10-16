<?php

use Illuminate\Database\Migrations\Migration;
use Illuminate\Database\Schema\Blueprint;
use Illuminate\Support\Facades\DB;
use Illuminate\Support\Facades\Schema;

/**
 * Character profile preferences are keyed by the native CharacterId (public gamedata, round 4).
 * `canary_player_id` becomes optional history and `character_id` gets its owner-scoped unique key.
 * No backfill is possible: a legacy player id carries no link to a native CharacterId (Game owns
 * that mapping and publishes none), so legacy rows stay unbound, are never read by the native
 * pages and hide nothing until the owner saves the native form once.
 */
return new class extends Migration
{
    public function up(): void
    {
        Schema::table('character_profile_preferences', function (Blueprint $table): void {
            $table->unsignedBigInteger('canary_player_id')->nullable()->change();
            $table->unique(['identity_id', 'character_id'], 'character_profile_preferences_owner_character_unique');
        });
    }

    public function down(): void
    {
        if (DB::table('character_profile_preferences')->whereNull('canary_player_id')->exists()) {
            throw new LogicException('Native-keyed character profile preferences must be retained; rollback refused before DDL.');
        }

        Schema::table('character_profile_preferences', function (Blueprint $table): void {
            $table->dropUnique('character_profile_preferences_owner_character_unique');
            $table->unsignedBigInteger('canary_player_id')->nullable(false)->change();
        });
    }
};
