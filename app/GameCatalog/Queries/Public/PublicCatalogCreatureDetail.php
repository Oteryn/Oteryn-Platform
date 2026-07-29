<?php

namespace App\GameCatalog\Queries\Public;

final readonly class PublicCatalogCreatureDetail
{
    /** @param list<PublicCatalogLootEntry> $loot */
    public function __construct(
        public string $slug,
        public string $name,
        public ?string $description,
        public int $health,
        public int $maxHealth,
        public int $experience,
        public int $speed,
        public int $armor,
        public int $defense,
        public ?string $mitigation,
        public bool $boss,
        public bool $rewardBoss,
        public ?string $bestiaryClass,
        public ?string $bestiaryRace,
        public ?int $bestiaryOccurrence,
        public ?int $bestiaryToKill,
        public ?int $charmPoints,
        public ?int $lookType,
        public array $loot,
    ) {}
}
