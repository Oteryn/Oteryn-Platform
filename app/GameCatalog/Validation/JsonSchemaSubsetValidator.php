<?php

namespace App\GameCatalog\Validation;

use DateTimeImmutable;
use Throwable;

final class JsonSchemaSubsetValidator
{
    /** @var array<string, mixed> */
    private array $rootSchema = [];

    /** @var list<array{code: string, path: string, message: string}> */
    private array $findings = [];

    public function __construct(private readonly int $maximumFindings = 200) {}

    /**
     * Validate the JSON Schema Draft 2020-12 keyword subset used by the
     * versioned Game Catalog schema. Unsupported keywords fail closed.
     *
     * @param  array<string, mixed>  $schema
     * @return list<array{code: string, path: string, message: string}>
     */
    public function validate(mixed $value, array $schema): array
    {
        $this->rootSchema = $schema;
        $this->findings = [];
        $this->validateNode($value, $schema, '$');

        return $this->findings;
    }

    /**
     * @param  array<string, mixed>  $schema
     */
    private function validateNode(mixed $value, array $schema, string $path): void
    {
        if ($this->limitReached()) {
            return;
        }

        $this->rejectUnsupportedKeywords($schema, $path);

        if (isset($schema['$ref'])) {
            if (! is_string($schema['$ref'])) {
                $this->add('schema.invalid_ref', $path, 'Schema reference must be a string.');

                return;
            }

            $resolved = $this->resolveReference($schema['$ref']);
            if ($resolved === null) {
                $this->add('schema.unresolved_ref', $path, 'Schema reference could not be resolved.');

                return;
            }

            $this->validateNode($value, $resolved, $path);
        }

        if (isset($schema['allOf'])) {
            if (! is_array($schema['allOf']) || ! array_is_list($schema['allOf'])) {
                $this->add('schema.invalid_all_of', $path, 'allOf must be an array.');
            } else {
                foreach ($schema['allOf'] as $candidate) {
                    if (! is_array($candidate)) {
                        $this->add('schema.invalid_all_of', $path, 'allOf entries must be schemas.');
                        continue;
                    }
                    $this->validateNode($value, $candidate, $path);
                }
            }
        }

        if (isset($schema['oneOf'])) {
            $this->validateOneOf($value, $schema['oneOf'], $path);
        }

        if (array_key_exists('const', $schema) && $value !== $schema['const']) {
            $this->add('schema.const', $path, 'Value does not match the required constant.');
        }

        if (isset($schema['enum'])) {
            if (! is_array($schema['enum']) || ! in_array($value, $schema['enum'], true)) {
                $this->add('schema.enum', $path, 'Value is not in the allowed enumeration.');
            }
        }

        if (isset($schema['type']) && ! $this->matchesDeclaredType($value, $schema['type'])) {
            $this->add('schema.type', $path, 'Value does not match the declared JSON type.');

            return;
        }

        if (is_object($value)) {
            $this->validateObject($value, $schema, $path);
        } elseif (is_array($value)) {
            $this->validateArray($value, $schema, $path);
        } elseif (is_string($value)) {
            $this->validateString($value, $schema, $path);
        } elseif (is_int($value) || is_float($value)) {
            $this->validateNumber($value, $schema, $path);
        }
    }

    private function validateOneOf(mixed $value, mixed $oneOf, string $path): void
    {
        if (! is_array($oneOf) || ! array_is_list($oneOf)) {
            $this->add('schema.invalid_one_of', $path, 'oneOf must be an array.');

            return;
        }

        $matches = 0;
        foreach ($oneOf as $candidate) {
            if (! is_array($candidate)) {
                $this->add('schema.invalid_one_of', $path, 'oneOf entries must be schemas.');
                continue;
            }

            $nested = new self($this->maximumFindings);
            if ($nested->validateAgainstRoot($value, $candidate, $this->rootSchema) === []) {
                ++$matches;
            }
        }

        if ($matches !== 1) {
            $this->add('schema.one_of', $path, 'Value must match exactly one oneOf schema.');
        }
    }

