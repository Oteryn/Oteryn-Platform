<?php

use Illuminate\Database\Migrations\Migration;
use Illuminate\Database\Schema\Blueprint;
use Illuminate\Support\Facades\DB;
use Illuminate\Support\Facades\Schema;

return new class extends Migration
{
    public function up(): void
    {
        Schema::table('game_catalog_loot_snapshots', function (Blueprint $table): void {
            $table->string('chance_model', 40)->default('rational_probability')->after('relation_snapshot_id');
            $table->unsignedBigInteger('chance_threshold')->nullable()->after('chance_denominator');
            $table->unsignedBigInteger('roll_maximum')->nullable()->after('chance_threshold');
            $table->unsignedBigInteger('chance_numerator')->nullable()->change();
            $table->unsignedBigInteger('chance_denominator')->nullable()->change();
        });
    }

    public function down(): void
    {
        $unrepresentable = DB::table('game_catalog_loot_snapshots')
            ->where('chance_model', '!=', 'rational_probability')
            ->orWhereNull('chance_numerator')
            ->orWhereNull('chance_denominator')
            ->exists();

        if ($unrepresentable) {
            throw new RuntimeException('Cannot remove the Game Catalog loot chance model while runtime-threshold rows exist.');
        }

        Schema::table('game_catalog_loot_snapshots', function (Blueprint $table): void {
            $table->unsignedBigInteger('chance_numerator')->nullable(false)->change();
            $table->unsignedBigInteger('chance_denominator')->nullable(false)->change();
            $table->dropColumn(['chance_model', 'chance_threshold', 'roll_maximum']);
        });
    }
};
