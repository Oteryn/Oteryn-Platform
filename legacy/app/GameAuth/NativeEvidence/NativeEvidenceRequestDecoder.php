<?php

namespace App\GameAuth\NativeEvidence;

use InvalidArgumentException;
use JsonException;

final class NativeEvidenceRequestDecoder
{
    /**
     * @return array<string, int|string>
     */
    public function decode(string $raw): array
    {
        if ($raw === '' || strlen($raw) > NativeEvidenceContract::MAX_REQUEST_BYTES) {
            throw new InvalidArgumentException('Native evidence request size is invalid.');
        }

        $this->assertNoDuplicateKeys($raw);

        try {
            $decoded = json_decode($raw, true, 3, JSON_THROW_ON_ERROR);
        } catch (JsonException $exception) {
            throw new InvalidArgumentException('Native evidence request JSON is invalid.', 0, $exception);
        }

        if (! is_array($decoded) || array_is_list($decoded) || count($decoded) > NativeEvidenceContract::MAX_MEMBERS) {
            throw new InvalidArgumentException('Native evidence request root is invalid.');
        }

        foreach ($decoded as $name => $value) {
            if (! is_string($name) || $name === '' || strlen($name) > NativeEvidenceContract::MAX_NAME_BYTES || preg_match('/^[\x20-\x7e]+$/', $name) !== 1) {
                throw new InvalidArgumentException('Native evidence member name is invalid.');
            }
            if (is_array($value) || is_object($value)) {
                throw new InvalidArgumentException('Nested native evidence values are forbidden.');
            }
        }

        $version = $decoded['version'] ?? null;
        $operation = $decoded['operation'] ?? null;
        if (! is_int($version) || ! is_string($operation) || NativeEvidenceContract::versionFor($operation) !== $version) {
            throw new InvalidArgumentException('Native evidence operation/version is invalid.');
        }

        $expected = match ($operation) {
            NativeEvidenceContract::FRESH_ACCOUNT, NativeEvidenceContract::RECOVERY_ACCOUNT => ['version', 'operation', 'account_id', 'purpose', 'scope'],
            NativeEvidenceContract::FRESH_TRUST, NativeEvidenceContract::RECOVERY_TRUST => ['version', 'operation', 'issuer', 'profile', 'key_purpose', 'key_id'],
            default => throw new InvalidArgumentException('Native evidence operation is unsupported.'),
        };
        $keys = array_keys($decoded);
        sort($keys);
        $sortedExpected = $expected;
        sort($sortedExpected);
        if ($keys !== $sortedExpected) {
            throw new InvalidArgumentException('Native evidence request fields are invalid.');
        }

        foreach ($expected as $field) {
            if (in_array($field, ['version', 'operation'], true)) {
                continue;
            }
            $value = $decoded[$field];
            if (! is_string($value)) {
                throw new InvalidArgumentException('Native evidence request scalar type is invalid.');
            }
            NativeEvidenceContract::assertBinding($value);
        }

        /** @var array<string, int|string> $decoded */
        $accountId = $decoded['account_id'] ?? null;
        if ($accountId !== null) {
            if (! is_string($accountId)) {
                throw new InvalidArgumentException('Native evidence AccountId type is invalid.');
            }
            NativeEvidenceContract::assertAccountId($accountId);
        }

        $keyId = $decoded['key_id'] ?? null;
        if ($keyId !== null) {
            if (! is_string($keyId)) {
                throw new InvalidArgumentException('Native evidence key id type is invalid.');
            }
            NativeEvidenceContract::assertKeyId($keyId);
        }

        return $decoded;
    }

    private function assertNoDuplicateKeys(string $raw): void
    {
        preg_match_all('/"((?:[^"\\\\]|\\\\.)*)"\s*:/s', $raw, $matches);
        $seen = [];
        foreach ($matches[1] as $encodedName) {
            try {
                $name = json_decode('"'.$encodedName.'"', true, 2, JSON_THROW_ON_ERROR);
            } catch (JsonException $exception) {
                throw new InvalidArgumentException('Native evidence member encoding is invalid.', 0, $exception);
            }
            if (! is_string($name) || isset($seen[$name])) {
                throw new InvalidArgumentException('Duplicate native evidence member is forbidden.');
            }
            $seen[$name] = true;
        }
    }
}
