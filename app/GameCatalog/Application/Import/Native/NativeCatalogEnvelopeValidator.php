<?php

namespace App\GameCatalog\Application\Import\Native;

use App\GameCatalog\Domain\CatalogValidationFinding;
use App\GameCatalog\Domain\Exceptions\CatalogValidationException;
use App\GameCatalog\Infrastructure\Json\DuplicateJsonKeyDetector;
use DateTimeImmutable;
use JsonException;
use LogicException;
use RuntimeException;
use stdClass;

/** @phpstan-import-type NativeCatalogPayload from ValidatedNativeCatalogSnapshot */
final class NativeCatalogEnvelopeValidator
{
    private readonly int $maximumFileBytes;

    public function __construct(?int $maximumFileBytes = null)
    {
        $maximumFileBytes ??= NativeCatalogContract::MAX_FILE_BYTES;
        if ($maximumFileBytes < 1 || $maximumFileBytes > NativeCatalogContract::MAX_FILE_BYTES) {
            throw new LogicException('Native Game Catalog file bound must remain within the locked hard maximum.');
        }

        $this->maximumFileBytes = $maximumFileBytes;
    }

    public function validate(string $path, ?string $expectedArtifactSha256 = null): ValidatedNativeCatalogSnapshot
    {
        [$contents, $fileSize] = $this->readBounded($path);
        $artifactSha256 = hash('sha256', $contents);

        if ($expectedArtifactSha256 !== null) {
            if (preg_match('/^[0-9a-f]{64}$/D', $expectedArtifactSha256) !== 1) {
                $this->fail('native.input.expected_hash_invalid', 'Expected artifact SHA-256 must be lowercase hexadecimal.', '$');
            }
            if (! hash_equals($expectedArtifactSha256, $artifactSha256)) {
                $this->fail('native.input.hash_mismatch', 'Native Game Catalog artifact SHA-256 does not match.', '$');
            }
        }

        if (str_starts_with($contents, "\xEF\xBB\xBF")) {
            $this->fail('native.input.bom_forbidden', 'UTF-8 BOM is forbidden for canonical native snapshots.', '$');
        }

        try {
            $duplicates = (new DuplicateJsonKeyDetector(NativeCatalogContract::JSON_DECODE_DEPTH))->find($contents);
        } catch (RuntimeException $exception) {
            $this->fail('native.input.invalid_json', $exception->getMessage(), '$', $exception);
        }
        if ($duplicates !== []) {
            $this->fail('native.input.duplicate_key', 'Duplicate JSON object key detected.', $duplicates[0]);
        }

        try {
            $decoded = json_decode($contents, false, NativeCatalogContract::JSON_DECODE_DEPTH, JSON_THROW_ON_ERROR);
        } catch (JsonException $exception) {
            $this->fail('native.input.invalid_json', 'Native Game Catalog artifact is not valid JSON.', '$', $exception);
        }

        if (! $decoded instanceof stdClass) {
            $this->fail('native.structure.root_object', 'Native Game Catalog root must be an object.', '$');
        }

        $this->validateEnvelope($decoded);
        $this->validateDigest($decoded);

        $payload = $this->toArray($decoded);
        /** @var NativeCatalogPayload $payload */

        return new ValidatedNativeCatalogSnapshot(
            payload: $payload,
            artifactSha256: $artifactSha256,
            fileSize: $fileSize,
            sourceLabel: $path,
        );
    }

    /** @return array{string, int} */
    private function readBounded(string $path): array
    {
        $handle = @fopen($path, 'rb');
        if ($handle === false) {
            $this->fail('native.input.unreadable', 'Native Game Catalog artifact is not readable.', '$');
        }

        try {
            $stat = fstat($handle);
            if (! is_array($stat) || ! isset($stat['size']) || ! is_int($stat['size'])) {
                $this->fail('native.input.stat_failed', 'Native Game Catalog artifact size is unavailable.', '$');
            }
            if ($stat['size'] > $this->maximumFileBytes) {
                $this->fail('native.input.file_too_large', 'Native Game Catalog artifact exceeds the file-size limit.', '$');
            }

            $contents = stream_get_contents($handle, $this->maximumFileBytes + 1);
            if (! is_string($contents)) {
                $this->fail('native.input.read_failed', 'Native Game Catalog artifact could not be read.', '$');
            }
            $fileSize = strlen($contents);
            if ($fileSize > $this->maximumFileBytes) {
                $this->fail('native.input.file_too_large', 'Native Game Catalog artifact exceeds the file-size limit.', '$');
            }

            return [$contents, $fileSize];
        } finally {
            fclose($handle);
        }
    }

