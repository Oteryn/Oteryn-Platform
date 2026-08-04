<?php

namespace App\GameAuth\Worlds;

final readonly class GameWorldRoute
{
    public function __construct(
        public int $id,
        public string $slug,
        public string $name,
        public string $region,
        public string $host,
        public int $port,
        public ?GameWorldProtocolPolicy $gameplayPolicy = null,
    ) {}
}
