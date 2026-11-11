<?php

use Illuminate\Database\Migrations\Migration;
use Illuminate\Database\Schema\Blueprint;
use Illuminate\Support\Facades\Schema;

return new class extends Migration
{
    public function up(): void
    {
        Schema::create('admin_audit_events', function (Blueprint $table): void {
            $table->id();
            $table->foreignId('actor_identity_id')->nullable()->constrained('identities')->nullOnDelete();
            $table->string('action', 96)->index();
            $table->string('target_type', 80);
            $table->string('target_id', 191)->nullable();
            $table->json('metadata')->nullable();
            $table->timestamp('occurred_at')->index();
        });
    }

    public function down(): void
    {
        Schema::dropIfExists('admin_audit_events');
    }
};
