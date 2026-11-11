<?php

namespace App\GameAuth\NativeLogin;

/**
 * Character and route facts for one grant, produced only by a NativeAdmissionScopeResolver from
 * Platform's Character read model (§5.4) and the World Registry route record plus the selected
 * runtime report (§7). Never built from client input.
 */
final readonly class NativeAdmissionScope
{
    public function __construct(
        public string $characterId,
        public string $worldId,
        public string $channelId,
        public string $routeRevision,
        public string $runtimeObservationRevision,
        public string $scopeOwnershipGeneration,
        public string $rulesetRevision,
        public string $contentRevision,
        public string $mapRevision,
        public string $worldPolicyRevision,
        public string $offerRevision,
    ) {}
}