    private function validateEnvelope(stdClass $payload): void
    {
        $this->assertExactFields($payload, [
            'contract_id',
            'schema_version',
            'snapshot_id',
            'content_authority_id',
            'authority_epoch',
            'source_revision',
            'generated_at',
            'ruleset_id',
            'content_profile_id',
            'required_capabilities',
            'capability_manifest',
            'completeness_manifest',
            'entities',
            'relations',
            'tombstones',
            'payload_digest',
        ], '$');

        if ($payload->contract_id !== NativeCatalogContract::CONTRACT_ID) {
            $this->fail('native.contract.contract_id', 'Unsupported native Game Catalog contract_id.', '$.contract_id');
        }
        if ($payload->schema_version !== NativeCatalogContract::SCHEMA_VERSION) {
            $this->fail('native.contract.schema_version', 'Unsupported native Game Catalog schema_version.', '$.schema_version');
        }
        if ($payload->content_authority_id !== NativeCatalogContract::CONTENT_AUTHORITY_ID) {
            $this->fail('native.contract.authority', 'Unsupported native Game Catalog content authority.', '$.content_authority_id');
        }

        $this->assertString($payload->authority_epoch, '$.authority_epoch', '/^[a-z0-9][a-z0-9._-]{0,127}$/D');
        $this->assertString($payload->source_revision, '$.source_revision', '/^[0-9a-f]{40}$/D');
        $this->assertUtcSeconds($payload->generated_at, '$.generated_at');
        $this->assertContentKey($payload->ruleset_id, '$.ruleset_id');
        $this->assertContentKey($payload->content_profile_id, '$.content_profile_id');
        $this->assertDigest($payload->snapshot_id, '$.snapshot_id');
        $this->assertDigest($payload->payload_digest, '$.payload_digest');

        $required = $this->validateRequiredCapabilities($payload->required_capabilities);
        $support = $this->validateCapabilityManifest($payload->capability_manifest);
        $completeness = $this->validateCompletenessManifest($payload->completeness_manifest);

        if (array_keys($support) !== array_keys($completeness)) {
            $this->fail(
                'native.completeness.coverage_mismatch',
                'Capability and completeness manifests must cover the same canonical capability IDs.',
                '$.completeness_manifest',
            );
        }
        foreach ($support as $capability => $supportState) {
            if ($supportState === 'unsupported' && $completeness[$capability] !== 'unknown') {
                $this->fail(
                    'native.completeness.unsupported_not_unknown',
                    "Unsupported capability {$capability} must have unknown completeness.",
                    '$.completeness_manifest',
                );
            }
        }
        foreach ($required as $capability) {
            if (($support[$capability] ?? null) !== 'supported') {
                $this->fail(
                    'native.capability.required_unsupported',
                    "Required capability {$capability} is not supported by this snapshot.",
                    '$.required_capabilities',
                );
            }
        }

        $entityKeys = $this->validateEntities($payload->entities, $support);
        $this->validateRelations($payload->relations, $support, $entityKeys);
        $this->validateTombstones($payload->tombstones, $support, $completeness, $entityKeys);
    }

    /** @return list<string> */
    private function validateRequiredCapabilities(mixed $value): array
    {
        $values = $this->assertList($value, '$.required_capabilities', NativeCatalogContract::MAX_CAPABILITIES);
        $result = [];
        $seen = [];
        foreach ($values as $index => $raw) {
            $capability = $this->assertCapabilityId($raw, "$.required_capabilities[{$index}]");
            if (isset($seen[$capability])) {
                $this->fail('native.capability.required_duplicate', 'Required capability IDs must be unique.', '$.required_capabilities');
            }
            if (! in_array($capability, NativeCatalogContract::KNOWN_CAPABILITIES, true)) {
                $this->fail('native.capability.unknown', "Unknown v1 capability {$capability}.", "$.required_capabilities[{$index}]");
            }
            $seen[$capability] = true;
            $result[] = $capability;
        }
        $this->assertSortedStrings($result, '$.required_capabilities', 'native.capability.required_ordering');

        return $result;
    }

