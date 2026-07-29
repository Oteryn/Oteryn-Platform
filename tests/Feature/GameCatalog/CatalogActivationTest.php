<?php

namespace Tests\Feature\GameCatalog;

use App\GameCatalog\Application\Activation\CatalogActivationService;
use App\GameCatalog\Application\Import\CatalogImportService;
use App\GameCatalog\Application\Import\ValidatedCatalogSnapshot;
use App\GameCatalog\Application\Verification\CatalogVerificationService;
use Carbon\CarbonImmutable;
use Illuminate\Foundation\Testing\RefreshDatabase;
use Illuminate\Support\Facades\DB;
use JsonException;
use RuntimeException;
use Tests\TestCase;

/** @phpstan-import-type CatalogPayload from ValidatedCatalogSnapshot */
final class CatalogActivationTest extends TestCase
{
    use RefreshDatabase;

    public function test_visibility_differs_by_target_release_and_activation_is_profile_scoped(): void
    {
        $snapshot = app(CatalogImportService::class)->import($this->fixturePath());
        $currentProfile = $this->createProfile('public-current', '15.20');
        $futureProfile = $this->createProfile('public-future', '15.21');
        $untouchedProfile = $this->createProfile('public-untouched', '15.20');

        $current = app(CatalogActivationService::class)->activate($snapshot->snapshotId, 'public-current');
        $future = app(CatalogActivationService::class)->activate($snapshot->snapshotId, 'public-future');

        self::assertSame(2, $current->visibleEntityCount);
        self::assertSame(1, $current->visibleRelationCount);
        self::assertSame(3, $future->visibleEntityCount);
        self::assertSame(2, $future->visibleRelationCount);
        self::assertNull(DB::table('game_catalog_profiles')->where('id', $untouchedProfile)->value('active_snapshot_id'));

        self::assertSame(1, DB::table('game_catalog_profile_entities')
            ->where('profile_id', $currentProfile)
            ->where('reason_code', 'outside_release')
            ->count());
        self::assertSame(1, DB::table('game_catalog_profile_entities')
            ->where('profile_id', $currentProfile)
            ->where('reason_code', 'incomplete')
            ->count());
        self::assertSame(1, DB::table('game_catalog_profile_relations')
            ->where('profile_id', $currentProfile)
            ->where('reason_code', 'outside_release')
            ->count());
        self::assertSame(0, DB::table('game_catalog_profile_entities')
            ->where('profile_id', $futureProfile)
            ->where('visible', true)
            ->whereIn('reason_code', ['outside_release', 'incomplete'])
            ->count());
    }

    public function test_activation_failure_preserves_previous_snapshot_and_projection(): void
    {
        $snapshot = app(CatalogImportService::class)->import($this->fixturePath());
        $profileId = $this->createProfile('public-current', '15.20');
        app(CatalogActivationService::class)->activate($snapshot->snapshotId, 'public-current');

        $activeBefore = DB::table('game_catalog_profiles')->where('id', $profileId)->value('active_snapshot_id');
        $projectionBefore = DB::table('game_catalog_profile_entities')->where('profile_id', $profileId)->orderBy('entity_snapshot_id')->pluck('reason_code')->all();

        $invalidSnapshotId = (int) DB::table('game_catalog_snapshots')->insertGetId([
            'contract_version' => 'oteryn.game-catalog',
            'schema_version' => '1.0.0',
            'content_sha256' => str_repeat('f', 64),
            'canary_commit_sha' => str_repeat('a', 40),
            'datapack_commit_sha' => null,
            'protocol_profile' => 'fixture-protocol',
            'runtime_release_id' => DB::table('game_catalog_releases')->where('key', '15.20')->value('id'),
            'content_target_release_id' => DB::table('game_catalog_releases')->where('key', '15.20')->value('id'),
            'verified_content_through_release_id' => DB::table('game_catalog_releases')->where('key', '15.20')->value('id'),
            'contains_content_through_release_id' => DB::table('game_catalog_releases')->where('key', '15.20')->value('id'),
            'appearances_sha256' => str_repeat('b', 64),
            'map_sha256' => null,
            'producer_build_id' => 'invalid-test',
            'generated_at' => CarbonImmutable::now('UTC'),
            'imported_at' => CarbonImmutable::now('UTC'),
            'status' => 'rejected',
            'entity_count' => 0,
            'relation_count' => 0,
            'validation_summary' => json_encode(['errors' => 1], JSON_THROW_ON_ERROR),
            'created_at' => CarbonImmutable::now('UTC'),
            'updated_at' => CarbonImmutable::now('UTC'),
        ]);

        try {
            app(CatalogActivationService::class)->activate($invalidSnapshotId, 'public-current');
            self::fail('Expected rejected snapshot activation to fail.');
        } catch (RuntimeException $exception) {
            self::assertStringContainsString('validated', $exception->getMessage());
        }

        self::assertSame($activeBefore, DB::table('game_catalog_profiles')->where('id', $profileId)->value('active_snapshot_id'));
        self::assertSame($projectionBefore, DB::table('game_catalog_profile_entities')->where('profile_id', $profileId)->orderBy('entity_snapshot_id')->pluck('reason_code')->all());
    }

