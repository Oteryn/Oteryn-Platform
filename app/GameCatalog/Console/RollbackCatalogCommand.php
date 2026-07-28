<?php

namespace App\GameCatalog\Console;

use App\GameCatalog\Application\Activation\CatalogActivationService;
use Illuminate\Console\Command;
use Throwable;

final class RollbackCatalogCommand extends Command
{
    protected $signature = 'game-catalog:rollback
        {snapshot-id : Earlier validated immutable snapshot ID}
        {--profile= : Target content profile key}';

    protected $description = 'Reactivate an earlier validated Game Catalog snapshot using normal activation checks';

    public function handle(CatalogActivationService $activation): int
    {
        $profileKey = (string) $this->option('profile');
        $snapshotId = filter_var($this->argument('snapshot-id'), FILTER_VALIDATE_INT, ['options' => ['min_range' => 1]]);
        if ($profileKey === '' || $snapshotId === false) {
            $this->error('A positive snapshot ID and --profile=<profile-key> are required.');

            return self::INVALID;
        }

        try {
            $result = $activation->activate((int) $snapshotId, $profileKey, rollback: true);
        } catch (Throwable $exception) {
            report($exception);
            $this->error($exception->getMessage());

            return self::FAILURE;
        }

        $this->info('Game Catalog profile rolled back transactionally.');
        $this->table(
            ['Profile', 'Previous snapshot', 'Active snapshot', 'Visible entities', 'Visible relations'],
            [[
                $result->profileKey,
                $result->previousSnapshotId ?? 'none',
                $result->snapshotId,
                $result->visibleEntityCount,
                $result->visibleRelationCount,
            ]],
        );

        return self::SUCCESS;
    }
}
