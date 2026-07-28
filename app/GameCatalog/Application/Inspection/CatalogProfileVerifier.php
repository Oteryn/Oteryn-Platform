<?php

namespace App\GameCatalog\Application\Inspection;

use Illuminate\Support\Facades\DB;
use RuntimeException;

final class CatalogProfileVerifier
{
    /** @return array<string, int|string> */
    public function verify(string $profileKey): array
    {
        $profile = DB::table('game_catalog_profiles')->where('key', $profileKey)->first();
        if ($profile === null || $profile->active_snapshot_id === null) {
            throw new RuntimeException("Game Catalog profile [{$profileKey}] has no active snapshot.");
        }

        $snapshotId = (int) $profile->active_snapshot_id;
        $snapshot = DB::table('game_catalog_snapshots')->where('id', $snapshotId)->first();
        if ($snapshot === null || $snapshot->status !== 'validated') {
            throw new RuntimeException('Active Game Catalog snapshot is missing or not validated.');
        }

        $projectedEntities = DB::table('game_catalog_profile_entities')
            ->where('profile_id', $profile->id)
            ->whereIn('entity_snapshot_id', DB::table('game_catalog_entity_snapshots')->where('snapshot_id', $snapshotId)->select('id'))
            ->count();
        $projectedRelations = DB::table('game_catalog_profile_relations')
            ->where('profile_id', $profile->id)
            ->whereIn('relation_snapshot_id', DB::table('game_catalog_relation_snapshots')->where('snapshot_id', $snapshotId)->select('id'))
            ->count();
        if ($projectedEntities !== (int) $snapshot->entity_count || $projectedRelations !== (int) $snapshot->relation_count) {
            throw new RuntimeException('Active Game Catalog visibility projection is incomplete.');
        }

        $visibleEntities = DB::table('game_catalog_profile_entities')
            ->where('profile_id', $profile->id)
            ->where('visible', true)
            ->count();
        $visibleRelations = DB::table('game_catalog_profile_relations')
            ->where('profile_id', $profile->id)
            ->where('visible', true)
            ->count();

        $invalidRelations = DB::table('game_catalog_profile_relations as visibility')
            ->join('game_catalog_relation_snapshots as relation', 'relation.id', '=', 'visibility.relation_snapshot_id')
            ->join('game_catalog_entity_snapshots as source_version', function ($join) use ($snapshotId): void {
                $join->on('source_version.entity_id', '=', 'relation.source_entity_id')
                    ->where('source_version.snapshot_id', '=', $snapshotId);
            })
            ->join('game_catalog_profile_entities as source_visibility', function ($join) use ($profile): void {
                $join->on('source_visibility.entity_snapshot_id', '=', 'source_version.id')
                    ->where('source_visibility.profile_id', '=', $profile->id);
            })
            ->join('game_catalog_entity_snapshots as target_version', function ($join) use ($snapshotId): void {
                $join->on('target_version.entity_id', '=', 'relation.target_entity_id')
                    ->where('target_version.snapshot_id', '=', $snapshotId);
            })
            ->join('game_catalog_profile_entities as target_visibility', function ($join) use ($profile): void {
                $join->on('target_visibility.entity_snapshot_id', '=', 'target_version.id')
                    ->where('target_visibility.profile_id', '=', $profile->id);
            })
            ->where('visibility.profile_id', $profile->id)
            ->where('visibility.visible', true)
            ->where(function ($query): void {
                $query->where('source_visibility.visible', false)
                    ->orWhere('target_visibility.visible', false);
            })
            ->exists();
        if ($invalidRelations) {
            throw new RuntimeException('Active Game Catalog contains a visible relation with a hidden endpoint.');
        }

        return [
            'profile' => $profileKey,
            'snapshot_id' => $snapshotId,
            'projected_entities' => $projectedEntities,
            'visible_entities' => $visibleEntities,
            'projected_relations' => $projectedRelations,
            'visible_relations' => $visibleRelations,
        ];
    }
}
