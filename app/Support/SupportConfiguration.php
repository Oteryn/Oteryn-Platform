<?php

namespace App\Support;

final class SupportConfiguration
{
    public static function positiveInteger(string $key, int $default): int
    {
        $value = config($key, $default);

        return is_int($value) && $value > 0 ? $value : $default;
    }
}
