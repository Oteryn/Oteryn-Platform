<?php

namespace App\GameAuth\NativeRuntimeStatus;

use UnexpectedValueException;

/** Typed reads of a query-builder row; an unexpected column type is an unavailable read model. */
final class NativeRuntimeStatusRow
{
    public static function string(object $row, string $column): string
    {
        $value = get_object_vars($row)[$column] ?? null;
        if (! is_string($value)) {
            throw new UnexpectedValueException('Runtime status column is not a string.');
        }

        return $value;
    }

    public static function int(object $row, string $column): int
    {
        $value = get_object_vars($row)[$column] ?? null;
        if (is_string($value) && preg_match('/^(0|[1-9][0-9]{0,18})$/D', $value) === 1) {
            $value = (int) $value;
        }
        if (! is_int($value)) {
            throw new UnexpectedValueException('Runtime status column is not an integer.');
        }

        return $value;
    }

    public static function bool(object $row, string $column): bool
    {
        $value = self::int($row, $column);
        if ($value !== 0 && $value !== 1) {
            throw new UnexpectedValueException('Runtime status column is not a boolean.');
        }

        return $value === 1;
    }
}
