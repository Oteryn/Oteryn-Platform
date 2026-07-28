<?php

namespace App\GameCatalog\Application\Profiles;

use App\Audit\AdminAuditRecorder;
use App\GameCatalog\Contract\GameCatalogContract;
use Illuminate\Support\Facades\DB;
use InvalidArgumentException;
use RuntimeException;

final readonly class CatalogProfileActivator
{
    public function __construct(
        private CatalogVisibilityProjector $projector,
        private AdminAuditRecorder $audit,
    ) {}

    public function activate(
        int $snapshotId,
        string $profileKey,
        ?int $actorIdentityId = null,
        string $action = 'activate',
        ?string $reason = null,
    ): CatalogActivationResult {
        if (! in_array($action, ['activate', 'rollback'], true)) {
            throw new InvalidArgumentException('Unsupported Game Catalog activation action.');
        }
        if ($reason !== null && mb_strlen($reason) > 500) {
            throw new InvalidArgumentException('Activation reason exceeds 500 characters.');
        }

        return DB::transaction(function () use ($snapshotId, $profileKey, $actorIdentityId, $action, $reason): CatalogActivationResult {
            $profile = DB::table('game_catalog_profiles')
                ->where('key', $profileKey)
                ->lockForUpdate()
                ->first();
            if ($profile === null) {
                throw new RuntimeException("Game Catalog profile [{$profileKey}] does not exist.");
            }

            $snapshot = DB::table('game_catalog_snapshots as snapshots')
                ->join('game_catalog_releases as verified', 'verified.id', '=', 'snapshots.verified_content_through_release_id')
                ->where('snapshots.id', $snapshotId)
                ->first([
                    'snapshots.id',
                    'snapshots.status',
                    'snapshots.contract_id',
                    'snapshots.schema_version',
                    'verified.release_order as verified_order',
                ]);
            if ($snapshot === null || $snapshot->status !== 'validated') {
                throw new RuntimeException('Target Game Catalog snapshot is not validated.');
            }
            if ($snapshot->contract_id !== GameCatalogContract::ID || $snapshot->schema_version !== GameCatalogContract::SCHEMA_VERSION) {
                throw new RuntimeException('Target Game Catalog snapshot is incompatible with this consumer.');
            }

            $targetRelease = DB::table('game_catalog_releases')->whereKey($profile->target_release_id)->first(['id', 'release_order']);
            if ($targetRelease === null) {
                throw new RuntimeException('Profile target release is missing.');
            }
            if (! DB::table('game_catalog_snapshot_releases')
                ->where('snapshot_id', $snapshotId)
                ->where('release_id', $targetRelease->id)
                ->exists()) {
                throw new RuntimeException('Profile target release is not declared by the target snapshot.');
            }
            if ((int) $targetRelease->release_order > (int) $snapshot->verified_order) {
                throw new RuntimeException('Profile target release exceeds the snapshot verified content boundary.');
            }

            $projection = $this->projector->rebuild((int) $profile->id, $snapshotId);
            $previousSnapshotId = $profile->active_snapshot_id === null ? null : (int) $profile->active_snapshot_id;

            DB::table('game_catalog_profiles')->whereKey($profile->id)->update([
                'active_snapshot_id' => $snapshotId,
                'lock_version' => DB::raw('lock_version + 1'),
                'updated_at' => now(),
            ]);

            DB::table('game_catalog_activation_history')->insert([
                'profile_id' => (int) $profile->id,
                'from_snapshot_id' => $previousSnapshotId,
                'to_snapshot_id' => $snapshotId,
                'actor_identity_id' => $actorIdentityId,
                'action' => $action,
                'reason' => $reason,
                'occurred_at' => now(),
            ]);

            $this->audit->record(
                $actorIdentityId,
                $action === 'rollback' ? 'game_catalog.snapshot_rolled_back' : 'game_catalog.snapshot_activated',
                'game_catalog_profile',
                (string) $profile->id,
                [
                    'profile_key' => $profileKey,
                    'from_snapshot_id' => $previousSnapshotId,
                    'to_snapshot_id' => $snapshotId,
                    'visible_entities' => $projection->visibleEntities,
                    'visible_relations' => $projection->visibleRelations,
                ],
            );

            return new CatalogActivationResult(
                (int) $profile->id,
                $previousSnapshotId,
                $snapshotId,
                $projection,
            );
        }, 3);
    }
}