    public function test_rollback_reuses_normal_visibility_checks_and_writes_audit_event(): void
    {
        $importer = app(CatalogImportService::class);
        $first = $importer->import($this->fixturePath());
        $secondPath = $this->temporarySnapshot(function (array &$payload): void {
            $payload['snapshot']['generated_at'] = '2026-01-02T00:00:00Z';
            $payload['snapshot']['producer_build_id'] = 'shared-fixture-v2';
            if (! isset($payload['entities'][2]) || $payload['entities'][2]['type'] !== 'item') {
                throw new RuntimeException('The shared fixture item layout changed unexpectedly.');
            }
            $payload['entities'][2]['data']['attack'] = 11;
        });

        try {
            $second = $importer->import($secondPath);
        } finally {
            @unlink($secondPath);
        }

        $profileId = $this->createProfile('public-current', '15.20');
        $activation = app(CatalogActivationService::class);
        $activation->activate($first->snapshotId, 'public-current');
        $activation->activate($second->snapshotId, 'public-current');
        $rollback = $activation->activate($first->snapshotId, 'public-current', rollback: true);

        self::assertTrue($rollback->rollback);
        self::assertSame($second->snapshotId, $rollback->previousSnapshotId);
        self::assertSame($first->snapshotId, $this->databaseInt(DB::table('game_catalog_profiles')->where('id', $profileId)->value('active_snapshot_id')));
        self::assertSame('snapshot.rollback', DB::table('game_catalog_audit_events')->latest('id')->value('action'));

        $verification = app(CatalogVerificationService::class)->verify('public-current');
        self::assertTrue($verification->isValid(), implode(', ', $verification->errors));
        self::assertSame(2, $verification->visibleEntityCount);
        self::assertSame(1, $verification->visibleRelationCount);
    }

    public function test_partial_and_unknown_availability_content_are_hidden_by_default(): void
    {
        $path = $this->temporarySnapshot(function (array &$payload): void {
            if (! isset($payload['entities'][0])) {
                throw new RuntimeException('The shared fixture entity layout changed unexpectedly.');
            }
            $payload['entities'][0]['availability'] = 'unknown';
        });

        try {
            $snapshot = app(CatalogImportService::class)->import($path);
        } finally {
            @unlink($path);
        }

        $profileId = $this->createProfile('public-current', '15.20');
        $activation = app(CatalogActivationService::class)->activate($snapshot->snapshotId, 'public-current');

        self::assertSame(1, $activation->visibleEntityCount);
        self::assertSame(0, $activation->visibleRelationCount);
        self::assertSame(1, DB::table('game_catalog_profile_entities')->where('profile_id', $profileId)->where('reason_code', 'availability_disallowed')->count());
        self::assertSame(2, DB::table('game_catalog_profile_relations')->where('profile_id', $profileId)->whereIn('reason_code', ['source_hidden', 'outside_release'])->count());
    }

    public function test_unknown_verified_content_boundary_cannot_be_activated(): void
    {
        $snapshot = app(CatalogImportService::class)->import($this->fixtureV11Path());
        $profileId = $this->createProfile('unknown-boundary', '15.20');

        try {
            app(CatalogActivationService::class)->activate($snapshot->snapshotId, 'unknown-boundary');
            self::fail('Expected activation with an unknown verified-content boundary to fail.');
        } catch (RuntimeException $exception) {
            self::assertStringContainsString('no verified-content boundary', $exception->getMessage());
        }

        self::assertNull(DB::table('game_catalog_profiles')->where('id', $profileId)->value('active_snapshot_id'));
        self::assertSame(0, DB::table('game_catalog_profile_entities')->where('profile_id', $profileId)->count());
        self::assertSame(0, DB::table('game_catalog_profile_relations')->where('profile_id', $profileId)->count());
    }

    private function createProfile(string $key, string $releaseKey): int
    {
        $now = CarbonImmutable::now('UTC');

        return (int) DB::table('game_catalog_profiles')->insertGetId([
            'key' => $key,
            'name' => $key,
            'target_release_id' => DB::table('game_catalog_releases')->where('key', $releaseKey)->value('id'),
            'active_snapshot_id' => null,
            'protocol_profile' => 'fixture-protocol',
            'complete_only' => true,
            'completeness_policy_key' => 'complete-only',
            'availability_policy_key' => 'public-proven',
            'validation_policy_key' => 'validated-snapshot',
            'public_enabled' => true,
            'allow_backports' => false,
            'lock_version' => 0,
            'created_at' => $now,
            'updated_at' => $now,
        ]);
    }

    private function databaseInt(mixed $value): int
    {
        if (is_int($value)) {
            return $value;
        }
        if (is_string($value) && preg_match('/^(?:0|[1-9][0-9]*)$/D', $value) === 1) {
            return (int) $value;
        }

        throw new RuntimeException('Expected an integer database value.');
    }

    private function fixturePath(): string
    {
        return base_path('tests/Fixtures/GameCatalog/v1/minimal-snapshot.json');
    }

    private function fixtureV11Path(): string
    {
        return base_path('tests/Fixtures/GameCatalog/v1.1/minimal-snapshot.json');
    }

    /** @param callable(CatalogPayload&): void $mutate */
    private function temporarySnapshot(callable $mutate): string
    {
        try {
            $contents = file_get_contents($this->fixturePath());
            if (! is_string($contents)) {
                throw new RuntimeException('The shared Game Catalog fixture could not be read.');
            }
            $decoded = json_decode($contents, true, 128, JSON_THROW_ON_ERROR);
            if (! is_array($decoded) || array_is_list($decoded)) {
                throw new RuntimeException('The shared Game Catalog fixture root is invalid.');
            }
            /** @var CatalogPayload $payload */
            $payload = $decoded;
            $mutate($payload);
            $json = json_encode($payload, JSON_THROW_ON_ERROR | JSON_PRETTY_PRINT | JSON_UNESCAPED_SLASHES | JSON_UNESCAPED_UNICODE)."\n";
        } catch (JsonException $exception) {
            self::fail($exception->getMessage());
        }

        $path = tempnam(sys_get_temp_dir(), 'game-catalog-activation-');
        self::assertIsString($path);
        file_put_contents($path, $json);

        return $path;
    }
}
