<?php

namespace App\GameAuth\DeviceSessions;

use SensitiveParameter;

final class DeviceSessionSecrets
{
    public function generate(): string
    {
        return 'otd1.'.rtrim(strtr(base64_encode(random_bytes(32)), '+/', '-_'), '=');
    }

    public function hash(#[SensitiveParameter] string $secret): string
    {
        return hash('sha256', $secret);
    }

    public function valid(#[SensitiveParameter] string $secret): bool
    {
        return preg_match('/\Aotd1\.[A-Za-z0-9_-]{43}\z/', $secret) === 1;
    }
}
