<?php

namespace App\Downloads;

use App\Cms\Editorial\EditorialContentType;
use App\Cms\Models\EditorialTranslation;
use App\Downloads\Models\ClientRelease;
use App\Downloads\Models\ClientReleaseArtifact;
use App\Downloads\Security\ArtifactUrlPolicy;
use App\Downloads\Updater\UpdaterStateProjector;
use App\Downloads\ViewModels\DownloadCenterViewModel;
use DateTimeInterface;
use Illuminate\Database\Eloquent\Collection;
use Throwable;

final readonly class PublicDownloadCenterQuery
{
    public function __construct(
        private ArtifactUrlPolicy $artifactUrls,
        private UpdaterStateProjector $updaterStates,
    ) {}

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

            $this->localizeReleaseNotes($releases);
        } catch (Throwable) {
            return new DownloadCenterViewModel(DownloadCenterState::UNAVAILABLE, [], $platform);
        }

        try {
            $this->updaterStates->annotate($releases);
        } catch (Throwable) {
            $this->updaterStates->markUnavailable($releases);
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

    /** @param Collection<int, ClientRelease> $releases */
    private function localizeReleaseNotes(Collection $releases): void
    {
        if (app()->getLocale() !== 'pl' || $releases->isEmpty()) {
            return;
        }

        $translations = EditorialTranslation::query()
            ->where('content_type', EditorialContentType::ClientRelease->value)
            ->where('locale', 'pl')
            ->whereIn('content_id', $releases->modelKeys())
            ->whereNotNull('body')
            ->whereNotNull('published_at')
            ->where('published_at', '<=', now())
            ->get()
            ->keyBy('content_id');

        foreach ($releases as $release) {
            $sourceHadNotes = is_string($release->release_notes) && trim($release->release_notes) !== '';
            $translation = $translations->get($release->id);
            $fresh = $translation instanceof EditorialTranslation
                && $release->updated_at instanceof DateTimeInterface
                && ! $translation->source_updated_at->lt($release->updated_at);

            $release->setAttribute('release_notes', $fresh ? $translation->body : null);
            $release->setAttribute(
                'release_notes_translation_unavailable',
                $sourceHadNotes && ! $fresh,
            );
        }
    }
}