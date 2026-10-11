<?php

namespace Tests\Feature\GameCatalog;

use App\GameCatalog\Application\Import\Native\NativeCatalogContract;
use App\GameCatalog\Application\Import\Native\NativeCatalogImportService;
use App\GameCatalog\Domain\Exceptions\CatalogValidationException;
use Illuminate\Foundation\Testing\RefreshDatabase;
use Illuminate\Support\Facades\DB;
use Illuminate\Testing\PendingCommand;
use Tests\TestCase;

final class NativeCatalogImportTest extends TestCase
{
    use RefreshDatabase;

    private const string FIXTURE_SHA256 = '4fe8ce4488c21396b588dc6b04179cde93c7a9b22b87427b6e4af3cda6e60d7d';

    public function test_native_import_is_disabled_by_default_and_writes_nothing(): void
    {
        $this->importCommand(['path' => $this->fixturePath(), '--native' => true])
            ->expectsOutputToContain('native.import.disabled')
            ->assertFailed();

        try {
            app(NativeCatalogImportService::class)->import($this->fixturePath());
            self::fail('Native import must be off unless configured.');
        } catch (CatalogValidationException $exception) {
            self::assertSame('native.import.disabled', $exception->findings[0]->code);
        }

        $this->assertNothingPersisted();
    }

    public function test_enabled_native_import_records_evidence_without_snapshot_or_activation(): void
    {
        config([NativeCatalogImportService::ENABLED_KEY => true]);

        $this->importCommand([
            'path' => $this->fixturePath(),
            '--native' => true,
            '--sha256' => self::FIXTURE_SHA256,
        ])
            ->expectsOutputToContain('Native Game export accepted')
            ->assertSuccessful();

        $run = DB::table('game_catalog_import_runs')->sole();
        self::assertSame(NativeCatalogImportService::ACCEPTED_STATUS, $run->status);
        self::assertSame(self::FIXTURE_SHA256, $run->content_sha256);
        self::assertNull($run->snapshot_id);
        self::assertSame('unsupported-snapshot.json', $run->source_label);

        self::assertIsString($run->summary);
        $summary = json_decode($run->summary, true, flags: JSON_THROW_ON_ERROR);
        self::assertIsArray($summary);
        self::assertSame('native', $summary['format']);
        self::assertSame(NativeCatalogContract::CONTRACT_ID, $summary['contract_id']);
        self::assertSame(NativeCatalogContract::CONTENT_AUTHORITY_ID, $summary['content_authority_id']);
        self::assertSame(0, $summary['entity_count']);
        self::assertFalse($summary['snapshot_persisted']);
        self::assertFalse($summary['activated']);
        foreach (['authority_epoch', 'source_revision', 'ruleset_id', 'content_profile_id', 'payload_digest', 'snapshot_id'] as $field) {
            self::assertIsString($summary[$field]);
            self::assertNotSame('', $summary[$field]);
        }

        self::assertSame(0, DB::table('game_catalog_snapshots')->count());
        self::assertSame(0, DB::table('game_catalog_entities')->count());
        self::assertSame(0, DB::table('game_catalog_profiles')->count());
    }

    public function test_repeated_native_import_is_deduplicated_by_artifact_hash(): void
    {
        config([NativeCatalogImportService::ENABLED_KEY => true]);
        $importer = app(NativeCatalogImportService::class);

        $first = $importer->import($this->fixturePath());
        $second = $importer->import($this->fixturePath());

        self::assertFalse($first->deduplicated);
        self::assertTrue($second->deduplicated);
        self::assertSame($first->importRunId, $second->importRunId);
        self::assertSame(1, DB::table('game_catalog_import_runs')->count());
    }

    public function test_enabled_native_import_still_fails_closed_on_hash_mismatch(): void
    {
        config([NativeCatalogImportService::ENABLED_KEY => true]);

        $this->importCommand([
            'path' => $this->fixturePath(),
            '--native' => true,
            '--sha256' => str_repeat('0', 64),
        ])
            ->expectsOutputToContain('native.input.hash_mismatch')
            ->assertFailed();

        $this->assertNothingPersisted();
    }

    public function test_non_boolean_switch_fails_closed(): void
    {
        config([NativeCatalogImportService::ENABLED_KEY => 'true']);

        $this->importCommand(['path' => $this->fixturePath(), '--native' => true])
            ->assertFailed();

        $this->assertNothingPersisted();
    }

    public function test_native_export_is_rejected_by_the_legacy_import_path(): void
    {
        config([NativeCatalogImportService::ENABLED_KEY => true]);

        $this->importCommand(['path' => $this->fixturePath()])
            ->expectsOutputToContain('Game Catalog import rejected.')
            ->assertFailed();

        self::assertSame(0, DB::table('game_catalog_snapshots')->count());
    }

    /** @param array<string, mixed> $parameters */
    private function importCommand(array $parameters): PendingCommand
    {
        $command = $this->artisan('game-catalog:import', $parameters);
        self::assertInstanceOf(PendingCommand::class, $command);

        return $command;
    }

    private function assertNothingPersisted(): void
    {
        self::assertSame(0, DB::table('game_catalog_import_runs')->count());
        self::assertSame(0, DB::table('game_catalog_snapshots')->count());
        self::assertSame(0, DB::table('game_catalog_validation_findings')->count());
    }

    private function fixturePath(): string
    {
        return base_path('tests/Fixtures/GameCatalog/native-v1/unsupported-snapshot.json');
    }
}
