<?php

namespace App\GameCatalog\Application\Profiles;

use Illuminate\Support\Facades\DB;
use RuntimeException;

final class CatalogVisibilityProjector
{
    private const NON_PUBLIC_AVAILABILITY = [
        'registered_only',
        'admin_only',
        'unreachable',
        'unknown',
    ];

    public function rebuild(int $profileId, int $snapshotId): VisibilityProjectionResult
    {
        $profile = DB::table('game_catalog_profiles as profiles')
            ->join('game_catalog_releases as target', 'target.id', '=', 'profiles.target_release_id')
            ->where('profiles.id', $profileId)
            ->first([
                'profiles.id',
                'profiles.complete_only',
                'profiles.allow_backports',
                'target.release_order as target_order',
            ]);
        if ($profile === null) {
            throw new RuntimeException('Game Catalog profile does not exist.');
        }

        $overrides = DB::table('game_catalog_profile_overrides')
            ->where('profile_id', $profileId)
            ->get(['entity_id', 'relation_snapshot_id', 'action'])
            ->all();
        $entityOverrides = [];
        $relationOverrides = [];
        foreach ($overrides as $override) {
            if ($override->entity_id !== null) {
                $entityOverrides[(int) $override->entity_id] = (string) $override->action;
            }
            if ($override->relation_snapshot_id !== null) {
                $relationOverrides[(int) $override->relation_snapshot_id] = (string) $override->action;
            }
        }

        DB::table('game_catalog_profile_relations')->where('profile_id', $profileId)->delete();
        DB::table('game_catalog_profile_entities')->where('profile_id', $profileId)->delete();

        $entityRows = DB::table('game_catalog_entity_snapshots as versions')
            ->join('game_catalog_entities as entities', 'entities.id', '=', 'versions.entity_id')
            ->leftJoin('game_catalog_releases as introduced', 'introduced.id', '=', 'versions.introduced_release_id')
            ->leftJoin('game_catalog_releases as removed', 'removed.id', '=', 'versions.removed_release_id')
            ->where('versions.snapshot_id', $snapshotId)
            ->orderBy('versions.id')
            ->get([
                'versions.id',
                'versions.entity_id',
                'versions.completeness',
                'versions.availability',
                'versions.runtime_present',
                'versions.enabled',
                'introduced.release_order as introduced_order',
                'removed.release_order as removed_order',
            ]);

        $computedAt = now();
        $entityVisibility = [];
        $visibleEntities = 0;
        $hiddenEntities = 0;
        foreach ($entityRows as $entity) {
            $reason = $this->entityReason(
                $entity,
                (int) $profile->target_order,
                (bool) $profile->complete_only,
                $entityOverrides[(int) $entity->entity_id] ?? null,
            );
            $visible = $reason === 'visible';
            $entityVisibility[(int) $entity->entity_id] = $visible;
            $visible ? ++$visibleEntities : ++$hiddenEntities;

            DB::table('game_catalog_profile_entities')->insert([
                'profile_id' => $profileId,
                'entity_snapshot_id' => (int) $entity->id,
                'visible' => $visible,
                'reason_code' => $reason,
                'computed_at' => $computedAt,
            ]);
        }

        $relationRows = DB::table('game_catalog_relation_snapshots as relations')
            ->leftJoin('game_catalog_releases as introduced', 'introduced.id', '=', 'relations.introduced_release_id')
            ->leftJoin('game_catalog_releases as removed', 'removed.id', '=', 'relations.removed_release_id')
            ->where('relations.snapshot_id', $snapshotId)
            ->orderBy('relations.id')
            ->get([
                'relations.id',
                'relations.source_entity_id',
                'relations.target_entity_id',
                'relations.completeness',
                'relations.enabled',
                'introduced.release_order as introduced_order',
                'removed.release_order as removed_order',
            ]);

        $visibleRelations = 0;
        $hiddenRelations = 0;
        foreach ($relationRows as $relation) {
            $reason = $this->relationReason(
                $relation,
                (int) $profile->target_order,
                (bool) $profile->complete_only,
                $entityVisibility,
                $relationOverrides[(int) $relation->id] ?? null,
            );
            $visible = $reason === 'visible';
            $visible ? ++$visibleRelations : ++$hiddenRelations;

            DB::table('game_catalog_profile_relations')->insert([
                'profile_id' => $profileId,
                'relation_snapshot_id' => (int) $relation->id,
                'visible' => $visible,
                'reason_code' => $reason,
                'computed_at' => $computedAt,
            ]);
        }

        $invalidVisibleRelations = DB::table('game_catalog_profile_relations as visible_relations')
            ->join('game_catalog_relation_snapshots as relations', 'relations.id', '=', 'visible_relations.relation_snapshot_id')
            ->leftJoin('game_catalog_entity_snapshots as source_versions', function ($join) use ($snapshotId): void {
                $join->on('source_versions.entity_id', '=', 'relations.source_entity_id')
                    ->where('source_versions.snapshot_id', '=', $snapshotId);
            })
            ->leftJoin('game_catalog_profile_entities as source_visibility', function ($join) use ($profileId): void {
                $join->on('source_visibility.entity_snapshot_id', '=', 'source_versions.id')
                    ->where('source_visibility.profile_id', '=', $profileId);
            })
            ->leftJoin('game_catalog_entity_snapshots as target_versions', function ($join) use ($snapshotId): void {
                $join->on('target_versions.entity_id', '=', 'relations.target_entity_id')
                    ->where('target_versions.snapshot_id', '=', $snapshotId);
            })
            ->leftJoin('game_catalog_profile_entities as target_visibility', function ($join) use ($profileId): void {
                $join->on('target_visibility.entity_snapshot_id', '=', 'target_versions.id')
                    ->where('target_visibility.profile_id', '=', $profileId);
            })
            ->where('visible_relations.profile_id', $profileId)
            ->where('visible_relations.visible', true)
            ->where(function ($query): void {
                $query->where('source_visibility.visible', '!=', true)
                    ->orWhereNull('source_visibility.visible')
                    ->orWhere('target_visibility.visible', '!=', true)
                    ->orWhereNull('target_visibility.visible');
            })
            ->exists();
        if ($invalidVisibleRelations) {
            throw new RuntimeException('Visible Game Catalog relation has a hidden or missing endpoint.');
        }

        return new VisibilityProjectionResult(
            $visibleEntities,
            $hiddenEntities,
            $visibleRelations,
            $hiddenRelations,
        );
    }

