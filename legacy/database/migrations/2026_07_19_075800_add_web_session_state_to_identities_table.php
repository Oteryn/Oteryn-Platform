<?php

use Illuminate\Database\Migrations\Migration;
use Illuminate\Database\Schema\Blueprint;
use Illuminate\Support\Facades\Schema;

return new class extends Migration
{
    public function up(): void
    {
        Schema::table('identities', function (Blueprint $table): void {
            $table->unsignedBigInteger('web_session_generation')->default(0);
            $table->timestamp('disabled_at')->nullable()->index();
        });
    }

    public function down(): void
    {
        Schema::table('identities', function (Blueprint $table): void {
            $table->dropIndex(['disabled_at']);
            $table->dropColumn(['web_session_generation', 'disabled_at']);
        });
    }
};
