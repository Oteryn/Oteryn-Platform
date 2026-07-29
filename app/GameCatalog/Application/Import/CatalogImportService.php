<?php

namespace App\GameCatalog\Application\Import;

use App\GameCatalog\Application\Configuration\CatalogConfiguration;
use App\GameCatalog\Domain\CatalogValidationFinding;
use App\GameCatalog\Domain\Exceptions\CatalogValidationException;
use App\GameCatalog\Infrastructure\Persistence\CatalogDatabaseRow;
use Carbon\CarbonImmutable;
use Illuminate\Database\ConnectionInterface;
use Illuminate\Support\Facades\DB;
use JsonException;
use Throwable;

/**
 * @phpstan-import-type CatalogRelease from ValidatedCatalogSnapshot
 * @phpstan-import-type CatalogEntity from ValidatedCatalogSnapshot
 * @phpstan-import-type CatalogItemData from ValidatedCatalogSnapshot
 * @phpstan-import-type CatalogCreatureData from ValidatedCatalogSnapshot
 * @phpstan-import-type CatalogRelation from ValidatedCatalogSnapshot
 */
final class CatalogImportService
{
    public function __construct(
        private readonly CatalogSnapshotValidator $validator,
        private readonly ConnectionInterface $database,
    ) {}

    public function import(string $path, ?string $expectedSha256 = null): CatalogImportResult
    {
        try {
            $validated = $this->validator->validate($path, $expectedSha256);
        } catch (CatalogValidationException $exception) {
            $this->recordRejectedValidation($path, $exception);

            throw $exception;
        }

        $existingSnapshot = DB::table('game_catalog_snapshots')
            ->where('content_sha256', $validated->contentSha256)
            ->first(['id']);

        if ($existingSnapshot !== null) {
            $existingSnapshotId = CatalogDatabaseRow::from($existingSnapshot)->int('id');
            $runId = $this->recordDeduplicatedRun($validated, $existingSnapshotId);

            return new CatalogImportResult(
                snapshotId: $existingSnapshotId,
                importRunId: $runId,
                contentSha256: $validated->contentSha256,
                deduplicated: true,
            );
        }

        $runId = $this->startImportRun($validated);

        try {
            $snapshotId = $this->database->transaction(function () use ($validated, $runId): int {
                $now = CarbonImmutable::now('UTC');
                $releaseIds = $this->persistReleases($validated->payload['releases'], $now);
                $snapshotId = $this->persistSnapshot($validated, $releaseIds, $now);
                $entityIds = $this->persistEntities($snapshotId, $validated->payload['entities'], $releaseIds, $now);
                $this->persistRelations($snapshotId, $validated->payload['relations'], $releaseIds, $entityIds, $now);
                $this->verifyPersistedSnapshot($snapshotId, $validated);

                DB::table('game_catalog_snapshots')->where('id', $snapshotId)->update([
                    'status' => 'validated',
                    'validation_summary' => $this->json([
                        'errors' => 0,
                        'warnings' => 0,
                        'schema_sha256' => $validated->schemaSha256,
                        'entity_count' => count($validated->payload['entities']),
                        'relation_count' => count($validated->payload['relations']),
                    ]),
                    'updated_at' => $now,
                ]);

                DB::table('game_catalog_import_runs')->where('id', $runId)->update([
                    'snapshot_id' => $snapshotId,
                    'status' => 'validated',
                    'finding_count' => 0,
                    'finished_at' => $now,
                    'summary' => $this->json([
                        'deduplicated' => false,
                        'snapshot_id' => $snapshotId,
                        'content_sha256' => $validated->contentSha256,
                    ]),
                    'updated_at' => $now,
                ]);

                return $snapshotId;
            }, 3);
        } catch (Throwable $exception) {
            $this->recordPersistenceFailure($runId, $validated, $exception);

            throw $exception;
        }

        return new CatalogImportResult(
            snapshotId: $snapshotId,
            importRunId: $runId,
            contentSha256: $validated->contentSha256,
            deduplicated: false,
        );
    }

