<?php

namespace App\GameAuth\NativeRuntimeStatus;

/**
 * A fresh, ready, ownership-bound runtime report for one scope: the runtime input of native route
 * selection (login contract §7.4 conditions 3 to 5). It carries no endpoint; the route comes from
 * the World Registry route record, whose `route_revision` must equal this one.
 */
final readonly class NativeRuntimeStatusRecord
{
    public function __construct(
        public string $worldId,
        public string $channelId,
        public string $routeRevision,
        public int $protocolMajor,
        public int $transportProfile,
        public string $scopeOwnershipGeneration,
        public string $runtimeObservationRevision,
        public string $rulesetRevision,
        public string $contentRevision,
        public string $mapRevision,
        public string $worldPolicyRevision,
        public string $offerRevision,
        public int $observedAt,
    ) {}
}
