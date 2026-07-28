<?php

namespace App\Marketplace\Contracts;

use App\Marketplace\Data\CharacterOwnershipState;
use App\Marketplace\Data\CharacterSnapshot;
use App\Marketplace\Data\CharacterTransferResult;

interface CanaryCharacterTransferGateway
{
    public function snapshotOwnedCharacter(int $sourceAccountId, int $playerId): CharacterSnapshot;

    public function transfer(
        int $playerId,
        int $expectedSourceAccountId,
        int $targetAccountId,
        bool $enforceTargetCharacterLimit,
    ): CharacterTransferResult;

    public function ownershipState(int $playerId): CharacterOwnershipState;
}
