<?php

use Illuminate\Database\Migrations\Migration;
use Illuminate\Database\Schema\Blueprint;
use Illuminate\Support\Facades\DB;
use Illuminate\Support\Facades\Schema;

return new class extends Migration
{
    private const OLD_PROFILE = 'oteryn.native.v1';

    private const OLD_SCHEMA_SHA256 = 'c7665223f09001e3294e9a03ab4784defed66b0ac04450e8679d4778421207f8';

    private const NEW_SCHEMA_SHA256 = '9c67f19525400fb9890d2a3541ceb6d02eb955061540ad39ca1c1d891c06eba9';

    public function up(): void
    {
        Schema::table('game_world_protocol_candidates', function (Blueprint $table): void {
            $table->dropUnique('world_protocol_candidate_tuple_unique');
            $table->unsignedTinyInteger('native_protocol_version')->nullable()->after('profile');
            $table->string('profile', 64)->nullable()->change();
        });

        DB::table('game_world_protocol_candidates')
            ->where('family', 'oteryn')
            ->where('profile', self::OLD_PROFILE)
            ->whereNull('native_protocol_version')
            ->update([
                'profile' => null,
                'native_protocol_version' => 1,
                'schema_revision' => 2,
                'schema_sha256' => self::NEW_SCHEMA_SHA256,
                'enabled' => false,
                'updated_at' => now(),
            ]);

        Schema::table('game_world_protocol_candidates', function (Blueprint $table): void {
            $table->unique(
                ['game_world_id', 'channel_id', 'family', 'native_protocol_version', 'transport', 'schema_revision'],
                'world_protocol_native_tuple_unique',
            );
            $table->unique(
                ['game_world_id', 'channel_id', 'family', 'profile', 'transport', 'schema_revision'],
                'world_protocol_compat_tuple_unique',
            );
        });
    }

    public function down(): void
    {
        DB::table('game_world_protocol_candidates')
            ->where('family', 'oteryn')
            ->where('native_protocol_version', 1)
            ->whereNull('profile')
            ->update([
                'profile' => self::OLD_PROFILE,
                'native_protocol_version' => null,
                'schema_revision' => 1,
                'schema_sha256' => self::OLD_SCHEMA_SHA256,
                'enabled' => false,
                'updated_at' => now(),
            ]);

        Schema::table('game_world_protocol_candidates', function (Blueprint $table): void {
            $table->dropUnique('world_protocol_native_tuple_unique');
            $table->dropUnique('world_protocol_compat_tuple_unique');
            $table->string('profile', 64)->nullable(false)->change();
            $table->dropColumn('native_protocol_version');
            $table->unique(
                ['game_world_id', 'channel_id', 'family', 'profile', 'transport', 'schema_revision'],
                'world_protocol_candidate_tuple_unique',
            );
        });
    }
};
