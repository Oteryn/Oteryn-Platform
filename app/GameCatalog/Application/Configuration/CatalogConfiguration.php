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

    /** @return array{path: string, sha256: string, activatable: bool}|null */
    public static function schemaContract(string $version): ?array
    {
        $schemas = config('game-catalog.schemas', []);
        if (! is_array($schemas) || array_is_list($schemas)) {
            throw new LogicException("Game Catalog configuration 'game-catalog.schemas' must be a version-keyed map.");
        }

        /** @var array<string, array{path: string, sha256: string, activatable: bool}> $normalized */
        $normalized = [];

        foreach ($schemas as $configuredVersion => $contract) {
            $path = is_array($contract) ? ($contract['path'] ?? null) : null;
            $sha256 = is_array($contract) ? ($contract['sha256'] ?? null) : null;
            $activatable = is_array($contract) ? ($contract['activatable'] ?? true) : null;

            if (
                ! is_string($configuredVersion)
                || $configuredVersion === ''
                || ! is_array($contract)
                || array_is_list($contract)
                || ! is_string($path)
                || $path === ''
                || ! is_string($sha256)
                || preg_match('/^[0-9a-f]{64}$/D', $sha256) !== 1
                || ! is_bool($activatable)
            ) {
                throw new LogicException("Game Catalog configuration 'game-catalog.schemas' contains an invalid contract.");
            }

            $normalized[$configuredVersion] = [
                'path' => $path,
                'sha256' => $sha256,
                'activatable' => $activatable,
            ];
        }

        return $normalized[$version] ?? null;
    }
}