    /**
     * @param  list<CatalogRelease>  $releases
     * @return array<string, int>
     */
    private function persistReleases(array $releases, CarbonImmutable $now): array
    {
        $ids = [];

        foreach ($releases as $release) {
            $existing = DB::table('game_catalog_releases')->where('key', $release['key'])->lockForUpdate()->first();
            $releasedAt = $release['released_at'] === null
                ? null
                : CarbonImmutable::parse($release['released_at'])->utc();

            if ($existing === null) {
                $ids[$release['key']] = (int) DB::table('game_catalog_releases')->insertGetId([
                    'key' => $release['key'],
                    'display_label' => $release['display_label'],
                    'major' => $release['major'],
                    'minor' => $release['minor'],
                    'patch' => $release['patch'],
                    'build' => $release['build'],
                    'release_order' => $release['release_order'],
                    'protocol_family' => $release['protocol_family'],
                    'released_at' => $releasedAt,
                    'created_at' => $now,
                    'updated_at' => $now,
                ]);

                continue;
            }

            $existingRow = CatalogDatabaseRow::from($existing);
            $this->assertReleaseMatches($existingRow, $release, $releasedAt);
            $ids[$release['key']] = $existingRow->int('id');
        }

        return $ids;
    }

    /** @param CatalogRelease $release */
    private function assertReleaseMatches(CatalogDatabaseRow $existing, array $release, ?CarbonImmutable $releasedAt): void
    {
        $existingReleasedAtValue = $existing->nullableString('released_at');
        $existingReleasedAt = $existingReleasedAtValue === null
            ? null
            : CarbonImmutable::parse($existingReleasedAtValue)->utc()->format('Y-m-d\TH:i:s.u\Z');
        $incomingReleasedAt = $releasedAt?->format('Y-m-d\TH:i:s.u\Z');

        $matches = $existing->string('display_label') === $release['display_label']
            && $existing->int('major') === $release['major']
            && $existing->int('minor') === $release['minor']
            && $existing->int('patch') === $release['patch']
            && $existing->nullableInt('build') === $release['build']
            && $existing->int('release_order') === $release['release_order']
            && $existing->nullableString('protocol_family') === $release['protocol_family']
            && $existingReleasedAt === $incomingReleasedAt;

        if (! $matches) {
            throw new CatalogValidationException([
                new CatalogValidationFinding(
                    severity: 'error',
                    code: 'import.release_conflict',
                    message: "Release '{$release['key']}' conflicts with the immutable release registry.",
                    path: '$.releases',
                ),
            ]);
        }
    }

    /** @param array<string, int> $releaseIds */
    private function persistSnapshot(ValidatedCatalogSnapshot $validated, array $releaseIds, CarbonImmutable $now): int
    {
        $snapshot = $validated->payload['snapshot'];

        return (int) DB::table('game_catalog_snapshots')->insertGetId([
            'contract_version' => $validated->payload['contract'],
            'schema_version' => $validated->payload['schema_version'],
            'content_sha256' => $validated->contentSha256,
            'canary_commit_sha' => $snapshot['canary_commit_sha'],
            'datapack_commit_sha' => $snapshot['datapack_commit_sha'],
            'protocol_profile' => $snapshot['protocol_profile'],
            'runtime_release_id' => $releaseIds[$snapshot['runtime_release']],
            'content_target_release_id' => $releaseIds[$snapshot['content_target_release']],
            'verified_content_through_release_id' => $snapshot['verified_content_through_release'] === null
                ? null
                : $releaseIds[$snapshot['verified_content_through_release']],
            'contains_content_through_release_id' => $snapshot['contains_content_through_release'] === null
                ? null
                : $releaseIds[$snapshot['contains_content_through_release']],
            'appearances_sha256' => $snapshot['appearances_sha256'],
            'map_sha256' => $snapshot['map_sha256'],
            'producer_build_id' => $snapshot['producer_build_id'],
            'generated_at' => CarbonImmutable::parse($snapshot['generated_at'])->utc(),
            'imported_at' => $now,
            'status' => 'validating',
            'entity_count' => $snapshot['entity_count'],
            'relation_count' => $snapshot['relation_count'],
            'validation_summary' => $this->json(['status' => 'validating']),
            'created_at' => $now,
            'updated_at' => $now,
        ]);
    }

