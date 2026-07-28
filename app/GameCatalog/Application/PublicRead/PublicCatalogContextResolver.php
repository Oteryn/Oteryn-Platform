<?php

namespace App\GameCatalog\Application\PublicRead;

use Illuminate\Support\Facades\DB;

final class PublicCatalogContextResolver
{
    public function resolve(): ?PublicCatalogContext
    {
        $profileKey = (string) config('game-catalog.public_profile_key', 'public');
        $row = DB::table('game_catalog_profiles as profiles')
            ->join('game_catalog_snapshots as snapshots', 'snapshots.id', '=', 'profiles.active_snapshot_id')
            ->join('game_catalog_releases as target_release', 'target_release.id', '=', 'profiles.target_release_id')
            ->join('game_catalog_releases as runtime_release', 'runtime_release.id', '=', 'snapshots.runtime_release_id')
            ->join('game_catalog_releases as content_target', 'content_target.id', '=', 'snapshots.content_target_release_id')
            ->join('game_catalog_releases as verified_release', 'verified_release.id', '=', 'snapshots.verified_content_through_release_id')
            ->leftJoin('game_catalog_releases as contains_release', 'contains_release.id', '=', 'snapshots.contains_content_through_release_id')
            ->where('profiles.key', $profileKey)
            ->where('profiles.public_enabled', true)
            ->where('snapshots.status', 'validated')
            ->first([
                'profiles.id as profile_id',
                'profiles.key as profile_key',
                'profiles.name as profile_name',
                'snapshots.id as snapshot_id',
                'snapshots.content_sha256 as snapshot_sha256',
                'snapshots.generated_at',
                'target_release.key as target_release',
                'runtime_release.key as runtime_release',
                'content_target.key as content_target_release',
                'verified_release.key as verified_release',
                'contains_release.key as contains_release',
            ]);

        if ($row === null) {
            return null;
        }

        return new PublicCatalogContext(
            profileId: (int) $row->profile_id,
            profileKey: (string) $row->profile_key,
            profileName: (string) $row->profile_name,
            snapshotId: (int) $row->snapshot_id,
            snapshotSha256: (string) $row->snapshot_sha256,
            targetRelease: (string) $row->target_release,
            runtimeRelease: (string) $row->runtime_release,
            contentTargetRelease: (string) $row->content_target_release,
            verifiedThroughRelease: (string) $row->verified_release,
            containsThroughRelease: $row->contains_release === null ? null : (string) $row->contains_release,
            generatedAt: (string) $row->generated_at,
        );
    }
}
