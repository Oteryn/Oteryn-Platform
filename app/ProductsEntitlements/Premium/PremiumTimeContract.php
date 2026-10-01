<?php

namespace App\ProductsEntitlements\Premium;

use InvalidArgumentException;

/**
 * Fixed values of oteryn.premium_time version 1 (docs/contracts/OTERYN_V2_PREMIUM_TIME_SNAPSHOT_CONTRACT.md).
 * These are product/version policy, not deployment configuration: changing one requires a new product version.
 */
final class PremiumTimeContract
{
    public const PRODUCT_ID = 'oteryn.premium_time';

    public const PRODUCT_VERSION = 1;

    public const ISSUANCE_OPERATOR_GRANT = 'OPERATOR_GRANT';

    public const EVENT_GRANT = 'GRANT';

    public const EVENT_REVOKE = 'REVOKE';

    public const STORED_ACTIVE = 'ACTIVE';

    public const STORED_REVOKED = 'REVOKED';

    public const STATE_ACTIVE = 'ACTIVE';

    public const STATE_NOT_YET_EFFECTIVE = 'NOT_YET_EFFECTIVE';

    public const STATE_EXPIRED = 'EXPIRED';

    public const STATE_REVOKED = 'REVOKED';

    public const STATE_NONE = 'NONE';

    public const MIN_GRANT_DAYS = 1;

    public const MAX_GRANT_DAYS = 366;

    public const MAX_HORIZON_SECONDS = 3660 * 86400;

    public const MAX_AUTHORITY_LEASE_SECONDS = 3600;

    public const MAX_CLOCK_SKEW_SECONDS = 5;

    public const SNAPSHOT_SCHEMA = 'oteryn.premium_snapshot.v1';

    public const PRODUCER_PROFILE = 'oteryn.entitlement.profile_b.v1';

    public const REQUEST_SCHEMA = 'oteryn.premium_snapshot_request.v1';

    public const MAX_REQUEST_BYTES = 256;

    public const MAX_RESPONSE_BYTES = 1024;

    /** 2^53 - 1: every JSON parser reads revisions exactly (contract 5.1). */
    public const MAX_REVISION = 9007199254740991;

    private const UUID = '/^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/D';

    public static function isUuid(mixed $value): bool
    {
        return is_string($value) && preg_match(self::UUID, $value) === 1;
    }

    public static function assertUuid(mixed $value, string $field): string
    {
        if (! is_string($value) || ! self::isUuid($value)) {
            throw new InvalidArgumentException("Premium time {$field} must be a canonical lower-case UUID.");
        }

        return $value;
    }

    /** A non-negative database integer (drivers may return numeric strings), or null. */
    public static function databaseInteger(mixed $value): ?int
    {
        if (is_int($value)) {
            return $value >= 0 ? $value : null;
        }

        return is_string($value) && preg_match('/^(0|[1-9][0-9]{0,15})$/D', $value) === 1 ? (int) $value : null;
    }

    public static function formatTime(int $unixSeconds): string
    {
        return gmdate('Y-m-d\TH:i:s\Z', $unixSeconds);
    }
}
