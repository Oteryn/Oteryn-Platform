<?php

namespace App\Console\Commands;

use App\Support\Retention\PruneSupportRetention as Pruner;
use Illuminate\Console\Command;

final class PruneSupportRetention extends Command
{
    protected $signature = 'support:prune-retention {--dry-run}';

    protected $description = 'Apply configured Platform support, report and enforcement retention rules';

    public function handle(Pruner $pruner): int
    {
        $dryRun = (bool) $this->option('dry-run');
        $result = $pruner->execute($dryRun);

        $this->info(sprintf(
            '%s support retention: %d ticket(s), %d report(s), %d enforcement record(s).',
            $dryRun ? 'Dry-run' : 'Applied',
            $result['tickets_deleted'],
            $result['reports_deleted'],
            $result['enforcement_anonymized'],
        ));

        return self::SUCCESS;
    }
}
