<?php

use Illuminate\Database\Migrations\Migration;
use Illuminate\Database\Schema\Blueprint;
use Illuminate\Support\Facades\DB;
use Illuminate\Support\Facades\Schema;

/**
 * Read-only Platform projection of Game Character Authority ListCharactersForAccount v1.
 * uint64 epochs/revisions remain canonical decimal strings. The singleton state serializes global
 * epoch changes and watermark freshness; account rows are snapshots, never Character authority.
 */
return new class extends Migration
{
    public function up(): void
    {
        Schema::create('native_account_character_projection_state', function (Blueprint $table): void {
            $table->unsignedTinyInteger('id')->primary();
            $table->string('highest_epoch', 20)->nullable();
            $table->string('watermark_epoch', 20)->nullable();
            $table->string('watermark_source_authority', 128)->nullable();
            $table->unsignedBigInteger('complete_through')->nullable();
            $table->unsignedBigInteger('watermark_observed_at')->nullable();
            $table->timestamps();
        });
        DB::table('native_account_character_projection_state')->insert(['id' => 1]);

        Schema::create('native_account_character_snapshots', function (Blueprint $table): void {
            $table->char('account_id', 36)->primary();
            $table->string('source_authority', 128);
            $table->string('projection_epoch', 20)->index();
            $table->string('projection_revision', 20);
            $table->unsignedBigInteger('source_observed_at');
            $table->char('content_digest', 64);
            $table->boolean('invalid')->default(false);
            $table->timestamps();
        });

        Schema::create('native_account_character_rows', function (Blueprint $table): void {
            $table->id();
            $table->char('account_id', 36)->index();
            $table->char('character_id', 36);
            $table->char('world_id', 36);
            $table->string('name', 64);
            $table->string('availability', 11);
            $table->timestamps();

            $table->unique(['account_id', 'character_id']);
            $table->foreign('account_id')
                ->references('account_id')
                ->on('native_account_character_snapshots')
                ->cascadeOnDelete();
        });
    }

    public function down(): void
    {
        if (DB::table('native_account_character_snapshots')->exists()
            || DB::table('native_account_character_rows')->exists()) {
            throw new LogicException('Native account-character projection contains authority-derived evidence; rollback refused before DDL.');
        }

        Schema::drop('native_account_character_rows');
        Schema::drop('native_account_character_snapshots');
        Schema::drop('native_account_character_projection_state');
    }
};
