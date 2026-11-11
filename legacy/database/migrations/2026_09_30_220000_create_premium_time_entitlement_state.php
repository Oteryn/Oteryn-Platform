<?php

use Illuminate\Database\Migrations\Migration;
use Illuminate\Database\Schema\Blueprint;
use Illuminate\Support\Facades\DB;
use Illuminate\Support\Facades\Schema;

/**
 * oteryn.premium_time version 1 (OTERYN_V2_PREMIUM_TIME_SNAPSHOT_CONTRACT.md). Times are Unix seconds so
 * interval, lease and refresh arithmetic is exact integer arithmetic in UTC.
 */
return new class extends Migration
{
    public function up(): void
    {
        // One row per account; every grant, revocation and snapshot locks it (contract 6.4).
        Schema::create('premium_time_authority', function (Blueprint $table): void {
            $table->foreignId('identity_id')->primary()->constrained('identities')->restrictOnDelete();
            $table->unsignedBigInteger('authority_revision')->default(0);
            $table->timestamp('updated_at');
        });

        Schema::create('premium_time_entitlements', function (Blueprint $table): void {
            $table->char('id', 36)->primary();
            $table->foreignId('identity_id')->unique()->constrained('identities')->restrictOnDelete();
            $table->string('product_id', 64);
            $table->unsignedSmallInteger('product_version');
            $table->string('state', 16);
            $table->unsignedBigInteger('lifecycle_revision');
            $table->unsignedBigInteger('effective_from');
            $table->unsignedBigInteger('effective_until');
            $table->timestamps();
        });

        // Append-only grant/revocation history; request_id is the operator idempotency key.
        Schema::create('premium_time_entitlement_events', function (Blueprint $table): void {
            $table->id();
            $table->char('entitlement_id', 36);
            $table->foreign('entitlement_id', 'premium_time_event_entitlement_fk')
                ->references('id')
                ->on('premium_time_entitlements')
                ->restrictOnDelete();
            $table->foreignId('identity_id')->constrained('identities')->restrictOnDelete();
            $table->foreignId('actor_identity_id')->constrained('identities')->restrictOnDelete();
            $table->char('request_id', 36)->unique();
            $table->string('event_type', 16);
            $table->string('issuance_source', 32);
            $table->unsignedSmallInteger('duration_days')->nullable();
            $table->char('reason_sha256', 64);
            $table->unsignedBigInteger('lifecycle_revision');
            $table->unsignedBigInteger('effective_from');
            $table->unsignedBigInteger('effective_until');
            $table->timestamp('created_at');

            $table->unique(['entitlement_id', 'lifecycle_revision'], 'premium_time_event_revision');
        });
    }

    public function down(): void
    {
        if (DB::table('premium_time_entitlement_events')->exists()
            || DB::table('premium_time_entitlements')->exists()
            || DB::table('premium_time_authority')->exists()) {
            throw new RuntimeException('Premium time entitlement state exists and cannot be removed by migration rollback.');
        }

        Schema::dropIfExists('premium_time_entitlement_events');
        Schema::dropIfExists('premium_time_entitlements');
        Schema::dropIfExists('premium_time_authority');
    }
};
