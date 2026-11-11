<?php

use Illuminate\Database\Migrations\Migration;
use Illuminate\Database\Schema\Blueprint;
use Illuminate\Support\Facades\Schema;

return new class extends Migration
{
    public function up(): void
    {
        Schema::create('game_login_tickets', function (Blueprint $table): void {
            $table->id();
            $table->char('ticket_hash', 64)->unique();
            $table->foreignId('identity_id')
                ->constrained('identities')
                ->restrictOnDelete();
            $table->unsignedInteger('canary_account_id');
            $table->string('audience', 100);
            $table->unsignedBigInteger('security_generation');
            $table->timestamp('expires_at')->index();
            $table->timestamp('used_at')->nullable()->index();
            $table->timestamp('created_at')->useCurrent();

            $table->index(['identity_id', 'expires_at']);
        });
    }

    public function down(): void
    {
        Schema::dropIfExists('game_login_tickets');
    }
};
