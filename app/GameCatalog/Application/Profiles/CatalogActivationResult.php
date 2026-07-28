<?php

namespace App\GameCatalog\Application\Profiles;

final readonly class CatalogActivationResult
{
    public function __construct(
        public int $profileId,
        public ?int $previousSnapshotId,
        public int $activeSnapshotId,
        public VisibilityProjectionResult $projection,
    ) {}
}
