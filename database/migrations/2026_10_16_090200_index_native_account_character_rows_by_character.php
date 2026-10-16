<?php

use Illuminate\Database\Migrations\Migration;
use Illuminate\Database\Schema\Blueprint;
use Illuminate\Support\Facades\Schema;

/**
 * Reverse ownership lookup for the public profile guild check: rows by character, then account, so the
 * bounded distinct-account probe is an index range scan instead of a full scan of the roster projection.
 */
return new class extends Migration
{
    public function up(): void
    {
        Schema::table('native_account_character_rows', function (Blueprint $table): void {
            $table->index(['character_id', 'account_id'], 'native_account_character_rows_character_index');
        });
    }

    public function down(): void
    {
        Schema::table('native_account_character_rows', function (Blueprint $table): void {
            $table->dropIndex('native_account_character_rows_character_index');
        });
    }
};
