<?php

namespace App\GameCatalog\Application\Import\Native;

final readonly class NativeCatalogImportResult
{
    public function __construct(
        public int $importRunId,
        public string $artifactSha256,
        public string $nativeSnapshotId,
        public string $sourceRevision,
        public bool $deduplicated,
    ) {}
}
