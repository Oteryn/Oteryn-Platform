<?php

namespace App\PublicPortal\Application\Search;

final readonly class SearchHit
{
    /** @param list<string> $badges */
    public function __construct(
        public string $sourceModule,
        public string $sourceType,
        public string $stablePublicId,
        public ?string $sourceRevision,
        public string $locale,
        public string $title,
        public ?string $snippetPlaintext,
        public string $canonicalUrl,
        public int $providerRank,
        public ?string $publishedOrEffectiveAt = null,
        public ?string $applicability = null,
        public ?string $freshness = null,
        public array $badges = [],
    ) {}
}
