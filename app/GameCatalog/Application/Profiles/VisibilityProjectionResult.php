<?php

namespace App\GameCatalog\Application\Profiles;

final readonly class VisibilityProjectionResult
{
    public function __construct(
        public int $visibleEntities,
        public int $hiddenEntities,
        public int $visibleRelations,
        public int $hiddenRelations,
    ) {}
}
