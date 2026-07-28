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
        $now = now();
        $runId = DB::table('game_catalog_import_runs')->insertGetId([
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

        try {
            $validated = $this->validator->validatePath($path, $expectedSha256);
            DB::table('game_catalog_import_runs')->whereKey($runId)->update([
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
                DB::table('game_catalog_import_runs')->whereKey($runId)->update([
                    'snapshot_id' => $snapshotId,
                    'status' => 'validated',
                    'summary' => 'Identical immutable snapshot already imported.',
                    'finished_at' => now(),
                    'updated_at' => now(),
                ]);

                return new CatalogImportResult($snapshotId, $runId, $validated->contentSha256, true);
            }

            $snapshotId = DB::transaction(function () use ($validated): int {
                /** @var array<string, mixed> $document */
                $document = $validated->document;
                /** @var list<array<string, mixed>> $releaseRows */
                $releaseRows = $document['releases'];
                $releaseIds = $this->persistReleases($releaseRows);
                /** @var array<string, mixed> $snapshot */
                $snapshot = $document['snapshot'];
                $now = now();

                $snapshotId = DB::table('game_catalog_snapshots')->insertGetId([
                    'contract_id' => GameCatalogContract::ID,
                    'schema_version' => GameCatalogContract::SCHEMA_VERSION,
                    'content_sha256' => $validated->contentSha256,
                    'byte_size' => $validated->byteSize,
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
                    'validation_summary' => json_encode(['errors' => 0, 'warnings' => 0], JSON_THROW_ON_ERROR),
                    'created_at' => $now,
                    'updated_at' => $now,
                ]);

                $this->ensureDefaultProfile($snapshot['content_target_release'], $releaseIds);

                /** @var list<array<string, mixed>> $entities */
                $entities = $document['entities'];
                $entityIds = $this->persistEntities($snapshotId, $entities, $releaseIds);
                /** @var list<array<string, mixed>> $relations */
                $relations = $document['relations'];
                $this->persistRelations($snapshotId, $relations, $releaseIds, $entityIds);

                $persistedEntities = DB::table('game_catalog_entity_snapshots')->where('snapshot_id', $snapshotId)->count();
                $persistedRelations = DB::table('game_catalog_relation_snapshots')->where('snapshot_id', $snapshotId)->count();
                if ($persistedEntities !== $snapshot['entity_count'] || $persistedRelations !== $snapshot['relation_count']) {
                    throw new RuntimeException('Persisted Game Catalog counts do not match the validated snapshot.');
                }

                return $snapshotId;
            }, 3);

            DB::table('game_catalog_import_runs')->whereKey($runId)->update([
                'snapshot_id' => $snapshotId,
                'status' => 'validated',
                'summary' => 'Snapshot imported as inactive immutable state.',
                'finished_at' => now(),
                'updated_at' => now(),
            ]);

            return new CatalogImportResult($snapshotId, $runId, $validated->contentSha256, false);
        } catch (CatalogValidationException $exception) {
            $this->rejectRun($runId, $exception->findings(), 'Snapshot validation failed.');
            throw $exception;
        } catch (Throwable $exception) {
            $this->rejectRun($runId, [], 'Snapshot import failed before activation.');
            throw $exception;
        }
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
                    $actual = $existing->{$field};
                    if ((string) ($actual ?? '') !== (string) ($value ?? '')) {
                        throw new CatalogValidationException([[
                            'code' => 'semantic.conflicting_release',
                            'path' => '$/releases',
                            'message' => "Retained release [{$release['key']}] conflicts with the imported definition.",
                        ]]);
                    }
                }
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
            $stable = DB::table('game_catalog_entities')
                ->where('entity_type', $entity['type'])
                ->where('canonical_key', $entity['canonical_key'])
                ->first();
            $now = now();
            $entityId = $stable === null
                ? DB::table('game_catalog_entities')->insertGetId([
                    'entity_type' => $entity['type'],
                    'canonical_key' => $entity['canonical_key'],
                    'created_at' => $now,
                    'updated_at' => $now,
                ])
                : (int) $stable->id;
            $entityIds[$entity['canonical_key']] = $entityId;

            $entitySnapshotId = DB::table('game_catalog_entity_snapshots')->insertGetId([
                'snapshot_id' => $snapshotId,
                'entity_id' => $entityId,
                'introduced_release_id' => $entity['introduced_in'] === null ? null : $releaseIds[$entity['introduced_in']],
                'removed_release_id' => $entity['removed_in'] === null ? null : $releaseIds[$entity['removed_in']],
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

            if ($entity['type'] === 'item') {
                $this->persistItem($entitySnapshotId, $entity['data']);
            } elseif ($entity['type'] === 'creature') {
                $this->persistCreature($entitySnapshotId, $entity['data']);
            } else {
                throw new RuntimeException('Unsupported Game Catalog entity type reached persistence.');
            }
        }

        return $entityIds;
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
                'introduced_release_id' => $relation['introduced_in'] === null ? null : $releaseIds[$relation['introduced_in']],
                'removed_release_id' => $relation['removed_in'] === null ? null : $releaseIds[$relation['removed_in']],
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

            DB::table('game_catalog_import_runs')->whereKey($runId)->update([
                'status' => 'rejected',
                'finding_count' => count($findings),
                'summary' => $summary,
                'finished_at' => now(),
                'updated_at' => now(),
            ]);
        });
    }

    private function json(mixed $value): ?string
    {
        return $value === null ? null : json_encode($value, JSON_THROW_ON_ERROR | JSON_UNESCAPED_SLASHES | JSON_UNESCAPED_UNICODE);
    }
}
