<?php

namespace Tests\Feature\GameAuth\Concerns;

use Illuminate\Support\Facades\Artisan;

/** Typed reads of a command's JSON output for the P6a command tests. */
trait ReadsJsonOutput
{
    /** @return array<mixed> */
    private static function jsonOutput(): array
    {
        $decoded = json_decode(Artisan::output(), true, 512, JSON_THROW_ON_ERROR);
        self::assertIsArray($decoded);

        return $decoded;
    }

    /** The value at a nested key path; fails the test when a step is missing or not an array. */
    private static function at(mixed $value, string ...$keys): mixed
    {
        foreach ($keys as $key) {
            self::assertIsArray($value);
            self::assertArrayHasKey($key, $value);
            $value = $value[$key];
        }

        return $value;
    }

    /** The string at a nested key path; fails the test when it is not a string. */
    private static function stringAt(mixed $value, string ...$keys): string
    {
        $found = self::at($value, ...$keys);
        self::assertIsString($found);

        return $found;
    }
}
