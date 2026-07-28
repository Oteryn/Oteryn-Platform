<?php

namespace App\Console\Commands;

use App\Identity\Models\Identity;
use App\Identity\Termination\AccountTerminationRejected;
use App\Identity\Termination\FinalizeIdentityTermination;
use Illuminate\Console\Command;
use Throwable;

final class FinalizeIdentityTerminations extends Command
{
    protected $signature = 'identity:finalize-terminations {--limit=100}';

    protected $description = 'Finalize due Platform Identity termination requests without deleting Canary data';

    public function handle(FinalizeIdentityTermination $finalize): int
    {
        $limit = filter_var($this->option('limit'), FILTER_VALIDATE_INT, [
            'options' => ['min_range' => 1, 'max_range' => 1000],
        ]);
        if ($limit === false) {
            $this->error('The --limit option must be an integer between 1 and 1000.');

            return self::INVALID;
        }

        $identities = Identity::query()
            ->whereNull('terminated_at')
            ->whereNotNull('termination_scheduled_for')
            ->where('termination_scheduled_for', '<=', now())
            ->orderBy('id')
            ->limit($limit)
            ->get(['id']);

        $finalized = 0;
        $blocked = 0;
        $failed = 0;

        foreach ($identities as $identity) {
            try {
                if ($finalize->execute($identity->id)) {
                    $finalized++;
                }
            } catch (AccountTerminationRejected) {
                $blocked++;
            } catch (Throwable) {
                $failed++;
            }
        }

        $this->info(sprintf(
            'Processed %d due termination request(s): %d finalized, %d blocked, %d failed.',
            $identities->count(),
            $finalized,
            $blocked,
            $failed,
        ));

        return $blocked === 0 && $failed === 0 ? self::SUCCESS : self::FAILURE;
    }
}
