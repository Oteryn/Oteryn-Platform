<?php

namespace App\GameAuth\CharacterBootstrapIntent;

use InvalidArgumentException;

final class CharacterBootstrapIntentContract
{
    public const VERSION = 1;

    public const VARIANT = 'OPERATOR_CONTROL_PLANE_BOOTSTRAP';

    public const OPERATION = 'INITIAL_CHARACTER_BOOTSTRAP';

    public const AUDIENCE = 'OTERYN_GAME_CHARACTER_AUTHORITY';

    public const MAX_REQUEST_BYTES = 512;

    public static function assertUuid(string $value, string $field): void
    {
        if (preg_match('/^[0-9a-f]{8}-[0-9a-f]{4}-[1-8][0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/', $value) !== 1) {
            throw new InvalidArgumentException("{$field} must be a canonical lower-case UUID.");
        }
    }

    public static function assertBinding(string $value, string $field, int $maximum = 128): void
    {
        if ($value === '' || strlen($value) > $maximum || preg_match('/^[A-Za-z0-9._:-]+$/', $value) !== 1) {
            throw new InvalidArgumentException("{$field} is invalid.");
        }
    }

    public static function assertPositiveRevision(string $value, string $field): void
    {
        if (preg_match('/^[1-9][0-9]{0,18}$/', $value) !== 1) {
            throw new InvalidArgumentException("{$field} must be a positive revision.");
        }
    }
}
