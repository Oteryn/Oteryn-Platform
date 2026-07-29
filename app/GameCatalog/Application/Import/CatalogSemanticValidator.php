<?php

namespace App\GameCatalog\Application\Import;

use App\GameCatalog\Domain\CatalogValidationFinding;

/**
 * @phpstan-import-type CatalogPayload from ValidatedCatalogSnapshot
 * @phpstan-import-type CatalogRelease from ValidatedCatalogSnapshot
 * @phpstan-import-type CatalogSnapshotMetadata from ValidatedCatalogSnapshot
 * @phpstan-import-type CatalogEntity from ValidatedCatalogSnapshot
 * @phpstan-import-type CatalogRelation from ValidatedCatalogSnapshot
 */
final class CatalogSemanticValidator
{
    /**
     * @param  CatalogPayload  $payload
     * @return list<CatalogValidationFinding>
     */
    public function validate(array $payload): array
    {
        $findings = [];
        $releases = $this->validateReleases($payload['releases'], $findings);
        $entityTypes = $this->validateEntities($payload['entities'], $releases, $findings);
        $this->validateRelations($payload['relations'], $releases, $entityTypes, $findings);
        $this->validateSnapshot($payload['snapshot'], $payload, $releases, $findings);

        return $findings;
    }

    /**
     * @param  list<CatalogRelease>  $releaseRows
     * @param  list<CatalogValidationFinding>  $findings
     * @return array<string, int>
     */
    private function validateReleases(array $releaseRows, array &$findings): array
    {
        $orders = [];
        $usedOrders = [];
        $previous = null;

        foreach ($releaseRows as $index => $release) {
            $path = '$.releases['.$index.']';
            $key = $release['key'];
            $order = $release['release_order'];

            if (isset($orders[$key])) {
                $findings[] = $this->finding('semantic.duplicate_release_key', "Duplicate release key '{$key}'.", $path.'.key');
            }
            if (isset($usedOrders[$order])) {
                $findings[] = $this->finding('semantic.duplicate_release_order', "Duplicate release_order '{$order}'.", $path.'.release_order');
            }

            $orders[$key] = $order;
            $usedOrders[$order] = true;
            $current = [$order, $key];
            if ($previous !== null && $previous >= $current) {
                $findings[] = $this->finding('semantic.release_ordering', 'Releases must be sorted by release_order and key.', $path);
            }
            $previous = $current;
        }

        return $orders;
    }

    /**
     * @param  list<CatalogEntity>  $entities
     * @param  array<string, int>  $releases
     * @param  list<CatalogValidationFinding>  $findings
     * @return array<string, string>
     */
    private function validateEntities(array $entities, array $releases, array &$findings): array
    {
        $types = [];
        $identifiers = [];
        $previous = null;

        foreach ($entities as $index => $entity) {
            $path = '$.entities['.$index.']';
            $key = $entity['canonical_key'];
            $type = $entity['type'];

            if (isset($types[$key])) {
                $findings[] = $this->finding('semantic.duplicate_entity_key', "Duplicate canonical key '{$key}'.", $path.'.canonical_key');
            }
            $types[$key] = $type;

            $current = [$type, $key];
            if ($previous !== null && $previous >= $current) {
                $findings[] = $this->finding('semantic.entity_ordering', 'Entities must be sorted by type and canonical_key.', $path);
            }
            $previous = $current;

            $this->validateRange($entity['introduced_in'], $entity['removed_in'], $releases, $path, $findings);
            $this->validateSourcePath($entity['source_path'], $path.'.source_path', $findings);

            $previousIdentifier = null;
            foreach ($entity['identifiers'] as $identifierIndex => $identifier) {
                $identifierPath = $path.'.identifiers['.$identifierIndex.']';
                $pair = [$identifier['namespace'], $identifier['value']];
                $identityKey = $identifier['namespace']."\0".$identifier['value'];

                if (isset($identifiers[$identityKey]) && $identifiers[$identityKey] !== $key) {
                    $findings[] = $this->finding(
                        'semantic.identifier_collision',
                        'Identifier resolves to more than one canonical entity.',
                        $identifierPath,
                    );
                }
                $identifiers[$identityKey] = $key;

                if ($previousIdentifier !== null && $previousIdentifier >= $pair) {
                    $findings[] = $this->finding('semantic.identifier_ordering', 'Identifiers must be sorted and unique.', $identifierPath);
                }
                $previousIdentifier = $pair;
            }

            if ($type === 'item' && ! str_starts_with($key, 'item:')) {
                $findings[] = $this->finding('semantic.item_key_namespace', 'Item canonical keys must use the item namespace.', $path.'.canonical_key');
            }
            if ($type === 'creature' && ! str_starts_with($key, 'creature:')) {
                $findings[] = $this->finding('semantic.creature_key_namespace', 'Creature canonical keys must use the creature namespace.', $path.'.canonical_key');
            }
        }

        return $types;
    }

