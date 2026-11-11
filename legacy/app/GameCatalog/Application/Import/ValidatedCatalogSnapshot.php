<?php

namespace App\GameCatalog\Application\Import;

/**
 * @phpstan-type CatalogRelease array{
 *   key: string,
 *   display_label: string,
 *   major: int,
 *   minor: int,
 *   patch: int,
 *   build: int|null,
 *   release_order: int,
 *   protocol_family: string|null,
 *   released_at: string|null
 * }
 * @phpstan-type CatalogSnapshotMetadata array{
 *   generated_at: string,
 *   canary_commit_sha: string,
 *   datapack_commit_sha: string|null,
 *   protocol_profile: string,
 *   runtime_release: string,
 *   content_target_release: string,
 *   verified_content_through_release: string|null,
 *   contains_content_through_release: string|null,
 *   appearances_sha256: string,
 *   map_sha256: string|null,
 *   producer_build_id: string|null,
 *   entity_count: int,
 *   relation_count: int
 * }
 * @phpstan-type CatalogIdentifier array{namespace: string, value: string}
 * @phpstan-type CatalogItemData array{
 *   server_id: int,
 *   client_id: int|null,
 *   ware_id: int|null,
 *   name: string,
 *   description: string|null,
 *   category: string,
 *   weapon_type: string|null,
 *   attack: int|null,
 *   defense: int|null,
 *   extra_defense: int|null,
 *   armor: int|null,
 *   range: int|null,
 *   weight: int|null,
 *   minimum_level: int|null,
 *   vocations: list<string>|null,
 *   slot_position: int|null,
 *   imbuement_slots: int|null,
 *   upgrade_classification: int|null,
 *   element_type: string|null,
 *   element_value: int|null,
 *   stackable: bool,
 *   pickupable: bool,
 *   image_key: string|null,
 *   attributes: array<array-key, mixed>
 * }
 * @phpstan-type CatalogCreatureData array{
 *   name: string,
 *   description: string|null,
 *   race_id: int|null,
 *   look_type: int|null,
 *   health: int,
 *   max_health: int,
 *   experience: int,
 *   speed: int,
 *   armor: int,
 *   defense: int,
 *   mitigation: int|float|null,
 *   is_boss: bool,
 *   is_reward_boss: bool,
 *   bestiary_class: string|null,
 *   bestiary_race: string|null,
 *   bestiary_occurrence: int|null,
 *   bestiary_to_kill: int|null,
 *   charm_points: int|null,
 *   elements: array<array-key, mixed>,
 *   immunities: array<array-key, mixed>,
 *   attacks: array<array-key, mixed>,
 *   defenses: array<array-key, mixed>,
 *   attributes: array<array-key, mixed>
 * }
 * @phpstan-type CatalogItemEntity array{
 *   type: 'item',
 *   canonical_key: string,
 *   introduced_in: string|null,
 *   removed_in: string|null,
 *   completeness: string,
 *   availability: string,
 *   runtime_present: bool,
 *   enabled: bool,
 *   identifiers: list<CatalogIdentifier>,
 *   source_path: string|null,
 *   data: CatalogItemData
 * }
 * @phpstan-type CatalogCreatureEntity array{
 *   type: 'creature',
 *   canonical_key: string,
 *   introduced_in: string|null,
 *   removed_in: string|null,
 *   completeness: string,
 *   availability: string,
 *   runtime_present: bool,
 *   enabled: bool,
 *   identifiers: list<CatalogIdentifier>,
 *   source_path: string|null,
 *   data: CatalogCreatureData
 * }
 * @phpstan-type CatalogEntity CatalogItemEntity|CatalogCreatureEntity
 * @phpstan-type CatalogRationalLootData array{
 *   chance_numerator: int,
 *   chance_denominator: int,
 *   minimum_count: int,
 *   maximum_count: int,
 *   container_path: string|null,
 *   condition_data: array<array-key, mixed>|null
 * }
 * @phpstan-type CatalogRuntimeThresholdLootData array{
 *   chance_model: 'canary_dynamic_threshold_v1',
 *   chance_threshold: int,
 *   roll_maximum: int,
 *   minimum_count: int,
 *   maximum_count: int,
 *   container_path: string|null,
 *   condition_data: array<array-key, mixed>|null
 * }
 * @phpstan-type CatalogLootData CatalogRationalLootData|CatalogRuntimeThresholdLootData
 * @phpstan-type CatalogRelation array{
 *   type: 'creature_loot',
 *   canonical_key: string,
 *   source: string,
 *   target: string,
 *   introduced_in: string|null,
 *   removed_in: string|null,
 *   completeness: string,
 *   enabled: bool,
 *   source_path: string|null,
 *   data: CatalogLootData
 * }
 * @phpstan-type CatalogPayload array{
 *   contract: string,
 *   schema_version: string,
 *   snapshot: CatalogSnapshotMetadata,
 *   releases: list<CatalogRelease>,
 *   entities: list<CatalogEntity>,
 *   relations: list<CatalogRelation>
 * }
 */
final readonly class ValidatedCatalogSnapshot
{
    /** @param CatalogPayload $payload */
    public function __construct(
        public array $payload,
        public string $contentSha256,
        public string $schemaSha256,
        public int $fileSize,
        public string $sourceLabel,
    ) {}
}
