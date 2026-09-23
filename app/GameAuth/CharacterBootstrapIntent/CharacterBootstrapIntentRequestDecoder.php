<?php

namespace App\GameAuth\CharacterBootstrapIntent;

use InvalidArgumentException;
use JsonException;

final class CharacterBootstrapIntentRequestDecoder
{
    /** @return array{contract_version: int, operation_id: string, audience: string} */
    public function decode(string $raw): array
    {
        if ($raw === '' || strlen($raw) > CharacterBootstrapIntentContract::MAX_REQUEST_BYTES) {
            throw new InvalidArgumentException('Character bootstrap intent request size is invalid.');
        }
        preg_match_all('/"((?:[^"\\\\]|\\\\.)*)"\s*:/s', $raw, $matches);
        $seen = [];
        foreach ($matches[1] as $encodedName) {
            try {
                $name = json_decode('"'.$encodedName.'"', true, 2, JSON_THROW_ON_ERROR);
            } catch (JsonException $exception) {
                throw new InvalidArgumentException('Character bootstrap member encoding is invalid.', 0, $exception);
            }
            if (! is_string($name) || isset($seen[$name])) {
                throw new InvalidArgumentException('Duplicate Character bootstrap member is forbidden.');
            }
            $seen[$name] = true;
        }
        try {
            $decoded = json_decode($raw, true, 2, JSON_THROW_ON_ERROR);
        } catch (JsonException $exception) {
            throw new InvalidArgumentException('Character bootstrap JSON is invalid.', 0, $exception);
        }
        if (! is_array($decoded) || array_is_list($decoded) || array_keys($decoded) !== ['contract_version', 'operation_id', 'audience']) {
            throw new InvalidArgumentException('Character bootstrap fields are invalid.');
        }
        if ($decoded['contract_version'] !== CharacterBootstrapIntentContract::VERSION
            || ! is_string($decoded['operation_id'])
            || $decoded['audience'] !== CharacterBootstrapIntentContract::AUDIENCE) {
            throw new InvalidArgumentException('Character bootstrap request is unsupported.');
        }
        CharacterBootstrapIntentContract::assertUuid($decoded['operation_id'], 'operation_id');

        /** @var array{contract_version: int, operation_id: string, audience: string} $decoded */
        return $decoded;
    }
}
