<?php

namespace App\GameCatalog\Application\Import;

use App\GameCatalog\Contract\GameCatalogContract;
use App\GameCatalog\Support\CanonicalJson;
use App\GameCatalog\Validation\CatalogValidationException;
use App\GameCatalog\Validation\GameCatalogDocumentValidator;
use Illuminate\Support\Facades\DB;
use RuntimeException;
use Throwable;

final readonly class CatalogImporter
{
    public function __construct(private GameCatalogDocumentValidator $validator) {}

    public function import(string $path, ?string $expectedSha256 = null): CatalogImportResult
    {
        $runId = $this->startRun($path);

        try {
            $validated = $this->validator->validatePath($path, $expectedSha256);
            DB::table('game_catalog_import_runs')->where('id', $runId)->update([
                'content_sha256' => $validated->contentSha256,
                'byte_size' => $validated->byteSize,
                'updated_at' => now(),
            ]);

            $existing = DB::table('game_catalog_snapshots')
                ->where('content_sha256', $validated->contentSha256)
                ->first(['id', 'status']);
            if ($existing !== null) {
                if ($existing->status !== 'validated') {
                    throw new RuntimeException('An identical snapshot hash exists in a non-validated state.');
                }

                $snapshotId = (int) $existing->id;
                $this->finishRun($runId, $snapshotId, 'Identical immutable snapshot already imported.');

                return new CatalogImportResult($snapshotId, $runId, $validated->contentSha256, true);
            }

            $snapshotId = DB::transaction(function () use ($validated): int {
                $document = $validated->document;
                $releaseIds = $this->persistReleases($document['releases']);
                $snapshotId = $this->persistSnapshot(
                    $document['snapshot'],
                    $releaseIds,
                    $validated->contentSha256,
                    $validated->byteSize,
                );

                foreach ($releaseIds as $releaseId) {
                    DB::table('game_catalog_snapshot_releases')->insert([
                        'snapshot_id' => $snapshotId,
                        'release_id' => $releaseId,
                    ]);
                }

                $this->ensureDefaultProfile($document['snapshot']['content_target_release'], $releaseIds);
                $entityIds = $this->persistEntities($snapshotId, $document['entities'], $releaseIds);
                $this->persistRelations($snapshotId, $document['relations'], $releaseIds, $entityIds);
                $this->verifyPersistedCounts(
                    $snapshotId,
                    (int) $document['snapshot']['entity_count'],
                    (int) $document['snapshot']['relation_count'],
                );

                return $snapshotId;
            }, 3);

            $this->finishRun($runId, $snapshotId, 'Snapshot imported as inactive immutable state.');

            return new CatalogImportResult($snapshotId, $runId, $validated->contentSha256, false);
        } catch (CatalogValidationException $exception) {
            $this->rejectRun($runId, $exception->findings(), 'Snapshot validation failed.');
            throw $exception;
        } catch (Throwable $exception) {
            $this->rejectRun($runId, [], 'Snapshot import failed before activation.');
            throw $exception;
        }
    }

    private function startRun(string $path): int
    {
        $now = now();

        return DB::table('game_catalog_import_runs')->insertGetId([
            'snapshot_id' => null,
            'content_sha256' => null,
            'source_name' => mb_substr(basename($path), 0, 255),
            'status' => 'validating',
            'byte_size' => null,
            'finding_count' => 0,
            'summary' => null,
            'started_at' => $now,
            'finished_at' => null,
            'created_at' => $now,
            'updated_at' => $now,
        ]);
    }

    private function finishRun(int $runId, int $snapshotId, string $summary): void
    {
        DB::table('game_catalog_import_runs')->where('id', $runId)->update([
            'snapshot_id' => $snapshotId,
            'status' => 'validated',
            'summary' => $summary,
            'finished_at' => now(),
            'updated_at' => now(),
        ]);
    }

    /**
     * @param  list<array<string, mixed>>  $releases
     * @return array<string, int>
     */
    private function persistReleases(array $releases): array
    {
        $ids = [];
        foreach ($releases as $release) {
            $existing = DB::table('game_catalog_releases')->where('key', $release['key'])->first();
            if ($existing !== null) {
                $this->assertReleaseMatches($existing, $release);
                $ids[$release['key']] = (int) $existing->id;
                continue;
            }

            $now = now();
            $ids[$release['key']] = DB::table('game_catalog_releases')->insertGetId([
                'key' => $release['key'],
                'display_label' => $release['display_label'],
                'major' => $release['major'],
                'minor' => $release['minor'],
                'patch' => $release['patch'],
                'build' => $release['build'],
                'release_order' => $release['release_order'],
                'protocol_family' => $release['protocol_family'],
                'released_at' => $release['released_at'],
                'created_at' => $now,
                'updated_at' => $now,
            ]);
        }

        return $ids;
    }

    /** @param array<string, mixed> $release */
    private function assertReleaseMatches(object $existing, array $release): void
    {
        $expected = [
            'display_label' => $release['display_label'],
            'major' => $release['major'],
            'minor' => $release['minor'],
            'patch' => $release['patch'],
            'build' => $release['build'],
            'release_order' => $release['release_order'],
            'protocol_family' => $release['protocol_family'],
        ];

        foreach ($expected as $field => $value) {
            if ((string) ($existing->{$field} ?? '') !== (string) ($value ?? '')) {
                throw new CatalogValidationException([[
                    'code' => 'semantic.conflicting_release',
                    'path' => '$/releases',
                    'message' => "Retained release [{$release['key']}] conflicts with the imported definition.",
                ]]);
            }
        }
    }

    /**
     * @param  array<string, mixed>  $snapshot
     * @param  array<string, int>  $releaseIds
     */
    private function persistSnapshot(
        array $snapshot,
        array $releaseIds,
        string $contentSha256,
        int $byteSize,
    ): int {
        $now = now();

        return DB::table('game_catalog_snapshots')->insertGetId([
            'contract_id' => GameCatalogContract::ID,
            'schema_version' => GameCatalogContract::SCHEMA_VERSION,
            'content_sha256' => $contentSha256,
            'byte_size' => $byteSize,
            'canary_commit_sha' => $snapshot['canary_commit_sha'],
            'datapack_commit_sha' => $snapshot['datapack_commit_sha'] ?? null,
            'protocol_profile' => $snapshot['protocol_profile'],
            'runtime_release_id' => $releaseIds[$snapshot['runtime_release']],
            'content_target_release_id' => $releaseIds[$snapshot['content_target_release']],
            'verified_content_through_release_id' => $releaseIds[$snapshot['verified_content_through_release']],
            'contains_content_through_release_id' => isset($snapshot['contains_content_through_release'])
                ? $releaseIds[$snapshot['contains_content_through_release']]
                : null,
            'appearances_sha256' => $snapshot['appearances_sha256'],
            'map_sha256' => $snapshot['map_sha256'] ?? null,
            'producer_build_id' => $snapshot['producer_build_id'] ?? null,
            'generated_at' => $snapshot['generated_at'],
            'imported_at' => $now,
            'status' => 'validated',
            'entity_count' => $snapshot['entity_count'],
            'relation_count' => $snapshot['relation_count'],
            'validation_summary' => $this->json(['errors' => 0, 'warnings' => 0]),
            'created_at' => $now,
            'updated_at' => $now,
        ]);
    }

    /** @param array<string, int> $releaseIds */
    private function ensureDefaultProfile(string $targetRelease, array $releaseIds): void
    {
        if (DB::table('game_catalog_profiles')->where('key', 'oteryn-current')->exists()) {
            return;
        }

        $now = now();
        DB::table('game_catalog_profiles')->insert([
            'key' => 'oteryn-current',
            'name' => 'Oteryn Current',
            'target_release_id' => $releaseIds[$targetRelease],
            'active_snapshot_id' => null,
            'complete_only' => true,
            'public_enabled' => true,
            'allow_backports' => false,
            'lock_version' => 0,
            'created_at' => $now,
            'updated_at' => $now,
        ]);
    }

    /**
     * @param  list<array<string, mixed>>  $entities
     * @param  array<string, int>  $releaseIds
     * @return array<string, int>
     */
    private function persistEntities(int $snapshotId, array $entities, array $releaseIds): array
    {
        $entityIds = [];
        foreach ($entities as $entity) {
            $entityId = $this->stableEntityId($entity['type'], $entity['canonical_key']);
            $entityIds[$entity['canonical_key']] = $entityId;
            $now = now();
            $entitySnapshotId = DB::table('game_catalog_entity_snapshots')->insertGetId([
                'snapshot_id' => $snapshotId,
                'entity_id' => $entityId,
                'introduced_release_id' => $this->releaseId($entity['introduced_in'], $releaseIds),
                'removed_release_id' => $this->releaseId($entity['removed_in'], $releaseIds),
                'completeness' => $entity['completeness'],
                'availability' => $entity['availability'],
                'runtime_present' => $entity['runtime_present'],
                'enabled' => $entity['enabled'],
                'data_sha256' => CanonicalJson::sha256($entity),
                'source_path' => $entity['source_path'],
                'source_key' => $entity['canonical_key'],
                'created_at' => $now,
                'updated_at' => $now,
            ]);

            foreach ($entity['identifiers'] as $identifier) {
                DB::table('game_catalog_entity_identifiers')->insert([
                    'entity_snapshot_id' => $entitySnapshotId,
                    'namespace' => $identifier['namespace'],
                    'value' => $identifier['value'],
                    'created_at' => $now,
                    'updated_at' => $now,
                ]);
            }

            match ($entity['type']) {
                'item' => $this->persistItem($entitySnapshotId, $entity['data']),
                'creature' => $this->persistCreature($entitySnapshotId, $entity['data']),
                default => throw new RuntimeException('Unsupported Game Catalog entity type reached persistence.'),
            };
        }

        return $entityIds;
    }

    private function stableEntityId(string $type, string $canonicalKey): int
    {
        $existing = DB::table('game_catalog_entities')
            ->where('entity_type', $type)
            ->where('canonical_key', $canonicalKey)
            ->value('id');
        if ($existing !== null) {
            return (int) $existing;
        }

        $now = now();

        return DB::table('game_catalog_entities')->insertGetId([
            'entity_type' => $type,
            'canonical_key' => $canonicalKey,
            'created_at' => $now,
            'updated_at' => $now,
        ]);
    }

    /** @param array<string, mixed> $data */
    private function persistItem(int $entitySnapshotId, array $data): void
    {
        DB::table('game_catalog_item_snapshots')->insert([
            'entity_snapshot_id' => $entitySnapshotId,
            'server_id' => $data['server_id'],
            'client_id' => $data['client_id'] ?? null,
            'ware_id' => $data['ware_id'] ?? null,
            'name' => $data['name'],
            'description' => $data['description'] ?? null,
            'category' => $data['category'],
            'weapon_type' => $data['weapon_type'] ?? null,
            'attack' => $data['attack'] ?? null,
            'defense' => $data['defense'] ?? null,
            'extra_defense' => $data['extra_defense'] ?? null,
            'armor' => $data['armor'] ?? null,
            'range' => $data['range'] ?? null,
            'weight' => $data['weight'] ?? null,
            'minimum_level' => $data['minimum_level'] ?? null,
            'vocations' => $this->json($data['vocations'] ?? null),
            'slot_position' => $data['slot_position'] ?? null,
            'imbuement_slots' => $data['imbuement_slots'] ?? null,
            'upgrade_classification' => $data['upgrade_classification'] ?? null,
            'element_type' => $data['element_type'] ?? null,
            'element_value' => $data['element_value'] ?? null,
            'stackable' => $data['stackable'],
            'pickupable' => $data['pickupable'],
            'image_key' => $data['image_key'] ?? null,
            'attributes' => $this->json($data['attributes']),
        ]);
    }

    /** @param array<string, mixed> $data */
    private function persistCreature(int $entitySnapshotId, array $data): void
    {
        DB::table('game_catalog_creature_snapshots')->insert([
            'entity_snapshot_id' => $entitySnapshotId,
            'name' => $data['name'],
            'description' => $data['description'] ?? null,
            'race_id' => $data['race_id'] ?? null,
            'look_type' => $data['look_type'] ?? null,
            'health' => $data['health'],
            'max_health' => $data['max_health'],
            'experience' => $data['experience'],
            'speed' => $data['speed'],
            'armor' => $data['armor'],
            'defense' => $data['defense'],
            'mitigation' => $data['mitigation'] ?? null,
            'is_boss' => $data['is_boss'],
            'is_reward_boss' => $data['is_reward_boss'],
            'bestiary_class' => $data['bestiary_class'] ?? null,
            'bestiary_race' => $data['bestiary_race'] ?? null,
            'bestiary_occurrence' => $data['bestiary_occurrence'] ?? null,
            'bestiary_to_kill' => $data['bestiary_to_kill'] ?? null,
            'charm_points' => $data['charm_points'] ?? null,
            'elements' => $this->json($data['elements']),
            'immunities' => $this->json($data['immunities']),
            'attacks' => $this->json($data['attacks']),
            'defenses' => $this->json($data['defenses']),
            'attributes' => $this->json($data['attributes']),
        ]);
    }

    /**
     * @param  list<array<string, mixed>>  $relations
     * @param  array<string, int>  $releaseIds
     * @param  array<string, int>  $entityIds
     */
    private function persistRelations(int $snapshotId, array $relations, array $releaseIds, array $entityIds): void
    {
        foreach ($relations as $relation) {
            $now = now();
            $relationId = DB::table('game_catalog_relation_snapshots')->insertGetId([
                'snapshot_id' => $snapshotId,
                'relation_type' => $relation['type'],
                'canonical_key' => $relation['canonical_key'],
                'source_entity_id' => $entityIds[$relation['source']],
                'target_entity_id' => $entityIds[$relation['target']],
                'introduced_release_id' => $this->releaseId($relation['introduced_in'], $releaseIds),
                'removed_release_id' => $this->releaseId($relation['removed_in'], $releaseIds),
                'completeness' => $relation['completeness'],
                'enabled' => $relation['enabled'],
                'data_sha256' => CanonicalJson::sha256($relation),
                'source_path' => $relation['source_path'],
                'attributes' => null,
                'created_at' => $now,
                'updated_at' => $now,
            ]);

            $data = $relation['data'];
            DB::table('game_catalog_loot_snapshots')->insert([
                'relation_snapshot_id' => $relationId,
                'chance_numerator' => $data['chance_numerator'],
                'chance_denominator' => $data['chance_denominator'],
                'minimum_count' => $data['minimum_count'],
                'maximum_count' => $data['maximum_count'],
                'container_path' => $data['container_path'],
                'condition_data' => $this->json($data['condition_data']),
            ]);
        }
    }

    private function verifyPersistedCounts(int $snapshotId, int $entityCount, int $relationCount): void
    {
        $persistedEntities = DB::table('game_catalog_entity_snapshots')->where('snapshot_id', $snapshotId)->count();
        $persistedRelations = DB::table('game_catalog_relation_snapshots')->where('snapshot_id', $snapshotId)->count();
        $danglingRelations = DB::table('game_catalog_relation_snapshots as relations')
            ->leftJoin('game_catalog_entity_snapshots as sources', function ($join) use ($snapshotId): void {
                $join->on('sources.entity_id', '=', 'relations.source_entity_id')
                    ->where('sources.snapshot_id', '=', $snapshotId);
            })
            ->leftJoin('game_catalog_entity_snapshots as targets', function ($join) use ($snapshotId): void {
                $join->on('targets.entity_id', '=', 'relations.target_entity_id')
                    ->where('targets.snapshot_id', '=', $snapshotId);
            })
            ->where('relations.snapshot_id', $snapshotId)
            ->where(function ($query): void {
                $query->whereNull('sources.id')->orWhereNull('targets.id');
            })
            ->exists();

        if ($persistedEntities !== $entityCount || $persistedRelations !== $relationCount || $danglingRelations) {
            throw new RuntimeException('Persisted Game Catalog counts or references do not match the validated snapshot.');
        }
    }

    /**
     * @param  list<array{code: string, path: string, message: string}>  $findings
     */
    private function rejectRun(int $runId, array $findings, string $summary): void
    {
        DB::transaction(function () use ($runId, $findings, $summary): void {
            foreach ($findings as $finding) {
                DB::table('game_catalog_validation_findings')->insert([
                    'import_run_id' => $runId,
                    'severity' => 'error',
                    'code' => mb_substr($finding['code'], 0, 120),
                    'path' => mb_substr($finding['path'], 0, 512),
                    'message' => mb_substr($finding['message'], 0, 4000),
                    'created_at' => now(),
                    'updated_at' => now(),
                ]);
            }

            DB::table('game_catalog_import_runs')->where('id', $runId)->update([
                'status' => 'rejected',
                'finding_count' => count($findings),
                'summary' => $summary,
                'finished_at' => now(),
                'updated_at' => now(),
            ]);
        });
    }

    /** @param array<string, int> $releaseIds */
    private function releaseId(mixed $releaseKey, array $releaseIds): ?int
    {
        return is_string($releaseKey) ? $releaseIds[$releaseKey] : null;
    }

    private function json(mixed $value): ?string
    {
        return $value === null
            ? null
            : json_encode($value, JSON_THROW_ON_ERROR | JSON_UNESCAPED_SLASHES | JSON_UNESCAPED_UNICODE);
    }
}
