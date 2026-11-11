<?php

use Illuminate\Database\Migrations\Migration;
use Illuminate\Database\Schema\Blueprint;
use Illuminate\Support\Facades\DB;
use Illuminate\Support\Facades\Schema;

return new class extends Migration
{
    public function up(): void
    {
        Schema::create('native_game_evidence_witness_stores', function (Blueprint $table): void {
            $table->unsignedTinyInteger('id')->primary();
            $table->char('store_id', 64)->unique('native_evidence_witness_store_id_unique');
            $table->timestamp('created_at');
        });

        Schema::table('native_game_signing_trust_profiles', function (Blueprint $table): void {
            $table->unsignedBigInteger('profile_version')->default(1)->after('key_purpose');
        });

        Schema::table('native_game_signing_trust_profiles', function (Blueprint $table): void {
            $table->dropUnique('native_signing_trust_profile_unique');
            $table->unique(
                ['issuer', 'profile', 'key_purpose', 'profile_version'],
                'native_signing_trust_profile_version_unique',
            );
        });
    }

    public function down(): void
    {
        if (config('game-auth.native_evidence.activated') !== false) {
            throw new LogicException('Native evidence hardening state cannot be rolled back unless activation is explicitly false.');
        }

        if (Schema::hasColumn('native_game_signing_trust_profiles', 'profile_version')
            && DB::table('native_game_signing_trust_profiles')->where('profile_version', '>', 1)->exists()) {
            throw new LogicException('Native signing trust profile versions cannot be collapsed after a successor version exists.');
        }

        if (Schema::hasColumn('native_game_signing_trust_profiles', 'profile_version')) {
            Schema::table('native_game_signing_trust_profiles', function (Blueprint $table): void {
                $table->dropUnique('native_signing_trust_profile_version_unique');
                $table->dropColumn('profile_version');
            });

            Schema::table('native_game_signing_trust_profiles', function (Blueprint $table): void {
                $table->unique(
                    ['issuer', 'profile', 'key_purpose'],
                    'native_signing_trust_profile_unique',
                );
            });
        }

        Schema::dropIfExists('native_game_evidence_witness_stores');
    }
};
