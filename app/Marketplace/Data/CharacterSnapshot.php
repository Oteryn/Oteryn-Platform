<?php

namespace App\Marketplace\Data;

final readonly class CharacterSnapshot
{
    /**
     * @param array<string, int|string|null> $publicData
     */
    public function __construct(
        public int $playerId,
        public int $accountId,
        public string $name,
        public int $deletion,
        public array $publicData,
    ) {}
}