    /**
     * @param  list<CatalogRelation>  $relations
     * @param  array<string, int>  $releases
     * @param  array<string, string>  $entityTypes
     * @param  list<CatalogValidationFinding>  $findings
     */
    private function validateRelations(array $relations, array $releases, array $entityTypes, array &$findings): void
    {
        $keys = [];
        $previous = null;

        foreach ($relations as $index => $relation) {
            $path = '$.relations['.$index.']';
            $key = $relation['canonical_key'];

            if (isset($keys[$key])) {
                $findings[] = $this->finding('semantic.duplicate_relation_key', "Duplicate relation key '{$key}'.", $path.'.canonical_key');
            }
            $keys[$key] = true;

            $current = [$relation['type'], $key];
            if ($previous !== null && $previous >= $current) {
                $findings[] = $this->finding('semantic.relation_ordering', 'Relations must be sorted by type and canonical_key.', $path);
            }
            $previous = $current;

            $source = $relation['source'];
            $target = $relation['target'];
            if (! isset($entityTypes[$source])) {
                $findings[] = $this->finding('semantic.dangling_source', "Relation source '{$source}' is missing.", $path.'.source');
            }
            if (! isset($entityTypes[$target])) {
                $findings[] = $this->finding('semantic.dangling_target', "Relation target '{$target}' is missing.", $path.'.target');
            }
            if (($entityTypes[$source] ?? null) !== 'creature') {
                $findings[] = $this->finding('semantic.loot_source_type', 'Creature loot source must resolve to a creature.', $path.'.source');
            }
            if (($entityTypes[$target] ?? null) !== 'item') {
                $findings[] = $this->finding('semantic.loot_target_type', 'Creature loot target must resolve to an item.', $path.'.target');
            }

            $this->validateRange($relation['introduced_in'], $relation['removed_in'], $releases, $path, $findings);
            $this->validateSourcePath($relation['source_path'], $path.'.source_path', $findings);

            $data = $relation['data'];
            if (! isset($data['chance_model']) && $data['chance_numerator'] > $data['chance_denominator']) {
                $findings[] = $this->finding('semantic.loot_probability', 'Loot chance numerator cannot exceed its denominator.', $path.'.data');
            }
            if ($data['minimum_count'] > $data['maximum_count']) {
                $findings[] = $this->finding('semantic.loot_count_range', 'Loot minimum_count cannot exceed maximum_count.', $path.'.data');
            }
        }
    }

    /**
     * @param  CatalogSnapshotMetadata  $snapshot
     * @param  CatalogPayload  $payload
     * @param  array<string, int>  $releases
     * @param  list<CatalogValidationFinding>  $findings
     */
    private function validateSnapshot(array $snapshot, array $payload, array $releases, array &$findings): void
    {
        if ($snapshot['entity_count'] !== count($payload['entities'])) {
            $findings[] = $this->finding('semantic.entity_count', 'Declared entity_count does not match the payload.', '$.snapshot.entity_count');
        }
        if ($snapshot['relation_count'] !== count($payload['relations'])) {
            $findings[] = $this->finding('semantic.relation_count', 'Declared relation_count does not match the payload.', '$.snapshot.relation_count');
        }

        foreach ([
            'runtime_release',
            'content_target_release',
            'verified_content_through_release',
            'contains_content_through_release',
        ] as $field) {
            $value = $snapshot[$field];
            if ($value !== null && ! isset($releases[$value])) {
                $findings[] = $this->finding('semantic.snapshot_release_reference', "Snapshot references unknown release '{$value}'.", '$.snapshot.'.$field);
            }
        }
    }

    /**
     * @param  array<string, int>  $releases
     * @param  list<CatalogValidationFinding>  $findings
     */
    private function validateRange(?string $introduced, ?string $removed, array $releases, string $path, array &$findings): void
    {
        if ($introduced !== null && ! isset($releases[$introduced])) {
            $findings[] = $this->finding('semantic.unknown_introduced_release', "Unknown introduced_in release '{$introduced}'.", $path.'.introduced_in');
        }
        if ($removed !== null && ! isset($releases[$removed])) {
            $findings[] = $this->finding('semantic.unknown_removed_release', "Unknown removed_in release '{$removed}'.", $path.'.removed_in');
        }
        if ($introduced !== null && $removed !== null && isset($releases[$introduced], $releases[$removed]) && $releases[$introduced] >= $releases[$removed]) {
            $findings[] = $this->finding('semantic.invalid_version_range', 'removed_in is an exclusive upper bound and must be later than introduced_in.', $path);
        }
    }

    /** @param list<CatalogValidationFinding> $findings */
    private function validateSourcePath(?string $sourcePath, string $path, array &$findings): void
    {
        if ($sourcePath === null) {
            return;
        }

        $normalized = str_replace('\\', '/', $sourcePath);
        $segments = explode('/', $normalized);
        $hasWindowsDrive = preg_match('/^[A-Za-z]:/', $normalized) === 1;

        if ($normalized === '' || str_starts_with($normalized, '/') || $hasWindowsDrive || in_array('..', $segments, true)) {
            $findings[] = $this->finding('semantic.unsafe_source_path', 'source_path must be a sanitized relative path.', $path);
        }
    }

    private function finding(string $code, string $message, string $path): CatalogValidationFinding
    {
        return new CatalogValidationFinding(
            severity: 'error',
            code: $code,
            message: $message,
            path: $path,
        );
    }
}
