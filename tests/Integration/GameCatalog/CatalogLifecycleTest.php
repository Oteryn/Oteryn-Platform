<?php

namespace Tests\Integration\GameCatalog;

use App\GameCatalog\Application\Import\CatalogImporter;
use App\GameCatalog\Application\Profiles\CatalogProfileActivator;
use App\GameCatalog\Validation\CatalogValidationException;
use Illuminate\Foundation\Testing\RefreshDatabase;
use Illuminate\Support\Facades\DB;
use Illuminate\Support\Facades\Schema;
use RuntimeException;
use Tests\TestCase;

final class CatalogLifecycleTest extends TestCase
{
    use RefreshDatabase;

    /** @var list<string> */
    private array $temporaryFiles = [];

    protected function tearDown(): void
    {
        foreach ($this->temporaryFiles as $path) {
            @unlink($path);
            @unlink($path.'.sha256');
        }

        parent::tearDown();
    }

    public function test_game_catalog_migrations_are_reversible(): void
    {
        self::assertTrue(Schema::hasTable('game_catalog_snapshots'));
        self::assertTrue(Schema::hasTable('game_catalog_snapshot_releases'));
        self::assertTrue(Schema::hasTable('game_catalog_profile_relations'));

        $membership = require base_path('database/migrations/2026_07_28_083200_create_game_catalog_snapshot_releases.php');
        $catalog = require base_path('database/migrations/2026_07_28_083000_create_game_catalog_tables.php');

        $membership->down();
        $catalog->down();

        self::assertFalse(Schema::hasTable('game_catalog_snapshot_releases'));
        self::assertFalse(Schema::hasTable('game_catalog_snapshots'));
        self::assertFalse(Schema::hasTable('game_catalog_profile_relations'));

        $catalog->up();
        $membership->up();

        self::assertTrue(Schema::hasTable('game_catalog_snapshots'));
        self::assertTrue(Schema::hasTable('game_catalog_snapshot_releases'));
        self::assertTrue(Schema::hasTable('game_catalog_profile_relations'));
    }

    public function test_valid_import_is_inactive_and_hash_idempotent(): void
    {
        $importer = $this->app->make(CatalogImporter::class);

        $first = $importer->import($this->fixturePath());
        $second = $importer->import($this->fixturePath());

        self::assertFalse($first->alreadyImported);
        self::assertTrue($second->alreadyImported);
        self::assertSame($first->snapshotId, $second->snapshotId);
        self::assertSame('ec0658bb11877240f2e22575180513dbff426b3df1fc2af8f20343ed0d424055', $first->contentSha256);
        self::assertSame(1, DB::table('game_catalog_snapshots')->count());
        self::assertSame(4, DB::table('game_catalog_entity_snapshots')->count());
        self::assertSame(2, DB::table('game_catalog_relation_snapshots')->count());
        self::assertSame(2, DB::table('game_catalog_snapshot_releases')->count());
        self::assertNull(DB::table('game_catalog_profiles')->where('key', 'oteryn-current')->value('active_snapshot_id'));
        self::assertSame(2, DB::table('game_catalog_import_runs')->where('status', 'validated')->count());
    }

    public function test_rejected_import_leaves_no_partial_catalog_state(): void
    {
        $invalid = $this->temporarySnapshotFromRawReplacement(
            '"target":"item:dragon-shield"',
            '"target":"item:missing"',
        );

        try {
            $this->app->make(CatalogImporter::class)->import($invalid);
            self::fail('Dangling relation import was accepted.');
        } catch (CatalogValidationException $exception) {
            self::assertContains('semantic.dangling_relation', array_column($exception->findings(), 'code'));
        }

        self::assertSame(0, DB::table('game_catalog_snapshots')->count());
        self::assertSame(0, DB::table('game_catalog_entities')->count());
        self::assertSame(0, DB::table('game_catalog_entity_snapshots')->count());
        self::assertSame(0, DB::table('game_catalog_relation_snapshots')->count());
        self::assertSame(1, DB::table('game_catalog_import_runs')->where('status', 'rejected')->count());
        self::assertGreaterThan(0, DB::table('game_catalog_validation_findings')->count());
    }