    /**
     * @param  array<string, mixed>  $schema
     * @param  array<string, mixed>  $rootSchema
     * @return list<array{code: string, path: string, message: string}>
     */
    private function validateAgainstRoot(mixed $value, array $schema, array $rootSchema): array
    {
        $this->rootSchema = $rootSchema;
        $this->findings = [];
        $this->validateNode($value, $schema, '$');

        return $this->findings;
    }

    /**
     * @param  array<string, mixed>  $schema
     */
    private function validateObject(object $value, array $schema, string $path): void
    {
        $properties = get_object_vars($value);

        if (isset($schema['minProperties']) && is_int($schema['minProperties']) && count($properties) < $schema['minProperties']) {
            $this->add('schema.min_properties', $path, 'Object contains fewer properties than allowed.');
        }
        if (isset($schema['maxProperties']) && is_int($schema['maxProperties']) && count($properties) > $schema['maxProperties']) {
            $this->add('schema.max_properties', $path, 'Object contains more properties than allowed.');
        }

        $declaredProperties = isset($schema['properties']) && is_array($schema['properties'])
            ? $schema['properties']
            : [];

        if (isset($schema['required'])) {
            if (! is_array($schema['required']) || ! array_is_list($schema['required'])) {
                $this->add('schema.invalid_required', $path, 'required must be an array.');
            } else {
                foreach ($schema['required'] as $required) {
                    if (! is_string($required)) {
                        $this->add('schema.invalid_required', $path, 'required entries must be strings.');
                        continue;
                    }
                    if (! array_key_exists($required, $properties)) {
                        $this->add('schema.required', $path.'/'.$this->escapePointer($required), "Required property [{$required}] is missing.");
                    }
                }
            }
        }

        foreach ($properties as $name => $propertyValue) {
            if (array_key_exists($name, $declaredProperties)) {
                $propertySchema = $declaredProperties[$name];
                if (! is_array($propertySchema)) {
                    $this->add('schema.invalid_property', $path.'/'.$this->escapePointer($name), 'Property schema is invalid.');
                    continue;
                }
                $this->validateNode($propertyValue, $propertySchema, $path.'/'.$this->escapePointer($name));
            } elseif (($schema['additionalProperties'] ?? true) === false) {
                $this->add('schema.additional_property', $path.'/'.$this->escapePointer($name), "Unknown property [{$name}] is not allowed.");
            }
        }
    }

    /**
     * @param  array<string, mixed>  $schema
     * @param  list<mixed>  $value
     */
    private function validateArray(array $value, array $schema, string $path): void
    {
        if (! array_is_list($value)) {
            $this->add('schema.array_shape', $path, 'JSON array must use sequential indexes.');

            return;
        }

        if (isset($schema['minItems']) && is_int($schema['minItems']) && count($value) < $schema['minItems']) {
            $this->add('schema.min_items', $path, 'Array contains fewer items than allowed.');
        }
        if (isset($schema['maxItems']) && is_int($schema['maxItems']) && count($value) > $schema['maxItems']) {
            $this->add('schema.max_items', $path, 'Array contains more items than allowed.');
        }

        if (isset($schema['items'])) {
            if (! is_array($schema['items'])) {
                $this->add('schema.invalid_items', $path, 'items must be a schema.');

                return;
            }
            foreach ($value as $index => $item) {
                $this->validateNode($item, $schema['items'], $path.'/'.$index);
            }
        }
    }

