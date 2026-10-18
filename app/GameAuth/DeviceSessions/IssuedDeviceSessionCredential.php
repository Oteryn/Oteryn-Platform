<?php

namespace App\GameAuth\DeviceSessions;

use Carbon\CarbonImmutable;
use LogicException;
use SensitiveParameter;

final readonly class IssuedDeviceSessionCredential
{
    public function __construct(
        #[SensitiveParameter] private string $secret,
        public string $familyId,
        public CarbonImmutable $absoluteExpiresAt,
        public CarbonImmutable $idleExpiresAt,
    ) {}

    /** Only the response adapter may export the credential to its TLS response body. */
    public function secret(): string
    {
        return $this->secret;
    }

    /** @return array<string, string> */
    public function __debugInfo(): array
    {
        return ['credential' => '[REDACTED]'];
    }

    /** @return never */
    public function __serialize(): array
    {
        throw new LogicException('Remembered device plaintext credentials cannot be serialized.');
    }
}
