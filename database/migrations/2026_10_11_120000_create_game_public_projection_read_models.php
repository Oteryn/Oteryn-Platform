<?php

use Illuminate\Database\Migrations\Migration;
use Illuminate\Database\Schema\Blueprint;
use Illuminate\Support\Facades\DB;
use Illuminate\Support\Facades\Schema;

/**
 * Read-only Platform read models of Game Public Projections v1 (removal plan packet C4).
 * uint64 epochs and revisions stay canonical decimal strings. The singleton state row serializes
 * ingestion and holds the highest epoch seen across every family; an entry or watermark below it
 * is never served. Rows are Game-owned public snapshots, never authority for anything.
 */
return new class extends Migration
{
    public function up(): void
    {
        Schema::create('game_public_projection_state', function (Blueprint $table): void {
            $table->unsignedTinyInteger('id')->primary();
            $table->string('highest_epoch', 20)->nullable();
            $table->timestamps();
        });
        DB::table('game_public_projection_state')->insert(['id' => 1]);

        Schema::create('game_public_projection_watermarks', function (Blueprint $table): void {
            $table->string('family', 32)->primary();
            $table->string('source_authority', 128);
            $table->string('projection_epoch', 20);
            $table->unsignedBigInteger('complete_through');
            $table->unsignedBigInteger('observed_at');
            $table->timestamps();
        });

        Schema::create('game_public_projection_snapshots', function (Blueprint $table): void {
            $table->id();
            $table->string('family', 32);
            // character_id, guild_id or world_id; "world_id:category:filter" for highscores and
            // "world_id:house_key" for houses. Internal join key only, never public (contract §7.3).
            $table->string('subject_key', 200);
            $table->char('world_id', 36)->nullable();
            $table->string('lookup_name', 64)->nullable();
            $table->boolean('present');
            $table->string('source_authority', 128);
            $table->string('projection_epoch', 20);
            $table->string('projection_revision', 20);
            $table->unsignedBigInteger('source_observed_at');
            $table->char('content_digest', 64);
            $table->longText('payload');
            $table->timestamps();

            $table->unique(['family', 'subject_key']);
            $table->index(['family', 'world_id', 'lookup_name']);
        });
    }

    public function down(): void
    {
        if (DB::table('game_public_projection_snapshots')->exists()
            || DB::table('game_public_projection_watermarks')->exists()) {
            throw new LogicException('Public projection read models contain Game-published data; rollback refused before DDL.');
        }

        Schema::drop('game_public_projection_snapshots');
        Schema::drop('game_public_projection_watermarks');
        Schema::drop('game_public_projection_state');
    }
};