    private function entityReason(object $entity, int $targetOrder, bool $completeOnly, ?string $override): string
    {
        if (in_array($override, ['exclude', 'force_hidden'], true)) {
            return 'profile_excluded';
        }
        if (! (bool) $entity->runtime_present) {
            return 'runtime_missing';
        }
        if (! (bool) $entity->enabled) {
            return 'disabled';
        }
        if ($entity->introduced_order !== null && (int) $entity->introduced_order > $targetOrder) {
            return 'future_release';
        }
        if ($entity->removed_order !== null && $targetOrder >= (int) $entity->removed_order) {
            return 'removed_before_target';
        }
        if ($completeOnly && $entity->completeness !== 'complete') {
            return match ($entity->completeness) {
                'partial' => 'partial',
                'unverified' => 'unverified',
                'disabled' => 'disabled',
                'missing_dependencies' => 'missing_dependency',
                default => 'unverified',
            };
        }
        if (in_array($entity->availability, self::NON_PUBLIC_AVAILABILITY, true)) {
            return 'availability_not_public';
        }

        return 'visible';
    }

    /** @param array<int, bool> $entityVisibility */
    private function relationReason(object $relation, int $targetOrder, bool $completeOnly, array $entityVisibility, ?string $override): string
    {
        if (in_array($override, ['exclude', 'force_hidden'], true)) {
            return 'profile_excluded';
        }
        if (! (bool) $relation->enabled) {
            return 'disabled';
        }
        if ($relation->introduced_order !== null && (int) $relation->introduced_order > $targetOrder) {
            return 'future_release';
        }
        if ($relation->removed_order !== null && $targetOrder >= (int) $relation->removed_order) {
            return 'removed_before_target';
        }
        if ($completeOnly && $relation->completeness !== 'complete') {
            return match ($relation->completeness) {
                'partial' => 'partial',
                'unverified' => 'unverified',
                'disabled' => 'disabled',
                'missing_dependencies' => 'missing_dependency',
                default => 'unverified',
            };
        }
        if (($entityVisibility[(int) $relation->source_entity_id] ?? false) !== true) {
            return 'source_hidden';
        }
        if ($relation->target_entity_id !== null && ($entityVisibility[(int) $relation->target_entity_id] ?? false) !== true) {
            return 'target_hidden';
        }

        return 'visible';
    }
}
