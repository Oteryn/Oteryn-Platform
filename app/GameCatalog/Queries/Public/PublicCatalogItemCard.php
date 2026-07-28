<?php

namespace App\GameCatalog\Queries\Public;

final readonly class PublicCatalogItemCard
{
    /** @param list<string> $vocations */
    public function __construct(
        public string $slug,
        public string $name,
        public ?string $summary,
        public string $category,
        public ?string $weaponType,
        public ?int $attack,
        public ?int $defense,
        public ?int $armor,
        public ?int $minimumLevel,
        public array $vocations,
        public ?string $imageKey,
    ) {}
}
