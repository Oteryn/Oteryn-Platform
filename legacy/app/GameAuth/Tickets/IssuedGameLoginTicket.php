<?php

namespace App\GameAuth\Tickets;

use Illuminate\Support\Carbon;

final readonly class IssuedGameLoginTicket
{
    public function __construct(
        public string $ticket,
        public Carbon $expiresAt,
    ) {}
}
