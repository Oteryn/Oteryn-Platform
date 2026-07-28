<?php

namespace App\GameCatalog\Infrastructure\Json;

use App\GameCatalog\Domain\CatalogValidationFinding;
use DateTimeImmutable;
use JsonException;
use RuntimeException;
use stdClass;

final class BundledJsonSchemaValidator
{
    /** @var array<string, mixed> */
    private array $rootSchema = [];

    /** @var list<CatalogValidationFinding> */
    private array $findings = [];

    public function __construct(private readonly int $maximumFindings = 2_000) {}

    /** @return list<CatalogValidationFinding> */
    public function validate(mixed $document, string $schemaJson): array
    {
        try {
            $schema = json_decode($schemaJson, true, 128, JSON_THROW_ON_ERROR);
        } catch (JsonException $exception) {
            throw new RuntimeException('The bundled Game Catalog schema is invalid JSON.', previous: $exception);
        }

        $schema = $this->stringKeyedSchema($schema, 'The bundled Game Catalog schema root must be an object.');

        $this->rootSchema = $schema;
        $this->findings = [];
        $this->validateNode($document, $schema, '$');

        return $this->findings;
    }

    /** @param array<string, mixed> $schema */
    private function validateNode(mixed $value, array $schema, string $path): void
    {
        if ($this->isFull()) {
            return;
        }

        if (isset($schema['$ref'])) {
            $reference = $schema['$ref'];
            if (! is_string($reference)) {
                throw new RuntimeException('The bundled schema contains a non-string $ref.');
            }
            $this->validateNode($value, $this->resolveReference($reference), $path);

            return;
        }

        if (isset($schema['allOf'])) {
            if (! is_array($schema['allOf'])) {
                throw new RuntimeException('The bundled schema contains an invalid allOf.');
            }
            foreach ($schema['allOf'] as $branch) {
                $branch = $this->stringKeyedSchema($branch, 'The bundled schema contains an invalid allOf branch.');
                $this->validateNode($value, $branch, $path);
            }
        }

        if (isset($schema['oneOf'])) {
            if (! is_array($schema['oneOf'])) {
                throw new RuntimeException('The bundled schema contains an invalid oneOf.');
            }

            $original = $this->findings;
            $matchingBranches = 0;
            foreach ($schema['oneOf'] as $branch) {
                $branch = $this->stringKeyedSchema($branch, 'The bundled schema contains an invalid oneOf branch.');

                $this->findings = [];
                $this->validateNode($value, $branch, $path);
                if ($this->findings === []) {
                    $matchingBranches++;
                }
            }
            $this->findings = $original;

            if ($matchingBranches !== 1) {
                $this->add('schema.one_of', 'Value must match exactly one allowed schema.', $path, [
                    'matching_branches' => $matchingBranches,
                ]);

                return;
            }
        }

        if (array_key_exists('const', $schema) && $value !== $schema['const']) {
            $this->add('schema.const', 'Value does not match the required constant.', $path);

            return;
        }

        if (isset($schema['enum'])) {
            if (! is_array($schema['enum']) || ! in_array($value, $schema['enum'], true)) {
                $this->add('schema.enum', 'Value is not in the allowed enumeration.', $path);

                return;
            }
        }

        if (isset($schema['type']) && ! $this->matchesType($value, $schema['type'])) {
            $this->add('schema.type', 'Value has an invalid JSON type.', $path, [
                'actual_type' => $this->jsonType($value),
            ]);

            return;
        }

        if ($value instanceof stdClass) {
            $this->validateObject($value, $schema, $path);
        } elseif (is_array($value)) {
            if (! array_is_list($value)) {
                throw new RuntimeException('Schema-validated JSON arrays must be represented as lists.');
            }
            $this->validateArray($value, $schema, $path);
        } elseif (is_string($value)) {
            $this->validateString($value, $schema, $path);
        } elseif (is_int($value) || is_float($value)) {
            $this->validateNumber($value, $schema, $path);
        }
    }

