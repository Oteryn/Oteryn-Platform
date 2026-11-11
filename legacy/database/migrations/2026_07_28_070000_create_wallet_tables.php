<?php

use Illuminate\Database\Migrations\Migration;
use Illuminate\Database\Schema\Blueprint;
use Illuminate\Support\Facades\Schema;

return new class extends Migration
{
    public function up(): void
    {
        Schema::create('wallet_accounts', function (Blueprint $table): void {
            $table->foreignId('identity_id')->primary()->constrained('identities')->cascadeOnDelete();
            $table->unsignedBigInteger('available_balance')->default(0);
            $table->unsignedBigInteger('reserved_balance')->default(0);
            $table->timestamps();
        });

        Schema::create('wallet_ledger_entries', function (Blueprint $table): void {
            $table->id();
            $table->foreignId('identity_id')->constrained('identities')->restrictOnDelete();
            $table->string('operation_type', 48)->index();
            $table->bigInteger('available_delta');
            $table->bigInteger('reserved_delta');
            $table->unsignedBigInteger('auction_id')->nullable()->index();
            $table->string('idempotency_key', 120)->unique();
            $table->json('metadata')->nullable();
            $table->timestamp('created_at');

            $table->index(['identity_id', 'created_at'], 'wallet_ledger_identity_history');
        });
    }

    public function down(): void
    {
        Schema::dropIfExists('wallet_ledger_entries');
        Schema::dropIfExists('wallet_accounts');
    }
};
