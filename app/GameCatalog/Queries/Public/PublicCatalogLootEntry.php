<?php

namespace App\GameCatalog\Queries\Public;

final readonly class PublicCatalogLootEntry
{
    public function __construct(
        public string $entityType,
        public string $slug,
        public string $name,
        public string $chanceModel,
        public ?int $chanceNumerator,
        public ?int $chanceDenominator,
        public ?int $chanceThreshold,
        public ?int $rollMaximum,
        public int $minimumCount,
        public int $maximumCount,
    ) {}
}
