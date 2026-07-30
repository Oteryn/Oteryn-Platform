<?php

namespace Tests\Feature\GameCatalog;

use App\GameCatalog\Application\Activation\CatalogActivationService;
use App\GameCatalog\Application\Import\CatalogImportService;
use App\GameCatalog\Application\Import\CatalogSnapshotValidator;
use App\GameCatalog\Application\Import\ValidatedCatalogSnapshot;
use App\GameCatalog\Domain\Exceptions\CatalogValidationException;
use Carbon\CarbonImmutable;
use Illuminate\Foundation\Testing\RefreshDatabase;
use Illuminate\Support\Facades\DB;
use JsonException;
use RuntimeException;
use Tests\TestCase;

/** @phpstan-import-type CatalogPayload from ValidatedCatalogSnapshot */
final class CatalogV13ConsumerTest extends TestCase
{
    use RefreshDatabase;

    public function test_schema_13_fixture_validates_and_imports_as_typed_inactive_snapshot(): void
    {
        $validated = app(CatalogSnapshotValidator::class)->validate($this->fixturePath());
        self::assertSame('1.3.0', $validated->payload['schema_version']);
        self::assertSame('0282c0ce4b995e4aded440b148dd4eb8a96a441e9924da182a2df2a0f2eef8a8', $validated->schemaSha256);
        self::assertSame('c4fd9b187e001065f68d90f93dc67f71bb2ff745fc43c3e73110d49b23407ce7', $validated->contentSha256);

        $result = app(CatalogImportService::class)->import($this->fixturePath());
        self::assertFalse($result->deduplicated);
        self::assertSame('validated', DB::table('game_catalog_snapshots')->where('id', $result->snapshotId)->value('status'));
        self::assertSame('1.3.0', DB::table('game_catalog_snapshots')->where('id', $result->snapshotId)->value('schema_version'));
        self::assertSame(4, DB::table('game_catalog_entity_snapshots')->where('snapshot_id', $result->snapshotId)->count());
        self::assertSame(1, DB::table('game_catalog_npc_snapshots')->count());
        self::assertSame(3, DB::table('game_catalog_relation_snapshots')->where('snapshot_id', $result->snapshotId)->count());
        self::assertSame(2, DB::table('game_catalog_shop_offer_snapshots')->count());
        self::assertSame(1, DB::table('game_catalog_loot_snapshots')->count());
        self::assertSame(0, DB::table('game_catalog_profiles')->whereNotNull('active_snapshot_id')->count());

        $npc = DB::table('game_catalog_npc_snapshots')->first();
        self::assertNotNull($npc);
        self::assertSame('fixture merchant', $npc->registry_key);
        self::assertSame('Fixture Merchant', $npc->runtime_name);
        self::assertNull($npc->display_name);
        self::assertSame(3031, $npc->currency_server_id);

        $offers = DB::table('game_catalog_shop_offer_snapshots')->orderBy('direction')->get();
        self::assertCount(2, $offers);
        $buy = $offers->firstWhere('direction', 'buy');
        $sell = $offers->firstWhere('direction', 'sell');
        self::assertNotNull($buy);
        self::assertNotNull($sell);
        self::assertSame(100, $buy->price_amount);
        self::assertNull($buy->storage_key);
        self::assertSame(50, $sell->price_amount);
        self::assertSame(1000, $sell->storage_key);
        self::assertSame(1, $sell->storage_value);

        $summary = DB::table('game_catalog_snapshots')->where('id', $result->snapshotId)->value('validation_summary');
        self::assertIsString($summary);
        self::assertStringContainsString('"npc":1', $summary);
        self::assertStringContainsString('"npc_buy_offer":1', $summary);
        self::assertStringContainsString('"npc_sell_offer":1', $summary);
        self::assertStringContainsString('"unknown_or_unverified_entity_count":4', $summary);
    }

    public function test_schema_13_rejects_dangling_currency_without_partial_persistence(): void
    {
        $path = $this->temporarySnapshot(function (array &$payload): void {
            $payload['relations'][1]['data']['currency']['item'] = 'item:missing-currency';
        });
        $this->assertRejected($path, 'semantic.dangling_currency');
    }

    public function test_schema_13_rejects_currency_server_id_mismatch_without_partial_persistence(): void
    {
        $path = $this->temporarySnapshot(function (array &$payload): void {
            $payload['entities'][3]['data']['currency']['server_id'] = 9999;
        });
        $this->assertRejected($path, 'semantic.currency_server_id');
    }

