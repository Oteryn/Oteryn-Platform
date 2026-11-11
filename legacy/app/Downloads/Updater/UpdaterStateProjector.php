<?php

namespace App\Downloads\Updater;

use App\Downloads\Models\ClientRelease;
use App\Downloads\Models\ClientReleaseArtifact;
use App\Downloads\Models\ClientUpdateGeneration;
use App\Downloads\Models\ClientUpdatePolicy;
use Illuminate\Database\Eloquent\Collection;

final class UpdaterStateProjector
{
    /** @param Collection<int, ClientRelease> $releases */
    public function annotate(Collection $releases): void
    {
        if ($releases->isEmpty()) {
            return;
        }

        $channels = $releases->pluck('channel')->unique()->values()->all();
        $activeByChannel = ClientUpdateGeneration::query()
            ->whereIn('channel', $channels)
            ->whereNotNull('activated_at')
            ->whereNull('superseded_at')
            ->with('policy')
            ->get()
            ->groupBy('channel');

        foreach ($releases as $release) {
            $activeCandidates = $activeByChannel->get($release->channel);

            if ($activeCandidates === null || $activeCandidates->isEmpty()) {
                $this->setReleaseState(
                    $release,
                    $release->updater_release_id === null ? 'browser_only' : 'pending',
                );

                continue;
            }

            if ($activeCandidates->count() !== 1) {
                $this->setReleaseState($release, 'degraded');

                continue;
            }

            $generation = $activeCandidates->firstOrFail();
            $policy = $generation->policy;
            $this->setGenerationAttributes($release, $generation, $policy);

            if (! $generation->metadata_expires_at->isFuture()) {
                $this->setReleaseState($release, 'trust_expired');

                continue;
            }

            if ($release->updater_withdrawn_at !== null) {
                $this->setReleaseState($release, 'withdrawn');

                continue;
            }

            /** @var list<string> $revokedReleaseIds */
            $revokedReleaseIds = $policy->revoked_release_ids;
            if ($release->updater_release_id !== null
                && in_array($release->updater_release_id, $revokedReleaseIds, true)) {
                $this->setReleaseState($release, 'revoked');

                continue;
            }

            if ($policy->current_release_id !== $release->id) {
                $this->setReleaseState($release, 'browser_mismatch');

                continue;
            }

            /** @var list<string> $revokedTargets */
            $revokedTargets = $policy->revoked_artifact_targets;
            $hasRevokedCurrentTarget = $release->artifacts->contains(
                static fn (ClientReleaseArtifact $artifact): bool => $artifact->updater_target_path !== null
                    && in_array($artifact->updater_target_path, $revokedTargets, true),
            );

            $this->setReleaseState($release, $hasRevokedCurrentTarget ? 'target_revoked' : 'active');
            $this->annotateArtifacts($release, $generation, $revokedTargets);
        }
    }

    /** @param Collection<int, ClientRelease> $releases */
    public function markUnavailable(Collection $releases): void
    {
        foreach ($releases as $release) {
            $this->setReleaseState(
                $release,
                $release->updater_release_id === null ? 'browser_only' : 'degraded',
            );
        }
    }

    private function setReleaseState(ClientRelease $release, string $state): void
    {
        $release->setAttribute('updater_public_state', $state);
    }

    private function setGenerationAttributes(
        ClientRelease $release,
        ClientUpdateGeneration $generation,
        ClientUpdatePolicy $policy,
    ): void {
        $release->setAttribute('updater_generation_id', $generation->generation_id);
        $release->setAttribute('updater_policy_revision', $policy->revision);
        $release->setAttribute('updater_update_mode', $policy->update_mode);
        $release->setAttribute(
            'updater_minimum_supported_release_sequence',
            $policy->minimum_supported_release_sequence,
        );
        $release->setAttribute('updater_metadata_expires_at', $generation->metadata_expires_at);
    }

    /** @param list<string> $revokedTargets */
    private function annotateArtifacts(
        ClientRelease $release,
        ClientUpdateGeneration $generation,
        array $revokedTargets,
    ): void {
        /** @var list<array{platform: string, architecture: string, target_path: string, length: int, sha256: string}> $generationTargets */
        $generationTargets = $generation->targets;
        $targetKeys = [];

        foreach ($generationTargets as $target) {
            $targetKeys[$target['platform'].'|'.$target['architecture'].'|'.$target['target_path']] = true;
        }

        foreach ($release->artifacts as $artifact) {
            if ($artifact->updater_target_path === null) {
                $artifact->setAttribute('updater_target_state', 'unavailable');

                continue;
            }

            if (in_array($artifact->updater_target_path, $revokedTargets, true)) {
                $artifact->setAttribute('updater_target_state', 'revoked');

                continue;
            }

            $key = $artifact->platform.'|'.$artifact->architecture.'|'.$artifact->updater_target_path;
            $artifact->setAttribute(
                'updater_target_state',
                isset($targetKeys[$key]) ? 'reconciled' : 'unavailable',
            );
        }
    }
}
