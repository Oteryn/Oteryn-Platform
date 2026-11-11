<?php

use Illuminate\Database\Migrations\Migration;
use Illuminate\Database\Schema\Blueprint;
use Illuminate\Support\Facades\DB;
use Illuminate\Support\Facades\Schema;

return new class extends Migration
{
    public function up(): void
    {
        Schema::create('character_bootstrap_intent_authority', function (Blueprint $table): void {
            $table->unsignedTinyInteger('id')->primary();
            $table->unsignedBigInteger('last_source_revision')->default(0);
            $table->char('last_issuer_decision_id', 36)->nullable();
            $table->timestamp('updated_at');
        });
        DB::table('character_bootstrap_intent_authority')->insert([
            'id' => 1,
            'last_source_revision' => 0,
            'last_issuer_decision_id' => null,
            'updated_at' => now(),
        ]);

        Schema::create('character_bootstrap_intents', function (Blueprint $table): void {
            $table->id();
            $table->foreignId('identity_id')->constrained('identities')->restrictOnDelete();
            $table->char('operation_id', 36)->unique();
            $table->char('issuer_decision_id', 36)->unique();
            $table->unsignedBigInteger('source_revision')->unique();
            $table->char('account_id', 36);
            $table->char('target_world_id', 36);
            $table->string('profile_revision', 128);
            $table->string('ruleset_revision', 128);
            $table->string('content_revision', 128);
            $table->string('starter_template_revision', 128);
            $table->unsignedBigInteger('issued_at_source');
            $table->unsignedBigInteger('expires_at_source');
            $table->text('intent_json');
            $table->char('intent_sha256', 64);
            $table->timestamp('created_at');
            $table->index(['account_id', 'created_at'], 'character_bootstrap_account_created_index');
        });
    }

    public function down(): void
    {
        if (Schema::hasTable('character_bootstrap_intents') && DB::table('character_bootstrap_intents')->exists()) {
            throw new LogicException('Issued Character bootstrap intents are immutable production authority and cannot be rolled back.');
        }
        Schema::dropIfExists('character_bootstrap_intents');
        Schema::dropIfExists('character_bootstrap_intent_authority');
    }
};