    public function test_same_snapshot_produces_distinct_visibility_for_two_target_releases(): void
    {
        $import = $this->app->make(CatalogImporter::class)->import($this->fixturePath());
        $activator = $this->app->make(CatalogProfileActivator::class);

        $current = $activator->activate($import->snapshotId, 'oteryn-current');
        self::assertSame(2, $current->projection->visibleEntities);
        self::assertSame(2, $current->projection->hiddenEntities);
        self::assertSame(1, $current->projection->visibleRelations);
        self::assertSame(1, $current->projection->hiddenRelations);
        $this->assertReason('oteryn-current', 'item:future-blade', false, 'future_release');
        $this->assertReason('oteryn-current', 'creature:unfinished-beast', false, 'partial');
        $this->assertRelationReason('oteryn-current', 'loot:dragon:future-blade', false, 'future_release');

        $futureReleaseId = (int) DB::table('game_catalog_releases')->where('key', '15.30')->value('id');
        $now = now();
        DB::table('game_catalog_profiles')->insert([
            'key' => 'oteryn-future-test',
            'name' => 'Oteryn Future Test',
            'target_release_id' => $futureReleaseId,
            'active_snapshot_id' => null,
            'complete_only' => true,
            'public_enabled' => false,
            'allow_backports' => false,
            'lock_version' => 0,
            'created_at' => $now,
            'updated_at' => $now,
        ]);

        $future = $activator->activate($import->snapshotId, 'oteryn-future-test');
        self::assertSame(3, $future->projection->visibleEntities);
        self::assertSame(1, $future->projection->hiddenEntities);
        self::assertSame(2, $future->projection->visibleRelations);
        self::assertSame(0, $future->projection->hiddenRelations);
        $this->assertReason('oteryn-future-test', 'item:future-blade', true, 'visible');
        $this->assertReason('oteryn-future-test', 'creature:unfinished-beast', false, 'partial');
        $this->assertRelationReason('oteryn-future-test', 'loot:dragon:future-blade', true, 'visible');

        $visibleEndpointViolations = DB::table('game_catalog_profile_relations as relation_visibility')
            ->join('game_catalog_relation_snapshots as relations', 'relations.id', '=', 'relation_visibility.relation_snapshot_id')
            ->join('game_catalog_entity_snapshots as source_versions', function ($join) use ($import): void {
                $join->on('source_versions.entity_id', '=', 'relations.source_entity_id')
                    ->where('source_versions.snapshot_id', '=', $import->snapshotId);
            })
            ->join('game_catalog_profile_entities as source_visibility', function ($join): void {
                $join->on('source_visibility.entity_snapshot_id', '=', 'source_versions.id')
                    ->on('source_visibility.profile_id', '=', 'relation_visibility.profile_id');
            })
            ->join('game_catalog_entity_snapshots as target_versions', function ($join) use ($import): void {
                $join->on('target_versions.entity_id', '=', 'relations.target_entity_id')
                    ->where('target_versions.snapshot_id', '=', $import->snapshotId);
            })
            ->join('game_catalog_profile_entities as target_visibility', function ($join): void {
                $join->on('target_visibility.entity_snapshot_id', '=', 'target_versions.id')
                    ->on('target_visibility.profile_id', '=', 'relation_visibility.profile_id');
            })
            ->where('relation_visibility.visible', true)
            ->where(function ($query): void {
                $query->where('source_visibility.visible', false)
                    ->orWhere('target_visibility.visible', false);
            })
            ->count();
        self::assertSame(0, $visibleEndpointViolations);
    }

