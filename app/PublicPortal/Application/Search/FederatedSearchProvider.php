<?php

namespace App\PublicPortal\Application\Search;

interface FederatedSearchProvider
{
    public function id(): string;

    public function search(FederatedSearchQuery $query): SearchProviderResult;
}
