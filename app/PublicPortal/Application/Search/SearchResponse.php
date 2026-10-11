<?php

namespace App\PublicPortal\Application\Search;

final readonly class SearchResponse
{
    /** @param list<SearchProviderResult> $groups */
    public function __construct(
        public FederatedSearchQuery $query,
        public FederatedSearchState $state,
        public array $groups,
    ) {}

    public function hitCount(): int
    {
        return array_sum(array_map(
            static fn (SearchProviderResult $group): int => count($group->hits),
            $this->groups,
        ));
    }
}
