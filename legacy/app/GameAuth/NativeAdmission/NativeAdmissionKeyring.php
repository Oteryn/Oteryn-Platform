<?php

namespace App\GameAuth\NativeAdmission;

use App\GameAuth\NativeEvidence\NativeEvidenceContract;
use Closure;
use InvalidArgumentException;
use LogicException;
use SensitiveParameter;

/**
 * Ed25519 signing keys of the native admission issuer (contract §9.1): the current key and
 * at most one retiring key kept for re-signing (§6.3, §9.3). Each key is read from an
 * externally injected secret file path; the key value itself is never configuration.
 *
 * The path must name a regular, non-symlink file owned by the process user with no group or other
 * permission bits. A standard Kubernetes secret volume exposes each key as a symlink, so it is
 * refused (fail closed); mount the key at a non-symlink path instead (production custody is U9).
 * The owner check needs the posix extension: without posix_geteuid() no key is loaded (fail closed).
 *
 * The secret keys live in a NativeAdmissionSecretKeys holder captured by a closure, so
 * var_dump(), print_r(), var_export(), an array cast and serialization of the keyring never show
 * key bytes. Residual risk: code that uses Reflection to read the closure's static variables and
 * then exports or array-casts the holder can still reach the bytes; that requires code execution
 * inside the issuer process, which already implies access to the key file.
 */
final class NativeAdmissionKeyring
{
    private const MAX_KEY_FILE_BYTES = 128;

    private const KEY_ID = '/\A[A-Za-z0-9._-]{1,64}\z/';

    private const SEED = '/\A[A-Za-z0-9_-]{43}\z/';

    private const FILE_TYPE_MASK = 0o170000;

    private const REGULAR_FILE = 0o100000;

    /** @var (Closure(string): (string|null))|null kid => 64-byte libsodium secret key */
    private ?Closure $secretKeyOf = null;

    /** @var list<string> */
    private array $loadedKeyIds = [];

    private ?string $currentKeyId = null;

    /** @return array<string, mixed> */
    public function __debugInfo(): array
    {
        return [
            'currentKeyId' => $this->currentKeyId,
            'loadedKeyIds' => $this->loadedKeyIds,
        ];
    }

    /** @return array<string, mixed> */
    public function __serialize(): array
    {
        throw new LogicException('The native admission keyring holds secret key material and cannot be serialized.');
    }

    /** @param array<string, mixed> $data */
    public function __unserialize(array $data): void
    {
        throw new LogicException('The native admission keyring cannot be unserialized.');
    }

    public function currentKeyId(): string
    {
        $this->load();

        return (string) $this->currentKeyId;
    }

    public function has(string $keyId): bool
    {
        $this->load();

        return in_array($keyId, $this->loadedKeyIds, true);
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
        $secretKey = $this->secretKeyOf instanceof Closure ? ($this->secretKeyOf)($keyId) : null;
        if (! is_string($secretKey) || $secretKey === '') {
            throw new NativeAdmissionUnavailable('Native admission signing key id is not loaded.');
        }

        return $secretKey;
    }

    private function load(): void
    {
        if ($this->secretKeyOf instanceof Closure) {
            return;
        }

        $currentId = $this->keyId(config('game-auth.native_admission.signing_key_id'), 'current');
        $keys = [$currentId => $this->readKeyFile(config('game-auth.native_admission.signing_key_file'), 'current')];
        $keyIds = [$currentId];

        $retiringFile = config('game-auth.native_admission.retiring_signing_key_file');
        $retiringId = config('game-auth.native_admission.retiring_signing_key_id');
        if (($retiringFile ?? '') !== '' || ($retiringId ?? '') !== '') {
            $retiringId = $this->keyId($retiringId, 'retiring');
            if ($retiringId === $currentId) {
                throw new NativeAdmissionUnavailable('Retiring native admission key id must differ from the current key id.');
            }
            $keys[$retiringId] = $this->readKeyFile($retiringFile, 'retiring');
            $keyIds[] = $retiringId;
        }

        $holder = new NativeAdmissionSecretKeys($keys);
        $this->currentKeyId = $currentId;
        $this->loadedKeyIds = $keyIds;
        $this->secretKeyOf = static fn (string $keyId): ?string => $holder->get($keyId);
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
        if (preg_match(self::KEY_ID, $value) !== 1) {
            throw new NativeAdmissionUnavailable("Native admission {$role} key id is invalid.");
        }

        return $value;
    }

    /**
     * Opens the key file once and validates the opened handle, so the checked file is the read file.
     * lstat() refuses a symlink at the path; the handle's device and inode must equal what lstat()
     * saw, so a path swapped to a symlink between the two calls is refused as well.
     *
     * @return non-empty-string
     */
    private function readKeyFile(mixed $path, string $role): string
    {
        if (! is_string($path) || $path === '' || ! str_starts_with($path, '/')) {
            throw new NativeAdmissionUnavailable("Native admission {$role} key file must be an absolute path.");
        }
        clearstatcache(true, $path);
        $link = @lstat($path);
        if ($link === false || ($link['mode'] & self::FILE_TYPE_MASK) !== self::REGULAR_FILE) {
            throw new NativeAdmissionUnavailable("Native admission {$role} key file is not a readable regular file.");
        }

        $handle = @fopen($path, 'rb');
        if ($handle === false) {
            throw new NativeAdmissionUnavailable("Native admission {$role} key file is not a readable regular file.");
        }
        try {
            $stat = fstat($handle);
            if ($stat === false
                || $stat['dev'] !== $link['dev']
                || $stat['ino'] !== $link['ino']
                || ($stat['mode'] & self::FILE_TYPE_MASK) !== self::REGULAR_FILE) {
                throw new NativeAdmissionUnavailable("Native admission {$role} key file is not a readable regular file.");
            }
            if (($stat['mode'] & 0o077) !== 0) {
                throw new NativeAdmissionUnavailable("Native admission {$role} key file must not be accessible to group or others.");
            }
            if (! function_exists('posix_geteuid')) {
                throw new NativeAdmissionUnavailable("Native admission {$role} key file ownership cannot be verified without the posix extension.");
            }
            if ($stat['uid'] !== posix_geteuid()) {
                throw new NativeAdmissionUnavailable("Native admission {$role} key file must be owned by the issuer process user.");
            }
            if ($stat['size'] > self::MAX_KEY_FILE_BYTES) {
                throw new NativeAdmissionUnavailable("Native admission {$role} key file is unreadable or oversized.");
            }
            $contents = stream_get_contents($handle, self::MAX_KEY_FILE_BYTES + 1);
        } finally {
            fclose($handle);
        }
        if (! is_string($contents) || strlen($contents) > self::MAX_KEY_FILE_BYTES) {
            throw new NativeAdmissionUnavailable("Native admission {$role} key file is unreadable or oversized.");
        }

        return $this->secretKeyFromSeed(
            str_ends_with($contents, "\n") ? substr($contents, 0, -1) : $contents,
            $role,
        );
    }

    /** @return non-empty-string */
    private function secretKeyFromSeed(#[SensitiveParameter] string $encoded, string $role): string
    {
        $seed = preg_match(self::SEED, $encoded) === 1
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
