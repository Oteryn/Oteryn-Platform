<?php

namespace App\Downloads\Actions;

use App\Audit\AdminAuditRecorder;
use App\Downloads\DownloadCatalog;
use App\Downloads\Models\ClientRelease;
use App\Downloads\Models\ClientReleaseArtifact;
use App\Downloads\Models\ClientUpdatePolicy;
use App\Downloads\Updater\UpdaterPolicyDocument;
use App\Identity\Models\Identity;
use Illuminate\Database\Eloquent\Collection;
use Illuminate\Support\Facades\DB;
use Illuminate\Support\Str;
use Illuminate\Validation\ValidationException;

final readonly class ApproveUpdaterPolicy
{
    public function __construct(
        private UpdaterPolicyDocument $documents,
        private AdminAuditRecorder $audit,
    ) {}

    /**
     * @param  list<string>  $revokedReleaseIds
     * @param  list<string>  $revokedArtifactTargets
     */
    public function execute(
        Identity $actor,
        string $operationId,
        string $channel,
        int $currentReleaseId,
        int $minimumSupportedReleaseSequence,
        string $updateMode,
        string $rollbackAuthorization,
        array $revokedReleaseIds = [],
        array $revokedArtifactTargets = [],
    ): ClientUpdatePolicy {
        if (! Str::isUuid($operationId)
            || ! in_array($channel, DownloadCatalog::channels(), true)
            || ! in_array($updateMode, DownloadCatalog::updateModes(), true)
            || ! in_array($rollbackAuthorization, DownloadCatalog::rollbackAuthorizations(), true)) {
            throw ValidationException::withMessages([
                'policy' => 'Updater policy identity or enum values are invalid.',
            ]);
        }

        $revokedReleaseIds = $this->sortedUnique($revokedReleaseIds);
        $revokedArtifactTargets = $this->sortedUnique($revokedArtifactTargets);

        return DB::transaction(function () use (
            $actor,
            $operationId,
            $channel,
            $currentReleaseId,
            $minimumSupportedReleaseSequence,
            $updateMode,
            $rollbackAuthorization,
            $revokedReleaseIds,
            $revokedArtifactTargets,
        ): ClientUpdatePolicy {
            /** @var Collection<int, ClientRelease> $channelReleases */
            $channelReleases = ClientRelease::query()
                ->where('channel', $channel)
                ->with('artifacts')
                ->orderBy('id')
                ->lockForUpdate()
                ->get();

            $existing = ClientUpdatePolicy::query()
                ->where('operation_id', $operationId)
                ->first();

            if ($existing instanceof ClientUpdatePolicy) {
                $this->assertExistingIntent(
                    $existing,
                    $channel,
                    $currentReleaseId,
                    $minimumSupportedReleaseSequence,
                    $updateMode,
                    $rollbackAuthorization,
                    $revokedReleaseIds,
                    $revokedArtifactTargets,
                );

                return $existing->load('currentRelease');
            }

            $currentRelease = $channelReleases->first(
                static fn (ClientRelease $release): bool => $release->id === $currentReleaseId,
            );

            if (! $currentRelease instanceof ClientRelease
                || $currentRelease->published_at === null
                || $currentRelease->updater_release_id === null
                || $currentRelease->updater_sequence === null
                || $currentRelease->updater_withdrawn_at !== null) {
                throw ValidationException::withMessages([
                    'current_release_id' => 'The selected release must be published, updater-enabled and not withdrawn in this channel.',
                ]);
            }

            $minimumReleaseExists = $channelReleases->contains(
                static fn (ClientRelease $release): bool => $release->updater_sequence === $minimumSupportedReleaseSequence,
            );
            if (! $minimumReleaseExists
                || $minimumSupportedReleaseSequence < 1
                || $minimumSupportedReleaseSequence > $currentRelease->updater_sequence) {
                throw ValidationException::withMessages([
                    'minimum_supported_release_sequence' => 'Minimum support must name an existing channel sequence no newer than the selected current release.',
                ]);
            }

            $knownReleaseIds = $channelReleases
                ->pluck('updater_release_id')
                ->filter(static fn (mixed $value): bool => is_string($value) && $value !== '')
                ->values()
                ->all();
            if (array_diff($revokedReleaseIds, $knownReleaseIds) !== []) {
                throw ValidationException::withMessages([
                    'revoked_release_ids' => 'Every revoked release identity must belong to this updater channel.',
                ]);
            }

            $knownTargetPaths = [];
            foreach ($channelReleases as $release) {
                foreach ($release->artifacts as $artifact) {
                    if ($artifact->updater_target_path !== null) {
                        $knownTargetPaths[] = $artifact->updater_target_path;
                    }
                }
            }
            if (array_diff($revokedArtifactTargets, $knownTargetPaths) !== []) {
                throw ValidationException::withMessages([
                    'revoked_artifact_targets' => 'Every revoked target must be an exact updater target in this channel.',
                ]);
            }

            $previous = ClientUpdatePolicy::query()
                ->where('channel', $channel)
                ->orderByDesc('revision')
                ->lockForUpdate()
                ->first();
            $revision = ($previous?->revision ?? 0) + 1;
            $isRollback = $previous instanceof ClientUpdatePolicy
                && $currentRelease->updater_sequence < $previous->current_release_sequence;

            if ($isRollback && $rollbackAuthorization !== DownloadCatalog::ROLLBACK_EXPLICIT) {
                throw ValidationException::withMessages([
                    'rollback_authorization' => 'Selecting an older immutable release requires explicit rollback authorization in a newer policy revision.',
                ]);
            }

            if (! $isRollback && $rollbackAuthorization === DownloadCatalog::ROLLBACK_EXPLICIT) {
                throw ValidationException::withMessages([
                    'rollback_authorization' => 'Explicit rollback authorization is valid only when a newer policy selects an older channel sequence.',
                ]);
            }

            $artifactTargets = $this->snapshotTargets($currentRelease);
            $policyDocument = $this->documents->encode(
                $revision,
                $channel,
                $currentRelease->updater_release_id,
                $currentRelease->updater_sequence,
                $currentRelease->version,
                $minimumSupportedReleaseSequence,
                $updateMode,
                $artifactTargets,
                $revokedReleaseIds,
                $revokedArtifactTargets,
                $rollbackAuthorization,
            );

            $policy = ClientUpdatePolicy::query()->create([
                'operation_id' => $operationId,
                'channel' => $channel,
                'revision' => $revision,
                'current_release_id' => $currentRelease->id,
                'current_release_sequence' => $currentRelease->updater_sequence,
                'minimum_supported_release_sequence' => $minimumSupportedReleaseSequence,
                'update_mode' => $updateMode,
                'artifact_targets' => $artifactTargets,
                'revoked_release_ids' => $revokedReleaseIds,
                'revoked_artifact_targets' => $revokedArtifactTargets,
                'rollback_authorization' => $rollbackAuthorization,
                'policy_target_path' => $this->documents->targetPath($channel),
                'policy_document_sha256' => $this->documents->sha256($policyDocument),
                'policy_document_length' => strlen($policyDocument),
                'approved_at' => now(),
            ]);

            $this->audit->record(
                $actor->id,
                'downloads.updater_policy_approved',
                'client_update_policy',
                (string) $policy->id,
                [
                    'operation_id' => $operationId,
                    'channel' => $channel,
                    'policy_revision' => $revision,
                    'current_release_id' => $currentRelease->updater_release_id,
                    'current_release_sequence' => $currentRelease->updater_sequence,
                    'minimum_supported_release_sequence' => $minimumSupportedReleaseSequence,
                    'update_mode' => $updateMode,
                    'rollback_authorization' => $rollbackAuthorization,
                    'revoked_release_count' => count($revokedReleaseIds),
                    'revoked_target_count' => count($revokedArtifactTargets),
                ],
            );

            return $policy->load('currentRelease');
        }, 3);
    }

    /** @return list<array{artifact_id: int, platform: string, architecture: string, target_path: string, size_bytes: int, supplied_sha256: string}> */
    private function snapshotTargets(ClientRelease $release): array
    {
        $targets = [];

        foreach ($release->artifacts as $artifact) {
            if (! $artifact->is_enabled) {
                continue;
            }

            if (! $artifact instanceof ClientReleaseArtifact || $artifact->updater_target_path === null) {
                throw ValidationException::withMessages([
                    'artifacts' => 'Every enabled artifact requires an exact updater target before policy approval.',
                ]);
            }

            $targets[] = [
                'artifact_id' => $artifact->id,
                'platform' => $artifact->platform,
                'architecture' => $artifact->architecture,
                'target_path' => $artifact->updater_target_path,
                'size_bytes' => $artifact->size_bytes,
                'supplied_sha256' => $artifact->sha256,
            ];
        }

        if ($targets === []) {
            throw ValidationException::withMessages([
                'artifacts' => 'At least one exact updater target is required for policy approval.',
            ]);
        }

        usort(
            $targets,
            static fn (array $left, array $right): int => strcmp(
                $left['platform'].'|'.$left['architecture'].'|'.$left['target_path'],
                $right['platform'].'|'.$right['architecture'].'|'.$right['target_path'],
            ),
        );

        return $targets;
    }

    /**
     * @param list<string> $revokedReleaseIds
     * @param list<string> $revokedArtifactTargets
     */
    private function assertExistingIntent(
        ClientUpdatePolicy $existing,
        string $channel,
        int $currentReleaseId,
        int $minimumSupportedReleaseSequence,
        string $updateMode,
        string $rollbackAuthorization,
        array $revokedReleaseIds,
        array $revokedArtifactTargets,
    ): void {
        $same = $existing->channel === $channel
            && $existing->current_release_id === $currentReleaseId
            && $existing->minimum_supported_release_sequence === $minimumSupportedReleaseSequence
            && $existing->update_mode === $updateMode
            && $existing->rollback_authorization === $rollbackAuthorization
            && $this->sortedUnique($existing->revoked_release_ids) === $revokedReleaseIds
            && $this->sortedUnique($existing->revoked_artifact_targets) === $revokedArtifactTargets;

        if (! $same) {
            throw ValidationException::withMessages([
                'operation_id' => 'This operation identity already belongs to a different updater policy intent.',
            ]);
        }
    }

    /** @param list<string> $values @return list<string> */
    private function sortedUnique(array $values): array
    {
        $values = array_values(array_unique($values));
        sort($values, SORT_STRING);

        return $values;
    }
}