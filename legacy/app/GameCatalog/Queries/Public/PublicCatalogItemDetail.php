<?php

namespace App\GameCatalog\Queries\Public;

final readonly class PublicCatalogItemDetail
{
    /**
     * @param  list<string>  $vocations
     * @param  list<PublicCatalogLootEntry>  $sources
     */
    public function __construct(
        public string $slug,
        public string $name,
        public ?string $description,
        public string $category,
        public ?string $weaponType,
        public int $serverId,
        public ?int $clientId,
        public ?int $attack,
        public ?int $defense,
        public ?int $extraDefense,
        public ?int $armor,
        public ?int $range,
        public ?int $weight,
        public ?int $minimumLevel,
        public array $vocations,
        public ?int $imbuementSlots,
        public ?string $elementType,
        public ?int $elementValue,
        public bool $stackable,
        public bool $pickupable,
        public ?string $imageKey,
        public array $sources,
    ) {}
}
