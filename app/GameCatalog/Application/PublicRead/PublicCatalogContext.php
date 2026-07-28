<?php

namespace App\GameCatalog\Application\PublicRead;

final readonly class PublicCatalogContext
{
    public function __construct(
        public int $profileId,
        public string $profileKey,
        public string $profileName,
        public int $snapshotId,
        public string $snapshotSha256,
        public string $targetRelease,
        public string $runtimeRelease,
        public string $contentTargetRelease,
        public string $verifiedThroughRelease,
        public ?string $containsThroughRelease,
        public string $generatedAt,
    ) {}
}
