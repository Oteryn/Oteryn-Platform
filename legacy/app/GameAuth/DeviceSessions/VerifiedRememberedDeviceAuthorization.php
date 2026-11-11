<?php

namespace App\GameAuth\DeviceSessions;

use App\Identity\Models\Identity;

/**
 * Transaction-scoped facts, not a bearer or an authorization reusable after the callback.
 * Construction and consumption belong to trusted Platform application services only.
 */
final readonly class VerifiedRememberedDeviceAuthorization
{
    public function __construct(
        private Identity $lockedIdentity,
        public int $identityId,
        public string $accountId,
        public string $oauthClientId,
        public int $gameAuthGeneration,
        public int $nativeSecurityGeneration,
        public string $familyId,
        public DeviceSessionPurpose $purpose,
    ) {}

    public function identity(): Identity
    {
        return $this->lockedIdentity;
    }
}
