<?php

namespace App\GameCatalog\Infrastructure\Persistence;

use LogicException;

final readonly class CatalogDatabaseRow
{
    /** @param array<string, mixed> $values */
    private function __construct(private array $values) {}

    public static function from(object $row): self
    {
        /** @var array<string, mixed> $values */
        $values = get_object_vars($row);

        return new self($values);
    }

    public function int(string $key): int
    {
        $value = $this->required($key);

        if (is_int($value)) {
            return $value;
        }

        if (is_string($value) && preg_match('/^-?(?:0|[1-9][0-9]*)$/D', $value) === 1) {
            return (int) $value;
        }

        throw $this->invalid($key, 'integer');
    }

    public function nullableInt(string $key): ?int
    {
        return $this->nullable($key) === null ? null : $this->int($key);
    }

    public function string(string $key): string
    {
        $value = $this->required($key);

        if (is_string($value)) {
            return $value;
        }

        throw $this->invalid($key, 'string');
    }

    public function nullableString(string $key): ?string
    {
        return $this->nullable($key) === null ? null : $this->string($key);
    }

    public function bool(string $key): bool
    {
        $value = $this->required($key);

        if (is_bool($value)) {
            return $value;
        }

        if ($value === 0 || $value === '0') {
            return false;
        }

        if ($value === 1 || $value === '1') {
            return true;
        }

        throw $this->invalid($key, 'boolean');
    }

    public function nullable(string $key): mixed
    {
        if (! array_key_exists($key, $this->values)) {
            throw new LogicException("Game Catalog database row is missing column '{$key}'.");
        }

        return $this->values[$key];
    }

    private function required(string $key): mixed
    {
        $value = $this->nullable($key);

        if ($value === null) {
            throw $this->invalid($key, 'non-null value');
        }

        return $value;
    }

    private function invalid(string $key, string $expected): LogicException
    {
        return new LogicException("Game Catalog database column '{$key}' is not a valid {$expected}.");
    }
}
