<?php

use Illuminate\Database\Migrations\Migration;
use Illuminate\Database\Schema\Blueprint;
use Illuminate\Support\Facades\DB;
use Illuminate\Support\Facades\Schema;

return new class extends Migration
{
    public function up(): void
    {
        Schema::create('native_device_session_families', function (Blueprint $table): void {
            $table->uuid('id')->primary();
            $table->foreignId('identity_id')->constrained('identities')->restrictOnDelete();
            $table->uuid('account_id');
            $table->foreignUuid('oauth_client_id')->constrained('oauth_clients')->restrictOnDelete();
            $table->char('enrollment_access_token_id', 80)->unique('native_device_enrollment_unique');
            $table->unsignedBigInteger('game_auth_generation');
            $table->unsignedBigInteger('native_security_generation');
            $table->unsignedBigInteger('current_sequence');
            $table->dateTime('absolute_expires_at');
            $table->dateTime('idle_expires_at');
            $table->dateTime('revoked_at')->nullable();
            $table->string('revocation_reason', 32)->nullable();
            $table->timestamps();
            $table->index(['identity_id', 'revoked_at'], 'native_device_owner_revocation');
        });

        Schema::create('native_device_session_credentials', function (Blueprint $table): void {
            $table->char('token_hash', 64)->primary();
            $table->foreignUuid('family_id')->constrained('native_device_session_families')->restrictOnDelete();
            $table->unsignedBigInteger('sequence');
            $table->dateTime('consumed_at')->nullable();
            $table->dateTime('issued_at');
            $table->unique(['family_id', 'sequence'], 'native_device_sequence_unique');
        });
    }

    public function down(): void
    {
        // A rollback must not silently erase replay/revocation history or reset authentication.
        if ((Schema::hasTable('native_device_session_credentials') && DB::table('native_device_session_credentials')->exists())
            || (Schema::hasTable('native_device_session_families') && DB::table('native_device_session_families')->exists())) {
            throw new RuntimeException('Native device session history requires an explicitly authorized retirement before rollback.');
        }

        Schema::dropIfExists('native_device_session_credentials');
        Schema::dropIfExists('native_device_session_families');
    }
};
