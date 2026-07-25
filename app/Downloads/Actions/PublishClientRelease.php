<?php

namespace App\Downloads\Actions;

use App\Audit\AdminAuditRecorder;
use App\Downloads\Models\ClientRelease;
use App\Downloads\Models\ClientReleaseArtifact;
use App\Downloads\Security\ArtifactUrlPolicy;
use App\Identity\Models\Identity;
use Illuminate\Support\Facades\DB;
use Illuminate\Validation\ValidationException;

final readonly class PublishClientRelease
{
    public function __construct(
        private ArtifactUrlPolicy $artifactUrls,
        private AdminAuditRecorder $audit,
    ) {}

    public function execute(Identity $actor, ClientRelease $release, bool $makeCurrent): ClientRelease
    {
        $releaseId = $release->id;
        $channel = $release->channel;

        return DB::transaction(function () use ($actor, $releaseId, $channel, $makeCurrent): ClientRelease {
            $channelReleases = ClientRelease::query()
                ->where('channel', $channel)
                ->orderBy('id')
                ->lockForUpdate()
                ->get();
            $storedRelease = $channelReleases->first(
                static fn (ClientRelease $candidate): bool => $candidate->id === $releaseId,
            );

            if (! $storedRelease instanceof ClientRelease) {
                throw ValidationException::withMessages([
                    'release' => 'The release no longer exists in the selected channel.',
                ]);
            }

            if ($storedRelease->published_at !== null) {
                if (! $makeCurrent) {
                    throw ValidationException::withMessages([
                        'release' => 'The release is already published.',
                    ]);
                }

                if ($storedRelease->is_current) {
                    throw ValidationException::withMessages([
                        'release' => 'The release is already the current build for this channel.',
                    ]);
                }
            }

            $enabledArtifacts = ClientReleaseArtifact::query()
                ->where('client_release_id', $storedRelease->id)
                ->where('is_enabled', true)
                ->orderBy('id')
                ->lockForUpdate()
                ->get();

            if ($enabledArtifacts->isEmpty()) {
                throw ValidationException::withMessages([
                    'artifacts' => 'At least one enabled artifact is required before publication.',
                ]);
            }

            foreach ($enabledArtifacts as $artifact) {
                $reason = $this->artifactUrls->rejectionReason($artifact->artifact_url);

                if ($reason !== null) {
                    throw ValidationException::withMessages([
                        'artifacts' => "Artifact {$artifact->filename} {$reason}",
                    ]);
                }
            }

            $firstPublication = $storedRelease->published_at === null;
            $publishedAt = $storedRelease->published_at ?? now();
            $current = $makeCurrent || $storedRelease->is_current;

            if ($makeCurrent) {
                ClientRelease::query()
                    ->where('channel', $channel)
                    ->where('id', '!=', $storedRelease->id)
                    ->where('is_current', true)
                    ->update(['is_current' => false]);
            }

            DB::table('client_releases')
                ->where('id', $storedRelease->id)
                ->update([
                    'published_at' => $publishedAt,
                    'is_current' => $current,
                    'updated_at' => now(),
                ]);

            $storedRelease->refresh();

            $this->audit->record(
                $actor->id,
                $firstPublication ? 'downloads.release_published' : 'downloads.release_current_set',
                'client_release',
                (string) $storedRelease->id,
                [
                    'version' => $storedRelease->version,
                    'channel' => $storedRelease->channel,
                    'current' => $storedRelease->is_current,
                    'enabled_artifact_count' => $enabledArtifacts->count(),
                ],
            );

            return $storedRelease->load('artifacts');
        }, 3);
    }
}
