<?php

namespace App\ProductsEntitlements\Premium;

use InvalidArgumentException;
use JsonException;

/** Strict decoder for `oteryn.premium_snapshot_request.v1` (contract 4.4). */
final class PremiumSnapshotRequestDecoder
{
    /** @return array{schema:string,account_id:string,nonce:string} */
    public function decode(string $raw): array
    {
        if ($raw === '' || strlen($raw) > PremiumTimeContract::MAX_REQUEST_BYTES) {
            throw new InvalidArgumentException('Premium snapshot request size is invalid.');
        }

        $this->assertNoDuplicateKeys($raw);

        try {
            $decoded = json_decode($raw, true, 2, JSON_THROW_ON_ERROR);
        } catch (JsonException $exception) {
            throw new InvalidArgumentException('Premium snapshot request JSON is invalid.', 0, $exception);
        }

        if (! is_array($decoded) || array_is_list($decoded)) {
            throw new InvalidArgumentException('Premium snapshot request members are invalid.');
        }
        $keys = array_keys($decoded);
        sort($keys);
        $schema = $decoded['schema'] ?? null;
        $accountId = $decoded['account_id'] ?? null;
        $nonce = $decoded['nonce'] ?? null;
        if ($keys !== ['account_id', 'nonce', 'schema']
            || $schema !== PremiumTimeContract::REQUEST_SCHEMA
            || ! is_string($nonce)
            || preg_match('/^[0-9a-f]{32}$/D', $nonce) !== 1) {
            throw new InvalidArgumentException('Premium snapshot request values are invalid.');
        }

        return [
            'schema' => $schema,
            'account_id' => PremiumTimeContract::assertUuid($accountId, 'account_id'),
            'nonce' => $nonce,
        ];
    }

    private function assertNoDuplicateKeys(string $raw): void
    {
        preg_match_all('/"((?:[^"\\\\]|\\\\.)*)"\s*:/s', $raw, $matches);
        $seen = [];
        foreach ($matches[1] as $encodedName) {
            try {
                $name = json_decode('"'.$encodedName.'"', true, 2, JSON_THROW_ON_ERROR);
            } catch (JsonException $exception) {
                throw new InvalidArgumentException('Premium snapshot request member encoding is invalid.', 0, $exception);
            }
            if (! is_string($name) || isset($seen[$name])) {
                throw new InvalidArgumentException('Duplicate premium snapshot request member is forbidden.');
            }
            $seen[$name] = true;
        }
    }
}
