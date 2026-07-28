<?php

namespace App\GameCatalog\Application\Import;

final readonly class CatalogImportResult
{
    public function __construct(
        public int $snapshotId,
        public int $importRunId,
        public string $contentSha256,
        public bool $alreadyImported,
    ) {}
}
