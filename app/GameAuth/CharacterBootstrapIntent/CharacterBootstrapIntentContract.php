<?php

namespace App\GameAuth\CharacterBootstrapIntent;

use App\Identity\Support\CanonicalAccountId;
use InvalidArgumentException;

final class CharacterBootstrapIntentContract
{
    public const CONTRACT_VERSION = 1;

    public const VARIANT = 'OPERATOR_CONTROL_PLANE_BOOTSTRAP';

    public const ISSUER_AUTHORITY = 'OTERYN_PLATFORM_CHARACTER_AUTHORITY';

    public const OPERATION = 'INITIAL_CHARACTER_BOOTSTRAP';

    public const AUDIENCE = 'OTERYN_GAME_CHARACTER_AUTHORITY';

    public const MAX_REQUEST_BYTES = 256;

    public const MAX_RESPONSE_BYTES = 4096;

    public const MAX_REVISION_BYTES = 128;

    private const UUID_PATTERN = '/\A[0-9a-f]{8}-[0-9a-f]{4}-[1-8][0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}\z/D';

    private const REVISION_PATTERN = '/\A[A-Za-z0-9][A-Za-z0-9._:-]{0,127}\z/D';

    public static function assertUuid(string $value, string $label): void
    {
        if (preg_match(self::UUID_PATTERN, $value) !== 1) {
            throw new InvalidArgumentException("{$label} must be a canonical lower-case UUID.");
        }
    }

    public static function assertAccountId(string $value): void
    {
        if (! CanonicalAccountId::isValid($value)) {
            throw new InvalidArgumentException('Persisted AccountId is not a canonical Platform AccountId.');
        }
    }

    public static function assertRevision(string $value, string $label): void
    {
        if (strlen($value) > self::MAX_REVISION_BYTES || preg_match(self::REVISION_PATTERN, $value) !== 1) {
            throw new InvalidArgumentException("{$label} is invalid.");
        }
    }
}
