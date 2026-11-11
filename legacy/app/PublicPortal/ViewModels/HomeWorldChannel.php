<?php

namespace App\PublicPortal\ViewModels;

final readonly class HomeWorldChannel
{
    public function __construct(
        public int $id,
        public string $name,
        public string $pvpType,
        public int $maxPlayers,
        public bool $maintenance,
        public ?string $maintenanceMessage,
        public ?string $runtimeStatus,
        public ?int $playersOnline,
    ) {}
}