    /** @param array<string, mixed> $schema */
    private function validateObject(stdClass $value, array $schema, string $path): void
    {
        $properties = get_object_vars($value);

        $maximumProperties = $this->schemaInt($schema, 'maxProperties');
        if ($maximumProperties !== null && count($properties) > $maximumProperties) {
            $this->add('schema.max_properties', 'Object contains too many properties.', $path, [
                'count' => count($properties),
                'maximum' => $maximumProperties,
            ]);
        }

        $defined = array_key_exists('properties', $schema)
            ? $this->stringKeyedSchema($schema['properties'], 'The bundled schema contains invalid properties.')
            : [];

        if (isset($schema['required'])) {
            if (! is_array($schema['required'])) {
                throw new RuntimeException('The bundled schema contains an invalid required list.');
            }
            foreach ($schema['required'] as $required) {
                if (! is_string($required)) {
                    throw new RuntimeException('The bundled schema contains a non-string required key.');
                }
                if (! array_key_exists($required, $properties)) {
                    $this->add('schema.required', "Required property '{$required}' is missing.", $path);
                }
            }
        }

        if (($schema['additionalProperties'] ?? null) === false) {
            foreach (array_keys($properties) as $name) {
                if (! array_key_exists($name, $defined)) {
                    $this->add('schema.additional_property', "Property '{$name}' is not allowed.", $path.'.'.$name);
                }
            }
        }

        foreach ($defined as $name => $propertySchema) {
            if (! is_array($propertySchema)) {
                throw new RuntimeException('The bundled schema contains an invalid property definition.');
            }
            $propertySchema = $this->stringKeyedSchema($propertySchema, 'The bundled schema contains an invalid property definition.');
            if (array_key_exists($name, $properties)) {
                $this->validateNode($properties[$name], $propertySchema, $path.'.'.$name);
            }
        }
    }

    /**
     * @param list<mixed> $value
     * @param array<string, mixed> $schema
     */
    private function validateArray(array $value, array $schema, string $path): void
    {
        $count = count($value);
        $minimumItems = $this->schemaInt($schema, 'minItems');
        if ($minimumItems !== null && $count < $minimumItems) {
            $this->add('schema.min_items', 'Array contains too few items.', $path, [
                'count' => $count,
                'minimum' => $minimumItems,
            ]);
        }
        $maximumItems = $this->schemaInt($schema, 'maxItems');
        if ($maximumItems !== null && $count > $maximumItems) {
            $this->add('schema.max_items', 'Array contains too many items.', $path, [
                'count' => $count,
                'maximum' => $maximumItems,
            ]);
        }

        if (isset($schema['items'])) {
            $itemSchema = $this->stringKeyedSchema($schema['items'], 'The bundled schema contains an invalid items definition.');
            foreach ($value as $index => $item) {
                $this->validateNode($item, $itemSchema, $path.'['.$index.']');
            }
        }
    }

    /** @param array<string, mixed> $schema */
    private function validateString(string $value, array $schema, string $path): void
    {
        $length = mb_strlen($value, 'UTF-8');
        $minimumLength = $this->schemaInt($schema, 'minLength');
        if ($minimumLength !== null && $length < $minimumLength) {
            $this->add('schema.min_length', 'String is shorter than allowed.', $path);
        }
        $maximumLength = $this->schemaInt($schema, 'maxLength');
        if ($maximumLength !== null && $length > $maximumLength) {
            $this->add('schema.max_length', 'String is longer than allowed.', $path);
        }
        if (isset($schema['pattern'])) {
            $pattern = $schema['pattern'];
            if (! is_string($pattern)) {
                throw new RuntimeException('The bundled schema contains a non-string pattern.');
            }
            $delimiter = '~';
            $escaped = str_replace($delimiter, '\\'.$delimiter, $pattern);
            if (preg_match($delimiter.$escaped.$delimiter.'uD', $value) !== 1) {
                $this->add('schema.pattern', 'String does not match the required pattern.', $path);
            }
        }
        if (($schema['format'] ?? null) === 'date-time' && ! $this->isDateTime($value)) {
            $this->add('schema.date_time', 'String is not a valid RFC3339 date-time.', $path);
        }
    }

