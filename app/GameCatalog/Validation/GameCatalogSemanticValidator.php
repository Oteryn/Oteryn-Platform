<?php

namespace App\GameCatalog\Validation;

final class GameCatalogSemanticValidator
{
    /**
     * @param  array<string, mixed>  $document
     * @return list<array{code: string, path: string, message: string}>
     */
    public function validate(array $document): array
    {
        /** @var list<array{code: string, path: string, message: string}> $findings */
        $findings = [];
        $releases = $this->releaseRegistry($document['releases'] ?? [], $findings);
        $entities = is_array($document['entities'] ?? null) ? $document['entities'] : [];
        $relations = is_array($document['relations'] ?? null) ? $document['relations'] : [];
        $snapshot = is_array($document['snapshot'] ?? null) ? $document['snapshot'] : [];

        if (($snapshot['entity_count'] ?? null) !== count($entities)) {
            $findings[] = $this->finding('semantic.entity_count', '$/snapshot/entity_count', 'Declared entity count does not match the document.');
        }
        if (($snapshot['relation_count'] ?? null) !== count($relations)) {
            $findings[] = $this->finding('semantic.relation_count', '$/snapshot/relation_count', 'Declared relation count does not match the document.');
        }

        foreach (['runtime_release', 'content_target_release', 'verified_content_through_release', 'contains_content_through_release'] as $field) {
            $reference = $snapshot[$field] ?? null;
            if ($reference !== null && (! is_string($reference) || ! isset($releases[$reference]))) {
                $findings[] = $this->finding('semantic.unknown_release', '$/snapshot/'.$field, "Snapshot field [{$field}] references an unknown release.");
            }
        }

        $entityKeys = [];
        $entitySortKeys = [];
        foreach ($entities as $index => $entity) {
            if (! is_array($entity)) {
                continue;
            }

            $type = $entity['type'] ?? null;
            $key = $entity['canonical_key'] ?? null;
            if (is_string($type) && is_string($key)) {
                $identity = $type.'|'.$key;
                if (isset($entityKeys[$identity])) {
                    $findings[] = $this->finding('semantic.duplicate_entity', "$/entities/{$index}/canonical_key", 'Duplicate entity canonical key for the same type.');
                }
                $entityKeys[$identity] = true;
                $entityKeys[$key] = true;
                $entitySortKeys[] = $type.'|'.$key;
            }

            $this->validateRange($entity, "$/entities/{$index}", $releases, $findings);
            $this->validateIdentifiers($entity['identifiers'] ?? [], "$/entities/{$index}/identifiers", $findings);
            $this->validateSourcePath($entity['source_path'] ?? null, "$/entities/{$index}/source_path", $findings);
        }
        if ($entitySortKeys !== $this->sorted($entitySortKeys)) {
            $findings[] = $this->finding('semantic.entity_order', '$/entities', 'Entities are not sorted by type and canonical key.');
        }

        $relationKeys = [];
        $relationSortKeys = [];
        foreach ($relations as $index => $relation) {
            if (! is_array($relation)) {
                continue;
            }

            $key = $relation['canonical_key'] ?? null;
            if (is_string($key)) {
                if (isset($relationKeys[$key])) {
                    $findings[] = $this->finding('semantic.duplicate_relation', "$/relations/{$index}/canonical_key", 'Duplicate relation canonical key.');
                }
                $relationKeys[$key] = true;
                $relationSortKeys[] = ($relation['type'] ?? '').'|'.$key;
            }

            foreach (['source', 'target'] as $endpoint) {
                $endpointKey = $relation[$endpoint] ?? null;
                if (! is_string($endpointKey) || ! isset($entityKeys[$endpointKey])) {
                    $findings[] = $this->finding('semantic.dangling_relation', "$/relations/{$index}/{$endpoint}", "Relation {$endpoint} does not resolve to an entity.");
                }
            }

            $this->validateRange($relation, "$/relations/{$index}", $releases, $findings);
            $this->validateSourcePath($relation['source_path'] ?? null, "$/relations/{$index}/source_path", $findings);

            $data = is_array($relation['data'] ?? null) ? $relation['data'] : [];
            $numerator = $data['chance_numerator'] ?? null;
            $denominator = $data['chance_denominator'] ?? null;
            if (is_int($numerator) && is_int($denominator) && $numerator > $denominator) {
                $findings[] = $this->finding('semantic.invalid_probability', "$/relations/{$index}/data/chance_numerator", 'Loot chance numerator exceeds its denominator.');
            }
            $minimum = $data['minimum_count'] ?? null;
            $maximum = $data['maximum_count'] ?? null;
            if (is_int($minimum) && is_int($maximum) && $maximum < $minimum) {
                $findings[] = $this->finding('semantic.invalid_count_range', "$/relations/{$index}/data/maximum_count", 'Loot maximum count is lower than minimum count.');
            }
        }
        if ($relationSortKeys !== $this->sorted($relationSortKeys)) {
            $findings[] = $this->finding('semantic.relation_order', '$/relations', 'Relations are not sorted by type and canonical key.');
        }

        return $findings;
    }

