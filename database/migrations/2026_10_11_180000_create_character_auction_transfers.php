<?php

use Illuminate\Database\Migrations\Migration;
use Illuminate\Database\Schema\Blueprint;
use Illuminate\Support\Facades\DB;
use Illuminate\Support\Facades\Schema;

/**
 * Bazaar ownership-transfer ledger (BAZAAR-NATIVE-1). Each row binds one saga step attempt
 * (auction, purpose, attempt) to exactly one Character Authority `TransferCharacterOwnership`
 * operation and its binding. Rows are append-only: the outcome is always Game's stored receipt for
 * that operation, never a column here.
 */
return new class extends Migration
{
    public function up(): void
    {
        Schema::create('character_auction_transfers', function (Blueprint $table): void {
            $table->id();
            // No foreign key yet: the auction row still requires the legacy engine columns. The
            // native saga packet adds the constraint once auctions are written with native ids.
            $table->unsignedBigInteger('auction_id');
            $table->string('purpose', 32);
            $table->unsignedSmallInteger('attempt');
            $table->char('operation_id', 36);
            $table->char('character_id', 36);
            $table->char('from_account_id', 36);
            $table->char('to_account_id', 36);
            $table->dateTime('created_at');
            $table->unique(['auction_id', 'purpose', 'attempt'], 'character_auction_transfer_step_unique');
            $table->unique('operation_id', 'character_auction_transfer_operation_unique');
            $table->index('character_id', 'character_auction_transfer_character');
        });
    }

    public function down(): void
    {
        // The ledger is the only link from a Bazaar sale to the Game ownership operation; never erase it silently.
        if (Schema::hasTable('character_auction_transfers') && DB::table('character_auction_transfers')->exists()) {
            throw new LogicException('Bazaar ownership transfers exist; rollback refused before DDL.');
        }

        Schema::dropIfExists('character_auction_transfers');
    }
};
