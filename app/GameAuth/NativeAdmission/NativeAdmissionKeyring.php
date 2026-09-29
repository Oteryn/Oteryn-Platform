<?php

namespace App\GameAuth\NativeAdmission;

use App\GameAuth\NativeEvidence\NativeEvidenceContract;
use InvalidArgumentException;
use SensitiveParameter;

/**
 * Ed25519 signing keys of the native admission issuer (contract §9.1): the current key and
 * at most one retiring key kept for re-signing (§6.3, §9.3). Each key is read from an
 * externally injected secret file path; the key value itself is never configuration.
 */
final class NativeAdmissionKeyring
{
    private const MAX_KEY_FILE_BYTES = 128;

    /** @var array<string, non-empty-string>|null kid => 64-byte libsodium secret key */
    private ?array $secretKeys = null;

    private ?string $currentKeyId = null;

    public function currentKeyId(): string
    {
        $this->load();

        return (string) $this->currentKeyId;
    }

    public function has(string $keyId): bool
    {
        $this->load();

        return isset($this->secretKeys[$keyId]);
    }

    /** Raw 32-byte public key derived from the loaded private key. */
    public function publicKey(string $keyId): string
    {
        return sodium_crypto_sign_publickey_from_secretkey($this->secretKey($keyId));
    }

    /** Detached pure Ed25519 (RFC 8032) signature over the exact bytes. */
    public function sign(string $keyId, string $message): string
    {
        return sodium_crypto_sign_detached($message, $this->secretKey($keyId));
    }

    /** @return non-empty-string */
    private function secretKey(string $keyId): string
    {
        $this->load();
        if (! isset($this->secretKeys[$keyId])) {
            throw new NativeAdmissionUnavailable('Native admission signing key id is not loaded.');
        }

        return $this->secretKeys[$keyId];
    }

    private function load(): void
    {
        if ($this->secretKeys !== null) {
            return;
        }

        $currentId = $this->keyId(config('game-auth.native_admission.signing_key_id'), 'current');
        $keys = [$currentId => $this->readKeyFile(config('game-auth.native_admission.signing_key_file'), 'current')];

        $retiringFile = config('game-auth.native_admission.retiring_signing_key_file');
        $retiringId = config('game-auth.native_admission.retiring_signing_key_id');
        if (($retiringFile ?? '') !== '' || ($retiringId ?? '') !== '') {
            $retiringId = $this->keyId($retiringId, 'retiring');
            if ($retiringId === $currentId) {
                throw new NativeAdmissionUnavailable('Retiring native admission key id must differ from the current key id.');
            }
            $keys[$retiringId] = $this->readKeyFile($retiringFile, 'retiring');
        }

        $this->currentKeyId = $currentId;
        $this->secretKeys = $keys;
    }

    private function keyId(mixed $value, string $role): string
    {
        if (! is_string($value)) {
            throw new NativeAdmissionUnavailable("Native admission {$role} key id is not configured.");
        }
        try {
            NativeEvidenceContract::assertKeyId($value);
        } catch (InvalidArgumentException) {
            throw new NativeAdmissionUnavailable("Native admission {$role} key id is invalid.");
        }

        return $value;
    }

    /** @return non-empty-string */
    private function readKeyFile(mixed $path, string $role): string
    {
        if (! is_string($path) || $path === '' || ! str_starts_with($path, '/')) {
            throw new NativeAdmissionUnavailable("Native admission {$role} key file must be an absolute path.");
        }
        clearstatcache(true, $path);
        if (is_link($path) || ! is_file($path) || ! is_readable($path)) {
            throw new NativeAdmissionUnavailable("Native admission {$role} key file is not a readable regular file.");
        }
        $mode = fileperms($path);
        if ($mode === false || ($mode & 0o077) !== 0) {
            throw new NativeAdmissionUnavailable("Native admission {$role} key file must not be accessible to group or others.");
        }
        $contents = file_get_contents($path, false, null, 0, self::MAX_KEY_FILE_BYTES + 1);
        if (! is_string($contents) || strlen($contents) > self::MAX_KEY_FILE_BYTES) {
            throw new NativeAdmissionUnavailable("Native admission {$role} key file is unreadable or oversized.");
        }

        return $this->secretKeyFromSeed(rtrim($contents, "\n"), $role);
    }

    /** @return non-empty-string */
    private function secretKeyFromSeed(#[SensitiveParameter] string $encoded, string $role): string
    {
        $seed = preg_match('/^[A-Za-z0-9_-]{43}$/', $encoded) === 1
            ? base64_decode(strtr($encoded.'=', '-_', '+/'), true)
            : false;
        if (! is_string($seed) || strlen($seed) !== SODIUM_CRYPTO_SIGN_SEEDBYTES
            || rtrim(strtr(base64_encode($seed), '+/', '-_'), '=') !== $encoded) {
            throw new NativeAdmissionUnavailable("Native admission {$role} key file must hold one canonical base64url 32-byte Ed25519 seed.");
        }

        $secretKey = sodium_crypto_sign_secretkey(sodium_crypto_sign_seed_keypair($seed));
        if (strlen($secretKey) !== SODIUM_CRYPTO_SIGN_SECRETKEYBYTES) {
            throw new NativeAdmissionUnavailable("Native admission {$role} key could not be derived.");
        }

        return $secretKey;
    }
}
