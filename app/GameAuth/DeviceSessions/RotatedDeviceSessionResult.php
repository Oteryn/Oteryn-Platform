<?php

namespace App\GameAuth\DeviceSessions;

use LogicException;

/** @template TResult */
final readonly class RotatedDeviceSessionResult
{
    /** @param TResult $result */
    public function __construct(
        private IssuedDeviceSessionCredential $credential,
        private mixed $result,
    ) {}

    public function credential(): IssuedDeviceSessionCredential
    {
        return $this->credential;
    }

    /** @return TResult */
    public function result(): mixed
    {
        return $this->result;
    }

    /** @return array<string, string> */
    public function __debugInfo(): array
    {
        return ['authorization' => '[REDACTED]'];
    }

    /** @return never */
    public function __serialize(): array
    {
        throw new LogicException('Remembered device authorization responses cannot be serialized.');
    }
}
