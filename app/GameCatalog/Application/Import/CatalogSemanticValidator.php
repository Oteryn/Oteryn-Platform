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
    /** @param CatalogPayload $payload
     * @return list<CatalogValidationFinding>
     */
    public function validate(array $payload): array
    {
        $findings = [];
        $releases = $this->validateReleases($payload['releases'], $findings);
        $entities = $this->validateEntities($payload['entities'], $releases, $findings);
        $this->validateEntityEndpoints($payload['entities'], $entities, $findings);
        $this->validateRelations($payload['relations'], $releases, $entities, $findings);
        $this->validateSnapshot($payload['snapshot'], $payload, $releases, $findings);

        return $findings;
    }

    /** @param list<CatalogRelease> $releaseRows
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

    /** @param list<CatalogEntity> $entities
     * @param  array<string, int>  $releases
     * @param  list<CatalogValidationFinding>  $findings
     * @return array<string, array{type: string, server_id: int|null}>
     */
    private function validateEntities(array $entities, array $releases, array &$findings): array
    {
        $facts = [];
        $identifiers = [];
        $previous = null;
        foreach ($entities as $index => $entity) {
            $path = '$.entities['.$index.']';
            $key = $entity['canonical_key'];
            $type = $entity['type'];
            if (isset($facts[$key])) {
                $findings[] = $this->finding('semantic.duplicate_entity_key', "Duplicate canonical key '{$key}'.", $path.'.canonical_key');
            }
            $facts[$key] = ['type' => $type, 'server_id' => $type === 'item' ? $entity['data']['server_id'] : null];
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
                    $findings[] = $this->finding('semantic.identifier_collision', 'Identifier resolves to more than one canonical entity.', $identifierPath);
                }
                $identifiers[$identityKey] = $key;
                if ($previousIdentifier !== null && $previousIdentifier >= $pair) {
                    $findings[] = $this->finding('semantic.identifier_ordering', 'Identifiers must be sorted and unique.', $identifierPath);
                }
                $previousIdentifier = $pair;
            }

            $expectedPrefix = match ($type) {
                'item' => 'item:',
                'creature' => 'creature:',
                'npc' => 'npc:',
            };
            if (! str_starts_with($key, $expectedPrefix)) {
                $findings[] = $this->finding('semantic.'.$type.'_key_namespace', ucfirst($type).' canonical keys must use the '.$type.' namespace.', $path.'.canonical_key');
            }

            if ($type === 'npc') {
                $previousAlias = null;
                foreach ($entity['data']['aliases'] as $aliasIndex => $alias) {
                    if ($previousAlias !== null && strcmp($previousAlias, $alias) >= 0) {
                        $findings[] = $this->finding('semantic.npc_alias_ordering', 'NPC aliases must be sorted and unique.', $path.'.data.aliases['.$aliasIndex.']');
                    }
                    $previousAlias = $alias;
                }
            }
        }

        return $facts;
    }

    /** @param list<CatalogEntity> $entities
     * @param  array<string, array{type: string, server_id: int|null}>  $facts
     * @param  list<CatalogValidationFinding>  $findings
     */
    private function validateEntityEndpoints(array $entities, array $facts, array &$findings): void
    {
        foreach ($entities as $index => $entity) {
            if ($entity['type'] !== 'npc') {
                continue;
            }
            $path = '$.entities['.$index.'].data.currency';
            $currency = $entity['data']['currency'];
            $currencyFact = $facts[$currency['item']] ?? null;
            if ($currencyFact === null) {
                $findings[] = $this->finding('semantic.dangling_currency', "NPC currency '{$currency['item']}' is missing.", $path.'.item');

                continue;
            }
            if ($currencyFact['type'] !== 'item') {
                $findings[] = $this->finding('semantic.currency_type', 'NPC currency must resolve to an item.', $path.'.item');

                continue;
            }
            if ($currencyFact['server_id'] !== $currency['server_id']) {
                $findings[] = $this->finding('semantic.currency_server_id', 'NPC currency server_id does not match the item endpoint.', $path.'.server_id');
            }
        }
    }

    /** @param list<CatalogRelation> $relations
     * @param  array<string, int>  $releases
     * @param  array<string, array{type: string, server_id: int|null}>  $entities
     * @param  list<CatalogValidationFinding>  $findings
     */
    private function validateRelations(array $relations, array $releases, array $entities, array &$findings): void
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
            if (! isset($entities[$source])) {
                $findings[] = $this->finding('semantic.dangling_source', "Relation source '{$source}' is missing.", $path.'.source');
            }
            if (! isset($entities[$target])) {
                $findings[] = $this->finding('semantic.dangling_target', "Relation target '{$target}' is missing.", $path.'.target');
            }
            $this->validateRange($relation['introduced_in'], $relation['removed_in'], $releases, $path, $findings);
            $this->validateSourcePath($relation['source_path'], $path.'.source_path', $findings);

            if ($relation['type'] === 'creature_loot') {
                $this->validateLootRelation($relation, $entities, $path, $findings);
            } else {
                $this->validateShopRelation($relation, $entities, $path, $findings);
            }
        }
    }

    /** @param CatalogRelation $relation
     * @param  array<string, array{type: string, server_id: int|null}>  $entities
     * @param  list<CatalogValidationFinding>  $findings
     */
    private function validateLootRelation(array $relation, array $entities, string $path, array &$findings): void
    {
        if (($entities[$relation['source']]['type'] ?? null) !== 'creature') {
            $findings[] = $this->finding('semantic.loot_source_type', 'Creature loot source must resolve to a creature.', $path.'.source');
        }
        if (($entities[$relation['target']]['type'] ?? null) !== 'item') {
            $findings[] = $this->finding('semantic.loot_target_type', 'Creature loot target must resolve to an item.', $path.'.target');
        }
        $data = $relation['data'];
        if (! isset($data['chance_model']) && $data['chance_numerator'] > $data['chance_denominator']) {
            $findings[] = $this->finding('semantic.loot_probability', 'Loot chance numerator cannot exceed its denominator.', $path.'.data');
        }
        if ($data['minimum_count'] > $data['maximum_count']) {
            $findings[] = $this->finding('semantic.loot_count_range', 'Loot minimum_count cannot exceed maximum_count.', $path.'.data');
        }
    }

    /** @param CatalogRelation $relation
     * @param  array<string, array{type: string, server_id: int|null}>  $entities
     * @param  list<CatalogValidationFinding>  $findings
     */
    private function validateShopRelation(array $relation, array $entities, string $path, array &$findings): void
    {
        if (($entities[$relation['source']]['type'] ?? null) !== 'npc') {
            $findings[] = $this->finding('semantic.shop_source_type', 'NPC shop source must resolve to an NPC.', $path.'.source');
        }
        if (($entities[$relation['target']]['type'] ?? null) !== 'item') {
            $findings[] = $this->finding('semantic.shop_target_type', 'NPC shop target must resolve to an item.', $path.'.target');
        }
        $data = $relation['data'];
        $currency = $data['currency'];
        $currencyFact = $entities[$currency['item']] ?? null;
        if ($currencyFact === null) {
            $findings[] = $this->finding('semantic.dangling_currency', "Shop currency '{$currency['item']}' is missing.", $path.'.data.currency.item');
        } elseif ($currencyFact['type'] !== 'item') {
            $findings[] = $this->finding('semantic.currency_type', 'Shop currency must resolve to an item.', $path.'.data.currency.item');
        } elseif ($currencyFact['server_id'] !== $currency['server_id']) {
            $findings[] = $this->finding('semantic.currency_server_id', 'Shop currency server_id does not match the item endpoint.', $path.'.data.currency.server_id');
        }

        $direction = $relation['type'] === 'npc_buy_offer' ? 'buy' : 'sell';
        $runtimePath = implode('.', array_map(static fn (int $part): string => (string) $part, $data['runtime_path']));
        $expectedKey = 'shop:'.$relation['source'].':'.$direction.':'.$relation['target'].':'.$runtimePath;
        if ($relation['canonical_key'] !== $expectedKey) {
            $findings[] = $this->finding('semantic.shop_canonical_identity', 'Shop relation canonical_key does not match direction, endpoints and runtime_path.', $path.'.canonical_key');
        }
        $hasStorage = $data['storage_requirement'] !== null;
        if ($hasStorage && $relation['availability'] !== 'conditional') {
            $findings[] = $this->finding('semantic.shop_condition_availability', 'A storage-conditioned shop offer must use conditional availability.', $path.'.availability');
        }
        if (! $hasStorage && $relation['availability'] === 'conditional') {
            $findings[] = $this->finding('semantic.shop_condition_missing', 'Conditional shop availability requires an exact storage condition in schema 1.3.0.', $path.'.data.storage_requirement');
        }
    }

    /** @param CatalogSnapshotMetadata $snapshot
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
        foreach (['runtime_release', 'content_target_release', 'verified_content_through_release', 'contains_content_through_release'] as $field) {
            $value = $snapshot[$field];
            if ($value !== null && ! isset($releases[$value])) {
                $findings[] = $this->finding('semantic.snapshot_release_reference', "Snapshot references unknown release '{$value}'.", '$.snapshot.'.$field);
            }
        }
    }

    /** @param array<string, int> $releases
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
        return new CatalogValidationFinding('error', $code, $message, $path);
    }
}
