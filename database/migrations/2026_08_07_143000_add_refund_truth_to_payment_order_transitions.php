<?php

use Illuminate\Database\Migrations\Migration;
use Illuminate\Database\Schema\Blueprint;
use Illuminate\Support\Facades\Schema;

return new class extends Migration
{
    public function up(): void
    {
        Schema::table('payment_order_transitions', function (Blueprint $table): void {
            $table->unsignedBigInteger('verified_refund_amount_minor')->nullable();
            $table->unsignedBigInteger('refunded_total_minor')->nullable();
        });
    }

    public function down(): void
    {
        throw new RuntimeException(
            'Refund settlement truth is forward-only and cannot be removed by migration rollback.',
        );
    }
};
