<?php

namespace App\GameCatalog\Queries\Public;

use App\GameCatalog\Application\PublicRead\PublicCatalogContext;

final readonly class PublicCatalogSummary
{
    public function __construct(
        public PublicCatalogContext $context,
        public int $itemCount,
        public int $creatureCount,
    ) {}
}
