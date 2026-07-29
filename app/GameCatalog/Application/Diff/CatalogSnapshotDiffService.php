<?php

namespace App\GameCatalog\Application\Diff;

use App\GameCatalog\Infrastructure\Persistence\CatalogDatabaseRow;
use Illuminate\Support\Facades\DB;
use RuntimeException;

final class CatalogSnapshotDiffService
{
    public function diff(int $snapshotA, int $snapshotB): CatalogSnapshotDiff
    {
        $this->requireSnapshot($snapshotA);
        $this->requireSnapshot($snapshotB);

        $entitiesA = $this->entityFingerprints($snapshotA);
        $entitiesB = $this->entityFingerprints($snapshotB);
        $relationsA = $this->relationFingerprints($snapshotA);
        $relationsB = $this->relationFingerprints($snapshotB);

        return new CatalogSnapshotDiff(
            snapshotA: $snapshotA,
            snapshotB: $snapshotB,
            addedEntities: $this->added($entitiesA, $entitiesB),
            removedEntities: $this->added($entitiesB, $entitiesA),
            changedEntities: $this->changed($entitiesA, $entitiesB),
            addedRelations: $this->added($relationsA, $relationsB),
            removedRelations: $this->added($relationsB, $relationsA),
            changedRelations: $this->changed($relationsA, $relationsB),
        );
    }

    private function requireSnapshot(int $snapshotId): void
    {
        if (! DB::table('game_catalog_snapshots')->where('id', $snapshotId)->exists()) {
            throw new RuntimeException("Game Catalog snapshot '{$snapshotId}' does not exist.");
        }
    }

    /** @return array<string, string> */
    private function entityFingerprints(int $snapshotId): array
    {
        $rows = DB::table('game_catalog_entity_snapshots as snapshots')
            ->join('game_catalog_entities as entities', 'entities.id', '=', 'snapshots.entity_id')
            ->where('snapshots.snapshot_id', $snapshotId)
            ->orderBy('entities.canonical_key')
            ->get([
                'entities.canonical_key',
                'entities.entity_type',
                'snapshots.introduced_release_id',
                'snapshots.removed_release_id',
                'snapshots.completeness',
                'snapshots.availability',
                'snapshots.runtime_present',
                'snapshots.enabled',
                'snapshots.data_sha256',
            ]);

        /** @var array<string, string> $result */
        $result = [];
        foreach ($rows as $row) {
            $databaseRow = CatalogDatabaseRow::from($row);
            $introducedReleaseId = $databaseRow->nullableInt('introduced_release_id');
            $removedReleaseId = $databaseRow->nullableInt('removed_release_id');
            $result[$databaseRow->string('canonical_key')] = hash('sha256', implode('|', [
                $databaseRow->string('entity_type'),
                $introducedReleaseId === null ? 'null' : (string) $introducedReleaseId,
                $removedReleaseId === null ? 'null' : (string) $removedReleaseId,
                $databaseRow->string('completeness'),
                $databaseRow->string('availability'),
                $databaseRow->bool('runtime_present') ? '1' : '0',
                $databaseRow->bool('enabled') ? '1' : '0',
                $databaseRow->string('data_sha256'),
            ]));
        }

        return $result;
    }

    /** @return array<string, string> */
    private function relationFingerprints(int $snapshotId): array
    {
        $rows = DB::table('game_catalog_relation_snapshots')
            ->where('snapshot_id', $snapshotId)
            ->orderBy('canonical_key')
            ->get([
                'canonical_key',
                'relation_type',
                'source_entity_id',
                'target_entity_id',
                'introduced_release_id',
                'removed_release_id',
                'completeness',
                'enabled',
                'data_sha256',
            ]);

        /** @var array<string, string> $result */
        $result = [];
        foreach ($rows as $row) {
            $databaseRow = CatalogDatabaseRow::from($row);
            $targetEntityId = $databaseRow->nullableInt('target_entity_id');
            $introducedReleaseId = $databaseRow->nullableInt('introduced_release_id');
            $removedReleaseId = $databaseRow->nullableInt('removed_release_id');
            $result[$databaseRow->string('canonical_key')] = hash('sha256', implode('|', [
                $databaseRow->string('relation_type'),
                (string) $databaseRow->int('source_entity_id'),
                $targetEntityId === null ? 'null' : (string) $targetEntityId,
                $introducedReleaseId === null ? 'null' : (string) $introducedReleaseId,
                $removedReleaseId === null ? 'null' : (string) $removedReleaseId,
                $databaseRow->string('completeness'),
                $databaseRow->bool('enabled') ? '1' : '0',
                $databaseRow->string('data_sha256'),
            ]));
        }

        return $result;
    }

    /**
     * @param  array<string, string>  $from
     * @param  array<string, string>  $to
     * @return list<string>
     */
    private function added(array $from, array $to): array
    {
        $keys = array_values(array_diff(array_keys($to), array_keys($from)));
        sort($keys, SORT_STRING);

        return $keys;
    }

    /**
     * @param  array<string, string>  $left
     * @param  array<string, string>  $right
     * @return list<string>
     */
    private function changed(array $left, array $right): array
    {
        $keys = [];
        foreach (array_intersect(array_keys($left), array_keys($right)) as $key) {
            if (! hash_equals($left[$key], $right[$key])) {
                $keys[] = $key;
            }
        }
        sort($keys, SORT_STRING);

        return $keys;
    }
}
