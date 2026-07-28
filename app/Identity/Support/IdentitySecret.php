<?php

namespace App\Identity\Support;

use LogicException;

final class IdentitySecret
{
    public static function generate(int $bytes = 32): string
    {
        if ($bytes < 16 || $bytes > 64) {
            throw new LogicException('Identity secret length is outside the supported boundary.');
        }

        return rtrim(strtr(base64_encode(random_bytes($bytes)), '+/', '-_'), '=');
    }

    public static function hash(string $secret): string
    {
        return hash('sha256', $secret);
    }

    public static function keyedHash(string $secret): string
    {
        $key = config('app.key');
        if (! is_string($key) || $key === '') {
            throw new LogicException('The application key is required for identity secret verification.');
        }

        return hash_hmac('sha256', $secret, $key);
    }
}
