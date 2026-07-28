<?php

namespace App\GameCatalog\Console;

use App\GameCatalog\Application\Diff\CatalogSnapshotDiffService;
use Illuminate\Console\Command;
use Throwable;

final class DiffCatalogCommand extends Command
{
    protected $signature = 'game-catalog:diff
        {snapshot-a : First immutable snapshot ID}
        {snapshot-b : Second immutable snapshot ID}';

    protected $description = 'Compare version, visibility inputs and typed data hashes between two snapshots';

    public function handle(CatalogSnapshotDiffService $diffService): int
    {
        $snapshotA = filter_var($this->argument('snapshot-a'), FILTER_VALIDATE_INT, ['options' => ['min_range' => 1]]);
        $snapshotB = filter_var($this->argument('snapshot-b'), FILTER_VALIDATE_INT, ['options' => ['min_range' => 1]]);
        if ($snapshotA === false || $snapshotB === false) {
            $this->error('Both snapshot IDs must be positive integers.');

            return self::INVALID;
        }

        try {
            $diff = $diffService->diff((int) $snapshotA, (int) $snapshotB);
        } catch (Throwable $exception) {
            report($exception);
            $this->error($exception->getMessage());

            return self::FAILURE;
        }

        $this->table(
            ['Scope', 'Added', 'Removed', 'Changed'],
            [
                ['entities', count($diff->addedEntities), count($diff->removedEntities), count($diff->changedEntities)],
                ['relations', count($diff->addedRelations), count($diff->removedRelations), count($diff->changedRelations)],
            ],
        );

        foreach ([
            'Added entities' => $diff->addedEntities,
            'Removed entities' => $diff->removedEntities,
            'Changed entities' => $diff->changedEntities,
            'Added relations' => $diff->addedRelations,
            'Removed relations' => $diff->removedRelations,
            'Changed relations' => $diff->changedRelations,
        ] as $label => $keys) {
            if ($keys !== []) {
                $this->line($label.': '.implode(', ', $keys));
            }
        }

        return self::SUCCESS;
    }
}
