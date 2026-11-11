<?php

namespace App\Marketplace\Data;

final readonly class CharacterTransferResult
{
    public const TRANSFERRED = 'transferred';

    public const ALREADY_TRANSFERRED = 'already_transferred';

    public function __construct(
        public string $status,
        public int $playerId,
        public int $accountId,
    ) {}

    public function changed(): bool
    {
        return $this->status === self::TRANSFERRED;
    }
}
