<?php

namespace App\GameCatalog\Queries\Public;

use App\GameCatalog\Application\PublicRead\PublicCatalogContext;

final readonly class PublicCatalogCreaturePage
{
    /**
     * @param list<PublicCatalogCreatureCard> $creatures
     * @param list<string> $bestiaryClasses
     */
    public function __construct(
        public PublicCatalogContext $context,
        public array $creatures,
        public array $bestiaryClasses,
        public string $query,
        public ?string $bestiaryClass,
        public bool $bossOnly,
        public int $page,
        public int $perPage,
        public int $total,
    ) {}

    public function lastPage(): int
    {
        return max(1, (int) ceil($this->total / $this->perPage));
    }
}
