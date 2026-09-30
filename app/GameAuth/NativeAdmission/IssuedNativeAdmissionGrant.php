<?php

namespace App\GameAuth\NativeAdmission;

use SensitiveParameter;

/**
 * One signed grant. The signing input and key id are what the attempt record keeps for
 * deterministic re-signing (contract §6.1); the token is returned to the client only.
 */
final readonly class IssuedNativeAdmissionGrant
{
    public function __construct(
        public string $keyId,
        #[SensitiveParameter] public string $signingInput,
        #[SensitiveParameter] public string $token,
        public int $issuedAt,
        public int $expiresAt,
    ) {}

    public function __debugInfo(): array
    {
        return ['keyId' => $this->keyId, 'issuedAt' => $this->issuedAt, 'expiresAt' => $this->expiresAt];
    }
}
