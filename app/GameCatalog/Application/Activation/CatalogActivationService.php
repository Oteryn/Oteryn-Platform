<?php

namespace App\GameCatalog\Application\Activation;

use Carbon\CarbonImmutable;
use Illuminate\Database\ConnectionInterface;
use Illuminate\Support\Facades\DB;
use JsonException;
use RuntimeException;

final class CatalogActivationService
{
    public function __construct(
        private readonly VisibilityProjector $visibilityProjector,
        private readonly ConnectionInterface $database,
    ) {}

    public function activate(
        int $snapshotId,
        string $profileKey,
        bool $rollback = false,
        ?int $actorIdentityId = null,
    ): CatalogActivationResult {
        return $this->database->transaction(function () use ($snapshotId, $profileKey, $rollback, $actorIdentityId): CatalogActivationResult {
            $profile = DB::table('game_catalog_profiles as profiles')
                ->join('game_catalog_releases as target_release', 'target_release.id', '=', 'profiles.target_release_id')
                ->where('profiles.key', $profileKey)
                ->lockForUpdate()
                ->first([
                    'profiles.id',
                    'profiles.key',
                    'profiles.active_snapshot_id',
                    'profiles.protocol_profile',
                    'profiles.complete_only',
                    'profiles.allow_backports',
                    'profiles.lock_version',
                    'target_release.release_order as target_release_order',
                ]);

            if ($profile === null) {
                throw new RuntimeException("Game Catalog profile '{$profileKey}' does not exist.");
            }

            $snapshot = DB::table('game_catalog_snapshots as snapshots')
                ->join('game_catalog_releases as content_target', 'content_target.id', '=', 'snapshots.content_target_release_id')
                ->leftJoin('game_catalog_releases as contains_content', 'contains_content.id', '=', 'snapshots.contains_content_through_release_id')
                ->where('snapshots.id', $snapshotId)
                ->first([
                    'snapshots.id',
                    'snapshots.status',
                    'snapshots.contract_version',
                    'snapshots.schema_version',
                    'snapshots.protocol_profile',
                    'snapshots.entity_count',
                    'snapshots.relation_count',
                    'content_target.release_order as content_target_order',
                    'contains_content.release_order as contains_content_order',
                ]);

            if ($snapshot === null) {
                throw new RuntimeException("Game Catalog snapshot '{$snapshotId}' does not exist.");
            }
            if ($snapshot->status !== 'validated') {
                throw new RuntimeException('Only validated immutable Game Catalog snapshots can be activated.');
            }
            if ($snapshot->contract_version !== config('game-catalog.contract') || $snapshot->schema_version !== config('game-catalog.schema_version')) {
                throw new RuntimeException('The Game Catalog snapshot contract is incompatible with this Platform build.');
            }
            if ($profile->protocol_profile !== null && $profile->protocol_profile !== $snapshot->protocol_profile) {
                throw new RuntimeException('The Game Catalog snapshot protocol profile is incompatible with the target profile.');
            }

            $contentBoundary = $snapshot->contains_content_order ?? $snapshot->content_target_order;
            if ((int) $profile->target_release_order > (int) $contentBoundary) {
                throw new RuntimeException('The Game Catalog snapshot does not declare content through the profile target release.');
            }

            $persistedEntityCount = DB::table('game_catalog_entity_snapshots')->where('snapshot_id', $snapshotId)->count();
            $persistedRelationCount = DB::table('game_catalog_relation_snapshots')->where('snapshot_id', $snapshotId)->count();
            if ($persistedEntityCount !== (int) $snapshot->entity_count || $persistedRelationCount !== (int) $snapshot->relation_count) {
                throw new RuntimeException('The Game Catalog snapshot persistence verification failed before activation.');
            }

            $computedAt = CarbonImmutable::now('UTC');
            $projection = $this->visibilityProjector->rebuild(
                profileId: (int) $profile->id,
                snapshotId: $snapshotId,
                targetReleaseOrder: (int) $profile->target_release_order,
                completeOnly: (bool) $profile->complete_only,
                allowBackports: (bool) $profile->allow_backports,
                computedAt: $computedAt,
            );

            DB::table('game_catalog_profiles')->where('id', $profile->id)->update([
                'active_snapshot_id' => $snapshotId,
                'lock_version' => (int) $profile->lock_version + 1,
                'updated_at' => $computedAt,
            ]);

            DB::table('game_catalog_audit_events')->insert([
                'action' => $rollback ? 'snapshot.rollback' : 'snapshot.activate',
                'actor_identity_id' => $actorIdentityId,
                'profile_id' => (int) $profile->id,
                'snapshot_id' => $snapshotId,
                'metadata' => $this->json([
                    'previous_snapshot_id' => $profile->active_snapshot_id === null ? null : (int) $profile->active_snapshot_id,
                    'target_snapshot_id' => $snapshotId,
                    'visible_entity_count' => $projection->visibleEntityCount,
                    'visible_relation_count' => $projection->visibleRelationCount,
                    'rollback' => $rollback,
                ]),
                'created_at' => $computedAt,
            ]);

            return new CatalogActivationResult(
                profileId: (int) $profile->id,
                profileKey: (string) $profile->key,
                snapshotId: $snapshotId,
                previousSnapshotId: $profile->active_snapshot_id === null ? null : (int) $profile->active_snapshot_id,
                visibleEntityCount: $projection->visibleEntityCount,
                visibleRelationCount: $projection->visibleRelationCount,
                rollback: $rollback,
            );
        }, 3);
    }

    private function json(mixed $value): string
    {
        try {
            return json_encode($value, JSON_THROW_ON_ERROR | JSON_UNESCAPED_SLASHES | JSON_UNESCAPED_UNICODE);
        } catch (JsonException $exception) {
            throw new RuntimeException('Game Catalog activation audit metadata could not be encoded.', previous: $exception);
        }
    }
}