    /** @return array<string, 'supported'|'unsupported'> */
    private function validateCapabilityManifest(mixed $value): array
    {
        $values = $this->assertList($value, '$.capability_manifest', NativeCatalogContract::MAX_CAPABILITIES);
        $result = [];
        $ordered = [];
        foreach ($values as $index => $raw) {
            $entry = $this->assertObject($raw, "$.capability_manifest[{$index}]");
            $this->assertExactFields($entry, ['capability_id', 'support'], "$.capability_manifest[{$index}]");
            $capability = $this->assertCapabilityId($entry->capability_id, "$.capability_manifest[{$index}].capability_id");
            if (! in_array($capability, NativeCatalogContract::KNOWN_CAPABILITIES, true)) {
                $this->fail('native.capability.unknown', "Unknown v1 capability {$capability}.", "$.capability_manifest[{$index}]");
            }
            if (array_key_exists($capability, $result)) {
                $this->fail('native.capability.duplicate', 'Capability manifest IDs must be unique.', '$.capability_manifest');
            }
            if ($entry->support !== 'supported' && $entry->support !== 'unsupported') {
                $this->fail('native.capability.support_invalid', 'Capability support must be supported or unsupported.', "$.capability_manifest[{$index}].support");
            }
            $result[$capability] = $entry->support;
            $ordered[] = $capability;
        }
        $this->assertSortedStrings($ordered, '$.capability_manifest', 'native.capability.ordering');
        ksort($result, SORT_STRING);

        return $result;
    }

    /** @return array<string, 'complete'|'partial'|'unknown'> */
    private function validateCompletenessManifest(mixed $value): array
    {
        $values = $this->assertList($value, '$.completeness_manifest', NativeCatalogContract::MAX_CAPABILITIES);
        $result = [];
        $ordered = [];
        foreach ($values as $index => $raw) {
            $entry = $this->assertObject($raw, "$.completeness_manifest[{$index}]");
            $this->assertExactFields($entry, ['capability_id', 'state'], "$.completeness_manifest[{$index}]");
            $capability = $this->assertCapabilityId($entry->capability_id, "$.completeness_manifest[{$index}].capability_id");
            if (! in_array($capability, NativeCatalogContract::KNOWN_CAPABILITIES, true)) {
                $this->fail('native.capability.unknown', "Unknown v1 capability {$capability}.", "$.completeness_manifest[{$index}]");
            }
            if (array_key_exists($capability, $result)) {
                $this->fail('native.completeness.duplicate', 'Completeness capability IDs must be unique.', '$.completeness_manifest');
            }
            if (! in_array($entry->state, ['complete', 'partial', 'unknown'], true)) {
                $this->fail('native.completeness.state_invalid', 'Completeness state is invalid.', "$.completeness_manifest[{$index}].state");
            }
            /** @var 'complete'|'partial'|'unknown' $state */
            $state = $entry->state;
            $result[$capability] = $state;
            $ordered[] = $capability;
        }
        $this->assertSortedStrings($ordered, '$.completeness_manifest', 'native.completeness.ordering');
        ksort($result, SORT_STRING);

        return $result;
    }

    /**
     * @param  array<string, 'supported'|'unsupported'>  $support
     * @return array<string, true>
     */
    private function validateEntities(mixed $value, array $support): array
    {
        $values = $this->assertList($value, '$.entities', NativeCatalogContract::MAX_ENTITIES);
        $keys = [];
        $lastSortKey = null;
        foreach ($values as $index => $raw) {
            $path = "$.entities[{$index}]";
            $entity = $this->assertObject($raw, $path);
            $this->assertExactFields($entity, ['type', 'content_key', 'capability_id', 'data'], $path);
            $type = $this->assertCapabilityId($entity->type, "{$path}.type");
            $contentKey = $this->assertContentKey($entity->content_key, "{$path}.content_key");
            $capability = $this->assertCapabilityId($entity->capability_id, "{$path}.capability_id");
            $this->assertSupportedCapability($capability, $support, $path);
            if (isset($keys[$contentKey])) {
                $this->fail('native.entity.duplicate', "Duplicate native entity {$contentKey}.", $path);
            }
            $data = $this->assertObject($entity->data, "{$path}.data");
            $this->validateData($data, "{$path}.data");

            $sortKey = $type."\0".$contentKey;
            if ($lastSortKey !== null && strcmp($lastSortKey, $sortKey) > 0) {
                $this->fail('native.entity.ordering', 'Native entities are not in canonical type/content_key order.', $path);
            }
            $lastSortKey = $sortKey;
            $keys[$contentKey] = true;
        }

        return $keys;
    }