    /**
     * @param  list<CatalogEntity>  $entities
     * @param  array<string, int>  $releaseIds
     * @return array<string, int>
     */
    private function persistEntities(int $snapshotId, array $entities, array $releaseIds, CarbonImmutable $now): array
    {
        $entityIds = [];

        foreach ($entities as $entity) {
            $stable = DB::table('game_catalog_entities')->where('canonical_key', $entity['canonical_key'])->lockForUpdate()->first();
            if ($stable === null) {
                $entityId = (int) DB::table('game_catalog_entities')->insertGetId([
                    'entity_type' => $entity['type'],
                    'canonical_key' => $entity['canonical_key'],
                    'created_at' => $now,
                    'updated_at' => $now,
                ]);
            } else {
                $stableRow = CatalogDatabaseRow::from($stable);
                if ($stableRow->string('entity_type') !== $entity['type']) {
                    throw new CatalogValidationException([
                        new CatalogValidationFinding(
                            severity: 'error',
                            code: 'import.entity_type_conflict',
                            message: "Canonical key '{$entity['canonical_key']}' conflicts with an existing entity type.",
                            path: '$.entities',
                        ),
                    ]);
                }
                $entityId = $stableRow->int('id');
            }

            $entityIds[$entity['canonical_key']] = $entityId;
            $entitySnapshotId = (int) DB::table('game_catalog_entity_snapshots')->insertGetId([
                'snapshot_id' => $snapshotId,
                'entity_id' => $entityId,
                'introduced_release_id' => $entity['introduced_in'] === null ? null : $releaseIds[$entity['introduced_in']],
                'removed_release_id' => $entity['removed_in'] === null ? null : $releaseIds[$entity['removed_in']],
                'completeness' => $entity['completeness'],
                'availability' => $entity['availability'],
                'runtime_present' => $entity['runtime_present'],
                'enabled' => $entity['enabled'],
                'data_sha256' => hash('sha256', $this->canonicalJson($entity['data'])),
                'source_path' => $entity['source_path'],
                'source_key' => $entity['canonical_key'],
                'created_at' => $now,
                'updated_at' => $now,
            ]);

            foreach ($entity['identifiers'] as $identifier) {
                DB::table('game_catalog_entity_identifiers')->insert([
                    'snapshot_id' => $snapshotId,
                    'entity_id' => $entityId,
                    'namespace' => $identifier['namespace'],
                    'value' => $identifier['value'],
                    'created_at' => $now,
                ]);
            }

            if ($entity['type'] === 'item') {
                $this->persistItem($entitySnapshotId, $entity['data']);
            } else {
                $this->persistCreature($entitySnapshotId, $entity['data']);
            }

            $this->markTranslationsStaleWhenSourceNameChanged($entityId, $entity['data']['name']);
        }

        return $entityIds;
    }

    /** @param CatalogItemData $data */
    private function persistItem(int $entitySnapshotId, array $data): void
    {
        DB::table('game_catalog_item_snapshots')->insert([
            'entity_snapshot_id' => $entitySnapshotId,
            'server_id' => $data['server_id'],
            'client_id' => $data['client_id'],
            'ware_id' => $data['ware_id'],
            'name' => $data['name'],
            'description' => $data['description'],
            'category' => $data['category'],
            'weapon_type' => $data['weapon_type'],
            'attack' => $data['attack'],
            'defense' => $data['defense'],
            'extra_defense' => $data['extra_defense'],
            'armor' => $data['armor'],
            'range' => $data['range'],
            'weight' => $data['weight'],
            'minimum_level' => $data['minimum_level'],
            'vocations' => $data['vocations'] === null ? null : $this->json($data['vocations']),
            'slot_position' => $data['slot_position'],
            'imbuement_slots' => $data['imbuement_slots'],
            'upgrade_classification' => $data['upgrade_classification'],
            'element_type' => $data['element_type'],
            'element_value' => $data['element_value'],
            'stackable' => $data['stackable'],
            'pickupable' => $data['pickupable'],
            'image_key' => $data['image_key'],
            'attributes' => $this->json($data['attributes']),
        ]);
    }

    /** @param CatalogCreatureData $data */
    private function persistCreature(int $entitySnapshotId, array $data): void
    {
        DB::table('game_catalog_creature_snapshots')->insert([
            'entity_snapshot_id' => $entitySnapshotId,
            'name' => $data['name'],
            'description' => $data['description'],
            'race_id' => $data['race_id'],
            'look_type' => $data['look_type'],
            'health' => $data['health'],
            'max_health' => $data['max_health'],
            'experience' => $data['experience'],
            'speed' => $data['speed'],
            'armor' => $data['armor'],
            'defense' => $data['defense'],
            'mitigation' => $data['mitigation'],
            'is_boss' => $data['is_boss'],
            'is_reward_boss' => $data['is_reward_boss'],
            'bestiary_class' => $data['bestiary_class'],
            'bestiary_race' => $data['bestiary_race'],
            'bestiary_occurrence' => $data['bestiary_occurrence'],
            'bestiary_to_kill' => $data['bestiary_to_kill'],
            'charm_points' => $data['charm_points'],
            'elements' => $this->json($data['elements']),
            'immunities' => $this->json($data['immunities']),
            'attacks' => $this->json($data['attacks']),
            'defenses' => $this->json($data['defenses']),
            'attributes' => $this->json($data['attributes']),
        ]);
    }

