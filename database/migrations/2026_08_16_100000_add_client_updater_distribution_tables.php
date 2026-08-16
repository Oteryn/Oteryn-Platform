<?php

use Illuminate\Database\Migrations\Migration;
use Illuminate\Database\Schema\Blueprint;
use Illuminate\Support\Facades\Schema;

return new class extends Migration
{
    public function up(): void
    {
        Schema::table('client_releases', function (Blueprint $table): void {
            $table->string('updater_release_id', 64)->nullable();
            $table->unsignedBigInteger('updater_sequence')->nullable();
            $table->timestamp('updater_enabled_at')->nullable();
            $table->timestamp('updater_withdrawn_at')->nullable();

            $table->unique('updater_release_id', 'client_releases_updater_release_id_unique');
            $table->unique(
                ['channel', 'updater_sequence'],
                'client_releases_updater_sequence_unique',
            );
        });

        Schema::table('client_release_artifacts', function (Blueprint $table): void {
            $table->string('updater_target_path', 512)->nullable();
            $table->unique('updater_target_path', 'client_release_artifacts_updater_target_unique');
        });

        Schema::create('client_update_policies', function (Blueprint $table): void {
            $table->id();
            $table->uuid('operation_id')->unique();
            $table->string('channel', 16);
            $table->unsignedBigInteger('revision');
            $table->foreignId('current_release_id')
                ->constrained('client_releases')
                ->restrictOnDelete();
            $table->unsignedBigInteger('current_release_sequence');
            $table->unsignedBigInteger('minimum_supported_release_sequence');
            $table->string('update_mode', 16);
            $table->json('artifact_targets');
            $table->json('revoked_release_ids');
            $table->json('revoked_artifact_targets');
            $table->string('rollback_authorization', 16);
            $table->string('policy_target_path', 255);
            $table->char('policy_document_sha256', 64);
            $table->unsignedBigInteger('policy_document_length');
            $table->timestamp('approved_at');
            $table->timestamps();

            $table->unique(['channel', 'revision'], 'client_update_policies_channel_revision_unique');
            $table->index(['channel', 'approved_at'], 'client_update_policies_channel_approved_index');
        });

        Schema::create('client_update_generations', function (Blueprint $table): void {
            $table->id();
            $table->string('generation_id', 128)->unique();
            $table->foreignId('policy_id')
                ->constrained('client_update_policies')
                ->restrictOnDelete();
            $table->string('channel', 16);
            $table->unsignedBigInteger('root_version');
            $table->unsignedBigInteger('targets_version');
            $table->unsignedBigInteger('snapshot_version');
            $table->unsignedBigInteger('timestamp_version');
            $table->timestamp('metadata_expires_at');
            $table->char('metadata_set_sha256', 64)->unique();
            $table->string('policy_target_path', 255);
            $table->char('policy_target_sha256', 64);
            $table->unsignedBigInteger('policy_target_length');
            $table->json('targets');
            $table->timestamp('reconciled_at');
            $table->timestamp('activated_at')->nullable();
            $table->timestamp('superseded_at')->nullable();
            $table->timestamps();

            $table->unique(
                ['channel', 'timestamp_version'],
                'client_update_generations_channel_timestamp_unique',
            );
            $table->index(
                ['channel', 'activated_at', 'superseded_at'],
                'client_update_generations_active_lookup',
            );
        });
    }

    public function down(): void
    {
        Schema::dropIfExists('client_update_generations');
        Schema::dropIfExists('client_update_policies');

        Schema::table('client_release_artifacts', function (Blueprint $table): void {
            $table->dropUnique('client_release_artifacts_updater_target_unique');
            $table->dropColumn('updater_target_path');
        });

        Schema::table('client_releases', function (Blueprint $table): void {
            $table->dropUnique('client_releases_updater_release_id_unique');
            $table->dropUnique('client_releases_updater_sequence_unique');
            $table->dropColumn([
                'updater_release_id',
                'updater_sequence',
                'updater_enabled_at',
                'updater_withdrawn_at',
            ]);
        });
    }
};