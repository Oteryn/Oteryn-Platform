<?php

namespace App\GameCatalog\Application\Inspection;

use Illuminate\Support\Facades\DB;
use RuntimeException;

final class CatalogDiffService
{
    /** @return array<string, mixed> */
    public function diff(int $fromSnapshotId, int $toSnapshotId): array
    {
        $this->requireSnapshot($fromSnapshotId);
        $this->requireSnapshot($toSnapshotId);

        $fromEntities = $this->entityHashes($fromSnapshotId);
        $toEntities = $this->entityHashes($toSnapshotId);
        $fromRelations = $this->relationHashes($fromSnapshotId);
        $toRelations = $this->relationHashes($toSnapshotId);

        return [
            'from_snapshot_id' => $fromSnapshotId,
            'to_snapshot_id' => $toSnapshotId,
            'entities' => $this->compare($fromEntities, $toEntities),
            'relations' => $this->compare($fromRelations, $toRelations),
        ];
    }

    private function requireSnapshot(int $snapshotId): void
    {
        if (! DB::table('game_catalog_snapshots')->whereKey($snapshotId)->exists()) {
            throw new RuntimeException("Game Catalog snapshot [{$snapshotId}] does not exist.");
        }
    }

    /** @return array<string, string> */
    private function entityHashes(int $snapshotId): array
    {
        $rows = DB::table('game_catalog_entity_snapshots as versions')
            ->join('game_catalog_entities as entities', 'entities.id', '=', 'versions.entity_id')
            ->where('versions.snapshot_id', $snapshotId)
            ->orderBy('entities.entity_type')
            ->orderBy('entities.canonical_key')
            ->get(['entities.entity_type', 'entities.canonical_key', 'versions.data_sha256']);

        $result = [];
        foreach ($rows as $row) {
            $result[$row->entity_type.'|'.$row->canonical_key] = $row->data_sha256;
        }

        return $result;
    }

    /** @return array<string, string> */
    private function relationHashes(int $snapshotId): array
    {
        $rows = DB::table('game_catalog_relation_snapshots')
            ->where('snapshot_id', $snapshotId)
            ->orderBy('relation_type')
            ->orderBy('canonical_key')
            ->get(['relation_type', 'canonical_key', 'data_sha256']);

        $result = [];
        foreach ($rows as $row) {
            $result[$row->relation_type.'|'.$row->canonical_key] = $row->data_sha256;
        }

        return $result;
    }

    /**
     * @param  array<string, string>  $from
     * @param  array<string, string>  $to
     * @return array{added: list<string>, removed: list<string>, changed: list<string>, unchanged_count: int}
     */
    private function compare(array $from, array $to): array
    {
        $added = array_values(array_diff(array_keys($to), array_keys($from)));
        $removed = array_values(array_diff(array_keys($from), array_keys($to)));
        $changed = [];
        $unchanged = 0;
        foreach (array_intersect(array_keys($from), array_keys($to)) as $key) {
            if (! hash_equals($from[$key], $to[$key])) {
                $changed[] = $key;
            } else {
                ++$unchanged;
            }
        }
        sort($added, SORT_STRING);
        sort($removed, SORT_STRING);
        sort($changed, SORT_STRING);

        return [
            'added' => array_slice($added, 0, 1000),
            'removed' => array_slice($removed, 0, 1000),
            'changed' => array_slice($changed, 0, 1000),
            'unchanged_count' => $unchanged,
        ];
    }
}
