<?php

namespace App\GameCatalog\Queries\Public;

final readonly class PublicCatalogLootEntry
{
    public function __construct(
        public string $entityType,
        public string $slug,
        public string $name,
        public int $chanceNumerator,
        public int $chanceDenominator,
        public int $minimumCount,
        public int $maximumCount,
    ) {}
}