    /** @param array<string, mixed> $schema */
    private function validateNumber(int|float $value, array $schema, string $path): void
    {
        $minimum = $this->schemaNumber($schema, 'minimum');
        if ($minimum !== null && $value < $minimum) {
            $this->add('schema.minimum', 'Number is below the allowed minimum.', $path);
        }
        $maximum = $this->schemaNumber($schema, 'maximum');
        if ($maximum !== null && $value > $maximum) {
            $this->add('schema.maximum', 'Number is above the allowed maximum.', $path);
        }
    }

    private function matchesType(mixed $value, mixed $expected): bool
    {
        $types = is_array($expected) ? $expected : [$expected];
        foreach ($types as $type) {
            if (! is_string($type)) {
                throw new RuntimeException('The bundled schema contains a non-string type.');
            }

            $matches = match ($type) {
                'object' => $value instanceof stdClass,
                'array' => is_array($value),
                'string' => is_string($value),
                'integer' => is_int($value),
                'number' => is_int($value) || is_float($value),
                'boolean' => is_bool($value),
                'null' => $value === null,
                default => throw new RuntimeException("The bundled schema uses unsupported type '{$type}'."),
            };

            if ($matches) {
                return true;
            }
        }

        return false;
    }

    /** @return array<string, mixed> */
    private function resolveReference(string $reference): array
    {
        if (! str_starts_with($reference, '#/')) {
            throw new RuntimeException("The bundled schema contains unsupported external reference '{$reference}'.");
        }

        $value = $this->rootSchema;
        foreach (explode('/', substr($reference, 2)) as $segment) {
            $segment = str_replace(['~1', '~0'], ['/', '~'], $segment);
            if (! is_array($value) || ! array_key_exists($segment, $value)) {
                throw new RuntimeException("The bundled schema contains unresolved reference '{$reference}'.");
            }
            $value = $value[$segment];
        }

        return $this->stringKeyedSchema(
            $value,
            "The bundled schema reference '{$reference}' does not resolve to an object.",
        );
    }

    /** @return array<string, mixed> */
    private function stringKeyedSchema(mixed $value, string $message): array
    {
        if (! is_array($value) || array_is_list($value)) {
            throw new RuntimeException($message);
        }

        foreach (array_keys($value) as $key) {
            if (! is_string($key)) {
                throw new RuntimeException($message);
            }
        }

        /** @var array<string, mixed> $value */
        return $value;
    }

    /** @param array<string, mixed> $schema */
    private function schemaInt(array $schema, string $key): ?int
    {
        if (! array_key_exists($key, $schema)) {
            return null;
        }

        $value = $schema[$key];
        if (! is_int($value) || $value < 0) {
            throw new RuntimeException("The bundled schema contains an invalid {$key} value.");
        }

        return $value;
    }

    /** @param array<string, mixed> $schema */
    private function schemaNumber(array $schema, string $key): int|float|null
    {
        if (! array_key_exists($key, $schema)) {
            return null;
        }

        $value = $schema[$key];
        if (! is_int($value) && ! is_float($value)) {
            throw new RuntimeException("The bundled schema contains an invalid {$key} value.");
        }

        return $value;
    }

    private function isDateTime(string $value): bool
    {
        if (preg_match('/^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}(?:\.\d+)?(?:Z|[+-]\d{2}:\d{2})$/D', $value) !== 1) {
            return false;
        }

        try {
            new DateTimeImmutable($value);

            return true;
        } catch (\Throwable) {
            return false;
        }
    }

    private function jsonType(mixed $value): string
    {
        return match (true) {
            $value instanceof stdClass => 'object',
            is_array($value) => 'array',
            is_string($value) => 'string',
            is_int($value) => 'integer',
            is_float($value) => 'number',
            is_bool($value) => 'boolean',
            $value === null => 'null',
            default => get_debug_type($value),
        };
    }

    /** @param array<string, bool|float|int|string|null> $context */
    private function add(string $code, string $message, string $path, array $context = []): void
    {
        if ($this->isFull()) {
            return;
        }

        $this->findings[] = new CatalogValidationFinding(
            severity: 'error',
            code: $code,
            message: $message,
            path: $path,
            context: $context,
        );
    }

    private function isFull(): bool
    {
        return count($this->findings) >= $this->maximumFindings;
    }
}
