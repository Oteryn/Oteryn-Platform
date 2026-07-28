<?php

namespace App\GameCatalog\Application\Activation;

final readonly class VisibilityProjectionResult
{
    public function __construct(
        public int $visibleEntityCount,
        public int $visibleRelationCount,
    ) {}
}
