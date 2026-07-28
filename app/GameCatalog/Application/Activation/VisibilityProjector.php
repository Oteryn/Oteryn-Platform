<?php

namespace App\GameCatalog\Application\Activation;

use Carbon\CarbonImmutable;
use Illuminate\Support\Facades\DB;
use RuntimeException;

final class VisibilityProjector
{
    public function rebuild(
        int $profileId,
        int $snapshotId,
        int $targetReleaseOrder,
        bool $completeOnly,
        bool $allowBackports,
        CarbonImmutable $computedAt,
    ): VisibilityProjectionResult {
        DB::table('game_catalog_profile_relations')->where('profile_id', $profileId)->delete();
        DB::table('game_catalog_profile_entities')->where('profile_id', $profileId)->delete();

        $overrides = DB::table('game_catalog_profile_overrides')
            ->where('profile_id', $profileId)
            ->get(['entity_id', 'relation_snapshot_id', 'action']);

        $excludedEntities = [];
        $backportedEntities = [];
        $excludedRelations = [];
        $backportedRelations = [];
        foreach ($overrides as $override) {
            if ($override->entity_id !== null) {
                if ($override->action === 'exclude') {
                    $excludedEntities[(int) $override->entity_id] = true;
                } elseif ($allowBackports && $override->action === 'approved_backport') {
                    $backportedEntities[(int) $override->entity_id] = true;
                }
            }
            if ($override->relation_snapshot_id !== null) {
                if ($override->action === 'exclude') {
                    $excludedRelations[(int) $override->relation_snapshot_id] = true;
                } elseif ($allowBackports && $override->action === 'approved_backport') {
                    $backportedRelations[(int) $override->relation_snapshot_id] = true;
                }
            }
        }

        $allowedItemAvailability = array_fill_keys((array) config('game-catalog.public_item_availability', []), true);
        $allowedCreatureAvailability = array_fill_keys((array) config('game-catalog.public_creature_availability', []), true);
        $visibleEntityIds = [];
        $visibleEntityCount = 0;

        DB::table('game_catalog_entity_snapshots as entity_snapshots')
            ->join('game_catalog_entities as entities', 'entities.id', '=', 'entity_snapshots.entity_id')
            ->leftJoin('game_catalog_releases as introduced', 'introduced.id', '=', 'entity_snapshots.introduced_release_id')
            ->leftJoin('game_catalog_releases as removed', 'removed.id', '=', 'entity_snapshots.removed_release_id')
            ->where('entity_snapshots.snapshot_id', $snapshotId)
            ->select([
                'entity_snapshots.id as entity_snapshot_id',
                'entity_snapshots.entity_id',
                'entity_snapshots.completeness',
                'entity_snapshots.availability',
                'entity_snapshots.runtime_present',
                'entity_snapshots.enabled',
                'entities.entity_type',
                'introduced.release_order as introduced_order',
                'removed.release_order as removed_order',
            ])
            ->orderBy('entity_snapshots.id')
            ->chunkById(1_000, function ($rows) use (
                $profileId,
                $targetReleaseOrder,
                $completeOnly,
                $computedAt,
                $excludedEntities,
                $backportedEntities,
                $allowedItemAvailability,
                $allowedCreatureAvailability,
                &$visibleEntityIds,
                &$visibleEntityCount,
            ): void {
                $insert = [];
                foreach ($rows as $row) {
                    $entityId = (int) $row->entity_id;
                    $outsideRelease = ($row->introduced_order !== null && $targetReleaseOrder < (int) $row->introduced_order)
                        || ($row->removed_order !== null && $targetReleaseOrder >= (int) $row->removed_order);
                    $availabilityAllowed = $row->entity_type === 'item'
                        ? isset($allowedItemAvailability[$row->availability])
                        : isset($allowedCreatureAvailability[$row->availability]);

                    $reason = match (true) {
                        isset($excludedEntities[$entityId]) => 'explicit_exclusion',
                        ! (bool) $row->runtime_present => 'runtime_absent',
                        ! (bool) $row->enabled => 'disabled',
                        $outsideRelease && ! isset($backportedEntities[$entityId]) => 'outside_release',
                        $completeOnly && $row->completeness !== 'complete' => 'incomplete',
                        ! $availabilityAllowed => 'availability_disallowed',
                        default => 'visible',
                    };
                    $visible = $reason === 'visible';
                    if ($visible) {
                        $visibleEntityIds[$entityId] = true;
                        $visibleEntityCount++;
                    }

                    $insert[] = [
                        'profile_id' => $profileId,
                        'entity_snapshot_id' => (int) $row->entity_snapshot_id,
                        'visible' => $visible,
                        'reason_code' => $reason,
                        'computed_at' => $computedAt,
                    ];
                }

                if ($insert !== []) {
                    DB::table('game_catalog_profile_entities')->insert($insert);
                }
            }, 'entity_snapshots.id', 'entity_snapshot_id');

        $visibleRelationCount = 0;
        DB::table('game_catalog_relation_snapshots as relations')
            ->leftJoin('game_catalog_releases as introduced', 'introduced.id', '=', 'relations.introduced_release_id')
            ->leftJoin('game_catalog_releases as removed', 'removed.id', '=', 'relations.removed_release_id')
            ->where('relations.snapshot_id', $snapshotId)
            ->select([
                'relations.id as relation_snapshot_id',
                'relations.source_entity_id',
                'relations.target_entity_id',
                'relations.completeness',
                'relations.enabled',
                'introduced.release_order as introduced_order',
                'removed.release_order as removed_order',
            ])
            ->orderBy('relations.id')
            ->chunkById(1_000, function ($rows) use (
                $profileId,
                $targetReleaseOrder,
                $completeOnly,
                $computedAt,
                $excludedRelations,
                $backportedRelations,
                $visibleEntityIds,
                &$visibleRelationCount,
            ): void {
                $insert = [];
                foreach ($rows as $row) {
                    $relationId = (int) $row->relation_snapshot_id;
                    $outsideRelease = ($row->introduced_order !== null && $targetReleaseOrder < (int) $row->introduced_order)
                        || ($row->removed_order !== null && $targetReleaseOrder >= (int) $row->removed_order);

                    $reason = match (true) {
                        isset($excludedRelations[$relationId]) => 'explicit_exclusion',
                        ! (bool) $row->enabled => 'disabled',
                        $outsideRelease && ! isset($backportedRelations[$relationId]) => 'outside_release',
                        $completeOnly && $row->completeness !== 'complete' => 'incomplete',
                        ! isset($visibleEntityIds[(int) $row->source_entity_id]) => 'source_hidden',
                        $row->target_entity_id === null || ! isset($visibleEntityIds[(int) $row->target_entity_id]) => 'target_hidden',
                        default => 'visible',
                    };
                    $visible = $reason === 'visible';
                    if ($visible) {
                        $visibleRelationCount++;
                    }

                    $insert[] = [
                        'profile_id' => $profileId,
                        'relation_snapshot_id' => $relationId,
                        'visible' => $visible,
                        'reason_code' => $reason,
                        'computed_at' => $computedAt,
                    ];
                }

                if ($insert !== []) {
                    DB::table('game_catalog_profile_relations')->insert($insert);
                }
            }, 'relations.id', 'relation_snapshot_id');

        $entityProjectionCount = DB::table('game_catalog_profile_entities')->where('profile_id', $profileId)->count();
        $relationProjectionCount = DB::table('game_catalog_profile_relations')->where('profile_id', $profileId)->count();
        $expectedEntityCount = DB::table('game_catalog_entity_snapshots')->where('snapshot_id', $snapshotId)->count();
        $expectedRelationCount = DB::table('game_catalog_relation_snapshots')->where('snapshot_id', $snapshotId)->count();

        if ($entityProjectionCount !== $expectedEntityCount || $relationProjectionCount !== $expectedRelationCount) {
            throw new RuntimeException('Game Catalog visibility projection count verification failed.');
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
            ->leftJoin('game_catalog_profile_entities as source_visibility', function ($join) use ($profileId): void {
                $join->on('source_visibility.entity_snapshot_id', '=', 'source_snapshot.id')
                    ->where('source_visibility.profile_id', '=', $profileId);
            })
            ->leftJoin('game_catalog_profile_entities as target_visibility', function ($join) use ($profileId): void {
                $join->on('target_visibility.entity_snapshot_id', '=', 'target_snapshot.id')
                    ->where('target_visibility.profile_id', '=', $profileId);
            })
            ->where('visibility.profile_id', $profileId)
            ->where('visibility.visible', true)
            ->where(function ($query): void {
                $query->whereNull('source_visibility.entity_snapshot_id')
                    ->orWhere('source_visibility.visible', false)
                    ->orWhereNull('target_visibility.entity_snapshot_id')
                    ->orWhere('target_visibility.visible', false);
            })
            ->count();

        if ($invalidVisibleRelations !== 0) {
            throw new RuntimeException('Visible Game Catalog relations contain hidden or missing endpoints.');
        }

        return new VisibilityProjectionResult($visibleEntityCount, $visibleRelationCount);
    }
}
