<?php

namespace App\GameCatalog\Application\Import\Native;

/**
 * @phpstan-type NativeCapabilitySupport array{capability_id: string, support: 'supported'|'unsupported'}
 * @phpstan-type NativeCompleteness array{capability_id: string, state: 'complete'|'partial'|'unknown'}
 * @phpstan-type NativeEntity array{type: string, content_key: string, capability_id: string, data: array<string, mixed>}
 * @phpstan-type NativeRelation array{type: string, relation_key: string, capability_id: string, source: string, target: string|null, data: array<string, mixed>}
 * @phpstan-type NativeTombstone array{content_key: string, capability_id: string, reason: string}
 * @phpstan-type NativeCatalogPayload array{
 *   contract_id: string,
 *   schema_version: string,
 *   snapshot_id: string,
 *   content_authority_id: string,
 *   authority_epoch: string,
 *   source_revision: string,
 *   generated_at: string,
 *   ruleset_id: string,
 *   content_profile_id: string,
 *   required_capabilities: list<string>,
 *   capability_manifest: list<NativeCapabilitySupport>,
 *   completeness_manifest: list<NativeCompleteness>,
 *   entities: list<NativeEntity>,
 *   relations: list<NativeRelation>,
 *   tombstones: list<NativeTombstone>,
 *   payload_digest: string
 * }
 */final readonly class ValidatedNativeCatalogSnapshot
{
    /** @param NativeCatalogPayload $payload */
    public function __construct(
        public array $payload,
        public string $artifactSha256,
        public int $fileSize,
        public string $sourceLabel,
    ) {}
}
