<?php

use Illuminate\Database\Migrations\Migration;
use Illuminate\Database\Schema\Blueprint;
use Illuminate\Support\Facades\Schema;

return new class extends Migration
{
    public function up(): void
    {
        Schema::create('payment_orders', function (Blueprint $table): void {
            $table->id();
            $table->uuid('public_id')->unique();
            $table->foreignId('identity_id')->constrained('identities')->restrictOnDelete();
            $table->string('provider', 64);
            $table->char('currency', 3);
            $table->unsignedBigInteger('amount_minor');
            $table->string('status', 32)->index();
            $table->string('idempotency_key', 120)->unique();
            $table->unsignedInteger('version')->default(1);
            $table->timestamps();

            $table->index(['identity_id', 'created_at'], 'payment_orders_identity_history');
        });

        Schema::create('payment_attempts', function (Blueprint $table): void {
            $table->id();
            $table->uuid('public_id')->unique();
            $table->foreignId('payment_order_id')->constrained('payment_orders')->restrictOnDelete();
            $table->string('provider', 64);
            $table->string('status', 32)->index();
            $table->string('provider_checkout_reference', 160)->nullable();
            $table->string('idempotency_key', 120)->unique();
            $table->string('sanitized_error_code', 64)->nullable();
            $table->timestamps();

            $table->index(['payment_order_id', 'created_at'], 'payment_attempts_order_history');
        });

        Schema::create('payment_provider_events', function (Blueprint $table): void {
            $table->id();
            $table->foreignId('payment_order_id')->nullable()->constrained('payment_orders')->restrictOnDelete();
            $table->string('provider', 64);
            $table->string('provider_event_id', 120);
            $table->string('event_type', 80)->index();
            $table->string('provider_object_reference', 120)->nullable();
            $table->char('payload_sha256', 64);
            $table->unsignedBigInteger('signature_timestamp');
            $table->string('processing_state', 32)->index();
            $table->string('failure_code', 64)->nullable();
            $table->json('metadata')->nullable();
            $table->timestamp('received_at');
            $table->timestamp('processed_at')->nullable();

            $table->unique(['provider', 'provider_event_id'], 'payment_provider_event_unique');
            $table->index(['payment_order_id', 'received_at'], 'payment_events_order_history');
        });

        Schema::create('payment_order_transitions', function (Blueprint $table): void {
            $table->id();
            $table->foreignId('payment_order_id')->constrained('payment_orders')->restrictOnDelete();
            $table->foreignId('payment_provider_event_id')->nullable()->constrained('payment_provider_events')->restrictOnDelete();
            $table->string('from_status', 32)->nullable();
            $table->string('to_status', 32);
            $table->string('reason', 64);
            $table->unsignedInteger('version');
            $table->timestamp('created_at');

            $table->unique(['payment_order_id', 'version'], 'payment_order_transition_version');
        });

        Schema::create('payment_reconciliation_entries', function (Blueprint $table): void {
            $table->id();
            $table->foreignId('payment_order_id')->nullable()->constrained('payment_orders')->restrictOnDelete();
            $table->foreignId('payment_provider_event_id')->nullable()->constrained('payment_provider_events')->restrictOnDelete();
            $table->string('issue_type', 64)->index();
            $table->string('state', 24)->index();
            $table->json('metadata')->nullable();
            $table->timestamp('created_at');
            $table->timestamp('resolved_at')->nullable();

            $table->index(['payment_order_id', 'state'], 'payment_reconciliation_order_state');
        });
    }

    public function down(): void
    {
        Schema::dropIfExists('payment_reconciliation_entries');
        Schema::dropIfExists('payment_order_transitions');
        Schema::dropIfExists('payment_provider_events');
        Schema::dropIfExists('payment_attempts');
        Schema::dropIfExists('payment_orders');
    }
};
