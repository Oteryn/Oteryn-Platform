<?php

namespace App\Downloads\Updater;

use App\Downloads\Models\ClientUpdatePolicy;

final class UpdaterPolicyDocument
{
    public const SCHEMA_VERSION = 1;

    public function targetPath(string $channel): string
    {
        return "channels/{$channel}/policy-v1.json";
    }

    /**
     * @param  list<array{artifact_id: int, platform: string, architecture: string, target_path: string, size_bytes: int, supplied_sha256: string}>  $artifactTargets
     * @param  list<string>  $revokedReleaseIds
     * @param  list<string>  $revokedArtifactTargets
     */
    public function encode(
        int $revision,
        string $channel,
        string $currentReleaseId,
        int $currentReleaseSequence,
        string $currentVersionDisplay,
        int $minimumSupportedReleaseSequence,
        string $updateMode,
        array $artifactTargets,
        array $revokedReleaseIds,
        array $revokedArtifactTargets,
        string $rollbackAuthorization,
    ): string {
        $publicTargets = array_map(
            static fn (array $target): array => [
                'platform' => $target['platform'],
                'architecture' => $target['architecture'],
                'target_path' => $target['target_path'],
            ],
            $artifactTargets,
        );
        usort(
            $publicTargets,
            static fn (array $left, array $right): int => strcmp(
                $left['platform'].'|'.$left['architecture'].'|'.$left['target_path'],
                $right['platform'].'|'.$right['architecture'].'|'.$right['target_path'],
            ),
        );

        $revokedReleaseIds = $this->sortedUnique($revokedReleaseIds);
        $revokedArtifactTargets = $this->sortedUnique($revokedArtifactTargets);

        return json_encode([
            'schema_version' => self::SCHEMA_VERSION,
            'policy_revision' => $revision,
            'channel' => $channel,
            'current_release_id' => $currentReleaseId,
            'current_release_sequence' => $currentReleaseSequence,
            'current_version_display' => $currentVersionDisplay,
            'minimum_supported_release_sequence' => $minimumSupportedReleaseSequence,
            'update_mode' => $updateMode,
            'artifacts' => $publicTargets,
            'revoked_release_ids' => $revokedReleaseIds,
            'revoked_artifact_targets' => $revokedArtifactTargets,
            'rollback_authorization' => $rollbackAuthorization,
        ], JSON_THROW_ON_ERROR | JSON_UNESCAPED_SLASHES);
    }

    public function encodePolicy(ClientUpdatePolicy $policy): string
    {
        $release = $policy->relationLoaded('currentRelease')
            ? $policy->currentRelease
            : $policy->currentRelease()->firstOrFail();

        /** @var list<array{artifact_id: int, platform: string, architecture: string, target_path: string, size_bytes: int, supplied_sha256: string}> $artifactTargets */
        $artifactTargets = $policy->artifact_targets;
        /** @var list<string> $revokedReleaseIds */
        $revokedReleaseIds = $policy->revoked_release_ids;
        /** @var list<string> $revokedArtifactTargets */
        $revokedArtifactTargets = $policy->revoked_artifact_targets;

        return $this->encode(
            $policy->revision,
            $policy->channel,
            (string) $release->updater_release_id,
            $policy->current_release_sequence,
            $release->version,
            $policy->minimum_supported_release_sequence,
            $policy->update_mode,
            $artifactTargets,
            $revokedReleaseIds,
            $revokedArtifactTargets,
            $policy->rollback_authorization,
        );
    }

    public function sha256(string $document): string
    {
        return hash('sha256', $document);
    }

    /** @param list<string> $values @return list<string> */
    private function sortedUnique(array $values): array
    {
        $values = array_values(array_unique($values));
        sort($values, SORT_STRING);

        return $values;
    }
}