    /**
     * @param  list<CatalogRelation>  $relations
     * @param  array<string, int>  $releaseIds
     * @param  array<string, int>  $entityIds
     */
    private function persistRelations(
        int $snapshotId,
        array $relations,
        array $releaseIds,
        array $entityIds,
        CarbonImmutable $now,
    ): void {
        foreach ($relations as $relation) {
            $relationSnapshotId = (int) DB::table('game_catalog_relation_snapshots')->insertGetId([
                'snapshot_id' => $snapshotId,
                'relation_type' => $relation['type'],
                'canonical_key' => $relation['canonical_key'],
                'source_entity_id' => $entityIds[$relation['source']],
                'target_entity_id' => $entityIds[$relation['target']],
                'introduced_release_id' => $relation['introduced_in'] === null ? null : $releaseIds[$relation['introduced_in']],
                'removed_release_id' => $relation['removed_in'] === null ? null : $releaseIds[$relation['removed_in']],
                'completeness' => $relation['completeness'],
                'enabled' => $relation['enabled'],
                'data_sha256' => hash('sha256', $this->canonicalJson($relation['data'])),
                'source_path' => $relation['source_path'],
                'attributes' => $this->json([]),
                'created_at' => $now,
                'updated_at' => $now,
            ]);

            $data = $relation['data'];
            DB::table('game_catalog_loot_snapshots')->insert([
                'relation_snapshot_id' => $relationSnapshotId,
                'chance_numerator' => $data['chance_numerator'],
                'chance_denominator' => $data['chance_denominator'],
                'minimum_count' => $data['minimum_count'],
                'maximum_count' => $data['maximum_count'],
                'container_path' => $data['container_path'],
                'condition_data' => $data['condition_data'] === null ? null : $this->json($data['condition_data']),
            ]);
        }
    }

    private function verifyPersistedSnapshot(int $snapshotId, ValidatedCatalogSnapshot $validated): void
    {
        $entityCount = DB::table('game_catalog_entity_snapshots')->where('snapshot_id', $snapshotId)->count();
        $relationCount = DB::table('game_catalog_relation_snapshots')->where('snapshot_id', $snapshotId)->count();
        $itemCount = DB::table('game_catalog_entity_snapshots as snapshots')
            ->join('game_catalog_item_snapshots as items', 'items.entity_snapshot_id', '=', 'snapshots.id')
            ->where('snapshots.snapshot_id', $snapshotId)
            ->count();
        $creatureCount = DB::table('game_catalog_entity_snapshots as snapshots')
            ->join('game_catalog_creature_snapshots as creatures', 'creatures.entity_snapshot_id', '=', 'snapshots.id')
            ->where('snapshots.snapshot_id', $snapshotId)
            ->count();
        $lootCount = DB::table('game_catalog_relation_snapshots as relations')
            ->join('game_catalog_loot_snapshots as loot', 'loot.relation_snapshot_id', '=', 'relations.id')
            ->where('relations.snapshot_id', $snapshotId)
            ->count();

        $declaredItems = count(array_filter($validated->payload['entities'], fn (array $entity): bool => $entity['type'] === 'item'));
        $declaredCreatures = count($validated->payload['entities']) - $declaredItems;

        if (
            $entityCount !== count($validated->payload['entities'])
            || $relationCount !== count($validated->payload['relations'])
            || $itemCount !== $declaredItems
            || $creatureCount !== $declaredCreatures
            || $lootCount !== count($validated->payload['relations'])
        ) {
            throw new CatalogValidationException([
                new CatalogValidationFinding(
                    severity: 'error',
                    code: 'import.persisted_count_mismatch',
                    message: 'Persisted Game Catalog counts do not match the validated snapshot.',
                    path: '$',
                ),
            ]);
        }
    }

    private function markTranslationsStaleWhenSourceNameChanged(int $entityId, string $sourceName): void
    {
        DB::table('game_catalog_entity_translations')
            ->where('entity_id', $entityId)
            ->where('source_name_sha256', '!=', hash('sha256', $sourceName))
            ->update([
                'translation_status' => 'stale',
                'updated_at' => CarbonImmutable::now('UTC'),
            ]);
    }

    private function startImportRun(ValidatedCatalogSnapshot $validated): int
    {
        $now = CarbonImmutable::now('UTC');

        return (int) DB::table('game_catalog_import_runs')->insertGetId([
            'content_sha256' => $validated->contentSha256,
            'snapshot_id' => null,
            'status' => 'importing',
            'source_label' => $validated->sourceLabel,
            'file_size' => $validated->fileSize,
            'finding_count' => 0,
            'started_at' => $now,
            'finished_at' => null,
            'summary' => $this->json(['status' => 'importing']),
            'created_at' => $now,
            'updated_at' => $now,
        ]);
    }

