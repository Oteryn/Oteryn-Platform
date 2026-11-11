<?php

use Illuminate\Database\Migrations\Migration;
use Illuminate\Database\Schema\Blueprint;
use Illuminate\Support\Facades\Schema;

return new class extends Migration
{
    public function up(): void
    {
        Schema::create('identity_security_events', function (Blueprint $table): void {
            $table->id();
            $table->foreignId('identity_id')
                ->nullable()
                ->constrained('identities')
                ->nullOnDelete();
            $table->string('event_type', 100);
            $table->timestamp('occurred_at');
            $table->index(['identity_id', 'event_type']);
        });
    }

    public function down(): void
    {
        Schema::dropIfExists('identity_security_events');
    }
};
