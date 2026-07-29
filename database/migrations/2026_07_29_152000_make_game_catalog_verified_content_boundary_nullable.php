<?php

use Illuminate\Database\Migrations\Migration;
use Illuminate\Database\Schema\Blueprint;
use Illuminate\Support\Facades\DB;
use Illuminate\Support\Facades\Schema;
use RuntimeException;

return new class extends Migration
{
    public function up(): void
    {
        if (! $this->usesSqlite()) {
            Schema::table('game_catalog_snapshots', function (Blueprint $table): void {
                $table->dropForeign('game_catalog_snapshots_verified_release_fk');
            });
        }

        Schema::table('game_catalog_snapshots', function (Blueprint $table): void {
            $table->unsignedBigInteger('verified_content_through_release_id')->nullable()->change();
        });

        if (! $this->usesSqlite()) {
            Schema::table('game_catalog_snapshots', function (Blueprint $table): void {
                $table->foreign('verified_content_through_release_id', 'game_catalog_snapshots_verified_release_fk')
                    ->references('id')
                    ->on('game_catalog_releases')
                    ->restrictOnDelete();
            });
        }
    }

    public function down(): void
    {
        if (DB::table('game_catalog_snapshots')->whereNull('verified_content_through_release_id')->exists()) {
            throw new RuntimeException('Cannot restore the non-null Game Catalog verified-content boundary while unknown boundaries exist.');
        }

        if (! $this->usesSqlite()) {
            Schema::table('game_catalog_snapshots', function (Blueprint $table): void {
                $table->dropForeign('game_catalog_snapshots_verified_release_fk');
            });
        }

        Schema::table('game_catalog_snapshots', function (Blueprint $table): void {
            $table->unsignedBigInteger('verified_content_through_release_id')->nullable(false)->change();
        });

        if (! $this->usesSqlite()) {
            Schema::table('game_catalog_snapshots', function (Blueprint $table): void {
                $table->foreign('verified_content_through_release_id', 'game_catalog_snapshots_verified_release_fk')
                    ->references('id')
                    ->on('game_catalog_releases')
                    ->restrictOnDelete();
            });
        }
    }

    private function usesSqlite(): bool
    {
        return DB::connection()->getDriverName() === 'sqlite';
    }
};
