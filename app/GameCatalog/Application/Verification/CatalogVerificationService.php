<?php

namespace App\GameCatalog\Application\Verification;

use Illuminate\Support\Facades\DB;
use RuntimeException;

final class CatalogVerificationService
{
    public function verify(string $profileKey): CatalogVerificationResult
    {
        $profile = DB::table('game_catalog_profiles')->where('key', $profileKey)->first([
            'id',
            'key',
            'active_snapshot_id',
        ]);

        if ($profile === null) {
            throw new RuntimeException("Game Catalog profile '{$profileKey}' does not exist.");
        }

        if ($profile->active_snapshot_id === null) {
            return new CatalogVerificationResult(
                profileKey: (string) $profile->key,
                snapshotId: null,
                projectedEntityCount: 0,
                visibleEntityCount: 0,
                projectedRelationCount: 0,
                visibleRelationCount: 0,
                errors: ['profile_has_no_active_snapshot'],
            );
        }

        $snapshotId = (int) $profile->active_snapshot_id;
        $snapshot = DB::table('game_catalog_snapshots')->where('id', $snapshotId)->first([
            'status',
            'entity_count',
            'relation_count',
        ]);
        if ($snapshot === null) {
            return new CatalogVerificationResult(
                profileKey: (string) $profile->key,
                snapshotId: $snapshotId,
                projectedEntityCount: 0,
                visibleEntityCount: 0,
                projectedRelationCount: 0,
                visibleRelationCount: 0,
                errors: ['active_snapshot_missing'],
            );
        }

        $errors = [];
        if ($snapshot->status !== 'validated') {
            $errors[] = 'active_snapshot_not_validated';
        }

        $persistedEntities = DB::table('game_catalog_entity_snapshots')->where('snapshot_id', $snapshotId)->count();
        $persistedRelations = DB::table('game_catalog_relation_snapshots')->where('snapshot_id', $snapshotId)->count();
        if ($persistedEntities !== (int) $snapshot->entity_count) {
            $errors[] = 'snapshot_entity_count_mismatch';
        }
        if ($persistedRelations !== (int) $snapshot->relation_count) {
            $errors[] = 'snapshot_relation_count_mismatch';
        }

        $projectedEntities = DB::table('game_catalog_profile_entities as visibility')
            ->join('game_catalog_entity_snapshots as snapshots', 'snapshots.id', '=', 'visibility.entity_snapshot_id')
            ->where('visibility.profile_id', $profile->id)
            ->where('snapshots.snapshot_id', $snapshotId)
            ->count();
        $visibleEntities = DB::table('game_catalog_profile_entities as visibility')
            ->join('game_catalog_entity_snapshots as snapshots', 'snapshots.id', '=', 'visibility.entity_snapshot_id')
            ->where('visibility.profile_id', $profile->id)
            ->where('snapshots.snapshot_id', $snapshotId)
            ->where('visibility.visible', true)
            ->count();
        $projectedRelations = DB::table('game_catalog_profile_relations as visibility')
            ->join('game_catalog_relation_snapshots as snapshots', 'snapshots.id', '=', 'visibility.relation_snapshot_id')
            ->where('visibility.profile_id', $profile->id)
            ->where('snapshots.snapshot_id', $snapshotId)
            ->count();
        $visibleRelations = DB::table('game_catalog_profile_relations as visibility')
            ->join('game_catalog_relation_snapshots as snapshots', 'snapshots.id', '=', 'visibility.relation_snapshot_id')
            ->where('visibility.profile_id', $profile->id)
            ->where('snapshots.snapshot_id', $snapshotId)
            ->where('visibility.visible', true)
            ->count();

        if ($projectedEntities !== $persistedEntities) {
            $errors[] = 'entity_projection_count_mismatch';
        }
        if ($projectedRelations !== $persistedRelations) {
            $errors[] = 'relation_projection_count_mismatch';
        }

        $foreignEntityRows = DB::table('game_catalog_profile_entities as visibility')
            ->join('game_catalog_entity_snapshots as snapshots', 'snapshots.id', '=', 'visibility.entity_snapshot_id')
            ->where('visibility.profile_id', $profile->id)
            ->where('snapshots.snapshot_id', '!=', $snapshotId)
            ->count();
        $foreignRelationRows = DB::table('game_catalog_profile_relations as visibility')
            ->join('game_catalog_relation_snapshots as snapshots', 'snapshots.id', '=', 'visibility.relation_snapshot_id')
            ->where('visibility.profile_id', $profile->id)
            ->where('snapshots.snapshot_id', '!=', $snapshotId)
            ->count();
        if ($foreignEntityRows !== 0 || $foreignRelationRows !== 0) {
            $errors[] = 'projection_contains_foreign_snapshot_rows';
        }

        $invalidVisibleRelations = DB::table('game_catalog_profile_relations as visibility')
            ->join('game_catalog_relation_snapshots as relations', 'relations.id', '=', 'visibility.relation_snapshot_id')
            ->leftJoin('game_catalog_entity_snapshots as source_snapshot', function ($join) use ($snapshotId): void {
                $join->on('source_snapshot.entity_id', '=', 'relations.source_entity_id')
                    ->where('source_snapshot.snapshot_id', '=', $snapshotId);
            })
            ->leftJoin('game_catalog_entity_snapshots as target_snapshot', function ($join) use ($snapshotId): void {
                $join->on('target_snapshot.entity_id', '=', 'relations.target_entity_id')
                    ->where('target_snapshot.snapshot_id', '=', $snapshotId);
            })
            ->leftJoin('game_catalog_profile_entities as source_visibility', function ($join) use ($profile): void {
                $join->on('source_visibility.entity_snapshot_id', '=', 'source_snapshot.id')
                    ->where('source_visibility.profile_id', '=', $profile->id);
            })
            ->leftJoin('game_catalog_profile_entities as target_visibility', function ($join) use ($profile): void {
                $join->on('target_visibility.entity_snapshot_id', '=', 'target_snapshot.id')
                    ->where('target_visibility.profile_id', '=', $profile->id);
            })
            ->where('visibility.profile_id', $profile->id)
            ->where('visibility.visible', true)
            ->where(function ($query): void {
                $query->whereNull('source_visibility.entity_snapshot_id')
                    ->orWhere('source_visibility.visible', false)
                    ->orWhereNull('target_visibility.entity_snapshot_id')
                    ->orWhere('target_visibility.visible', false);
            })
            ->count();
        if ($invalidVisibleRelations !== 0) {
            $errors[] = 'visible_relation_endpoint_invalid';
        }

        return new CatalogVerificationResult(
            profileKey: (string) $profile->key,
            snapshotId: $snapshotId,
            projectedEntityCount: $projectedEntities,
            visibleEntityCount: $visibleEntities,
            projectedRelationCount: $projectedRelations,
            visibleRelationCount: $visibleRelations,
            errors: array_values(array_unique($errors)),
        );
    }
}
