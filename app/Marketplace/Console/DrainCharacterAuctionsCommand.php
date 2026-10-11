<?php

namespace App\Marketplace\Console;

use App\Marketplace\Actions\ReconcileCharacterAuctions;
use App\Marketplace\Actions\ReportCharacterAuctionDrain;
use Illuminate\Console\Command;
use Throwable;

final class DrainCharacterAuctionsCommand extends Command
{
    protected $signature = 'marketplace:drain {--report-only : Report without running reconciliation} {--limit=100 : Auctions to reconcile in this run}';

    protected $description = 'Reconcile in-flight Character Bazaar auctions and report whether the frozen Bazaar is drained';

    public function handle(ReconcileCharacterAuctions $reconcile, ReportCharacterAuctionDrain $report): int
    {
        $limitOption = $this->option('limit');
        $limit = is_string($limitOption)
            ? filter_var($limitOption, FILTER_VALIDATE_INT, ['options' => ['min_range' => 1, 'max_range' => 1000]])
            : false;

        if ($limit === false) {
            $this->error('The --limit option must be an integer between 1 and 1000.');

            return 1;
        }

        try {
            if ($this->option('report-only') !== true) {
                $reconcile->run($limit);
            }

            $result = $report->run();
        } catch (Throwable) {
            $this->error('Character Bazaar drain could not be completed.');

            return 1;
        }

        $this->line('Listings frozen: '.($result['frozen'] ? 'yes' : 'no'));
        foreach ($result['by_status'] as $status => $total) {
            $this->line(sprintf('%s: %d', $status, $total));
        }
        $this->line('Non-terminal auctions: '.$result['non_terminal']);
        if ($result['latest_ends_at'] !== null) {
            $this->line('Latest active auction ends at: '.$result['latest_ends_at']);
        }

        if (! $result['frozen']) {
            $this->error('New listings are not frozen; the Bazaar cannot be drained.');

            return 2;
        }

        if (! $result['drained']) {
            $this->warn('Not drained.');

            return 2;
        }

        $this->info('Drained: no non-terminal auctions remain.');

        return 0;
    }
}
