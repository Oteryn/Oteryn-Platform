<?php

use Illuminate\Database\Migrations\Migration;
use Illuminate\Database\Schema\Blueprint;
use Illuminate\Support\Facades\Schema;

return new class extends Migration
{
    public function up(): void
    {
        Schema::create('cast_account_controls', function (Blueprint $table): void {
            $table->string('account_id', 36)->primary();
            $table->foreign('account_id')->references('account_id')->on('identities')->cascadeOnDelete();
        });
        Schema::create('cast_access_policies', function (Blueprint $table): void {
            $table->char('cast_id', 32)->primary();
            $table->uuid('node_id');
            $table->string('owner_account_id', 36);
            $table->foreign('owner_account_id')->references('account_id')->on('identities')->cascadeOnDelete();
            $table->unsignedBigInteger('epoch');
            $table->unsignedBigInteger('access_revision');
            $table->unsignedSmallInteger('delay_seconds')->default(0);
            $table->unsignedSmallInteger('viewer_cap')->default(50);
            $table->string('password_hash')->nullable();
            $table->boolean('password_required')->default(false);
            // A short-lived routing lease, never a durable cast-on preference.
            $table->timestampTz('expires_at')->index();
        });
        Schema::create('cast_viewer_reservations', function (Blueprint $table): void {
            $table->uuid('reservation_id')->primary();
            $table->char('ticket_hash', 64)->unique();
            $table->char('cast_id', 32);
            $table->foreign('cast_id')->references('cast_id')->on('cast_access_policies')->cascadeOnDelete();
            $table->string('account_id', 36);
            $table->foreign('account_id')->references('account_id')->on('identities')->cascadeOnDelete();
            $table->unsignedBigInteger('security_generation');
            $table->unsignedBigInteger('access_revision');
            $table->boolean('consumed')->default(false);
            $table->boolean('chat_selected')->default(false);
            $table->timestampTz('expires_at');
            $table->index(['account_id', 'expires_at']);
            $table->index(['cast_id', 'expires_at']);
        });
    }

    public function down(): void
    {
        Schema::dropIfExists('cast_viewer_reservations');
        Schema::dropIfExists('cast_access_policies');
        Schema::dropIfExists('cast_account_controls');
    }
};
