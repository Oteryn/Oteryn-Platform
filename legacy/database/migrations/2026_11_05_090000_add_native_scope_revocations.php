<?php

use Illuminate\Database\Migrations\Migration;
use Illuminate\Database\Schema\Blueprint;
use Illuminate\Support\Facades\DB;
use Illuminate\Support\Facades\Schema;

return new class extends Migration
{
    public function up(): void
    {
        Schema::table('native_scope_assignments', function (Blueprint $table): void {
            $table->string('node_identity', 128)->nullable()->change();
            $table->unsignedBigInteger('assigned_at')->nullable()->change();
            $table->unsignedBigInteger('revoked_at')->nullable();
        });
    }

    public function down(): void
    {
        if (DB::table('native_scope_assignments')->whereNotNull('revoked_at')->exists()) {
            throw new LogicException('rollback refused: scope revocations exist');
        }

        Schema::table('native_scope_assignments', function (Blueprint $table): void {
            $table->dropColumn('revoked_at');
            $table->string('node_identity', 128)->nullable(false)->change();
            $table->unsignedBigInteger('assigned_at')->nullable(false)->change();
        });
    }
};
