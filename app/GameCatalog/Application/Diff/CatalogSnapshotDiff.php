<?php

namespace App\GameCatalog\Application\Diff;

final readonly class CatalogSnapshotDiff
{
    /**
     * @param  list<string>  $addedEntities
     * @param  list<string>  $removedEntities
     * @param  list<string>  $changedEntities
     * @param  list<string>  $addedRelations
     * @param  list<string>  $removedRelations
     * @param  list<string>  $changedRelations
     */
    public function __construct(
        public int $snapshotA,
        public int $snapshotB,
        public array $addedEntities,
        public array $removedEntities,
        public array $changedEntities,
        public array $addedRelations,
        public array $removedRelations,
        public array $changedRelations,
    ) {}
}
