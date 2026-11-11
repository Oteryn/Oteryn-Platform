<?php

namespace App\GameAuth\NativeAdmission;

use LogicException;
use SensitiveParameter;

/**
 * Holder of the loaded Ed25519 secret keys. NativeAdmissionKeyring reaches it only through a
 * closure, so neither var_export() nor an array cast of the keyring reaches this object; debug
 * output of the holder is redacted and serialization is refused.
 *
 * @internal
 */
final readonly class NativeAdmissionSecretKeys
{
    /** @param array<string, non-empty-string> $keys kid => 64-byte libsodium secret key */
    public function __construct(#[SensitiveParameter] private array $keys) {}

    /** @return non-empty-string|null */
    public function get(string $keyId): ?string
    {
        return $this->keys[$keyId] ?? null;
    }

    /** @return array<string, mixed> */
    public function __debugInfo(): array
    {
        return ['keyIds' => array_keys($this->keys)];
    }

    /** @return array<string, mixed> */
    public function __serialize(): array
    {
        throw new LogicException('Native admission secret keys cannot be serialized.');
    }

    /** @param array<string, mixed> $data */
    public function __unserialize(array $data): void
    {
        throw new LogicException('Native admission secret keys cannot be unserialized.');
    }
}
