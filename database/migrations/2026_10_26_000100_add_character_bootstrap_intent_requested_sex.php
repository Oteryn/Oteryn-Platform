<?php

use Illuminate\Database\Migrations\Migration;
use Illuminate\Database\Schema\Blueprint;
use Illuminate\Support\Facades\Schema;

/**
 * APP-3: contract version 3 binds explicit creation sex. Historical intents keep
 * NULL rather than guessed sex and fail closed on current reads and retries.
 */
return new class extends Migration
{
    public function up(): void
    {
        Schema::table('character_bootstrap_intents', function (Blueprint $table): void {
            $table->string('requested_sex', 6)->nullable()->after('requested_name');
        });
    }

    public function down(): void
    {
        Schema::table('character_bootstrap_intents', function (Blueprint $table): void {
            $table->dropColumn('requested_sex');
        });
    }
};
