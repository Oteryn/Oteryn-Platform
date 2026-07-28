<?php

namespace App\GameCatalog\Application\Activation;

use App\GameCatalog\Application\Configuration\CatalogConfiguration;
use App\GameCatalog\Infrastructure\Persistence\CatalogDatabaseRow;
use Carbon\CarbonImmutable;
use Illuminate\Database\Query\Builder;
use Illuminate\Database\Query\JoinClause;
use Illuminate\Support\Collection;
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

        /** @var array<int, true> $excludedEntities */
        $excludedEntities = [];
        /** @var array<int, true> $backportedEntities */
        $backportedEntities = [];
        /** @var array<int, true> $excludedRelations */
        $excludedRelations = [];
        /** @var array<int, true> $backportedRelations */
        $backportedRelations = [];
        foreach ($overrides as $override) {
            $overrideRow = CatalogDatabaseRow::from($override);
            $action = $overrideRow->string('action');
            $entityId = $overrideRow->nullableInt('entity_id');
            $relationSnapshotId = $overrideRow->nullableInt('relation_snapshot_id');

            if ($entityId !== null) {
                if ($action === 'exclude') {
                    $excludedEntities[$entityId] = true;
                } elseif ($allowBackports && $action === 'approved_backport') {
                    $backportedEntities[$entityId] = true;
                }
            }
            if ($relationSnapshotId !== null) {
                if ($action === 'exclude') {
                    $excludedRelations[$relationSnapshotId] = true;
                } elseif ($allowBackports && $action === 'approved_backport') {
                    $backportedRelations[$relationSnapshotId] = true;
                }
            }
        }

        $allowedItemAvailability = array_fill_keys(CatalogConfiguration::stringList('game-catalog.public_item_availability'), true);
        $allowedCreatureAvailability = array_fill_keys(CatalogConfiguration::stringList('game-catalog.public_creature_availability'), true);
        /** @var array<int, true> $visibleEntityIds */
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
            ->chunkById(1_000, function (Collection $rows) use (
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
                /** @var Collection<int, object> $rows */
                /** @var list<array{profile_id: int, entity_snapshot_id: int, visible: bool, reason_code: string, computed_at: CarbonImmutable}> $insert */
                $insert = [];
                foreach ($rows as $row) {
                    if (! is_object($row)) {
                        throw new RuntimeException('Game Catalog entity projection query returned an invalid row.');
                    }
                    $entityRow = CatalogDatabaseRow::from($row);
                    $entityId = $entityRow->int('entity_id');
                    $introducedOrder = $entityRow->nullableInt('introduced_order');
                    $removedOrder = $entityRow->nullableInt('removed_order');
                    $outsideRelease = ($introducedOrder !== null && $targetReleaseOrder < $introducedOrder)
                        || ($removedOrder !== null && $targetReleaseOrder >= $removedOrder);
                    $availability = $entityRow->string('availability');
                    $availabilityAllowed = $entityRow->string('entity_type') === 'item'
                        ? isset($allowedItemAvailability[$availability])
                        : isset($allowedCreatureAvailability[$availability]);

                    $reason = match (true) {
                        isset($excludedEntities[$entityId]) => 'explicit_exclusion',
                        ! $entityRow->bool('runtime_present') => 'runtime_absent',
                        ! $entityRow->bool('enabled') => 'disabled',
                        $outsideRelease && ! isset($backportedEntities[$entityId]) => 'outside_release',
                        $completeOnly && $entityRow->string('completeness') !== 'complete' => 'incomplete',
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
                        'entity_snapshot_id' => $entityRow->int('entity_snapshot_id'),
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
            ->chunkById(1_000, function (Collection $rows) use (
                $profileId,
                $targetReleaseOrder,
                $completeOnly,
                $computedAt,
                $excludedRelations,
                $backportedRelations,
                $visibleEntityIds,
                &$visibleRelationCount,
            ): void {
                /** @var Collection<int, object> $rows */
                /** @var list<array{profile_id: int, relation_snapshot_id: int, visible: bool, reason_code: string, computed_at: CarbonImmutable}> $insert */
                $insert = [];
                foreach ($rows as $row) {
                    if (! is_object($row)) {
                        throw new RuntimeException('Game Catalog relation projection query returned an invalid row.');
                    }
                    $relationRow = CatalogDatabaseRow::from($row);
                    $relationId = $relationRow->int('relation_snapshot_id');
                    $introducedOrder = $relationRow->nullableInt('introduced_order');
                    $removedOrder = $relationRow->nullableInt('removed_order');
                    $outsideRelease = ($introducedOrder !== null && $targetReleaseOrder < $introducedOrder)
                        || ($removedOrder !== null && $targetReleaseOrder >= $removedOrder);
                    $sourceEntityId = $relationRow->int('source_entity_id');
                    $targetEntityId = $relationRow->nullableInt('target_entity_id');

                    $reason = match (true) {
                        isset($excludedRelations[$relationId]) => 'explicit_exclusion',
                        ! $relationRow->bool('enabled') => 'disabled',
                        $outsideRelease && ! isset($backportedRelations[$relationId]) => 'outside_release',
                        $completeOnly && $relationRow->string('completeness') !== 'complete' => 'incomplete',
                        ! isset($visibleEntityIds[$sourceEntityId]) => 'source_hidden',
                        $targetEntityId === null || ! isset($visibleEntityIds[$targetEntityId]) => 'target_hidden',
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
            ->leftJoin('game_catalog_entity_snapshots as source_snapshot', function (JoinClause $join) use ($snapshotId): void {
                $join->on('source_snapshot.entity_id', '=', 'relations.source_entity_id')
                    ->where('source_snapshot.snapshot_id', '=', $snapshotId);
            })
            ->leftJoin('game_catalog_entity_snapshots as target_snapshot', function (JoinClause $join) use ($snapshotId): void {
                $join->on('target_snapshot.entity_id', '=', 'relations.target_entity_id')
                    ->where('target_snapshot.snapshot_id', '=', $snapshotId);
            })
            ->leftJoin('game_catalog_profile_entities as source_visibility', function (JoinClause $join) use ($profileId): void {
                $join->on('source_visibility.entity_snapshot_id', '=', 'source_snapshot.id')
                    ->where('source_visibility.profile_id', '=', $profileId);
            })
            ->leftJoin('game_catalog_profile_entities as target_visibility', function (JoinClause $join) use ($profileId): void {
                $join->on('target_visibility.entity_snapshot_id', '=', 'target_snapshot.id')
                    ->where('target_visibility.profile_id', '=', $profileId);
            })
            ->where('visibility.profile_id', $profileId)
            ->where('visibility.visible', true)
            ->where(function (Builder $query): void {
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
