<?php

namespace App\GameCatalog\Application\Activation;

final readonly class CatalogActivationResult
{
    public function __construct(
        public int $profileId,
        public string $profileKey,
        public int $snapshotId,
        public ?int $previousSnapshotId,
        public int $visibleEntityCount,
        public int $visibleRelationCount,
        public bool $rollback,
    ) {}
}
