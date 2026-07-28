<?php

use Illuminate\Database\Migrations\Migration;
use Illuminate\Database\Schema\Blueprint;
use Illuminate\Support\Facades\Schema;

return new class extends Migration
{
    public function up(): void
    {
        Schema::table('identities', function (Blueprint $table): void {
            $table->boolean('public_account_association')->default(false);
            $table->boolean('public_status_visible')->default(false);
            $table->timestamp('email_change_available_at')->nullable();
            $table->timestamp('termination_requested_at')->nullable();
            $table->timestamp('termination_scheduled_for')->nullable()->index();
            $table->timestamp('terminated_at')->nullable()->index();
            $table->char('terminated_email_hash', 64)->nullable();
        });

        Schema::create('identity_web_sessions', function (Blueprint $table): void {
            $table->uuid('id')->primary();
            $table->foreignId('identity_id')->constrained('identities')->cascadeOnDelete();
            $table->unsignedBigInteger('generation');
            $table->string('user_agent', 160)->nullable();
            $table->char('ip_hash', 64)->nullable();
            $table->timestamp('issued_at');
            $table->timestamp('last_seen_at');
            $table->timestamp('expires_at')->index();
            $table->timestamp('revoked_at')->nullable()->index();
            $table->timestamps();

            $table->index(['identity_id', 'revoked_at', 'last_seen_at'], 'identity_web_sessions_owner_state_index');
        });

        Schema::create('identity_email_change_requests', function (Blueprint $table): void {
            $table->uuid('id')->primary();
            $table->foreignId('identity_id')->constrained('identities')->cascadeOnDelete();
            $table->string('old_email', 254);
            $table->string('new_email', 254);
            $table->char('verification_token_hash', 64)->unique();
            $table->char('recovery_token_hash', 64)->unique();
            $table->timestamp('requested_at');
            $table->timestamp('expires_at')->index();
            $table->timestamp('confirmed_at')->nullable();
            $table->timestamp('recoverable_until')->nullable();
            $table->timestamp('recovered_at')->nullable();
            $table->timestamp('cancelled_at')->nullable();
            $table->timestamps();

            $table->index(['identity_id', 'requested_at'], 'identity_email_changes_owner_index');
            $table->index(['new_email', 'confirmed_at'], 'identity_email_changes_new_email_index');
        });

        Schema::create('identity_recovery_keys', function (Blueprint $table): void {
            $table->foreignId('identity_id')->primary()->constrained('identities')->cascadeOnDelete();
            $table->char('key_hash', 64);
            $table->timestamp('generated_at');
            $table->timestamp('used_at')->nullable();
            $table->timestamp('revoked_at')->nullable();
            $table->timestamps();
        });
    }

    public function down(): void
    {
        Schema::dropIfExists('identity_recovery_keys');
        Schema::dropIfExists('identity_email_change_requests');
        Schema::dropIfExists('identity_web_sessions');

        Schema::table('identities', function (Blueprint $table): void {
            $table->dropIndex(['termination_scheduled_for']);
            $table->dropIndex(['terminated_at']);
            $table->dropColumn([
                'public_account_association',
                'public_status_visible',
                'email_change_available_at',
                'termination_requested_at',
                'termination_scheduled_for',
                'terminated_at',
                'terminated_email_hash',
            ]);
        });
    }
};