    public function test_failed_activation_preserves_previous_snapshot_and_rollback_restores_it(): void
    {
        $importer = $this->app->make(CatalogImporter::class);
        $activator = $this->app->make(CatalogProfileActivator::class);
        $first = $importer->import($this->fixturePath());
        $secondPath = $this->temporarySnapshotFromRawReplacement(
            '"producer_build_id":"shared-fixture-v1"',
            '"producer_build_id":"shared-fixture-v2"',
        );
        $second = $importer->import($secondPath);

        $activator->activate($first->snapshotId, 'oteryn-current');
        $activator->activate($second->snapshotId, 'oteryn-current');
        self::assertSame($second->snapshotId, (int) DB::table('game_catalog_profiles')->where('key', 'oteryn-current')->value('active_snapshot_id'));

        DB::table('game_catalog_snapshots')->where('id', $first->snapshotId)->update(['status' => 'rejected']);
        try {
            $activator->activate($first->snapshotId, 'oteryn-current');
            self::fail('Rejected snapshot was activated.');
        } catch (RuntimeException $exception) {
            self::assertSame('Target Game Catalog snapshot is not validated.', $exception->getMessage());
        }
        self::assertSame($second->snapshotId, (int) DB::table('game_catalog_profiles')->where('key', 'oteryn-current')->value('active_snapshot_id'));

        DB::table('game_catalog_snapshots')->where('id', $first->snapshotId)->update(['status' => 'validated']);
        $rollback = $activator->activate($first->snapshotId, 'oteryn-current', null, 'rollback', 'Integration rollback proof');
        self::assertSame($second->snapshotId, $rollback->previousSnapshotId);
        self::assertSame($first->snapshotId, $rollback->activeSnapshotId);
        self::assertSame($first->snapshotId, (int) DB::table('game_catalog_profiles')->where('key', 'oteryn-current')->value('active_snapshot_id'));
        self::assertDatabaseHas('game_catalog_activation_history', [
            'profile_id' => $rollback->profileId,
            'from_snapshot_id' => $second->snapshotId,
            'to_snapshot_id' => $first->snapshotId,
            'action' => 'rollback',
        ]);
        self::assertDatabaseHas('admin_audit_events', [
            'action' => 'game_catalog.snapshot_rolled_back',
            'target_type' => 'game_catalog_profile',
        ]);
    }

    private function fixturePath(): string
    {
        return base_path('resources/fixtures/game-catalog/minimal-snapshot.json');
    }

    private function temporarySnapshotFromRawReplacement(string $search, string $replacement): string
    {
        $raw = file_get_contents($this->fixturePath());
        self::assertIsString($raw);
        self::assertStringContainsString($search, $raw);
        $path = tempnam(sys_get_temp_dir(), 'oteryn-catalog-lifecycle-');
        self::assertIsString($path);
        $this->temporaryFiles[] = $path;
        file_put_contents($path, str_replace($search, $replacement, $raw));

        return $path;
    }

    private function assertReason(string $profileKey, string $canonicalKey, bool $visible, string $reason): void
    {
        $row = DB::table('game_catalog_profile_entities as visibility')
            ->join('game_catalog_profiles as profiles', 'profiles.id', '=', 'visibility.profile_id')
            ->join('game_catalog_entity_snapshots as versions', 'versions.id', '=', 'visibility.entity_snapshot_id')
            ->join('game_catalog_entities as entities', 'entities.id', '=', 'versions.entity_id')
            ->where('profiles.key', $profileKey)
            ->where('entities.canonical_key', $canonicalKey)
            ->first(['visibility.visible', 'visibility.reason_code']);

        self::assertNotNull($row);
        self::assertSame($visible, (bool) $row->visible);
        self::assertSame($reason, $row->reason_code);
    }

    private function assertRelationReason(string $profileKey, string $canonicalKey, bool $visible, string $reason): void
    {
        $row = DB::table('game_catalog_profile_relations as visibility')
            ->join('game_catalog_profiles as profiles', 'profiles.id', '=', 'visibility.profile_id')
            ->join('game_catalog_relation_snapshots as relations', 'relations.id', '=', 'visibility.relation_snapshot_id')
            ->where('profiles.key', $profileKey)
            ->where('relations.canonical_key', $canonicalKey)
            ->first(['visibility.visible', 'visibility.reason_code']);

        self::assertNotNull($row);
        self::assertSame($visible, (bool) $row->visible);
        self::assertSame($reason, $row->reason_code);
    }
}
