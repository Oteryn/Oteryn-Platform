<?php

namespace App\Downloads;

use App\Downloads\Models\ClientRelease;
use App\Downloads\Models\ClientReleaseArtifact;
use App\Downloads\Security\ArtifactUrlPolicy;
use App\Downloads\ViewModels\DownloadCenterViewModel;
use Throwable;

final readonly class PublicDownloadCenterQuery
{
    public function __construct(private ArtifactUrlPolicy $artifactUrls) {}

    public function get(?string $platform = null): DownloadCenterViewModel
    {
        try {
            $releases = ClientRelease::query()
                ->whereNotNull('published_at')
                ->where('published_at', '<=', now())
                ->where('is_current', true)
                ->with('artifacts')
                ->orderByRaw('CASE WHEN channel = ? THEN 0 ELSE 1 END', [DownloadCatalog::CHANNEL_STABLE])
                ->orderByDesc('published_at')
                ->orderByDesc('id')
                ->get();
        } catch (Throwable) {
            return new DownloadCenterViewModel(DownloadCenterState::UNAVAILABLE, [], $platform);
        }

        if ($releases->isEmpty()) {
            return new DownloadCenterViewModel(DownloadCenterState::EMPTY, [], $platform);
        }

        /** @var list<ClientRelease> $publicReleases */
        $publicReleases = [];
        $rejectedArtifactSeen = false;

        foreach ($releases as $release) {
            $approved = $release->artifacts
                ->filter(function (ClientReleaseArtifact $artifact) use (&$rejectedArtifactSeen): bool {
                    if (! $artifact->is_enabled) {
                        return false;
                    }

                    if ($this->artifactUrls->isApproved($artifact->artifact_url)) {
                        return true;
                    }

                    $rejectedArtifactSeen = true;

                    return false;
                })
                ->sortBy(static fn (ClientReleaseArtifact $artifact): string => sprintf(
                    '%s|%s|%020d',
                    $artifact->platform,
                    $artifact->architecture,
                    $artifact->id,
                ))
                ->values();

            if ($platform !== null) {
                $approved = $approved->where('platform', $platform)->values();
            }

            if ($approved->isEmpty()) {
                continue;
            }

            $release->setRelation('artifacts', $approved);
            $publicReleases[] = $release;
        }

        if ($publicReleases === []) {
            return new DownloadCenterViewModel(
                $rejectedArtifactSeen ? DownloadCenterState::UNAVAILABLE : DownloadCenterState::EMPTY,
                [],
                $platform,
            );
        }

        return new DownloadCenterViewModel(DownloadCenterState::AVAILABLE, $publicReleases, $platform);
    }
}