    /**
     * @param  mixed  $releaseRows
     * @param  list<array{code: string, path: string, message: string}>  $findings
     * @return array<string, int>
     */
    private function releaseRegistry(mixed $releaseRows, array &$findings): array
    {
        $registry = [];
        $orders = [];
        $sortKeys = [];
        if (! is_array($releaseRows)) {
            return $registry;
        }

        foreach ($releaseRows as $index => $release) {
            if (! is_array($release)) {
                continue;
            }
            $key = $release['key'] ?? null;
            $order = $release['release_order'] ?? null;
            if (! is_string($key) || ! is_int($order)) {
                continue;
            }
            if (isset($registry[$key])) {
                $findings[] = $this->finding('semantic.duplicate_release', "$/releases/{$index}/key", 'Duplicate release key.');
            }
            if (isset($orders[$order])) {
                $findings[] = $this->finding('semantic.duplicate_release_order', "$/releases/{$index}/release_order", 'Duplicate release_order.');
            }
            $registry[$key] = $order;
            $orders[$order] = true;
            $sortKeys[] = sprintf('%020d|%s', $order, $key);
        }

        if ($sortKeys !== $this->sorted($sortKeys)) {
            $findings[] = $this->finding('semantic.release_order', '$/releases', 'Releases are not sorted by release_order and key.');
        }

        return $registry;
    }

    /**
     * @param  array<string, mixed>  $record
     * @param  array<string, int>  $releases
     * @param  list<array{code: string, path: string, message: string}>  $findings
     */
    private function validateRange(array $record, string $path, array $releases, array &$findings): void
    {
        $introduced = $record['introduced_in'] ?? null;
        $removed = $record['removed_in'] ?? null;

        foreach (['introduced_in' => $introduced, 'removed_in' => $removed] as $field => $reference) {
            if ($reference !== null && (! is_string($reference) || ! isset($releases[$reference]))) {
                $findings[] = $this->finding('semantic.unknown_release', $path.'/'.$field, "Version range references unknown release [{$field}].");
            }
        }

        if (is_string($introduced) && is_string($removed) && isset($releases[$introduced], $releases[$removed]) && $releases[$removed] <= $releases[$introduced]) {
            $findings[] = $this->finding('semantic.invalid_version_range', $path.'/removed_in', 'removed_in must be an exclusive release later than introduced_in.');
        }
    }

    /**
     * @param  mixed  $identifiers
     * @param  list<array{code: string, path: string, message: string}>  $findings
     */
    private function validateIdentifiers(mixed $identifiers, string $path, array &$findings): void
    {
        if (! is_array($identifiers)) {
            return;
        }

        $seen = [];
        $sortKeys = [];
        foreach ($identifiers as $index => $identifier) {
            if (! is_array($identifier)) {
                continue;
            }
            $namespace = $identifier['namespace'] ?? null;
            $value = $identifier['value'] ?? null;
            if (! is_string($namespace) || ! is_string($value)) {
                continue;
            }
            $key = $namespace.'|'.$value;
            if (isset($seen[$key])) {
                $findings[] = $this->finding('semantic.duplicate_identifier', $path.'/'.$index, 'Duplicate namespaced identifier.');
            }
            $seen[$key] = true;
            $sortKeys[] = $key;
        }

        if ($sortKeys !== $this->sorted($sortKeys)) {
            $findings[] = $this->finding('semantic.identifier_order', $path, 'Identifiers are not sorted by namespace and value.');
        }
    }

    /**
     * @param  list<array{code: string, path: string, message: string}>  $findings
     */
    private function validateSourcePath(mixed $sourcePath, string $path, array &$findings): void
    {
        if (! is_string($sourcePath)) {
            return;
        }

        if ($sourcePath === '' || str_contains($sourcePath, '\\') || preg_match('/^[A-Za-z]:/', $sourcePath) === 1) {
            $findings[] = $this->finding('semantic.unsafe_source_path', $path, 'Source path must be a non-empty portable repository-relative path.');
        }
    }

    /** @param list<string> $values @return list<string> */
    private function sorted(array $values): array
    {
        $sorted = $values;
        sort($sorted, SORT_STRING);

        return $sorted;
    }

    /** @return array{code: string, path: string, message: string} */
    private function finding(string $code, string $path, string $message): array
    {
        return compact('code', 'path', 'message');
    }
}
