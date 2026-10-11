<?php

use Illuminate\Database\Migrations\Migration;
use Illuminate\Database\Schema\Blueprint;
use Illuminate\Support\Facades\DB;
use Illuminate\Support\Facades\Schema;

/**
 * Platform Character Authority command intents and Game receipts (Game contract
 * `oteryn-game-character-authority-commands-v1` §§4-5). Intents are immutable once written; the
 * row id is the intent's `source_revision`. Receipts keep history: a restore reopen (§5.4) marks
 * the current receipt superseded and stores exactly one new one.
 */
return new class extends Migration
{
    public function up(): void
    {
        Schema::create('character_command_intents', function (Blueprint $table): void {
            $table->id();
            $table->string('issuer_authority', 128);
            $table->char('operation_id', 36);
            $table->string('command', 64);
            $table->char('request_key_sha256', 64);
            $table->char('binding_sha256', 64);
            $table->unsignedBigInteger('issued_at_source');
            $table->unsignedBigInteger('expires_at_source');
            $table->text('intent_json')->nullable();
            $table->char('intent_sha256', 64)->nullable();
            $table->string('state', 16)->default('PENDING');
            $table->unsignedBigInteger('current_receipt_id')->nullable();
            $table->timestamps();
            $table->unique(['issuer_authority', 'operation_id'], 'character_command_intent_operation_unique');
            $table->unique(['issuer_authority', 'request_key_sha256'], 'character_command_intent_request_key_unique');
            $table->index(['issuer_authority', 'state', 'id'], 'character_command_intent_pending');
        });

        Schema::create('character_command_receipts', function (Blueprint $table): void {
            $table->id();
            $table->foreignId('intent_id')->constrained('character_command_intents')->restrictOnDelete();
            $table->string('issuer_authority', 128);
            $table->char('operation_id', 36);
            $table->string('command', 64);
            $table->string('outcome', 16);
            $table->string('result_code', 64);
            $table->char('recovery_generation', 20);
            $table->text('receipt_json');
            $table->char('receipt_sha256', 64);
            $table->dateTime('superseded_at')->nullable();
            $table->dateTime('created_at');
            $table->index(['issuer_authority', 'operation_id'], 'character_command_receipt_operation');
            $table->index(['recovery_generation'], 'character_command_receipt_generation');
        });
    }

    public function down(): void
    {
        // Intents and receipts are cross-repository evidence of Character mutations; never erase them silently.
        if ((Schema::hasTable('character_command_receipts') && DB::table('character_command_receipts')->exists())
            || (Schema::hasTable('character_command_intents') && DB::table('character_command_intents')->exists())) {
            throw new LogicException('Character command intents or receipts exist; rollback refused before DDL.');
        }

        Schema::dropIfExists('character_command_receipts');
        Schema::dropIfExists('character_command_intents');
    }
};