    public function test_schema_13_rejects_shop_identity_mismatch_without_partial_persistence(): void
    {
        $path = $this->temporarySnapshot(function (array &$payload): void {
            $payload['relations'][1]['canonical_key'] = 'shop:npc:fixture-merchant:buy:item:fixture-shield:1';
        });
        $this->assertRejected($path, 'semantic.shop_canonical_identity');
    }

    public function test_schema_13_is_inactive_import_only_even_with_a_profile(): void
    {
        $snapshot = app(CatalogImportService::class)->import($this->fixturePath());
        $releaseId = $this->integerDatabaseValue(DB::table('game_catalog_releases')->where('key', '15.25')->value('id'));
        $now = CarbonImmutable::now('UTC');
        DB::table('game_catalog_profiles')->insert([
            'key' => 'schema-13-review', 'name' => 'Schema 1.3 review', 'target_release_id' => $releaseId, 'active_snapshot_id' => null,
            'protocol_profile' => 'fixture-protocol', 'complete_only' => true, 'completeness_policy_key' => 'complete-only',
            'availability_policy_key' => 'public-proven', 'validation_policy_key' => 'validated-snapshot', 'public_enabled' => false,
            'allow_backports' => false, 'lock_version' => 0, 'created_at' => $now, 'updated_at' => $now,
        ]);

        try {
            app(CatalogActivationService::class)->activate($snapshot->snapshotId, 'schema-13-review');
            self::fail('Expected schema 1.3 activation to remain blocked.');
        } catch (RuntimeException $exception) {
            self::assertStringContainsString('inactive import and review only', $exception->getMessage());
        }

        self::assertNull(DB::table('game_catalog_profiles')->where('key', 'schema-13-review')->value('active_snapshot_id'));
        self::assertSame(0, DB::table('game_catalog_profile_entities')->count());
        self::assertSame(0, DB::table('game_catalog_profile_relations')->count());
        self::assertSame(0, DB::table('game_catalog_audit_events')->count());
    }

    private function assertRejected(string $path, string $expectedCode): void
    {
        try {
            app(CatalogImportService::class)->import($path);
            self::fail('Expected schema 1.3 candidate to be rejected.');
        } catch (CatalogValidationException $exception) {
            self::assertContains($expectedCode, array_map(static fn ($finding): string => $finding->code, $exception->findings));
        } finally {
            @unlink($path);
        }
        self::assertSame(0, DB::table('game_catalog_snapshots')->count());
        self::assertSame(0, DB::table('game_catalog_releases')->count());
        self::assertSame(0, DB::table('game_catalog_entities')->count());
        self::assertSame(0, DB::table('game_catalog_npc_snapshots')->count());
        self::assertSame(0, DB::table('game_catalog_shop_offer_snapshots')->count());
        self::assertSame(1, DB::table('game_catalog_import_runs')->where('status', 'rejected')->count());
    }

    private function fixturePath(): string
    {
        return base_path('tests/Fixtures/GameCatalog/v1.3/minimal-snapshot.json');
    }

    /** @param callable(CatalogPayload&): void $mutate */
    private function temporarySnapshot(callable $mutate): string
    {
        try {
            $contents = file_get_contents($this->fixturePath());
            if (! is_string($contents)) {
                throw new RuntimeException('The schema 1.3 fixture could not be read.');
            }
            $decoded = json_decode($contents, true, 128, JSON_THROW_ON_ERROR);
            if (! is_array($decoded) || array_is_list($decoded)) {
                throw new RuntimeException('The schema 1.3 fixture root is invalid.');
            }
            /** @var CatalogPayload $payload */
            $payload = $decoded;
            $mutate($payload);
            $json = json_encode($payload, JSON_THROW_ON_ERROR | JSON_PRETTY_PRINT | JSON_UNESCAPED_SLASHES | JSON_UNESCAPED_UNICODE)."\n";
        } catch (JsonException $exception) {
            self::fail($exception->getMessage());
        }

        $path = tempnam(sys_get_temp_dir(), 'game-catalog-v13-');
        self::assertIsString($path);
        file_put_contents($path, $json);

        return $path;
    }

    private function integerDatabaseValue(mixed $value): int
    {
        if (is_int($value)) {
            return $value;
        }
        if (is_string($value) && ctype_digit($value)) {
            return (int) $value;
        }
        throw new RuntimeException('Expected an integer-compatible database id.');
    }
}
