<?php

use Illuminate\Database\Migrations\Migration;
use Illuminate\Database\Schema\Blueprint;
use Illuminate\Support\Facades\Schema;

return new class extends Migration
{
    public function up(): void
    {
        Schema::create('game_catalog_snapshot_releases', function (Blueprint $table): void {
            $table->foreignId('snapshot_id')->constrained('game_catalog_snapshots')->cascadeOnDelete();
            $table->foreignId('release_id')->constrained('game_catalog_releases');
            $table->primary(['snapshot_id', 'release_id'], 'catalog_snapshot_release_primary');
        });
    }

    public function down(): void
    {
        Schema::dropIfExists('game_catalog_snapshot_releases');
    }
};
