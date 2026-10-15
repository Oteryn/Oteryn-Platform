<?php

use Illuminate\Database\Migrations\Migration;
use Illuminate\Database\Schema\Blueprint;
use Illuminate\Support\Facades\DB;
use Illuminate\Support\Facades\Schema;

/**
 * Game restore notices and the Character command intents each one reopened (Platform contract
 * `docs/contracts/OTERYN_V2_GAME_RESTORE_RECONCILIATION_CONTRACT.md`, Game commands contract §5.4).
 * A notice row and its reopen rows are written in one transaction; a reopen row keeps the
 * superseded receipt and later names the one new receipt Game published for that operation.
 */
return new class extends Migration
{
    public function up(): void
    {
        Schema::create('character_restore_notices', function (Blueprint $table): void {
            $table->id();
            $table->char('restore_event_id', 36)->unique('character_restore_notice_event_unique');
            $table->string('source_authority', 128);
            $table->char('restored_generation', 20);
            $table->char('new_recovery_generation', 20);
            $table->text('notice_json');
            $table->char('notice_sha256', 64);
            $table->unsignedInteger('reopened_count');
            // Platform clock, record only: the drill measures received → acknowledged. Nothing selects by it.
            $table->dateTime('received_at');
            $table->dateTime('acknowledged_at')->nullable();
        });

        Schema::create('character_restore_reopens', function (Blueprint $table): void {
            $table->id();
            $table->foreignId('notice_id')->constrained('character_restore_notices')->restrictOnDelete();
            $table->foreignId('intent_id')->constrained('character_command_intents')->restrictOnDelete();
            $table->foreignId('superseded_receipt_id')->constrained('character_command_receipts')->restrictOnDelete();
            $table->foreignId('new_receipt_id')->nullable()->constrained('character_command_receipts')->restrictOnDelete();
            $table->dateTime('created_at');
            $table->unique(['notice_id', 'intent_id'], 'character_restore_reopen_unique');
            $table->index(['intent_id', 'new_receipt_id'], 'character_restore_reopen_open');
        });
    }

    public function down(): void
    {
        // Restore notices and reopens are cross-repository evidence of a Game restore; never erase them silently.
        if ((Schema::hasTable('character_restore_reopens') && DB::table('character_restore_reopens')->exists())
            || (Schema::hasTable('character_restore_notices') && DB::table('character_restore_notices')->exists())) {
            throw new LogicException('Game restore notices exist; rollback refused before DDL.');
        }

        Schema::dropIfExists('character_restore_reopens');
        Schema::dropIfExists('character_restore_notices');
    }
};
