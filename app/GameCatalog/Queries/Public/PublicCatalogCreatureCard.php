<?php

namespace App\GameCatalog\Queries\Public;

final readonly class PublicCatalogCreatureCard
{
    public function __construct(
        public string $slug,
        public string $name,
        public ?string $summary,
        public int $health,
        public int $experience,
        public ?string $bestiaryClass,
        public bool $boss,
        public ?int $lookType,
    ) {}
}
