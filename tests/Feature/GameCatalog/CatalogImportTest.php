<?php

namespace Tests\Feature\GameCatalog;

use App\GameCatalog\Application\Import\CatalogImportService;
use App\GameCatalog\Application\Import\CatalogSnapshotValidator;
use App\GameCatalog\Domain\Exceptions\CatalogValidationException;
use Illuminate\Foundation\Testing\RefreshDatabase;
use Illuminate\Support\Facades\DB;
use JsonException;
use Tests\TestCase;

final class CatalogImportTest extends TestCase
{
    use RefreshDatabase;

    public function test_shared_fixture_validates_against_the_registered_schema_hash(): void
    {
        $validated = app(CatalogSnapshotValidator::class)->validate($this->fixturePath());

        self::assertSame('oteryn.game-catalog', $validated->payload['contract']);
        self::assertSame('1.0.0', $validated->payload['schema_version']);
        self::assertSame('c947e461c1ee8f6fbf511c9890b61135d2585d6c16e2e99a0f72dd5a946c2181', $validated->contentSha256);
        self::assertCount(4, $validated->payload['entities']);
        self::assertCount(2, $validated->payload['relations']);
    }

    public function test_valid_import_is_transactional_inactive_and_idempotent_by_content_hash(): void
    {
        $importer = app(CatalogImportService::class);

        $first = $importer->import($this->fixturePath());
        $second = $importer->import($this->fixturePath());

        self::assertFalse($first->deduplicated);
        self::assertTrue($second->deduplicated);
        self::assertSame($first->snapshotId, $second->snapshotId);
        self::assertSame(1, DB::table('game_catalog_snapshots')->count());
        self::assertSame(2, DB::table('game_catalog_import_runs')->count());
        self::assertSame(4, DB::table('game_catalog_entity_snapshots')->count());
        self::assertSame(2, DB::table('game_catalog_item_snapshots')->count());
        self::assertSame(2, DB::table('game_catalog_creature_snapshots')->count());
        self::assertSame(2, DB::table('game_catalog_relation_snapshots')->count());
        self::assertSame(2, DB::table('game_catalog_loot_snapshots')->count());
        self::assertSame(0, DB::table('game_catalog_profiles')->count());
        self::assertSame('validated', DB::table('game_catalog_snapshots')->value('status'));
    }

    public function test_semantically_invalid_relation_is_rejected_without_partial_catalogue_state(): void
    {
        $path = $this->temporarySnapshot(function (array &$payload): void {
            $payload['relations'][0]['target'] = 'item:missing-fixture-item';
        });

        try {
            app(CatalogImportService::class)->import($path);
            self::fail('Expected the invalid relation to be rejected.');
        } catch (CatalogValidationException $exception) {
            self::assertContains('semantic.dangling_target', array_map(
                static fn ($finding): string => $finding->code,
                $exception->findings,
            ));
        } finally {
            @unlink($path);
        }

        self::assertSame(0, DB::table('game_catalog_snapshots')->count());
        self::assertSame(0, DB::table('game_catalog_releases')->count());
        self::assertSame(0, DB::table('game_catalog_entities')->count());
        self::assertSame(1, DB::table('game_catalog_import_runs')->where('status', 'rejected')->count());
        self::assertGreaterThan(0, DB::table('game_catalog_validation_findings')->count());
    }

    public function test_unsupported_schema_version_is_rejected_before_import(): void
    {
        $path = $this->temporarySnapshot(function (array &$payload): void {
            $payload['schema_version'] = '2.0.0';
        });

        try {
            app(CatalogImportService::class)->import($path);
            self::fail('Expected the unsupported schema version to be rejected.');
        } catch (CatalogValidationException $exception) {
            self::assertContains('schema.const', array_map(
                static fn ($finding): string => $finding->code,
                $exception->findings,
            ));
        } finally {
            @unlink($path);
        }

        self::assertSame(0, DB::table('game_catalog_snapshots')->count());
    }

    public function test_operator_commands_validate_and_import_without_activation(): void
    {
        $this->artisan('game-catalog:validate', ['path' => $this->fixturePath()])
            ->assertSuccessful();

        $this->artisan('game-catalog:import', ['path' => $this->fixturePath()])
            ->expectsOutputToContain('without activation')
            ->assertSuccessful();

        self::assertSame(1, DB::table('game_catalog_snapshots')->where('status', 'validated')->count());
        self::assertSame(0, DB::table('game_catalog_profiles')->whereNotNull('active_snapshot_id')->count());
    }

    private function fixturePath(): string
    {
        return base_path('tests/Fixtures/GameCatalog/v1/minimal-snapshot.json');
    }

    /**
     * @param  callable(array<string, mixed>&): void  $mutate
     */
    private function temporarySnapshot(callable $mutate): string
    {
        try {
            $payload = json_decode(
                file_get_contents($this->fixturePath()),
                true,
                128,
                JSON_THROW_ON_ERROR,
            );
            $mutate($payload);
            $json = json_encode(
                $payload,
                JSON_THROW_ON_ERROR | JSON_PRETTY_PRINT | JSON_UNESCAPED_SLASHES | JSON_UNESCAPED_UNICODE,
            )."\n";
        } catch (JsonException $exception) {
            self::fail($exception->getMessage());
        }

        $path = tempnam(sys_get_temp_dir(), 'game-catalog-test-');
        self::assertIsString($path);
        file_put_contents($path, $json);

        return $path;
    }
}
