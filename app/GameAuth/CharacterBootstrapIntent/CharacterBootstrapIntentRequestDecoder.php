<?php

namespace App\GameAuth\CharacterBootstrapIntent;

use InvalidArgumentException;
use JsonException;

final class CharacterBootstrapIntentRequestDecoder
{
    /** @return array{contract_version:int,operation_id:string} */
    public function decode(string $raw): array
    {
        if ($raw === '' || strlen($raw) > CharacterBootstrapIntentContract::MAX_REQUEST_BYTES) {
            throw new InvalidArgumentException('Character bootstrap-intent request size is invalid.');
        }

        $this->assertNoDuplicateKeys($raw);

        try {
            $decoded = json_decode($raw, true, 2, JSON_THROW_ON_ERROR);
        } catch (JsonException $exception) {
            throw new InvalidArgumentException('Character bootstrap-intent JSON is invalid.', 0, $exception);
        }

        if (! is_array($decoded) || array_is_list($decoded)) {
            throw new InvalidArgumentException('Character bootstrap-intent request fields are invalid.');
        }
        $keys = array_keys($decoded);
        sort($keys);
        if ($keys !== ['contract_version', 'operation_id']) {
            throw new InvalidArgumentException('Character bootstrap-intent request fields are invalid.');
        }
        if ($decoded['contract_version'] !== CharacterBootstrapIntentContract::CONTRACT_VERSION
            || ! is_string($decoded['operation_id'])) {
            throw new InvalidArgumentException('Character bootstrap-intent request values are invalid.');
        }
        CharacterBootstrapIntentContract::assertUuid($decoded['operation_id'], 'operation_id');

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
                throw new InvalidArgumentException('Character bootstrap-intent member encoding is invalid.', 0, $exception);
            }
            if (! is_string($name) || isset($seen[$name])) {
                throw new InvalidArgumentException('Duplicate Character bootstrap-intent member is forbidden.');
            }
            $seen[$name] = true;
        }
    }
}
