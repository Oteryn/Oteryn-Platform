<?php

namespace App\GameAuth\NativeEvidence;

use App\Identity\Support\CanonicalAccountId;
use InvalidArgumentException;

final class NativeEvidenceContract
{
    public const FRESH_ACCOUNT = 'ReadAccountSecurityV1';

    public const FRESH_TRUST = 'ReadFreshSigningTrustV1';

    public const RECOVERY_ACCOUNT = 'ReadRecoveryAccountSecurityV2';

    public const RECOVERY_TRUST = 'ReadRecoverySigningTrustV2';

    public const FRESH_ISSUER = 'urn:oteryn:platform:game-admission';

    public const FRESH_PROFILE = 'oteryn-pre-admission-v1';

    public const RECOVERY_ISSUER = 'urn:oteryn:platform:game-recovery';

    public const RECOVERY_PROFILE = 'oteryn-reauth-recovery-v1';

    public const RECOVERY_PURPOSE = 'platform_security';

    public const RECOVERY_SCOPE = 'existing_actor_recovery';

    public const RECOVERY_KEY_PURPOSE = 'existing_actor_recovery';

    public const MAX_REQUEST_BYTES = 1024;

    public const MAX_RESPONSE_BYTES = 8192;

    public const MAX_MEMBERS = 16;

    public const MAX_NAME_BYTES = 64;

    public const MAX_STRING_BYTES = 256;

    /** @return array{version:int,operation:string,result:string} */
    public static function failure(int $version, string $operation, string $result): array
    {
        if (! in_array($result, ['not_found', 'unavailable', 'unauthorized', 'unsupported'], true)) {
            throw new InvalidArgumentException('Unsupported native evidence failure result.');
        }

        return ['version' => $version, 'operation' => $operation, 'result' => $result];
    }

    public static function assertAccountId(string $accountId): void
    {
        if (! CanonicalAccountId::isValid($accountId)) {
            throw new InvalidArgumentException('Invalid canonical AccountId.');
        }
    }

    public static function assertBinding(string $value): void
    {
        if ($value === '' || strlen($value) > self::MAX_STRING_BYTES || preg_match('/^[\x20-\x7e]+$/', $value) !== 1) {
            throw new InvalidArgumentException('Invalid native evidence binding.');
        }
    }

    public static function assertKeyId(string $keyId): void
    {
        if (strlen($keyId) < 1 || strlen($keyId) > 64 || preg_match('/^[A-Za-z0-9._-]+$/', $keyId) !== 1) {
            throw new InvalidArgumentException('Invalid native evidence key id.');
        }
    }

    public static function encodePublicKey(string $bytes): string
    {
        if (strlen($bytes) !== 32) {
            throw new InvalidArgumentException('Native signing public key must be exactly 32 bytes.');
        }

        return rtrim(strtr(base64_encode($bytes), '+/', '-_'), '=');
    }

    public static function assertEncodedPublicKey(string $encoded): void
    {
        if (strlen($encoded) !== 43 || preg_match('/^[A-Za-z0-9_-]{43}$/', $encoded) !== 1) {
            throw new NativeEvidenceUnavailable('Stored native signing public key is invalid.');
        }

        $decoded = base64_decode(strtr($encoded.'=', '-_', '+/'), true);
        if (! is_string($decoded) || strlen($decoded) !== 32 || self::encodePublicKey($decoded) !== $encoded) {
            throw new NativeEvidenceUnavailable('Stored native signing public key is non-canonical.');
        }
    }

    public static function versionFor(string $operation): int
    {
        return match ($operation) {
            self::FRESH_ACCOUNT, self::FRESH_TRUST => 1,
            self::RECOVERY_ACCOUNT, self::RECOVERY_TRUST => 2,
            default => throw new InvalidArgumentException('Unsupported native evidence operation.'),
        };
    }
}
