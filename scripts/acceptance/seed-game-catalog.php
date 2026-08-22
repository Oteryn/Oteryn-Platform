<?php

declare(strict_types=1);

use App\GameCatalog\Application\Activation\CatalogActivationService;
use App\GameCatalog\Application\Import\CatalogImportService;
use Carbon\CarbonImmutable;
use Illuminate\Contracts\Console\Kernel;
use Illuminate\Support\Facades\DB;

require __DIR__.'/../../vendor/autoload.php';

$app = require __DIR__.'/../../bootstrap/app.php';
$app->make(Kernel::class)->bootstrap();

if (! $app->environment('acceptance')) {
    fwrite(STDERR, "Game Catalog fixture seeding is restricted to the acceptance environment.\n");
    exit(2);
}

$importService = $app->make(CatalogImportService::class);
$fixturePath = base_path('tests/Fixtures/GameCatalog/v1/minimal-snapshot.json');
$import = $importService->import($fixturePath);
$comparisonPath = tempnam(sys_get_temp_dir(), 'oteryn-game-catalog-acceptance-');

if ($comparisonPath === false) {
    throw new RuntimeException('Could not allocate the acceptance comparison snapshot.');
}

try {
    $fixtureJson = file_get_contents($fixturePath);
    if ($fixtureJson === false) {
        throw new RuntimeException('Could not read the acceptance Game Catalog fixture.');
    }

    /** @var array<string, mixed> $comparisonPayload */
    $comparisonPayload = json_decode($fixtureJson, true, 512, JSON_THROW_ON_ERROR);
    $comparisonPayload['snapshot']['producer_build_id'] = 'shared-fixture-v1-comparison';
    $comparisonPayload['snapshot']['generated_at'] = '2026-01-02T00:00:00Z';
    $comparisonPayload['entities'][2]['data']['attack'] = 11;

    $encodedComparison = json_encode(
        $comparisonPayload,
        JSON_THROW_ON_ERROR | JSON_UNESCAPED_SLASHES | JSON_UNESCAPED_UNICODE | JSON_PRETTY_PRINT,
    );
    if (file_put_contents($comparisonPath, $encodedComparison."\n") === false) {
        throw new RuntimeException('Could not write the acceptance comparison snapshot.');
    }

    $comparisonImport = $importService->import($comparisonPath);
} finally {
    @unlink($comparisonPath);
}

$releaseId = DB::table('game_catalog_releases')->where('key', '15.20')->value('id');

if (! is_int($releaseId) && (! is_string($releaseId) || preg_match('/^(?:0|[1-9][0-9]*)$/D', $releaseId) !== 1)) {
    fwrite(STDERR, "The acceptance Game Catalog release could not be resolved.\n");
    exit(1);
}

$now = CarbonImmutable::now('UTC');
DB::table('game_catalog_profiles')->updateOrInsert(
    ['key' => 'public'],
    [
        'name' => 'Public Game Catalog',
        'target_release_id' => (int) $releaseId,
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
    ],
);

$activation = $app->make(CatalogActivationService::class)->activate($import->snapshotId, 'public');
$entityId = DB::table('game_catalog_entities')->where('canonical_key', 'item:fixture-sword')->value('id');

if (! is_int($entityId) && (! is_string($entityId) || preg_match('/^(?:0|[1-9][0-9]*)$/D', $entityId) !== 1)) {
    fwrite(STDERR, "The acceptance Game Catalog item could not be resolved.\n");
    exit(1);
}

DB::table('game_catalog_entity_translations')->updateOrInsert(
    [
        'entity_id' => (int) $entityId,
        'locale' => 'pl',
    ],
    [
        'display_name' => 'Miecz testowy',
        'slug' => 'miecz-testowy',
        'summary' => 'Zatwierdzone tłumaczenie testowe.',
        'description_markdown' => null,
        'source_name_sha256' => hash('sha256', 'Fixture Sword'),
        'translation_status' => 'approved',
        'created_at' => $now,
        'updated_at' => $now,
    ],
);

fwrite(STDOUT, json_encode([
    'snapshot_id' => $import->snapshotId,
    'comparison_snapshot_id' => $comparisonImport->snapshotId,
    'profile_id' => $activation->profileId,
    'visible_entity_count' => $activation->visibleEntityCount,
    'visible_relation_count' => $activation->visibleRelationCount,
], JSON_THROW_ON_ERROR | JSON_UNESCAPED_SLASHES).PHP_EOL);
