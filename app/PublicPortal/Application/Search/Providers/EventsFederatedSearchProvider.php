<?php

namespace App\PublicPortal\Application\Search\Providers;

use App\Events\Queries\EventCalendarQuery;
use App\PublicPortal\Application\Search\FederatedSearchProvider;
use App\PublicPortal\Application\Search\FederatedSearchQuery;
use App\PublicPortal\Application\Search\SearchHit;
use App\PublicPortal\Application\Search\SearchProviderResult;

final readonly class EventsFederatedSearchProvider implements FederatedSearchProvider
{
    public function __construct(private EventCalendarQuery $events) {}

    public function id(): string
    {
        return 'events';
    }

    public function search(FederatedSearchQuery $query): SearchProviderResult
    {
        if (! $query->allowsType('event')) {
            return new SearchProviderResult($this->id(), true, []);
        }

        $events = $this->events->searchPublic(
            $query->locale,
            $query->query,
            $query->page,
            $query->limit,
        );

        $hits = [];
        foreach ($events as $index => $event) {
            $hits[] = new SearchHit(
                sourceModule: 'Events',
                sourceType: 'event',
                stablePublicId: 'event-'.$event['id'],
                sourceRevision: null,
                locale: $query->locale,
                title: $event['title'],
                snippetPlaintext: ProviderSearchSupport::snippet($event['summary']),
                canonicalUrl: route('events.show', ['locale' => $query->locale, 'slug' => $event['slug']]),
                providerRank: $index + 1,
                publishedOrEffectiveAt: $event['starts_at']->toIso8601String(),
                badges: [$event['status']],
            );
        }

        return new SearchProviderResult($this->id(), true, $hits);
    }
}
