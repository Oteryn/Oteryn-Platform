<?php

namespace App\GameCatalog\Queries\Public;

use App\GameCatalog\Application\PublicRead\PublicCatalogContext;

final readonly class PublicCatalogItemPage
{
    /**
     * @param list<PublicCatalogItemCard> $items
     * @param list<string> $categories
     * @param list<string> $weaponTypes
     */
    public function __construct(
        public PublicCatalogContext $context,
        public array $items,
        public array $categories,
        public array $weaponTypes,
        public string $query,
        public ?string $category,
        public ?string $weaponType,
        public int $page,
        public int $perPage,
        public int $total,
    ) {}

    public function lastPage(): int
    {
        return max(1, (int) ceil($this->total / $this->perPage));
    }
}
