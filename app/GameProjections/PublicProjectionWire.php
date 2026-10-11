<?php

namespace App\GameProjections;

use App\GameAuth\NativeAccountCharacters\NativeAccountCharactersWire;
use InvalidArgumentException;
use JsonException;

/**
 * Wire primitives of Game Public Projections v1 §4.0. Ids, names, non-zero uint64 values, the
 * authority namespace and Unix times reuse the LCFA consumer's validators, as the contract reuses
 * the LCFA wire rules by reference. Every failure is an InvalidArgumentException (HTTP 400).
 */
final class PublicProjectionWire
{
    public const AUTHORITY = '/^[A-Za-z0-9._:\/-]{1,128}$/D';

    public const CONTENT_KEY = '/^[a-z0-9._:-]{1,128}$/D';

    /** Five nested containers plus their scalar members (§4.0: "nests at most 5 levels"). */
    private const MAX_DEPTH = 6;

    /** @return array<string, mixed> */
    public static function decodeExact(string $raw, int $maxBytes): array
    {
        if ($raw === '' || strlen($raw) > $maxBytes || preg_match('/[\t\n\r]/', $raw) === 1) {
            throw new InvalidArgumentException('Public projection size or whitespace is invalid.');
        }

        try {
            $decoded = json_decode($raw, true, self::MAX_DEPTH, JSON_THROW_ON_ERROR);
        } catch (JsonException $exception) {
            throw new InvalidArgumentException('Public projection JSON is invalid.', 0, $exception);
        }
        if (! is_array($decoded) || array_is_list($decoded)) {
            throw new InvalidArgumentException('Public projection root is invalid.');
        }
        // Re-encoding refuses duplicate members (collapsed on decode), escapes, insignificant
        // whitespace and non-canonical numbers; `null` members are refused by the typed readers.
        $canonical = json_encode($decoded, JSON_THROW_ON_ERROR | JSON_UNESCAPED_SLASHES | JSON_UNESCAPED_UNICODE);
        if (! hash_equals($canonical, $raw)) {
            throw new InvalidArgumentException('Public projection is not canonical JSON.');
        }

        /** @var array<string, mixed> $decoded */
        return $decoded;
    }

    /**
     * @param  array<string, mixed>  $object
     * @param  list<string>  $keys
     */
    public static function exactKeys(array $object, array $keys): void
    {
        NativeAccountCharactersWire::exactKeys($object, $keys);
    }

    /** @return array<string, mixed> */
    public static function object(mixed $value): array
    {
        if (! is_array($value) || $value === [] || array_is_list($value)) {
            throw new InvalidArgumentException('Public projection object is invalid.');
        }

        /** @var array<string, mixed> $value */
        return $value;
    }

    /** @return list<mixed> */
    public static function list(mixed $value, int $min, int $max): array
    {
        if (! is_array($value) || ! array_is_list($value) || count($value) < $min || count($value) > $max) {
            throw new InvalidArgumentException('Public projection list is invalid.');
        }

        return $value;
    }

    /** @param array<string, mixed> $object */
    public static function uuid7(array $object, string $key): string
    {
        return NativeAccountCharactersWire::uuid7($object, $key);
    }

    /** @param array<string, mixed> $object */
    public static function authority(array $object): string
    {
        return NativeAccountCharactersWire::text($object, 'source_authority', self::AUTHORITY);
    }

    /** @param array<string, mixed> $object */
    public static function contentKey(array $object, string $key): string
    {
        return NativeAccountCharactersWire::text($object, $key, self::CONTENT_KEY);
    }

    /**
     * Canonical non-zero decimal uint64 string.
     *
     * @param  array<string, mixed>  $object
     */
    public static function uint64(array $object, string $key): string
    {
        return NativeAccountCharactersWire::uint64($object, $key);
    }

    /**
     * Canonical decimal uint64 string where the contract allows `0`.
     *
     * @param  array<string, mixed>  $object
     */
    public static function uint64OrZero(array $object, string $key): string
    {
        if (($object[$key] ?? null) === '0') {
            return '0';
        }

        return self::uint64($object, $key);
    }

    /**
     * Unix seconds; `0` allowed (envelope and watermark times, LCFA §4).
     *
     * @param  array<string, mixed>  $object
     */
    public static function unixTime(array $object, string $key): int
    {
        return NativeAccountCharactersWire::unixTime($object, $key);
    }

    /**
     * Non-zero Unix seconds (body times, §4.0).
     *
     * @param  array<string, mixed>  $object
     */
    public static function bodyTime(array $object, string $key): int
    {
        $value = NativeAccountCharactersWire::unixTime($object, $key);
        if ($value === 0) {
            throw new InvalidArgumentException('Public projection time is invalid.');
        }

        return $value;
    }

    /**
     * Canonical JSON integer in a closed range.
     *
     * @param  array<string, mixed>  $object
     */
    public static function int(array $object, string $key, int $min, int $max): int
    {
        $value = $object[$key] ?? null;
        if (! is_int($value) || $value < $min || $value > $max) {
            throw new InvalidArgumentException('Public projection integer is invalid.');
        }

        return $value;
    }

    /** @param array<string, mixed> $object */
    public static function bool(array $object, string $key): bool
    {
        $value = $object[$key] ?? null;
        if (! is_bool($value)) {
            throw new InvalidArgumentException('Public projection boolean is invalid.');
        }

        return $value;
    }

    /**
     * @param  array<string, mixed>  $object
     * @param  list<string>  $allowed
     */
    public static function oneOf(array $object, string $key, array $allowed): string
    {
        $value = $object[$key] ?? null;
        if (! is_string($value) || ! in_array($value, $allowed, true)) {
            throw new InvalidArgumentException('Public projection enumeration is invalid.');
        }

        return $value;
    }

    public static function name(mixed $value): string
    {
        return NativeAccountCharactersWire::characterName($value);
    }

    public static function compareUint64(string $left, string $right): int
    {
        return NativeAccountCharactersWire::compareUint64($left, $right);
    }
}
