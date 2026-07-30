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
 * @phpstan-import-type CatalogRelation from ValidatedCatalogSnapshot
 * @phpstan-import-type CatalogItemData from ValidatedCatalogSnapshot
 * @phpstan-import-type CatalogCreatureData from ValidatedCatalogSnapshot
 * @phpstan-import-type CatalogNpcData from ValidatedCatalogSnapshot
 * @phpstan-import-type CatalogLootData from ValidatedCatalogSnapshot
 * @phpstan-import-type CatalogShopOfferData from ValidatedCatalogSnapshot
 */
final class CatalogImportService
{
    public function __construct(private readonly CatalogSnapshotValidator $validator, private readonly ConnectionInterface $database) {}

    public function import(string $path, ?string $expectedSha256 = null): CatalogImportResult
    {
        try {
            $validated = $this->validator->validate($path, $expectedSha256);
        } catch (CatalogValidationException $exception) {
            $this->recordRejectedValidation($path, $exception);
            throw $exception;
        }
        $existing = DB::table('game_catalog_snapshots')->where('content_sha256', $validated->contentSha256)->first(['id']);
        if ($existing !== null) {
            $snapshotId = CatalogDatabaseRow::from($existing)->int('id');

            return new CatalogImportResult($snapshotId, $this->recordDeduplicatedRun($validated, $snapshotId), $validated->contentSha256, true);
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
                $summary = $this->validationSummary($validated);
                DB::table('game_catalog_snapshots')->where('id', $snapshotId)->update([
                    'status' => 'validated', 'validation_summary' => $this->json($summary), 'updated_at' => $now,
                ]);
                DB::table('game_catalog_import_runs')->where('id', $runId)->update([
                    'snapshot_id' => $snapshotId, 'status' => 'validated', 'finding_count' => 0, 'finished_at' => $now,
                    'summary' => $this->json(['deduplicated' => false, 'snapshot_id' => $snapshotId, 'content_sha256' => $validated->contentSha256, 'entity_types' => $summary['entity_types'], 'relation_types' => $summary['relation_types']]),
                    'updated_at' => $now,
                ]);

                return $snapshotId;
            }, 3);
        } catch (Throwable $exception) {
            $this->recordPersistenceFailure($runId, $validated, $exception);
            throw $exception;
        }

        return new CatalogImportResult($snapshotId, $runId, $validated->contentSha256, false);
    }

    /** @param list<CatalogRelease> $releases
     * @return array<string, int>
     */
    private function persistReleases(array $releases, CarbonImmutable $now): array
    {
        $ids = [];
        foreach ($releases as $release) {
            $existing = DB::table('game_catalog_releases')->where('key', $release['key'])->lockForUpdate()->first();
            $releasedAt = $release['released_at'] === null ? null : CarbonImmutable::parse($release['released_at'])->utc();
            if ($existing === null) {
                $ids[$release['key']] = (int) DB::table('game_catalog_releases')->insertGetId([
                    'key' => $release['key'], 'display_label' => $release['display_label'], 'major' => $release['major'], 'minor' => $release['minor'], 'patch' => $release['patch'],
                    'build' => $release['build'], 'release_order' => $release['release_order'], 'protocol_family' => $release['protocol_family'], 'released_at' => $releasedAt,
                    'created_at' => $now, 'updated_at' => $now,
                ]);

                continue;
            }
            $row = CatalogDatabaseRow::from($existing);
            $storedReleasedAt = $row->nullableString('released_at');
            $storedReleasedAt = $storedReleasedAt === null ? null : CarbonImmutable::parse($storedReleasedAt)->utc()->format('Y-m-d\TH:i:s.u\Z');
            $matches = $row->string('display_label') === $release['display_label'] && $row->int('major') === $release['major'] && $row->int('minor') === $release['minor']
                && $row->int('patch') === $release['patch'] && $row->nullableInt('build') === $release['build'] && $row->int('release_order') === $release['release_order']
                && $row->nullableString('protocol_family') === $release['protocol_family'] && $storedReleasedAt === $releasedAt?->format('Y-m-d\TH:i:s.u\Z');
            if (! $matches) {
                throw new CatalogValidationException([new CatalogValidationFinding('error', 'import.release_conflict', "Release '{$release['key']}' conflicts with the immutable release registry.", '$.releases')]);
            }
            $ids[$release['key']] = $row->int('id');
        }

        return $ids;
    }

    /** @param array<string, int> $releaseIds */
    private function persistSnapshot(ValidatedCatalogSnapshot $validated, array $releaseIds, CarbonImmutable $now): int
    {
        $s = $validated->payload['snapshot'];

        return (int) DB::table('game_catalog_snapshots')->insertGetId([
            'contract_version' => $validated->payload['contract'], 'schema_version' => $validated->payload['schema_version'], 'content_sha256' => $validated->contentSha256,
            'canary_commit_sha' => $s['canary_commit_sha'], 'datapack_commit_sha' => $s['datapack_commit_sha'], 'protocol_profile' => $s['protocol_profile'],
            'runtime_release_id' => $releaseIds[$s['runtime_release']], 'content_target_release_id' => $releaseIds[$s['content_target_release']],
            'verified_content_through_release_id' => $s['verified_content_through_release'] === null ? null : $releaseIds[$s['verified_content_through_release']],
            'contains_content_through_release_id' => $s['contains_content_through_release'] === null ? null : $releaseIds[$s['contains_content_through_release']],
            'appearances_sha256' => $s['appearances_sha256'], 'map_sha256' => $s['map_sha256'], 'producer_build_id' => $s['producer_build_id'],
            'generated_at' => CarbonImmutable::parse($s['generated_at'])->utc(), 'imported_at' => $now, 'status' => 'validating',
            'entity_count' => $s['entity_count'], 'relation_count' => $s['relation_count'], 'validation_summary' => $this->json(['status' => 'validating']),
            'created_at' => $now, 'updated_at' => $now,
        ]);
    }

    /** @param list<CatalogEntity> $entities
     * @param  array<string, int>  $releaseIds
     * @return array<string, int>
     */
    private function persistEntities(int $snapshotId, array $entities, array $releaseIds, CarbonImmutable $now): array
    {
        $ids = [];
        foreach ($entities as $entity) {
            $stable = DB::table('game_catalog_entities')->where('canonical_key', $entity['canonical_key'])->lockForUpdate()->first();
            if ($stable === null) {
                $entityId = (int) DB::table('game_catalog_entities')->insertGetId(['entity_type' => $entity['type'], 'canonical_key' => $entity['canonical_key'], 'created_at' => $now, 'updated_at' => $now]);
            } else {
                $row = CatalogDatabaseRow::from($stable);
                if ($row->string('entity_type') !== $entity['type']) {
                    throw new CatalogValidationException([new CatalogValidationFinding('error', 'import.entity_type_conflict', "Canonical key '{$entity['canonical_key']}' conflicts with an existing entity type.", '$.entities')]);
                }
                $entityId = $row->int('id');
            }
            $ids[$entity['canonical_key']] = $entityId;
            $entitySnapshotId = (int) DB::table('game_catalog_entity_snapshots')->insertGetId([
                'snapshot_id' => $snapshotId, 'entity_id' => $entityId,
                'introduced_release_id' => $entity['introduced_in'] === null ? null : $releaseIds[$entity['introduced_in']],
                'removed_release_id' => $entity['removed_in'] === null ? null : $releaseIds[$entity['removed_in']],
                'completeness' => $entity['completeness'], 'availability' => $entity['availability'], 'runtime_present' => $entity['runtime_present'], 'enabled' => $entity['enabled'],
                'data_sha256' => hash('sha256', $this->canonicalJson($entity['data'])), 'source_path' => $entity['source_path'], 'source_key' => $entity['canonical_key'],
                'created_at' => $now, 'updated_at' => $now,
            ]);
            foreach ($entity['identifiers'] as $identifier) {
                DB::table('game_catalog_entity_identifiers')->insert(['snapshot_id' => $snapshotId, 'entity_id' => $entityId, 'namespace' => $identifier['namespace'], 'value' => $identifier['value'], 'created_at' => $now]);
            }
            $name = match ($entity['type']) {
                'item' => $this->persistItem($entitySnapshotId, $entity['data']),
                'creature' => $this->persistCreature($entitySnapshotId, $entity['data']),
                'npc' => $this->persistNpc($entitySnapshotId, $entity['data'], $ids),
            };
            $this->markTranslationsStaleWhenSourceNameChanged($entityId, $name);
        }

        return $ids;
    }

    /** @param CatalogItemData $data */
    private function persistItem(int $id, array $data): string
    {
        $row = $data;
        $row['entity_snapshot_id'] = $id;
        $row['vocations'] = $data['vocations'] === null ? null : $this->json($data['vocations']);
        $row['attributes'] = $this->json($data['attributes']);
        DB::table('game_catalog_item_snapshots')->insert($row);

        return $data['name'];
    }

    /** @param CatalogCreatureData $data */
    private function persistCreature(int $id, array $data): string
    {
        $row = $data;
        $row['entity_snapshot_id'] = $id;
        foreach (['elements', 'immunities', 'attacks', 'defenses', 'attributes'] as $field) {
            $row[$field] = $this->json($data[$field]);
        }
        DB::table('game_catalog_creature_snapshots')->insert($row);

        return $data['name'];
    }

    /** @param CatalogNpcData $data
     * @param  array<string, int>  $entityIds
     */
    private function persistNpc(int $id, array $data, array $entityIds): string
    {
        DB::table('game_catalog_npc_snapshots')->insert([
            'entity_snapshot_id' => $id, 'registry_key' => $data['registry_key'], 'runtime_name' => $data['runtime_name'], 'display_name' => $data['display_name'],
            'type_name' => $data['type_name'], 'name_description' => $data['name_description'], 'aliases' => $this->json($data['aliases']),
            'registration_status' => $data['registration_status'], 'currency_entity_id' => $entityIds[$data['currency']['item']],
            'currency_server_id' => $data['currency']['server_id'], 'attributes' => $this->json($data['attributes']),
        ]);

        return $data['display_name'] ?? $data['runtime_name'];
    }

    /** @param list<CatalogRelation> $relations
     * @param  array<string, int>  $releaseIds
     * @param  array<string, int>  $entityIds
     */
    private function persistRelations(int $snapshotId, array $relations, array $releaseIds, array $entityIds, CarbonImmutable $now): void
    {
        foreach ($relations as $relation) {
            $id = (int) DB::table('game_catalog_relation_snapshots')->insertGetId([
                'snapshot_id' => $snapshotId, 'relation_type' => $relation['type'], 'canonical_key' => $relation['canonical_key'],
                'source_entity_id' => $entityIds[$relation['source']], 'target_entity_id' => $entityIds[$relation['target']],
                'introduced_release_id' => $relation['introduced_in'] === null ? null : $releaseIds[$relation['introduced_in']],
                'removed_release_id' => $relation['removed_in'] === null ? null : $releaseIds[$relation['removed_in']],
                'completeness' => $relation['completeness'], 'availability' => $relation['type'] === 'creature_loot' ? null : $relation['availability'],
                'enabled' => $relation['enabled'], 'data_sha256' => hash('sha256', $this->canonicalJson($relation['data'])), 'source_path' => $relation['source_path'],
                'attributes' => $this->json([]), 'created_at' => $now, 'updated_at' => $now,
            ]);
            if ($relation['type'] === 'creature_loot') {
                $this->persistLoot($id, $relation['data']);
            } else {
                $this->persistShopOffer($id, $relation['type'] === 'npc_buy_offer' ? 'buy' : 'sell', $relation['data'], $entityIds);
            }
        }
    }

    /** @param CatalogLootData $data */
    private function persistLoot(int $id, array $data): void
    {
        $threshold = isset($data['chance_model']);
        DB::table('game_catalog_loot_snapshots')->insert([
            'relation_snapshot_id' => $id, 'chance_model' => $threshold ? $data['chance_model'] : 'rational_probability',
            'chance_numerator' => $threshold ? null : $data['chance_numerator'], 'chance_denominator' => $threshold ? null : $data['chance_denominator'],
            'chance_threshold' => $threshold ? $data['chance_threshold'] : null, 'roll_maximum' => $threshold ? $data['roll_maximum'] : null,
            'minimum_count' => $data['minimum_count'], 'maximum_count' => $data['maximum_count'], 'container_path' => $data['container_path'],
            'condition_data' => $data['condition_data'] === null ? null : $this->json($data['condition_data']),
        ]);
    }

    /** @param CatalogShopOfferData $data
     * @param  array<string, int>  $entityIds
     */
    private function persistShopOffer(int $id, string $direction, array $data, array $entityIds): void
    {
        $storage = $data['storage_requirement'];
        DB::table('game_catalog_shop_offer_snapshots')->insert([
            'relation_snapshot_id' => $id, 'direction' => $direction, 'currency_entity_id' => $entityIds[$data['currency']['item']],
            'currency_server_id' => $data['currency']['server_id'], 'runtime_path' => $this->json($data['runtime_path']), 'item_name' => $data['item_name'],
            'item_subtype' => $data['item_subtype'], 'priced_item_count' => $data['priced_item_count'], 'price_amount' => $data['price_amount'],
            'storage_key' => $storage['key'] ?? null, 'storage_value' => $storage['value'] ?? null, 'attributes' => $this->json($data['attributes']),
        ]);
    }

    private function verifyPersistedSnapshot(int $snapshotId, ValidatedCatalogSnapshot $validated): void
    {
        $entities = $this->typeCounts($validated->payload['entities']);
        $relations = $this->typeCounts($validated->payload['relations']);
        $actual = [
            'entities' => DB::table('game_catalog_entity_snapshots')->where('snapshot_id', $snapshotId)->count(),
            'relations' => DB::table('game_catalog_relation_snapshots')->where('snapshot_id', $snapshotId)->count(),
            'item' => $this->typedEntityCount($snapshotId, 'game_catalog_item_snapshots'), 'creature' => $this->typedEntityCount($snapshotId, 'game_catalog_creature_snapshots'),
            'npc' => $this->typedEntityCount($snapshotId, 'game_catalog_npc_snapshots'), 'creature_loot' => $this->typedRelationCount($snapshotId, 'game_catalog_loot_snapshots', 'creature_loot'),
            'npc_buy_offer' => $this->typedShopCount($snapshotId, 'npc_buy_offer', 'buy'), 'npc_sell_offer' => $this->typedShopCount($snapshotId, 'npc_sell_offer', 'sell'),
        ];
        if ($actual['entities'] !== count($validated->payload['entities']) || $actual['relations'] !== count($validated->payload['relations'])
            || $actual['item'] !== ($entities['item'] ?? 0) || $actual['creature'] !== ($entities['creature'] ?? 0) || $actual['npc'] !== ($entities['npc'] ?? 0)
            || $actual['creature_loot'] !== ($relations['creature_loot'] ?? 0) || $actual['npc_buy_offer'] !== ($relations['npc_buy_offer'] ?? 0)
            || $actual['npc_sell_offer'] !== ($relations['npc_sell_offer'] ?? 0)) {
            throw new CatalogValidationException([new CatalogValidationFinding('error', 'import.persisted_count_mismatch', 'Persisted Game Catalog typed counts do not match the validated snapshot.', '$')]);
        }
    }

    private function typedEntityCount(int $snapshotId, string $table): int
    {
        return DB::table('game_catalog_entity_snapshots as snapshots')->join($table.' as typed', 'typed.entity_snapshot_id', '=', 'snapshots.id')->where('snapshots.snapshot_id', $snapshotId)->count();
    }

    private function typedRelationCount(int $snapshotId, string $table, string $type): int
    {
        return DB::table('game_catalog_relation_snapshots as relations')->join($table.' as typed', 'typed.relation_snapshot_id', '=', 'relations.id')->where('relations.snapshot_id', $snapshotId)->where('relations.relation_type', $type)->count();
    }

    private function typedShopCount(int $snapshotId, string $type, string $direction): int
    {
        return DB::table('game_catalog_relation_snapshots as relations')->join('game_catalog_shop_offer_snapshots as offers', 'offers.relation_snapshot_id', '=', 'relations.id')
            ->where('relations.snapshot_id', $snapshotId)->where('relations.relation_type', $type)->where('offers.direction', $direction)->count();
    }

    /** @param list<array{type: string}> $rows
     * @return array<string, int>
     */
    private function typeCounts(array $rows): array
    {
        $counts = [];
        foreach ($rows as $row) {
            $counts[$row['type']] = ($counts[$row['type']] ?? 0) + 1;
        }
        ksort($counts, SORT_STRING);

        return $counts;
    }

    /** @return array<string, mixed> */
    private function validationSummary(ValidatedCatalogSnapshot $validated): array
    {
        $unknown = 0;
        foreach ($validated->payload['entities'] as $entity) {
            if ($entity['availability'] === 'unknown' || $entity['completeness'] === 'unverified') {
                $unknown++;
            }
        }

        return ['errors' => 0, 'warnings' => 0, 'schema_sha256' => $validated->schemaSha256, 'entity_count' => count($validated->payload['entities']),
            'relation_count' => count($validated->payload['relations']), 'entity_types' => $this->typeCounts($validated->payload['entities']),
            'relation_types' => $this->typeCounts($validated->payload['relations']), 'unknown_or_unverified_entity_count' => $unknown];
    }

    private function markTranslationsStaleWhenSourceNameChanged(int $entityId, string $sourceName): void
    {
        DB::table('game_catalog_entity_translations')->where('entity_id', $entityId)->where('source_name_sha256', '!=', hash('sha256', $sourceName))
            ->update(['translation_status' => 'stale', 'updated_at' => CarbonImmutable::now('UTC')]);
    }

    private function startImportRun(ValidatedCatalogSnapshot $validated): int
    {
        $now = CarbonImmutable::now('UTC');

        return (int) DB::table('game_catalog_import_runs')->insertGetId(['content_sha256' => $validated->contentSha256, 'snapshot_id' => null, 'status' => 'importing',
            'source_label' => $validated->sourceLabel, 'file_size' => $validated->fileSize, 'finding_count' => 0, 'started_at' => $now, 'finished_at' => null,
            'summary' => $this->json(['status' => 'importing']), 'created_at' => $now, 'updated_at' => $now]);
    }

    private function recordDeduplicatedRun(ValidatedCatalogSnapshot $validated, int $snapshotId): int
    {
        $now = CarbonImmutable::now('UTC');

        return (int) DB::table('game_catalog_import_runs')->insertGetId(['content_sha256' => $validated->contentSha256, 'snapshot_id' => $snapshotId, 'status' => 'deduplicated',
            'source_label' => $validated->sourceLabel, 'file_size' => $validated->fileSize, 'finding_count' => 0, 'started_at' => $now, 'finished_at' => $now,
            'summary' => $this->json(['deduplicated' => true, 'snapshot_id' => $snapshotId]), 'created_at' => $now, 'updated_at' => $now]);
    }

    private function recordRejectedValidation(string $path, CatalogValidationException $exception): void
    {
        if ($exception->contentSha256 === null || $exception->fileSize === null) {
            return;
        }
        $now = CarbonImmutable::now('UTC');
        $findings = array_slice($exception->findings, 0, CatalogConfiguration::positiveInt('game-catalog.limits.validation_findings', 2_000));
        $runId = (int) DB::table('game_catalog_import_runs')->insertGetId(['content_sha256' => $exception->contentSha256, 'snapshot_id' => null, 'status' => 'rejected',
            'source_label' => basename($path), 'file_size' => $exception->fileSize, 'finding_count' => count($findings), 'started_at' => $now, 'finished_at' => $now,
            'summary' => $this->json(['validation_failed' => true]), 'created_at' => $now, 'updated_at' => $now]);
        foreach ($findings as $finding) {
            $this->insertFinding($runId, null, $finding, $now);
        }
    }

    private function recordPersistenceFailure(int $runId, ValidatedCatalogSnapshot $validated, Throwable $exception): void
    {
        $now = CarbonImmutable::now('UTC');
        $finding = new CatalogValidationFinding('error', 'import.persistence_failure', mb_substr($exception->getMessage(), 0, 1_000, 'UTF-8'), '$');
        DB::table('game_catalog_import_runs')->where('id', $runId)->update(['snapshot_id' => null, 'status' => 'rejected', 'finding_count' => 1, 'finished_at' => $now,
            'summary' => $this->json(['validation_failed' => false, 'content_sha256' => $validated->contentSha256]), 'updated_at' => $now]);
        $this->insertFinding($runId, null, $finding, $now);
    }

    private function insertFinding(int $runId, ?int $snapshotId, CatalogValidationFinding $finding, CarbonImmutable $now): void
    {
        DB::table('game_catalog_validation_findings')->insert(['import_run_id' => $runId, 'snapshot_id' => $snapshotId,
            'severity' => mb_substr($finding->severity, 0, 16, 'UTF-8'), 'code' => mb_substr($finding->code, 0, 80, 'UTF-8'),
            'path' => $finding->path === null ? null : mb_substr($finding->path, 0, 512, 'UTF-8'), 'message' => mb_substr($finding->message, 0, 1_000, 'UTF-8'),
            'context' => $finding->context === [] ? null : $this->json($finding->context), 'created_at' => $now]);
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
            return json_encode($value, JSON_THROW_ON_ERROR | JSON_UNESCAPED_SLASHES | JSON_UNESCAPED_UNICODE | JSON_PRESERVE_ZERO_FRACTION);
        } catch (JsonException $exception) {
            throw new CatalogValidationException([new CatalogValidationFinding('error', 'import.json_encoding', 'Validated catalogue data could not be encoded for persistence.', '$')], previous: $exception);
        }
    }
}
