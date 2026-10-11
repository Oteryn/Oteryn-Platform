<?php

namespace Tests\Feature\GameCatalog;

use App\GameCatalog\Application\Import\CatalogImportService;
use Illuminate\Database\Migrations\Migration;
use Illuminate\Foundation\Testing\DatabaseMigrations;
use Illuminate\Support\Facades\DB;
use Illuminate\Support\Facades\Schema;
use LogicException;
use ReflectionMethod;
use Tests\TestCase;

final class SnapshotSourceProvenanceMigrationTest extends TestCase
{
    use DatabaseMigrations;

    private const string FIXTURE_COMMIT = 'aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa';

    public function test_import_writes_source_provenance(): void
    {
        $snapshotId = app(CatalogImportService::class)->import($this->fixturePath())->snapshotId;

        self::assertSame(self::FIXTURE_COMMIT, DB::table('game_catalog_snapshots')->where('id', $snapshotId)->value('source_commit_sha'));
    }

    public function test_round_trip_with_existing_snapshots_copies_legacy_provenance(): void
    {
        $snapshotId = app(CatalogImportService::class)->import($this->fixturePath())->snapshotId;
        $before = DB::table('game_catalog_snapshots')->where('id', $snapshotId)->first();
        self::assertNotNull($before);

        $this->applyMigration('down');

        self::assertFalse(Schema::hasColumn('game_catalog_snapshots', 'source_commit_sha'));
        self::assertSame(1, DB::table('game_catalog_snapshots')->count());

        // up() repopulates the new column from the legacy column alone.
        $this->applyMigration('up');

        $after = DB::table('game_catalog_snapshots')->where('id', $snapshotId)->first();
        self::assertNotNull($after);
        self::assertSame(self::FIXTURE_COMMIT, $after->source_commit_sha);
        self::assertSame($before->content_sha256, $after->content_sha256);
        self::assertSame($before->status, $after->status);
    }

    public function test_up_leaves_snapshot_rows_otherwise_unchanged(): void
    {
        app(CatalogImportService::class)->import($this->fixturePath());
        $this->applyMigration('down');
        $before = array_map(fn (object $row): array => (array) $row, DB::table('game_catalog_snapshots')->orderBy('id')->get()->all());

        $this->applyMigration('up');

        $after = array_map(function (object $row): array {
            $values = (array) $row;
            unset($values['source_commit_sha']);

            return $values;
        }, DB::table('game_catalog_snapshots')->orderBy('id')->get()->all());
        self::assertSame($before, $after);
    }

    public function test_rows_without_source_provenance_roll_back(): void
    {
        $snapshotId = app(CatalogImportService::class)->import($this->fixturePath())->snapshotId;
        // A row written by an application release that predates the column.
        DB::table('game_catalog_snapshots')->where('id', $snapshotId)->update(['source_commit_sha' => null]);

        $this->applyMigration('down');
        self::assertFalse(Schema::hasColumn('game_catalog_snapshots', 'source_commit_sha'));

        $this->applyMigration('up');
        self::assertSame(self::FIXTURE_COMMIT, DB::table('game_catalog_snapshots')->where('id', $snapshotId)->value('source_commit_sha'));
    }

    public function test_diverging_source_provenance_refuses_rollback_before_ddl(): void
    {
        $snapshotId = app(CatalogImportService::class)->import($this->fixturePath())->snapshotId;
        DB::table('game_catalog_snapshots')->where('id', $snapshotId)->update(['source_commit_sha' => str_repeat('b', 40)]);

        try {
            $this->applyMigration('down');
            self::fail('Diverging source provenance must survive rollback.');
        } catch (LogicException $exception) {
            self::assertStringContainsString('before DDL', $exception->getMessage());
        }

        self::assertTrue(Schema::hasColumn('game_catalog_snapshots', 'source_commit_sha'));
        self::assertSame(str_repeat('b', 40), DB::table('game_catalog_snapshots')->where('id', $snapshotId)->value('source_commit_sha'));

        // Let the test runner's own rollback pass.
        DB::table('game_catalog_snapshots')->delete();
    }

    /** @param 'up'|'down' $direction */
    private function applyMigration(string $direction): void
    {
        $migration = require database_path('migrations/2026_10_11_120000_add_source_commit_sha_to_game_catalog_snapshots.php');
        self::assertInstanceOf(Migration::class, $migration);

        (new ReflectionMethod($migration, $direction))->invoke($migration);
    }

    private function fixturePath(): string
    {
        return base_path('tests/Fixtures/GameCatalog/v1/minimal-snapshot.json');
    }
}
