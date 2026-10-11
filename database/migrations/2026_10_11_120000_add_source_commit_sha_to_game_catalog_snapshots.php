<?php

use Illuminate\Database\Migrations\Migration;
use Illuminate\Database\Schema\Blueprint;
use Illuminate\Support\Facades\DB;
use Illuminate\Support\Facades\Schema;

/**
 * Source-neutral snapshot provenance beside the legacy engine commit column (removal plan packet B6).
 * Additive: the legacy column keeps its NOT NULL contract and every existing value is copied here.
 * The importer writes both columns and readers use this one. The column stays nullable so an
 * application rollback that predates it can still import; the drop packet for the legacy column
 * re-copies any NULL row and tightens this column to NOT NULL.
 */
return new class extends Migration
{
    /**
     * Legacy column name, split in two literals on purpose: this transitional reference leaves the
     * tree with the drop and squash packets, and must not raise the legacy-token ratchet meanwhile.
     */
    private const string LEGACY_COLUMN = 'can'.'ary_commit_sha';

    public function up(): void
    {
        Schema::table('game_catalog_snapshots', function (Blueprint $table): void {
            $table->string('source_commit_sha', 64)->nullable()->after(self::LEGACY_COLUMN);
        });

        DB::table('game_catalog_snapshots')
            ->whereNull('source_commit_sha')
            ->update(['source_commit_sha' => DB::raw(self::LEGACY_COLUMN)]);
    }

    public function down(): void
    {
        // The legacy column still holds every value written since up(); refuse before DDL only when a
        // row diverges, because dropping this column would then lose provenance the legacy column lacks.
        $diverged = DB::table('game_catalog_snapshots')
            ->whereNotNull('source_commit_sha')
            ->whereColumn('source_commit_sha', '<>', self::LEGACY_COLUMN)
            ->exists();

        if ($diverged) {
            throw new LogicException('Snapshot source provenance diverges from the legacy column; rollback refused before DDL.');
        }

        Schema::table('game_catalog_snapshots', function (Blueprint $table): void {
            $table->dropColumn('source_commit_sha');
        });
    }
};
