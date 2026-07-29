<?php

namespace App\GameCatalog\Console;

use App\GameCatalog\Application\Verification\CatalogVerificationService;
use Illuminate\Console\Command;
use Throwable;

final class VerifyCatalogCommand extends Command
{
    protected $signature = 'game-catalog:verify
        {--profile= : Active content profile key}';

    protected $description = 'Verify an active Game Catalog profile and its visibility projections';

    public function handle(CatalogVerificationService $verification): int
    {
        $profileKey = (string) $this->option('profile');
        if ($profileKey === '') {
            $this->error('--profile=<profile-key> is required.');

            return self::INVALID;
        }

        try {
            $result = $verification->verify($profileKey);
        } catch (Throwable $exception) {
            report($exception);
            $this->error($exception->getMessage());

            return self::FAILURE;
        }

        $this->table(
            ['Profile', 'Snapshot', 'Projected entities', 'Visible entities', 'Projected relations', 'Visible relations'],
            [[
                $result->profileKey,
                $result->snapshotId ?? 'none',
                $result->projectedEntityCount,
                $result->visibleEntityCount,
                $result->projectedRelationCount,
                $result->visibleRelationCount,
            ]],
        );

        if (! $result->isValid()) {
            foreach ($result->errors as $error) {
                $this->error($error);
            }

            return self::FAILURE;
        }

        $this->info('Game Catalog profile verification passed.');

        return self::SUCCESS;
    }
}
