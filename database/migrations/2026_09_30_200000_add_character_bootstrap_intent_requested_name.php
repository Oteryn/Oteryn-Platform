<?php

use Illuminate\Database\Migrations\Migration;
use Illuminate\Database\Schema\Blueprint;
use Illuminate\Support\Facades\Schema;

/**
 * CHAR-NAME-1: bootstrap intent contract version 2 binds the requested Character name.
 * Additive and nullable: a version 1 row issued before this migration has no name, is no
 * longer decodable as a current intent and fails closed (unavailable) on read or retry.
 */
return new class extends Migration
{
    public function up(): void
    {
        Schema::table('character_bootstrap_intents', function (Blueprint $table): void {
            $table->string('requested_name', 29)->nullable()->after('target_world_id');
        });
    }

    public function down(): void
    {
        Schema::table('character_bootstrap_intents', function (Blueprint $table): void {
            $table->dropColumn('requested_name');
        });
    }
};
