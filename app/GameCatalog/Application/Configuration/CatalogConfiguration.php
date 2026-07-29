<?php

namespace App\GameCatalog\Application\Configuration;

use LogicException;

final class CatalogConfiguration
{
    public static function string(string $key, ?string $default = null): string
    {
        $value = config($key, $default);

        if (! is_string($value) || $value === '') {
            throw new LogicException("Game Catalog configuration '{$key}' must be a non-empty string.");
        }

        return $value;
    }

    public static function positiveInt(string $key, int $default): int
    {
        $value = config($key, $default);

        if (! is_int($value) || $value < 1) {
            throw new LogicException("Game Catalog configuration '{$key}' must be a positive integer.");
        }

        return $value;
    }

    /** @return list<string> */
    public static function stringList(string $key): array
    {
        $value = config($key, []);

        if (! is_array($value) || ! array_is_list($value)) {
            throw new LogicException("Game Catalog configuration '{$key}' must be a list of strings.");
        }

        $result = [];
        foreach ($value as $entry) {
            if (! is_string($entry) || $entry === '') {
                throw new LogicException("Game Catalog configuration '{$key}' contains an invalid entry.");
            }
            $result[] = $entry;
        }

        return $result;
    }
}
