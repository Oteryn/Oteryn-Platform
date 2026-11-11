<?php

namespace App\Identity\Termination;

use App\Audit\SecurityEventRecorder;
use App\Identity\Models\Identity;
use Illuminate\Support\Facades\DB;

final class CancelIdentityTermination
{
    public function __construct(
        private readonly SecurityEventRecorder $securityEvents,
    ) {}

    public function execute(Identity $identity): Identity
    {
        return DB::transaction(function () use ($identity): Identity {
            $locked = Identity::query()->lockForUpdate()->find($identity->id);
            if (! $locked instanceof Identity || $locked->terminated_at !== null) {
                throw new AccountTerminationRejected(__('identity.errors.termination_cannot_cancel'));
            }

            if (! $locked->hasPendingTermination()) {
                throw new AccountTerminationRejected(__('identity.errors.termination_none_pending'));
            }

            $locked->forceFill([
                'termination_requested_at' => null,
                'termination_scheduled_for' => null,
            ])->save();
            $this->securityEvents->recordIdentityTerminationCancelled($locked->id);

            return $locked->refresh();
        });
    }
}
