<?php

use Illuminate\Database\Migrations\Migration;
use Illuminate\Database\Schema\Blueprint;
use Illuminate\Support\Facades\DB;
use Illuminate\Support\Facades\Schema;

return new class extends Migration
{
    public function up(): void
    {
        Schema::create('character_bootstrap_intent_authority_states', function (Blueprint $table): void {
            $table->unsignedTinyInteger('id')->primary();
            $table->unsignedBigInteger('source_revision');
            $table->timestamp('updated_at');
        });
        DB::table('character_bootstrap_intent_authority_states')->insert([
            'id' => 1,
            'source_revision' => 0,
            'updated_at' => now(),
        ]);

        Schema::create('character_bootstrap_intents', function (Blueprint $table): void {
            $table->id();
            $table->unsignedBigInteger('source_revision')->unique();
            $table->uuid('operation_id')->unique();
            $table->uuid('issuer_decision_id')->unique();
            $table->string('issuer_authority', 128);
            $table->foreignId('identity_id')->constrained('identities')->restrictOnDelete();
            $table->uuid('account_id');
            $table->string('target_world_id', 64);
            $table->string('profile_revision', 19);
            $table->string('ruleset_revision', 19);
            $table->string('content_revision', 19);
            $table->string('starter_template_revision', 19);
            $table->unsignedBigInteger('issued_at_source');
            $table->unsignedBigInteger('expires_at_source');
            $table->timestamp('created_at');
            $table->index(['account_id', 'created_at'], 'character_bootstrap_account_created_index');
        });
    }

    public function down(): void
    {
        if (config('game-auth.character_bootstrap_intent.activated') !== false) {
            throw new LogicException('Character bootstrap intent authority cannot be rolled back unless activation is explicitly false.');
        }
        Schema::dropIfExists('character_bootstrap_intents');
        Schema::dropIfExists('character_bootstrap_intent_authority_states');
    }
};
