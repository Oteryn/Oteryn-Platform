<?php

use Illuminate\Database\Migrations\Migration;
use Illuminate\Database\Schema\Blueprint;
use Illuminate\Support\Facades\DB;
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
        $hasRefundEvidence = DB::table('payment_order_transitions')
            ->whereNotNull('verified_refund_amount_minor')
            ->orWhereNotNull('refunded_total_minor')
            ->exists();

        if ($hasRefundEvidence) {
            throw new RuntimeException(
                'Refund settlement truth is populated and cannot be removed by migration rollback.',
            );
        }

        Schema::table('payment_order_transitions', function (Blueprint $table): void {
            $table->dropColumn([
                'verified_refund_amount_minor',
                'refunded_total_minor',
            ]);
        });
    }
};