    /**
     * @param  array<string, 'supported'|'unsupported'>  $support
     * @param  array<string, true>  $entityKeys
     */
    private function validateRelations(mixed $value, array $support, array $entityKeys): void
    {
        $values = $this->assertList($value, '$.relations', NativeCatalogContract::MAX_RELATIONS);
        $relationKeys = [];
        $lastSortKey = null;
        foreach ($values as $index => $raw) {
            $path = "$.relations[{$index}]";
            $relation = $this->assertObject($raw, $path);
            $this->assertExactFields(
                $relation,
                ['type', 'relation_key', 'capability_id', 'source', 'target', 'data'],
                $path,
            );
            $type = $this->assertCapabilityId($relation->type, "{$path}.type");
            $relationKey = $this->assertContentKey($relation->relation_key, "{$path}.relation_key");
            $capability = $this->assertCapabilityId($relation->capability_id, "{$path}.capability_id");
            $this->assertSupportedCapability($capability, $support, $path);
            if (isset($relationKeys[$relationKey])) {
                $this->fail('native.relation.duplicate', "Duplicate native relation {$relationKey}.", $path);
            }
            $source = $this->assertContentKey($relation->source, "{$path}.source");
            if (! isset($entityKeys[$source])) {
                $this->fail('native.relation.dangling_source', "Dangling native relation source {$source}.", "{$path}.source");
            }
            $target = null;
            if ($relation->target !== null) {
                $target = $this->assertContentKey($relation->target, "{$path}.target");
                if (! isset($entityKeys[$target])) {
                    $this->fail('native.relation.dangling_target', "Dangling native relation target {$target}.", "{$path}.target");
                }
            }
            $data = $this->assertObject($relation->data, "{$path}.data");
            $this->validateData($data, "{$path}.data");

            $sortKey = $type."\0".$relationKey;
            if ($lastSortKey !== null && strcmp($lastSortKey, $sortKey) > 0) {
                $this->fail('native.relation.ordering', 'Native relations are not in canonical type/relation_key order.', $path);
            }
            $lastSortKey = $sortKey;
            $relationKeys[$relationKey] = true;
        }
    }

    /**
     * @param  array<string, 'supported'|'unsupported'>  $support
     * @param  array<string, 'complete'|'partial'|'unknown'>  $completeness
     * @param  array<string, true>  $entityKeys
     */
    private function validateTombstones(mixed $value, array $support, array $completeness, array $entityKeys): void
    {
        $values = $this->assertList($value, '$.tombstones', NativeCatalogContract::MAX_TOMBSTONES);
        $tombstoneKeys = [];
        $lastKey = null;
        foreach ($values as $index => $raw) {
            $path = "$.tombstones[{$index}]";
            $tombstone = $this->assertObject($raw, $path);
            $this->assertExactFields($tombstone, ['content_key', 'capability_id', 'reason'], $path);
            $contentKey = $this->assertContentKey($tombstone->content_key, "{$path}.content_key");
            $capability = $this->assertCapabilityId($tombstone->capability_id, "{$path}.capability_id");
            $this->assertSupportedCapability($capability, $support, $path);
            if (($completeness[$capability] ?? null) !== 'complete') {
                $this->fail(
                    'native.tombstone.incomplete_capability',
                    "Tombstone {$contentKey} cannot prove absence without complete capability coverage.",
                    $path,
                );
            }
            if (isset($entityKeys[$contentKey])) {
                $this->fail('native.tombstone.contradiction', "Entity {$contentKey} cannot also be tombstoned.", $path);
            }
            if (isset($tombstoneKeys[$contentKey])) {
                $this->fail('native.tombstone.duplicate', "Duplicate tombstone {$contentKey}.", $path);
            }
            $this->assertString($tombstone->reason, "{$path}.reason");
            if ($lastKey !== null && strcmp($lastKey, $contentKey) > 0) {
                $this->fail('native.tombstone.ordering', 'Native tombstones are not in canonical content_key order.', $path);
            }
            $lastKey = $contentKey;
            $tombstoneKeys[$contentKey] = true;
        }
    }

