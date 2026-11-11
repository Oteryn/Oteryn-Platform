<?php

use Illuminate\Database\Migrations\Migration;
use Illuminate\Database\Schema\Blueprint;
use Illuminate\Support\Facades\DB;
use Illuminate\Support\Facades\Schema;

/**
 * Native runtime-status read model (OTERYN_V2_NATIVE_GATEWAY_LOGIN_CONTRACT §7.2; Game
 * oteryn-game-native-runtime-status-v1). Additive only. uint64 epochs, generations and revisions are
 * stored as canonical decimal strings. native_scope_assignments is written only by the
 * ReportScopeAssignmentV1 ingestion (follow-up); runtime reports are accepted only against it.
 */
return new class extends Migration
{
    public function up(): void
    {
        Schema::create('native_scope_assignments', function (Blueprint $table): void {
            $table->id();
            $table->char('world_id', 36);
            $table->char('channel_id', 36);
            $table->string('assignment_epoch', 20)->index();
            $table->string('ownership_generation', 20);
            $table->string('node_identity', 128);
            $table->unsignedBigInteger('assigned_at');
            $table->timestamps();

            $table->unique(['world_id', 'channel_id']);
        });

        Schema::create('native_runtime_status_reports', function (Blueprint $table): void {
            $table->id();
            $table->char('world_id', 36);
            $table->char('channel_id', 36);
            $table->string('node_identity', 128);
            $table->string('source_authority', 128);
            $table->char('node_id', 36);
            $table->string('assignment_epoch', 20);
            $table->string('scope_ownership_generation', 20);
            $table->string('source_revision', 20);
            $table->string('decision_identity', 128);
            $table->boolean('ready');
            $table->unsignedBigInteger('published_at');
            $table->unsignedBigInteger('observed_at');
            $table->unsignedTinyInteger('protocol_major');
            $table->unsignedTinyInteger('transport_profile');
            $table->string('route_revision', 64);
            $table->string('runtime_observation_revision', 64);
            $table->string('ruleset_revision', 64);
            $table->string('content_revision', 64);
            $table->string('map_revision', 64);
            $table->string('world_policy_revision', 64);
            $table->string('offer_revision', 64);
            $table->char('content_digest', 64);
            $table->boolean('invalid')->default(false);
            $table->timestamps();

            $table->unique(['world_id', 'channel_id']);
        });
    }

    public function down(): void
    {
        // Ownership and runtime evidence must survive: refuse before any DDL while rows exist.
        if (DB::table('native_scope_assignments')->exists() || DB::table('native_runtime_status_reports')->exists()) {
            throw new LogicException('Native scope assignments and runtime status reports must be retained; rollback refused before DDL.');
        }

        Schema::drop('native_runtime_status_reports');
        Schema::drop('native_scope_assignments');
    }
};
