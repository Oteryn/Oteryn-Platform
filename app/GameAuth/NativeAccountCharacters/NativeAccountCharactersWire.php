<?php

namespace App\GameAuth\NativeAccountCharacters;

use InvalidArgumentException;
use JsonException;
use Normalizer;

final class NativeAccountCharactersWire
{
    public const SNAPSHOT_MAX_BYTES = 16384;

    public const WATERMARK_MAX_BYTES = 512;

    private const UINT64_MAX = '18446744073709551615';

    private const UUID7 = '/^[0-9a-f]{8}-[0-9a-f]{4}-7[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/D';

    private const AUTHORITY = '/^[A-Za-z0-9._:\/-]{1,128}$/D';

    /** Numeric order of two canonical non-zero uint64 decimal strings. */
    public static function compareUint64(string $left, string $right): int
    {
        return (strlen($left) <=> strlen($right)) ?: (strcmp($left, $right) <=> 0);
    }

    /** @return array<string, mixed> */
    public static function decodeExact(string $raw, int $maxBytes): array
    {
        if ($raw === '' || strlen($raw) > $maxBytes || str_contains($raw, "\n") || str_contains($raw, "\r") || str_contains($raw, "\t")) {
            throw new InvalidArgumentException('Native account-character publication size or whitespace is invalid.');
        }

        try {
            $decoded = json_decode($raw, true, 4, JSON_THROW_ON_ERROR);
        } catch (JsonException $exception) {
            throw new InvalidArgumentException('Native account-character publication JSON is invalid.', 0, $exception);
        }
        if (! is_array($decoded) || array_is_list($decoded)) {
            throw new InvalidArgumentException('Native account-character publication root is invalid.');
        }
        /** @var array<string, mixed> $decoded */
        $canonical = json_encode($decoded, JSON_THROW_ON_ERROR | JSON_UNESCAPED_SLASHES | JSON_UNESCAPED_UNICODE);
        if (! hash_equals($canonical, $raw)) {
            throw new InvalidArgumentException('Native account-character publication is not canonical JSON.');
        }

        return $decoded;
    }

    /**
     * @param  array<string, mixed>  $object
     * @param  list<string>  $keys
     */
    public static function exactKeys(array $object, array $keys): void
    {
        $actual = array_keys($object);
        sort($actual);
        sort($keys);
        if ($actual !== $keys) {
            throw new InvalidArgumentException('Native account-character publication member set is invalid.');
        }
    }

    /** @param array<string, mixed> $object */
    public static function text(array $object, string $key, string $pattern): string
    {
        $value = $object[$key] ?? null;
        if (! is_string($value) || preg_match($pattern, $value) !== 1) {
            throw new InvalidArgumentException('Native account-character publication member is invalid.');
        }

        return $value;
    }

    /** @param array<string, mixed> $object */
    public static function uuid7(array $object, string $key): string
    {
        return self::text($object, $key, self::UUID7);
    }

    /** @param array<string, mixed> $object */
    public static function authority(array $object): string
    {
        return self::text($object, 'source_authority', self::AUTHORITY);
    }

    /** @param array<string, mixed> $object */
    public static function uint64(array $object, string $key): string
    {
        $value = $object[$key] ?? null;
        if (! is_string($value)
            || preg_match('/^[1-9][0-9]{0,19}$/D', $value) !== 1
            || self::compareUint64($value, self::UINT64_MAX) > 0) {
            throw new InvalidArgumentException('Native account-character uint64 member is invalid.');
        }

        return $value;
    }

    /** @param array<string, mixed> $object */
    public static function unixTime(array $object, string $key): int
    {
        $value = $object[$key] ?? null;
        if (! is_string($value)
            || preg_match('/^(0|[1-9][0-9]{0,18})$/D', $value) !== 1
            || self::compareUnsigned($value, (string) PHP_INT_MAX) > 0) {
            throw new InvalidArgumentException('Native account-character timestamp is invalid.');
        }

        return (int) $value;
    }

    public static function characterName(mixed $value): string
    {
        if (! is_string($value)
            || strlen($value) < 1
            || strlen($value) > 64
            || preg_match('//u', $value) !== 1
            || str_contains($value, '"')
            || str_contains($value, '\\')
            || preg_match('/[\p{Cc}\p{Cf}]/u', $value) === 1
            || Normalizer::normalize($value, Normalizer::FORM_C) !== $value) {
            throw new InvalidArgumentException('Native account-character display name is invalid.');
        }

        return $value;
    }

    private static function compareUnsigned(string $left, string $right): int
    {
        return (strlen($left) <=> strlen($right)) ?: (strcmp($left, $right) <=> 0);
    }
}
