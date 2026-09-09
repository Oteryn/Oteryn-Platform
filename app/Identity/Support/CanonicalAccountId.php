<?php

namespace App\Identity\Support;

use LogicException;

final class CanonicalAccountId
{
    private const PATTERN = '/^[0-9a-f]{8}-[0-9a-f]{4}-7[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/';

    public static function generate(): string
    {
        $milliseconds = (int) floor(microtime(true) * 1000);
        $timeHex = str_pad(dechex($milliseconds), 12, '0', STR_PAD_LEFT);
        $bytes = hex2bin($timeHex).random_bytes(10);

        if (! is_string($bytes) || strlen($bytes) !== 16) {
            throw new LogicException('Unable to generate canonical AccountId bytes.');
        }

        $bytes[6] = chr((ord($bytes[6]) & 0x0F) | 0x70);
        $bytes[8] = chr((ord($bytes[8]) & 0x3F) | 0x80);
        $hex = bin2hex($bytes);

        return substr($hex, 0, 8).'-'
            .substr($hex, 8, 4).'-'
            .substr($hex, 12, 4).'-'
            .substr($hex, 16, 4).'-'
            .substr($hex, 20, 12);
    }

    public static function isValid(mixed $value): bool
    {
        return is_string($value) && preg_match(self::PATTERN, $value) === 1;
    }
}
