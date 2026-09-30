<?php

namespace App\GameAuth\Tickets;

use SensitiveParameter;

final class GameLoginTicketSecrets
{
    private const ENTROPY_BYTES = 32;

    public function generate(): string
    {
        return rtrim(strtr(base64_encode(random_bytes(self::ENTROPY_BYTES)), '+/', '-_'), '=');
    }

    public function hash(#[SensitiveParameter] string $ticket): string
    {
        return hash('sha256', $ticket);
    }
}
