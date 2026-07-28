<?php

namespace App\Marketplace\Data;

final readonly class CharacterOwnershipState
{
    public function __construct(
        public int $playerId,
        public int $accountId,
        public int $deletion,
        public bool $hasClusterSession,
    ) {}
}
