<?php

namespace App\PublicPortal\Application\Search;

final readonly class SearchProviderResult
{
    /** @param list<SearchHit> $hits */
    public function __construct(
        public string $provider,
        public bool $available,
        public array $hits,
    ) {}

    public static function unavailable(string $provider): self
    {
        return new self($provider, false, []);
    }
}
