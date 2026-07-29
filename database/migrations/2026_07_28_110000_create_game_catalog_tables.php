<?php

use Illuminate\Database\Migrations\Migration;
use Illuminate\Database\Schema\Blueprint;
use Illuminate\Support\Facades\Schema;

return new class extends Migration
{
    public function up(): void
    {
        Schema::create('game_catalog_releases', function (Blueprint $table): void {
            $table->id();
            $table->string('key', 32)->unique();
            $table->string('display_label', 48);
            $table->unsignedSmallInteger('major');
            $table->unsignedSmallInteger('minor');
            $table->unsignedSmallInteger('patch');
            $table->unsignedBigInteger('build')->nullable();
            $table->unsignedBigInteger('release_order')->unique();
            $table->string('protocol_family', 80)->nullable();
            $table->timestamp('released_at')->nullable();
            $table->timestamps();
        });

        Schema::create('game_catalog_snapshots', function (Blueprint $table): void {
            $table->id();
            $table->string('contract_version', 80);
            $table->string('schema_version', 32);
            $table->char('content_sha256', 64)->unique();
            $table->string('canary_commit_sha', 64);
            $table->string('datapack_commit_sha', 64)->nullable();
            $table->string('protocol_profile', 80);
            $table->foreignId('runtime_release_id')->constrained('game_catalog_releases')->restrictOnDelete();
            $table->foreignId('content_target_release_id')->constrained('game_catalog_releases')->restrictOnDelete();
            $table->foreignId('verified_content_through_release_id');
            $table->foreign('verified_content_through_release_id', 'game_catalog_snapshots_verified_release_fk')
                ->references('id')
                ->on('game_catalog_releases')
                ->restrictOnDelete();
            $table->foreignId('contains_content_through_release_id')->nullable();
            $table->foreign('contains_content_through_release_id', 'game_catalog_snapshots_contains_release_fk')
                ->references('id')
                ->on('game_catalog_releases')
                ->restrictOnDelete();
            $table->char('appearances_sha256', 64);
            $table->char('map_sha256', 64)->nullable();
            $table->string('producer_build_id', 160)->nullable();
            $table->timestamp('generated_at');
            $table->timestamp('imported_at');
            $table->string('status', 24)->index();
            $table->unsignedInteger('entity_count');
            $table->unsignedInteger('relation_count');
            $table->json('validation_summary');
            $table->timestamps();
        });

        Schema::create('game_catalog_profiles', function (Blueprint $table): void {
            $table->id();
            $table->string('key', 80)->unique();
            $table->string('name', 160);
            $table->foreignId('target_release_id')->constrained('game_catalog_releases')->restrictOnDelete();
            $table->foreignId('active_snapshot_id')->nullable()->constrained('game_catalog_snapshots')->restrictOnDelete();
            $table->boolean('complete_only')->default(true);
            $table->boolean('public_enabled')->default(false);
            $table->boolean('allow_backports')->default(false);
            $table->unsignedBigInteger('lock_version')->default(0);
            $table->timestamps();
        });

        Schema::create('game_catalog_entities', function (Blueprint $table): void {
            $table->id();
            $table->string('entity_type', 32);
            $table->string('canonical_key', 180);
            $table->timestamps();
            $table->unique(['entity_type', 'canonical_key'], 'game_catalog_entities_type_key_unique');
            $table->unique('canonical_key', 'game_catalog_entities_canonical_key_unique');
        });

        Schema::create('game_catalog_entity_snapshots', function (Blueprint $table): void {
            $table->id();
            $table->foreignId('snapshot_id')->constrained('game_catalog_snapshots')->cascadeOnDelete();
            $table->foreignId('entity_id')->constrained('game_catalog_entities')->restrictOnDelete();
            $table->foreignId('introduced_release_id')->nullable()->constrained('game_catalog_releases')->restrictOnDelete();
            $table->foreignId('removed_release_id')->nullable()->constrained('game_catalog_releases')->restrictOnDelete();
            $table->string('completeness', 32);
            $table->string('availability', 32);
            $table->boolean('runtime_present');
            $table->boolean('enabled');
            $table->char('data_sha256', 64);
            $table->string('source_path', 512)->nullable();
            $table->string('source_key', 180)->nullable();
            $table->timestamps();
            $table->unique(['snapshot_id', 'entity_id'], 'game_catalog_entity_snapshots_unique');
            $table->index(['snapshot_id', 'completeness', 'availability'], 'game_catalog_entity_visibility_index');
        });

        Schema::create('game_catalog_entity_identifiers', function (Blueprint $table): void {
            $table->id();
            $table->foreignId('snapshot_id')->constrained('game_catalog_snapshots')->cascadeOnDelete();
            $table->foreignId('entity_id')->constrained('game_catalog_entities')->restrictOnDelete();
            $table->string('namespace', 80);
            $table->string('value', 160);
            $table->timestamp('created_at');
            $table->unique(['snapshot_id', 'namespace', 'value'], 'game_catalog_identifier_snapshot_value_unique');
            $table->unique(['snapshot_id', 'entity_id', 'namespace', 'value'], 'game_catalog_identifier_entity_unique');
        });

        Schema::create('game_catalog_item_snapshots', function (Blueprint $table): void {
            $table->foreignId('entity_snapshot_id')->primary()->constrained('game_catalog_entity_snapshots')->cascadeOnDelete();
            $table->unsignedInteger('server_id');
            $table->unsignedBigInteger('client_id')->nullable();
            $table->unsignedInteger('ware_id')->nullable();
            $table->string('name', 200);
            $table->text('description')->nullable();
            $table->string('category', 80)->index();
            $table->string('weapon_type', 40)->nullable()->index();
            $table->smallInteger('attack')->nullable();
            $table->smallInteger('defense')->nullable();
            $table->smallInteger('extra_defense')->nullable();
            $table->smallInteger('armor')->nullable();
            $table->unsignedSmallInteger('range')->nullable();
            $table->unsignedBigInteger('weight')->nullable();
            $table->unsignedInteger('minimum_level')->nullable();
            $table->json('vocations')->nullable();
            $table->unsignedBigInteger('slot_position')->nullable();
            $table->unsignedSmallInteger('imbuement_slots')->nullable();
            $table->unsignedSmallInteger('upgrade_classification')->nullable();
            $table->string('element_type', 40)->nullable();
            $table->smallInteger('element_value')->nullable();
            $table->boolean('stackable');
            $table->boolean('pickupable');
            $table->string('image_key', 160)->nullable();
            $table->json('attributes');
            $table->index(['category', 'weapon_type', 'name'], 'game_catalog_item_filter_index');
        });

        Schema::create('game_catalog_creature_snapshots', function (Blueprint $table): void {
            $table->foreignId('entity_snapshot_id')->primary()->constrained('game_catalog_entity_snapshots')->cascadeOnDelete();
            $table->string('name', 200)->index();
            $table->text('description')->nullable();
            $table->unsignedInteger('race_id')->nullable();
            $table->unsignedInteger('look_type')->nullable();
            $table->unsignedBigInteger('health');
            $table->unsignedBigInteger('max_health');
            $table->unsignedBigInteger('experience');
            $table->unsignedInteger('speed');
            $table->smallInteger('armor');
            $table->smallInteger('defense');
            $table->decimal('mitigation', 8, 3)->nullable();
            $table->boolean('is_boss')->index();
            $table->boolean('is_reward_boss');
            $table->string('bestiary_class', 120)->nullable()->index();
            $table->string('bestiary_race', 120)->nullable();
            $table->unsignedSmallInteger('bestiary_occurrence')->nullable();
            $table->unsignedInteger('bestiary_to_kill')->nullable();
            $table->unsignedInteger('charm_points')->nullable();
            $table->json('elements');
            $table->json('immunities');
            $table->json('attacks');
            $table->json('defenses');
            $table->json('attributes');
        });

        Schema::create('game_catalog_relation_snapshots', function (Blueprint $table): void {
            $table->id();
            $table->foreignId('snapshot_id')->constrained('game_catalog_snapshots')->cascadeOnDelete();
            $table->string('relation_type', 32);
            $table->string('canonical_key', 240);
            $table->foreignId('source_entity_id')->constrained('game_catalog_entities')->restrictOnDelete();
            $table->foreignId('target_entity_id')->nullable()->constrained('game_catalog_entities')->restrictOnDelete();
            $table->foreignId('introduced_release_id')->nullable()->constrained('game_catalog_releases')->restrictOnDelete();
            $table->foreignId('removed_release_id')->nullable()->constrained('game_catalog_releases')->restrictOnDelete();
            $table->string('completeness', 32);
            $table->boolean('enabled');
            $table->char('data_sha256', 64);
            $table->string('source_path', 512)->nullable();
            $table->json('attributes');
            $table->timestamps();
            $table->unique(['snapshot_id', 'relation_type', 'canonical_key'], 'game_catalog_relation_snapshots_unique');
            $table->index(['snapshot_id', 'source_entity_id', 'target_entity_id'], 'game_catalog_relation_endpoints_index');
        });

        Schema::create('game_catalog_loot_snapshots', function (Blueprint $table): void {
            $table->foreignId('relation_snapshot_id')->primary()->constrained('game_catalog_relation_snapshots')->cascadeOnDelete();
            $table->unsignedBigInteger('chance_numerator');
            $table->unsignedBigInteger('chance_denominator');
            $table->unsignedInteger('minimum_count');
            $table->unsignedInteger('maximum_count');
            $table->string('container_path', 512)->nullable();
            $table->json('condition_data')->nullable();
        });

        Schema::create('game_catalog_import_runs', function (Blueprint $table): void {
            $table->id();
            $table->char('content_sha256', 64)->index();
            $table->foreignId('snapshot_id')->nullable()->constrained('game_catalog_snapshots')->nullOnDelete();
            $table->string('status', 24)->index();
            $table->string('source_label', 255);
            $table->unsignedBigInteger('file_size');
            $table->unsignedInteger('finding_count')->default(0);
            $table->timestamp('started_at');
            $table->timestamp('finished_at')->nullable();
            $table->json('summary');
            $table->timestamps();
        });

        Schema::create('game_catalog_validation_findings', function (Blueprint $table): void {
            $table->id();
            $table->foreignId('import_run_id')->nullable()->constrained('game_catalog_import_runs')->cascadeOnDelete();
            $table->foreignId('snapshot_id')->nullable()->constrained('game_catalog_snapshots')->cascadeOnDelete();
            $table->string('severity', 16);
            $table->string('code', 80);
            $table->string('path', 512)->nullable();
            $table->string('message', 1000);
            $table->json('context')->nullable();
            $table->timestamp('created_at');
            $table->index(['snapshot_id', 'severity'], 'game_catalog_findings_snapshot_index');
        });

        Schema::create('game_catalog_profile_entities', function (Blueprint $table): void {
            $table->foreignId('profile_id')->constrained('game_catalog_profiles')->cascadeOnDelete();
            $table->foreignId('entity_snapshot_id')->constrained('game_catalog_entity_snapshots')->cascadeOnDelete();
            $table->boolean('visible')->index();
            $table->string('reason_code', 48)->index();
            $table->timestamp('computed_at');
            $table->primary(['profile_id', 'entity_snapshot_id']);
        });

        Schema::create('game_catalog_profile_relations', function (Blueprint $table): void {
            $table->foreignId('profile_id')->constrained('game_catalog_profiles')->cascadeOnDelete();
            $table->foreignId('relation_snapshot_id')->constrained('game_catalog_relation_snapshots')->cascadeOnDelete();
            $table->boolean('visible')->index();
            $table->string('reason_code', 48)->index();
            $table->timestamp('computed_at');
            $table->primary(['profile_id', 'relation_snapshot_id']);
        });

        Schema::create('game_catalog_entity_translations', function (Blueprint $table): void {
            $table->id();
            $table->foreignId('entity_id')->constrained('game_catalog_entities')->cascadeOnDelete();
            $table->string('locale', 12);
            $table->string('display_name', 200);
            $table->string('slug', 200);
            $table->text('summary')->nullable();
            $table->longText('description_markdown')->nullable();
            $table->char('source_name_sha256', 64);
            $table->string('translation_status', 24);
            $table->timestamps();
            $table->unique(['entity_id', 'locale'], 'game_catalog_translation_entity_locale_unique');
            $table->unique(['locale', 'slug'], 'game_catalog_translation_locale_slug_unique');
        });

        Schema::create('game_catalog_wiki_links', function (Blueprint $table): void {
            $table->foreignId('entity_id')->constrained('game_catalog_entities')->cascadeOnDelete();
            $table->unsignedBigInteger('wiki_article_id');
            $table->string('link_type', 32);
            $table->unsignedInteger('sort_order')->default(0);
            $table->primary(['entity_id', 'wiki_article_id', 'link_type'], 'game_catalog_wiki_links_primary');
            $table->index('wiki_article_id');
        });

        Schema::create('game_catalog_profile_overrides', function (Blueprint $table): void {
            $table->id();
            $table->foreignId('profile_id')->constrained('game_catalog_profiles')->cascadeOnDelete();
            $table->foreignId('entity_id')->nullable()->constrained('game_catalog_entities')->cascadeOnDelete();
            $table->foreignId('relation_snapshot_id')->nullable()->constrained('game_catalog_relation_snapshots')->cascadeOnDelete();
            $table->string('action', 32);
            $table->string('reason', 500);
            $table->unsignedBigInteger('approved_by_identity_id');
            $table->timestamp('approved_at');
            $table->timestamp('expires_at')->nullable();
            $table->timestamps();
            $table->index(['profile_id', 'entity_id'], 'game_catalog_override_entity_index');
            $table->index(['profile_id', 'relation_snapshot_id'], 'game_catalog_override_relation_index');
        });

        Schema::create('game_catalog_audit_events', function (Blueprint $table): void {
            $table->id();
            $table->foreignId('profile_id')->nullable()->constrained('game_catalog_profiles')->nullOnDelete();
            $table->foreignId('snapshot_id')->nullable()->constrained('game_catalog_snapshots')->nullOnDelete();
            $table->string('action', 80)->index();
            $table->unsignedBigInteger('actor_identity_id')->nullable();
            $table->json('metadata');
            $table->timestamp('created_at');
        });
    }

    public function down(): void
    {
        Schema::dropIfExists('game_catalog_audit_events');
        Schema::dropIfExists('game_catalog_profile_overrides');
        Schema::dropIfExists('game_catalog_wiki_links');
        Schema::dropIfExists('game_catalog_entity_translations');
        Schema::dropIfExists('game_catalog_profile_relations');
        Schema::dropIfExists('game_catalog_profile_entities');
        Schema::dropIfExists('game_catalog_validation_findings');
        Schema::dropIfExists('game_catalog_import_runs');
        Schema::dropIfExists('game_catalog_loot_snapshots');
        Schema::dropIfExists('game_catalog_relation_snapshots');
        Schema::dropIfExists('game_catalog_creature_snapshots');
        Schema::dropIfExists('game_catalog_item_snapshots');
        Schema::dropIfExists('game_catalog_entity_identifiers');
        Schema::dropIfExists('game_catalog_entity_snapshots');
        Schema::dropIfExists('game_catalog_entities');
        Schema::dropIfExists('game_catalog_profiles');
        Schema::dropIfExists('game_catalog_snapshots');
        Schema::dropIfExists('game_catalog_releases');
    }
};