    private function validateData(mixed $value, string $path, int $depth = 0): void
    {
        if ($depth > NativeCatalogContract::MAX_DATA_DEPTH) {
            $this->fail('native.data.depth', 'Native data exceeds the locked nesting-depth limit.', $path);
        }
        if ($value instanceof stdClass) {
            $properties = get_object_vars($value);
            if (count($properties) > NativeCatalogContract::MAX_OBJECT_MEMBERS) {
                $this->fail('native.data.object_members', 'Native data object exceeds the member limit.', $path);
            }
            foreach ($properties as $key => $child) {
                if ($key === '' || strlen($key) > NativeCatalogContract::MAX_STRING_BYTES) {
                    $this->fail('native.data.object_key', 'Native data object key is empty or too large.', $path);
                }
                $this->validateData($child, $path.'.'.$key, $depth + 1);
            }

            return;
        }
        if (is_array($value)) {
            if (count($value) > NativeCatalogContract::MAX_ARRAY_ENTRIES) {
                $this->fail('native.data.array_entries', 'Native data array exceeds the entry limit.', $path);
            }
            foreach ($value as $index => $child) {
                $this->validateData($child, "{$path}[{$index}]", $depth + 1);
            }

            return;
        }
        if (is_float($value)) {
            $this->fail('native.data.float_forbidden', 'Floating-point values are forbidden in native catalog data.', $path);
        }
        if (is_int($value) || is_bool($value) || $value === null) {
            return;
        }
        if (is_string($value)) {
            if (strlen($value) > NativeCatalogContract::MAX_STRING_BYTES) {
                $this->fail('native.data.string_bytes', 'Native data string exceeds the UTF-8 byte limit.', $path);
            }

            return;
        }

        $this->fail('native.data.type', 'Native data contains an unsupported JSON value type.', $path);
    }

    private function validateDigest(stdClass $payload): void
    {
        $integrityPayload = clone $payload;
        unset($integrityPayload->snapshot_id, $integrityPayload->payload_digest);
        $expected = 'sha256:'.hash('sha256', $this->canonicalJson($integrityPayload));
        if (! hash_equals($expected, $payload->payload_digest)) {
            $this->fail('native.digest.mismatch', 'Native Game Catalog payload digest does not match canonical content.', '$.payload_digest');
        }
        if (! hash_equals($expected, $payload->snapshot_id)) {
            $this->fail('native.digest.snapshot_id_mismatch', 'Native Game Catalog snapshot_id must equal the canonical payload digest.', '$.snapshot_id');
        }
    }

    private function canonicalJson(mixed $value): string
    {
        try {
            return json_encode(
                $this->canonicalize($value),
                JSON_THROW_ON_ERROR
                    | JSON_UNESCAPED_SLASHES
                    | JSON_UNESCAPED_UNICODE
                    | JSON_UNESCAPED_LINE_TERMINATORS,
            );
        } catch (JsonException $exception) {
            $this->fail('native.digest.encoding_failed', 'Native Game Catalog canonical encoding failed.', '$', $exception);
        }
    }

    private function canonicalize(mixed $value): mixed
    {
        if ($value instanceof stdClass) {
            $properties = get_object_vars($value);
            ksort($properties, SORT_STRING);
            $object = new stdClass;
            foreach ($properties as $key => $child) {
                $object->{$key} = $this->canonicalize($child);
            }

            return $object;
        }
        if (is_array($value)) {
            $result = [];
            foreach ($value as $index => $child) {
                $result[$index] = $this->canonicalize($child);
            }

            return $result;
        }

        return $value;
    }

    /** @param list<string> $expectedFields */
    private function assertExactFields(stdClass $object, array $expectedFields, string $path): void
    {
        $actual = array_keys(get_object_vars($object));
        sort($actual, SORT_STRING);
        sort($expectedFields, SORT_STRING);
        if ($actual !== $expectedFields) {
            $this->fail('native.structure.fields', 'Native Game Catalog object has missing or unknown fields.', $path);
        }
    }

