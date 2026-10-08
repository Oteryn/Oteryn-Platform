<?php

use Illuminate\Database\Migrations\Migration;
use Illuminate\Database\Schema\Blueprint;
use Illuminate\Support\Facades\DB;
use Illuminate\Support\Facades\Schema;

/**
 * Native AccountId/CharacterId columns beside the legacy Canary ids (DECANARY-IDS-1). Additive and
 * nullable: no backfill, existing Canary ids stay authoritative until the cutover packet, and the
 * Canary ids are archived and dropped later without any mapping to AccountId.
 */
return new class extends Migration
{
    public function up(): void
    {
        Schema::table('character_auctions', function (Blueprint $table): void {
            $table->char('seller_account_id', 36)->nullable()->after('escrow_canary_account_id');
            $table->char('escrow_account_id', 36)->nullable()->after('seller_account_id');
            $table->char('character_id', 36)->nullable()->after('active_player_id');

            $table->index('seller_account_id', 'character_auctions_seller_account_index');
            $table->index('escrow_account_id', 'character_auctions_escrow_account_index');
            $table->index('character_id', 'character_auctions_character_index');
        });

        Schema::table('character_profile_preferences', function (Blueprint $table): void {
            $table->char('character_id', 36)->nullable()->after('canary_player_id');

            $table->index('character_id', 'character_profile_preferences_character_index');
        });
    }

    public function down(): void
    {
        // Native ids must survive: refuse before any DDL once a native id has been written.
        if (DB::table('character_auctions')->whereNotNull('seller_account_id')->exists()
            || DB::table('character_auctions')->whereNotNull('escrow_account_id')->exists()
            || DB::table('character_auctions')->whereNotNull('character_id')->exists()
            || DB::table('character_profile_preferences')->whereNotNull('character_id')->exists()) {
            throw new LogicException('Native account and character ids must be retained; rollback refused before DDL.');
        }

        Schema::table('character_profile_preferences', function (Blueprint $table): void {
            $table->dropIndex('character_profile_preferences_character_index');
            $table->dropColumn('character_id');
        });

        Schema::table('character_auctions', function (Blueprint $table): void {
            $table->dropIndex('character_auctions_seller_account_index');
            $table->dropIndex('character_auctions_escrow_account_index');
            $table->dropIndex('character_auctions_character_index');
            $table->dropColumn(['seller_account_id', 'escrow_account_id', 'character_id']);
        });
    }
};
