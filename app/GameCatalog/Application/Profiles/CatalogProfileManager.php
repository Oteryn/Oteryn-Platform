<?php

namespace App\GameCatalog\Application\Profiles;

use App\Audit\AdminAuditRecorder;
use Illuminate\Support\Facades\DB;
use RuntimeException;

final readonly class CatalogProfileManager
{
    public function __construct(
        private CatalogVisibilityProjector $projector,
        private AdminAuditRecorder $audit,
    ) {}

    public function update(
        int $profileId,
        string $targetReleaseKey,
        bool $completeOnly,
        bool $publicEnabled,
        ?int $actorIdentityId,
    ): VisibilityProjectionResult|null {
        return DB::transaction(function () use ($profileId, $targetReleaseKey, $completeOnly, $publicEnabled, $actorIdentityId): ?VisibilityProjectionResult {
            $profile = DB::table('game_catalog_profiles')->whereKey($profileId)->lockForUpdate()->first();
            if ($profile === null) {
                throw new RuntimeException('Game Catalog profile does not exist.');
            }
            $target = DB::table('game_catalog_releases')->where('key', $targetReleaseKey)->first(['id', 'release_order']);
            if ($target === null) {
                throw new RuntimeException('Requested Game Catalog target release does not exist.');
            }

            if ($profile->active_snapshot_id !== null) {
                $snapshot = DB::table('game_catalog_snapshots as snapshots')
                    ->join('game_catalog_releases as verified', 'verified.id', '=', 'snapshots.verified_content_through_release_id')
                    ->where('snapshots.id', $profile->active_snapshot_id)
                    ->first(['snapshots.id', 'snapshots.status', 'verified.release_order as verified_order']);
                if ($snapshot === null || $snapshot->status !== 'validated') {
                    throw new RuntimeException('Active Game Catalog snapshot is unavailable.');
                }
                if (! DB::table('game_catalog_snapshot_releases')
                    ->where('snapshot_id', $snapshot->id)
                    ->where('release_id', $target->id)
                    ->exists()) {
                    throw new RuntimeException('Target release is not declared by the active snapshot.');
                }
                if ((int) $target->release_order > (int) $snapshot->verified_order) {
                    throw new RuntimeException('Target release exceeds the active snapshot verified boundary.');
                }
            }

            DB::table('game_catalog_profiles')->whereKey($profileId)->update([
                'target_release_id' => $target->id,
                'complete_only' => $completeOnly,
                'public_enabled' => $publicEnabled,
                'lock_version' => DB::raw('lock_version + 1'),
                'updated_at' => now(),
            ]);

            $projection = $profile->active_snapshot_id === null
                ? null
                : $this->projector->rebuild($profileId, (int) $profile->active_snapshot_id);

            $this->audit->record(
                $actorIdentityId,
                'game_catalog.profile_updated',
                'game_catalog_profile',
                (string) $profileId,
                [
                    'profile_key' => (string) $profile->key,
                    'target_release' => $targetReleaseKey,
                    'complete_only' => $completeOnly,
                    'public_enabled' => $publicEnabled,
                ],
            );

            return $projection;
        }, 3);
    }
}
