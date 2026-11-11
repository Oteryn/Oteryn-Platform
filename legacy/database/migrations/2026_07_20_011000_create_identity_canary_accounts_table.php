<?php

use Illuminate\Database\Migrations\Migration;
use Illuminate\Database\Schema\Blueprint;
use Illuminate\Support\Facades\Schema;

return new class extends Migration
{
    public function up(): void
    {
        Schema::create('identity_canary_accounts', function (Blueprint $table): void {
            $table->foreignId('identity_id')
                ->primary()
                ->constrained('identities')
                ->restrictOnDelete();
            $table->unsignedInteger('canary_account_id')->nullable()->unique();
            $table->char('provisioning_name', 32)->unique();
            $table->unsignedInteger('canary_creation_epoch');
            $table->string('status', 20)->default('pending')->index();
            $table->string('last_failure_code', 64)->nullable();
            $table->timestamp('last_attempt_at')->nullable();
            $table->timestamp('ready_at')->nullable();
            $table->timestamps();
        });
    }

    public function down(): void
    {
        Schema::dropIfExists('identity_canary_accounts');
    }
};
