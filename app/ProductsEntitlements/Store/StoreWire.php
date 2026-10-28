<?php

namespace App\ProductsEntitlements\Store;

use App\GameAuth\NativeAccountCharacters\NativeAccountCharactersWire;
use InvalidArgumentException;

final class StoreWire
{
    /** @param array<string, mixed> $object */
    public static function encode(array $object, int $maxBytes): string
    {
        $raw = json_encode($object, JSON_THROW_ON_ERROR | JSON_UNESCAPED_SLASHES | JSON_UNESCAPED_UNICODE);
        if (strlen($raw) > $maxBytes || str_contains($raw, '\\')) {
            throw new InvalidArgumentException('Store JSON exceeds its bound or needs escaping.');
        }

        return $raw;
    }

    /** Canonical decoding reuses the command exchange decoder, including its depth limit. */
    public static function sourceAuthority(string $raw, int $maxBytes, string $operation): string
    {
        $object = NativeAccountCharactersWire::decodeExact($raw, $maxBytes);
        if (str_contains($raw, '\\')
            || array_slice(array_keys($object), 0, 3) !== ['contract_version', 'operation', 'source_authority']
            || $object['contract_version'] !== 1 || $object['operation'] !== $operation) {
            throw new InvalidArgumentException('Invalid Store canonical head.');
        }

        return NativeAccountCharactersWire::authority($object);
    }
}
