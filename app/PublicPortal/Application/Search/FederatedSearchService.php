<?php

namespace App\PublicPortal\Application\Search;

use App\PublicPortal\Application\Search\Providers\AnnouncementsFederatedSearchProvider;
use App\PublicPortal\Application\Search\Providers\CmsFederatedSearchProvider;
use App\PublicPortal\Application\Search\Providers\EventsFederatedSearchProvider;
use App\PublicPortal\Application\Search\Providers\GameCatalogFederatedSearchProvider;
use App\PublicPortal\Application\Search\Providers\WikiFederatedSearchProvider;
use Illuminate\Support\Facades\Log;
use Throwable;

final readonly class FederatedSearchService
{
    /** @var array<string, FederatedSearchProvider> */
    private array $providers;

    public function __construct(
        CmsFederatedSearchProvider $cms,
        WikiFederatedSearchProvider $wiki,
        GameCatalogFederatedSearchProvider $gameCatalog,
        EventsFederatedSearchProvider $events,
        AnnouncementsFederatedSearchProvider $announcements,
    ) {
        $this->providers = [
            $cms->id() => $cms,
            $wiki->id() => $wiki,
            $gameCatalog->id() => $gameCatalog,
            $events->id() => $events,
            $announcements->id() => $announcements,
        ];
    }

    public function search(FederatedSearchQuery $query): SearchResponse
    {
        $groups = [];
        $available = 0;

        foreach ($query->providers as $providerId) {
            $provider = $this->providers[$providerId] ?? null;
            if ($provider === null) {
                $groups[] = SearchProviderResult::unavailable($providerId);
                continue;
            }

            try {
                $group = $provider->search($query);
            } catch (Throwable $exception) {
                Log::warning('Federated search provider unavailable.', [
                    'provider' => $providerId,
                    'exception_class' => $exception::class,
                ]);
                $group = SearchProviderResult::unavailable($providerId);
            }

            if ($group->available) {
                $available++;
            }
            $groups[] = $group;
        }

        $state = match (true) {
            $available === count($groups) => FederatedSearchState::Complete,
            $available === 0 => FederatedSearchState::Unavailable,
            default => FederatedSearchState::Partial,
        };

        return new SearchResponse($query, $state, $groups);
    }
}
