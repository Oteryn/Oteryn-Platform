<?php

use Illuminate\Database\Migrations\Migration;
use Illuminate\Database\Schema\Blueprint;
use Illuminate\Support\Facades\Schema;

return new class extends Migration
{
    public function up(): void
    {
        Schema::create('player_companion_session_analyses', function (Blueprint $table): void {
            $table->id();
            $table->foreignId('identity_id')->constrained('identities')->cascadeOnDelete();
            $table->string('label', 80)->nullable();
            $table->string('source_format', 48);
            $table->string('parser_version', 32);
            $table->string('formula_version', 48);
            $table->json('applicability');
            $table->unsignedInteger('session_seconds');
            $table->bigInteger('experience_gain')->nullable();
            $table->bigInteger('loot_value')->nullable();
            $table->bigInteger('supplies_value')->nullable();
            $table->bigInteger('balance_value')->nullable();
            $table->bigInteger('damage')->nullable();
            $table->bigInteger('healing')->nullable();
            $table->bigInteger('experience_per_hour')->nullable();
            $table->bigInteger('profit_per_hour')->nullable();
            $table->unsignedSmallInteger('participant_count')->default(0);
            $table->json('participants');
            $table->json('settlements');
            $table->timestamps();

            $table->index(['identity_id', 'created_at'], 'pc_session_analysis_owner_created_idx');
        });
    }

    public function down(): void
    {
        Schema::dropIfExists('player_companion_session_analyses');
    }
};
