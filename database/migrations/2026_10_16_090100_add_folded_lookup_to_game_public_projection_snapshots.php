<?php

use Illuminate\Database\Migrations\Migration;
use Illuminate\Database\Schema\Blueprint;
use Illuminate\Support\Facades\DB;
use Illuminate\Support\Facades\Schema;

/**
 * ASCII case-folded lookup name for the name-resolution query (contract §7.3), so the lookup is an
 * index seek instead of LOWER() over the family. Written by ingestion; backfilled here.
 */
return new class extends Migration
{
    public function up(): void
    {
        Schema::table('game_public_projection_snapshots', function (Blueprint $table): void {
            $table->string('lookup_folded', 64)->nullable()->after('lookup_name');
            $table->index(['family', 'lookup_folded'], 'game_public_projection_snapshots_folded_index');
        });

        DB::table('game_public_projection_snapshots')->whereNotNull('lookup_name')->orderBy('id')
            ->each(function (object $row): void {
                $name = get_object_vars($row)['lookup_name'] ?? null;
                if (is_string($name)) {
                    DB::table('game_public_projection_snapshots')->where('id', get_object_vars($row)['id'])
                        ->update(['lookup_folded' => strtolower($name)]);
                }
            });
    }

    public function down(): void
    {
        Schema::table('game_public_projection_snapshots', function (Blueprint $table): void {
            $table->dropIndex('game_public_projection_snapshots_folded_index');
            $table->dropColumn('lookup_folded');
        });
    }
};