    /**
     * @param  array<string, mixed>  $schema
     */
    private function validateString(string $value, array $schema, string $path): void
    {
        $length = mb_strlen($value, 'UTF-8');
        if (isset($schema['minLength']) && is_int($schema['minLength']) && $length < $schema['minLength']) {
            $this->add('schema.min_length', $path, 'String is shorter than allowed.');
        }
        if (isset($schema['maxLength']) && is_int($schema['maxLength']) && $length > $schema['maxLength']) {
            $this->add('schema.max_length', $path, 'String is longer than allowed.');
        }

        if (isset($schema['pattern'])) {
            if (! is_string($schema['pattern'])) {
                $this->add('schema.invalid_pattern', $path, 'Schema pattern must be a string.');
            } else {
                $matched = @preg_match('~'.$schema['pattern'].'~uD', $value);
                if ($matched !== 1) {
                    $this->add('schema.pattern', $path, 'String does not match the required pattern.');
                }
            }
        }

        if (($schema['format'] ?? null) === 'date-time' && ! $this->isDateTime($value)) {
            $this->add('schema.date_time', $path, 'String is not a valid RFC 3339 date-time.');
        }
    }

    /**
     * @param  array<string, mixed>  $schema
     */
    private function validateNumber(int|float $value, array $schema, string $path): void
    {
        if (isset($schema['minimum']) && is_int($schema['minimum']) || is_float($schema['minimum'] ?? null)) {
            if ($value < $schema['minimum']) {
                $this->add('schema.minimum', $path, 'Number is below the allowed minimum.');
            }
        }
        if (isset($schema['maximum']) && is_int($schema['maximum']) || is_float($schema['maximum'] ?? null)) {
            if ($value > $schema['maximum']) {
                $this->add('schema.maximum', $path, 'Number exceeds the allowed maximum.');
            }
        }
    }

    private function matchesDeclaredType(mixed $value, mixed $declared): bool
    {
        $types = is_string($declared) ? [$declared] : $declared;
        if (! is_array($types) || ! array_is_list($types)) {
            return false;
        }

        foreach ($types as $type) {
            if (! is_string($type)) {
                continue;
            }

            $matches = match ($type) {
                'null' => $value === null,
                'boolean' => is_bool($value),
                'integer' => is_int($value),
                'number' => is_int($value) || is_float($value),
                'string' => is_string($value),
                'array' => is_array($value) && array_is_list($value),
                'object' => is_object($value),
                default => false,
            };

            if ($matches) {
                return true;
            }
        }

        return false;
    }

    /**
     * @return array<string, mixed>|null
     */
    private function resolveReference(string $reference): ?array
    {
        if (! str_starts_with($reference, '#/')) {
            return null;
        }

        $current = $this->rootSchema;
        foreach (explode('/', substr($reference, 2)) as $segment) {
            $segment = str_replace(['~1', '~0'], ['/', '~'], $segment);
            if (! is_array($current) || ! array_key_exists($segment, $current)) {
                return null;
            }
            $current = $current[$segment];
        }

        return is_array($current) ? $current : null;
    }

    /**
     * @param  array<string, mixed>  $schema
     */
    private function rejectUnsupportedKeywords(array $schema, string $path): void
    {
        $supported = [
            '$schema', '$id', '$defs', '$ref', 'title', 'description',
            'type', 'additionalProperties', 'required', 'properties',
            'minProperties', 'maxProperties', 'items', 'minItems', 'maxItems',
            'minLength', 'maxLength', 'pattern', 'format', 'minimum', 'maximum',
            'enum', 'const', 'oneOf', 'allOf',
        ];

        foreach (array_keys($schema) as $keyword) {
            if (! in_array($keyword, $supported, true)) {
                $this->add('schema.unsupported_keyword', $path, "Unsupported schema keyword [{$keyword}].");
            }
        }
    }

    private function isDateTime(string $value): bool
    {
        if (preg_match('/^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}(?:\.\d+)?(?:Z|[+-]\d{2}:\d{2})$/D', $value) !== 1) {
            return false;
        }

        try {
            new DateTimeImmutable($value);

            return true;
        } catch (Throwable) {
            return false;
        }
    }

    private function escapePointer(string $value): string
    {
        return str_replace(['~', '/'], ['~0', '~1'], $value);
    }

    private function add(string $code, string $path, string $message): void
    {
        if ($this->limitReached()) {
            return;
        }

        $this->findings[] = compact('code', 'path', 'message');
    }

    private function limitReached(): bool
    {
        return count($this->findings) >= $this->maximumFindings;
    }
}
