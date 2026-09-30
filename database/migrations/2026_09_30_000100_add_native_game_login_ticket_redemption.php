<?php

use Illuminate\Database\Migrations\Migration;
use Illuminate\Database\Schema\Blueprint;
use Illuminate\Support\Facades\DB;
use Illuminate\Support\Facades\Schema;

/**
 * Native Game Login Ticket kind and the attempt_ref idempotency record
 * (OTERYN_V2_NATIVE_GATEWAY_LOGIN_CONTRACT §4, §6). Additive: existing Canary tickets keep
 * their canary_account_id; a native ticket stores the canonical AccountId instead.
 */
return new class extends Migration
{
    public function up(): void
    {
        Schema::table('game_login_tickets', function (Blueprint $table): void {
            $table->unsignedInteger('canary_account_id')->nullable()->change();
            $table->char('account_id', 36)->nullable()->after('canary_account_id');
            $table->unsignedBigInteger('native_security_generation')->nullable()->after('security_generation');
            $table->char('attempt_ref', 36)->nullable()->after('used_at');
        });

        Schema::create('native_admission_attempts', function (Blueprint $table): void {
            $table->id();
            $table->char('attempt_ref', 36)->unique();
            $table->char('ticket_hash', 64);
            $table->char('account_id', 36);
            $table->char('character_id', 36);
            $table->char('requested_channel_id', 36)->nullable();
            $table->char('world_id', 36);
            $table->char('channel_id', 36);
            $table->char('offer_digest', 64);
            // Exact JWS signing input for deterministic re-signing; no signature and no token (§6.1).
            // Retirement to an audit row (§6.3) waits for the retention decision (U13).
            $table->text('signing_input')->nullable();
            $table->string('key_id', 64);
            $table->unsignedBigInteger('issued_at');
            $table->unsignedBigInteger('expires_at')->index();
            $table->timestamp('created_at')->useCurrent();

            $table->index(['account_id', 'created_at']);
        });
    }

    public function down(): void
    {
        Schema::dropIfExists('native_admission_attempts');

        // Native tickets live at most 60 seconds and cannot exist without canary_account_id after rollback.
        DB::table('game_login_tickets')->whereNull('canary_account_id')->delete();

        Schema::table('game_login_tickets', function (Blueprint $table): void {
            $table->dropColumn(['account_id', 'native_security_generation', 'attempt_ref']);
        });
        Schema::table('game_login_tickets', function (Blueprint $table): void {
            $table->unsignedInteger('canary_account_id')->nullable(false)->change();
        });
    }
};