    private function recordDeduplicatedRun(ValidatedCatalogSnapshot $validated, int $snapshotId): int
    {
        $now = CarbonImmutable::now('UTC');

        return (int) DB::table('game_catalog_import_runs')->insertGetId([
            'content_sha256' => $validated->contentSha256,
            'snapshot_id' => $snapshotId,
            'status' => 'deduplicated',
            'source_label' => $validated->sourceLabel,
            'file_size' => $validated->fileSize,
            'finding_count' => 0,
            'started_at' => $now,
            'finished_at' => $now,
            'summary' => $this->json([
                'deduplicated' => true,
                'snapshot_id' => $snapshotId,
            ]),
            'created_at' => $now,
            'updated_at' => $now,
        ]);
    }

    private function recordRejectedValidation(string $path, CatalogValidationException $exception): void
    {
        if ($exception->contentSha256 === null || $exception->fileSize === null) {
            return;
        }

        $now = CarbonImmutable::now('UTC');
        $maximumFindings = CatalogConfiguration::positiveInt('game-catalog.limits.validation_findings', 2_000);
        $findings = array_slice($exception->findings, 0, $maximumFindings);
        $runId = (int) DB::table('game_catalog_import_runs')->insertGetId([
            'content_sha256' => $exception->contentSha256,
            'snapshot_id' => null,
            'status' => 'rejected',
            'source_label' => basename($path),
            'file_size' => $exception->fileSize,
            'finding_count' => count($findings),
            'started_at' => $now,
            'finished_at' => $now,
            'summary' => $this->json(['validation_failed' => true]),
            'created_at' => $now,
            'updated_at' => $now,
        ]);

        foreach ($findings as $finding) {
            $this->insertFinding($runId, null, $finding, $now);
        }
    }

    private function recordPersistenceFailure(int $runId, ValidatedCatalogSnapshot $validated, Throwable $exception): void
    {
        $now = CarbonImmutable::now('UTC');
        $finding = new CatalogValidationFinding(
            severity: 'error',
            code: 'import.persistence_failure',
            message: mb_substr($exception->getMessage(), 0, 1_000, 'UTF-8'),
            path: '$',
        );

        DB::table('game_catalog_import_runs')->where('id', $runId)->update([
            'snapshot_id' => null,
            'status' => 'rejected',
            'finding_count' => 1,
            'finished_at' => $now,
            'summary' => $this->json([
                'validation_failed' => false,
                'content_sha256' => $validated->contentSha256,
            ]),
            'updated_at' => $now,
        ]);
        $this->insertFinding($runId, null, $finding, $now);
    }

    private function insertFinding(int $runId, ?int $snapshotId, CatalogValidationFinding $finding, CarbonImmutable $now): void
    {
        DB::table('game_catalog_validation_findings')->insert([
            'import_run_id' => $runId,
            'snapshot_id' => $snapshotId,
            'severity' => mb_substr($finding->severity, 0, 16, 'UTF-8'),
            'code' => mb_substr($finding->code, 0, 80, 'UTF-8'),
            'path' => $finding->path === null ? null : mb_substr($finding->path, 0, 512, 'UTF-8'),
            'message' => mb_substr($finding->message, 0, 1_000, 'UTF-8'),
            'context' => $finding->context === [] ? null : $this->json($finding->context),
            'created_at' => $now,
        ]);
    }

    private function canonicalJson(mixed $value): string
    {
        return $this->json($this->sortRecursively($value));
    }

    private function sortRecursively(mixed $value): mixed
    {
        if (! is_array($value)) {
            return $value;
        }

        if (array_is_list($value)) {
            return array_map(fn (mixed $item): mixed => $this->sortRecursively($item), $value);
        }

        ksort($value, SORT_STRING);
        foreach ($value as $key => $item) {
            $value[$key] = $this->sortRecursively($item);
        }

        return $value;
    }

    private function json(mixed $value): string
    {
        try {
            return json_encode(
                $value,
                JSON_THROW_ON_ERROR | JSON_UNESCAPED_SLASHES | JSON_UNESCAPED_UNICODE | JSON_PRESERVE_ZERO_FRACTION,
            );
        } catch (JsonException $exception) {
            throw new CatalogValidationException([
                new CatalogValidationFinding('error', 'import.json_encoding', 'Validated catalogue data could not be encoded for persistence.', '$'),
            ], previous: $exception);
        }
    }
}
