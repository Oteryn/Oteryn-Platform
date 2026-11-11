<?php

use Illuminate\Database\Migrations\Migration;
use Illuminate\Database\Schema\Blueprint;
use Illuminate\Support\Facades\DB;
use Illuminate\Support\Facades\Schema;

return new class extends Migration
{
    public function up(): void
    {
        Schema::table('game_worlds', function (Blueprint $table): void {
            $table->uuid('world_id')->nullable()->unique();
        });

        Schema::create('game_channels', function (Blueprint $table): void {
            $table->id();
            $table->foreignId('game_world_id')->constrained('game_worlds')->restrictOnDelete();
            $table->uuid('channel_id')->unique();
            $table->string('channel_key', 64);
            $table->timestamps();
            $table->unique(['game_world_id', 'channel_key']);
        });
    }

    public function down(): void
    {
        // Even inactive issuance must survive: login eligibility is not identity lifetime.
        if (DB::table('game_worlds')->whereNotNull('world_id')->exists()
            || DB::table('game_channels')->exists()) {
            throw new LogicException('Issued native topology identities must be retained; rollback refused before DDL.');
        }

        Schema::drop('game_channels');
        Schema::table('game_worlds', function (Blueprint $table): void {
            $table->dropUnique(['world_id']);
            $table->dropColumn('world_id');
        });
    }
};