    /** @return list<mixed> */
    private function assertList(mixed $value, string $path, int $maximum): array
    {
        if (! is_array($value) || ! array_is_list($value)) {
            $this->fail('native.structure.list', 'Native Game Catalog value must be a JSON array.', $path);
        }
        if (count($value) > $maximum) {
            $this->fail('native.structure.count', 'Native Game Catalog collection exceeds its locked entry limit.', $path);
        }

        return $value;
    }

    private function assertObject(mixed $value, string $path): stdClass
    {
        if (! $value instanceof stdClass) {
            $this->fail('native.structure.object', 'Native Game Catalog value must be a JSON object.', $path);
        }

        return $value;
    }

    private function assertString(mixed $value, string $path, ?string $pattern = null): string
    {
        if (! is_string($value) || $value === '') {
            $this->fail('native.string.invalid', 'Native Game Catalog string must be non-empty.', $path);
        }
        if (strlen($value) > NativeCatalogContract::MAX_STRING_BYTES) {
            $this->fail('native.string.bytes', 'Native Game Catalog string exceeds the UTF-8 byte limit.', $path);
        }
        if ($pattern !== null && preg_match($pattern, $value) !== 1) {
            $this->fail('native.string.grammar', 'Native Game Catalog string has invalid grammar.', $path);
        }

        return $value;
    }

    private function assertCapabilityId(mixed $value, string $path): string
    {
        return $this->assertString($value, $path, '/^[a-z][a-z0-9._-]{0,63}$/D');
    }

    private function assertContentKey(mixed $value, string $path): string
    {
        return $this->assertString($value, $path, '/^[a-z][a-z0-9_.-]*:[a-z][a-z0-9_.-]*$/D');
    }

    private function assertDigest(mixed $value, string $path): string
    {
        return $this->assertString($value, $path, '/^sha256:[0-9a-f]{64}$/D');
    }

    private function assertUtcSeconds(mixed $value, string $path): string
    {
        $text = $this->assertString(
            $value,
            $path,
            '/^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}Z$/D',
        );
        $date = DateTimeImmutable::createFromFormat('!Y-m-d\TH:i:s\Z', $text);
        if ($date === false || $date->format('Y-m-d\TH:i:s\Z') !== $text) {
            $this->fail('native.timestamp.invalid', 'Native Game Catalog generated_at is invalid.', $path);
        }

        return $text;
    }

    /** @param list<string> $values */
    private function assertSortedStrings(array $values, string $path, string $code): void
    {
        $sorted = $values;
        sort($sorted, SORT_STRING);
        if ($values !== $sorted) {
            $this->fail($code, 'Native Game Catalog collection is not in canonical order.', $path);
        }
    }

    /** @param array<string, 'supported'|'unsupported'> $support */
    private function assertSupportedCapability(string $capability, array $support, string $path): void
    {
        if (! array_key_exists($capability, $support)) {
            $this->fail('native.capability.undeclared', "Capability {$capability} is not declared by the snapshot.", $path);
        }
        if ($support[$capability] !== 'supported') {
            $this->fail('native.capability.unsupported_record', "Capability {$capability} does not authorize records in this snapshot.", $path);
        }
    }

    /** @return array<string, mixed>|list<mixed>|bool|float|int|string|null */
    private function toArray(mixed $value): array|bool|float|int|string|null
    {
        if ($value instanceof stdClass) {
            $result = [];
            foreach (get_object_vars($value) as $key => $child) {
                $result[$key] = $this->toArray($child);
            }

            return $result;
        }
        if (is_array($value)) {
            $result = [];
            foreach ($value as $index => $child) {
                $result[$index] = $this->toArray($child);
            }

            return $result;
        }
        if (is_bool($value) || is_float($value) || is_int($value) || is_string($value) || $value === null) {
            return $value;
        }

        $this->fail('native.structure.value_type', 'Native Game Catalog decoded value has an unsupported type.', '$');
    }

    private function fail(
        string $code,
        string $message,
        ?string $path = null,
        ?\Throwable $previous = null,
    ): never {
        throw new CatalogValidationException(
            findings: [
                new CatalogValidationFinding(
                    severity: 'error',
                    code: $code,
                    message: $message,
                    path: $path,
                ),
            ],
            previous: $previous,
        );
    }
}
