<?php

use Illuminate\Database\Migrations\Migration;
use Illuminate\Database\Schema\Blueprint;
use Illuminate\Support\Facades\DB;
use Illuminate\Support\Facades\Schema;

/**
 * Epoch lock for the native runtime-status read model (Game oteryn-game-native-runtime-status-v1 §6,
 * login contract §7.2 restore reset). One row, locked exclusively by ReportScopeAssignmentV1 ingestion
 * and shared by ReportRuntimeStatusV1 ingestion, so the highest assignment_epoch cannot change between
 * a runtime report's epoch check and its write. Lock order: this row, then native_scope_assignments,
 * then native_runtime_status_reports. Additive only.
 */
return new class extends Migration
{
    public function up(): void
    {
        Schema::create('native_assignment_epoch_locks', function (Blueprint $table): void {
            $table->unsignedTinyInteger('id')->primary();
        });
        DB::table('native_assignment_epoch_locks')->insert(['id' => 1]);
    }

    public function down(): void
    {
        // The lock serializes epoch raises: refuse before any DDL while assignments exist.
        if (DB::table('native_scope_assignments')->exists()) {
            throw new LogicException('Native scope assignments exist; epoch lock rollback refused before DDL.');
        }

        Schema::drop('native_assignment_epoch_locks');
    }
};
