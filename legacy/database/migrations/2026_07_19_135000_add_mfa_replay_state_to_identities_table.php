<?php

use Illuminate\Database\Migrations\Migration;
use Illuminate\Database\Schema\Blueprint;
use Illuminate\Support\Facades\Schema;

return new class extends Migration
{
    public function up(): void
    {
        Schema::table('identities', function (Blueprint $table): void {
            $table->unsignedBigInteger('two_factor_last_used_timestep')->nullable();
        });
    }

    public function down(): void
    {
        Schema::table('identities', function (Blueprint $table): void {
            $table->dropColumn('two_factor_last_used_timestep');
        });
    }
};
