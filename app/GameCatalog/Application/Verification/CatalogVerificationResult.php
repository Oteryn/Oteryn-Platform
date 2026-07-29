<?php

namespace App\GameCatalog\Application\Verification;

final readonly class CatalogVerificationResult
{
    /**
     * @param  list<string>  $errors
     */
    public function __construct(
        public string $profileKey,
        public ?int $snapshotId,
        public int $projectedEntityCount,
        public int $visibleEntityCount,
        public int $projectedRelationCount,
        public int $visibleRelationCount,
        public array $errors,
    ) {}

    public function isValid(): bool
    {
        return $this->errors === [];
    }
}
