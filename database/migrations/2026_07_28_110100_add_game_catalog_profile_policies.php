<?php

use Illuminate\Database\Migrations\Migration;
use Illuminate\Database\Schema\Blueprint;
use Illuminate\Support\Facades\Schema;

return new class extends Migration
{
    public function up(): void
    {
        Schema::table('game_catalog_profiles', function (Blueprint $table): void {
            $table->string('protocol_profile', 80)->nullable()->after('active_snapshot_id');
            $table->string('completeness_policy_key', 80)->default('complete-only')->after('complete_only');
            $table->string('availability_policy_key', 80)->default('public-proven')->after('completeness_policy_key');
            $table->string('validation_policy_key', 80)->default('validated-snapshot')->after('availability_policy_key');
        });
    }

    public function down(): void
    {
        Schema::table('game_catalog_profiles', function (Blueprint $table): void {
            $table->dropColumn([
                'protocol_profile',
                'completeness_policy_key',
                'availability_policy_key',
                'validation_policy_key',
            ]);
        });
    }
};
