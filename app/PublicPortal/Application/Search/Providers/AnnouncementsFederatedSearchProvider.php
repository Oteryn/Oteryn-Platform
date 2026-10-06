<?php

namespace App\PublicPortal\Application\Search\Providers;

use App\Announcements\Models\SiteAnnouncement;
use App\Announcements\Queries\ActiveAnnouncementQuery;
use App\PublicPortal\Application\Search\FederatedSearchProvider;
use App\PublicPortal\Application\Search\FederatedSearchQuery;
use App\PublicPortal\Application\Search\SearchHit;
use App\PublicPortal\Application\Search\SearchProviderResult;

final readonly class AnnouncementsFederatedSearchProvider implements FederatedSearchProvider
{
    public function __construct(private ActiveAnnouncementQuery $announcements) {}

    public function id(): string
    {
        return 'announcements';
    }

    public function search(FederatedSearchQuery $query): SearchProviderResult
    {
        if (! $query->allowsType('announcement')) {
            return new SearchProviderResult($this->id(), true, []);
        }

        $rows = $this->announcements->searchActive(
            $query->query,
            $query->page,
            $query->limit,
        );

        $hits = [];
        foreach ($rows as $index => $announcement) {
            $hits[] = $this->hit($announcement, $query->locale, $index + 1);
        }

        return new SearchProviderResult($this->id(), true, $hits);
    }

    private function hit(SiteAnnouncement $announcement, string $locale, int $rank): SearchHit
    {
        $url = $announcement->action_url;
        if (! is_string($url) || ! str_starts_with($url, '/') || str_starts_with($url, '//')) {
            throw new \LogicException('Searchable announcement lost its local canonical action path.');
        }

        return new SearchHit(
            sourceModule: 'Announcements',
            sourceType: 'announcement',
            stablePublicId: 'announcement-'.$announcement->id,
            sourceRevision: (string) $announcement->lock_version,
            locale: $locale,
            title: $announcement->title,
            snippetPlaintext: ProviderSearchSupport::snippet($announcement->body),
            canonicalUrl: $url,
            providerRank: $rank,
            publishedOrEffectiveAt: $announcement->starts_at->toIso8601String(),
            badges: [$announcement->severity],
        );
    }
}
