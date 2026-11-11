<?php

namespace App\Casts;

final readonly class CastReservation
{
    public function __construct(
        public string $reservationId,
        public string $accountId,
        public string $castId,
        public string $nodeId,
        public int $epoch,
        public int $accessRevision,
        public int $expiresAt,
        public bool $chatSelected = false,
        #[\SensitiveParameter] public ?string $ticket = null,
    ) {}
}
