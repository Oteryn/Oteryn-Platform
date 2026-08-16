<?php

namespace App\Downloads\Actions;

use App\Audit\AdminAuditRecorder;
use App\Downloads\Models\ClientRelease;
use App\Downloads\Models\ClientReleaseArtifact;
use App\Identity\Models\Identity;
use Illuminate\Support\Facades\DB;
use Illuminate\Support\Str;
use Illuminate\Validation\ValidationException;

final readonly class EnableUpdaterRelease
{
    public function __construct(private AdminAuditRecorder $audit) {}

    public function execute(Identity $actor, ClientRelease $release): ClientRelease
    {
        $releaseId = $release->id;
        $channel = $release->channel;

        return DB::transaction(function () use ($actor, $releaseId, $channel): ClientRelease {
            $channelReleases = ClientRelease::query()
                ->where('channel', $channel)
                ->orderBy('id')
                ->lockForUpdate()
                ->get();
            $storedRelease = $channelReleases->first(
                static fn (ClientRelease $candidate): bool => $candidate->id === $releaseId,
            );

            if (! $storedRelease instanceof ClientRelease || $storedRelease->published_at === null) {
                throw ValidationException::withMessages([
                    'release' => 'Only an existing immutable published release can receive an updater identity.',
                ]);
            }

            $enabledArtifacts = ClientReleaseArtifact::query()
                ->where('client_release_id', $storedRelease->id)
                ->where('is_enabled', true)
                ->orderBy('id')
                ->lockForUpdate()
                ->get();

            if ($enabledArtifacts->isEmpty()) {
                throw ValidationException::withMessages([
                    'artifacts' => 'At least one enabled immutable artifact is required for updater distribution.',
                ]);
            }

            if ($storedRelease->updater_release_id !== null) {
                foreach ($enabledArtifacts as $artifact) {
                    if ($artifact->updater_target_path === null) {
                        throw ValidationException::withMessages([
                            'release' => 'The updater identity exists but an enabled artifact target is missing. Reconciliation is required.',
                        ]);
                    }
                }

                return $storedRelease->load('artifacts');
            }

            $sequence = (int) ($channelReleases->max('updater_sequence') ?? 0) + 1;
            $updaterReleaseId = 'rel_'.Str::uuid()->toString();

            $storedRelease->forceFill([
                'updater_release_id' => $updaterReleaseId,
                'updater_sequence' => $sequence,
                'updater_enabled_at' => now(),
            ])->save();

            foreach ($enabledArtifacts as $artifact) {
                $artifact->forceFill([
                    'updater_target_path' => sprintf(
                        'channels/%s/releases/%s/%s/%s/%s',
                        $channel,
                        $updaterReleaseId,
                        $artifact->platform,
                        $artifact->architecture,
                        rawurlencode($artifact->filename),
                    ),
                ])->save();
            }

            $this->audit->record(
                $actor->id,
                'downloads.updater_release_enabled',
                'client_release',
                (string) $storedRelease->id,
                [
                    'channel' => $channel,
                    'updater_release_id' => $updaterReleaseId,
                    'updater_sequence' => $sequence,
                    'target_count' => $enabledArtifacts->count(),
                ],
            );

            return $storedRelease->refresh()->load('artifacts');
        }, 3);
    }
}