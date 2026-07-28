<?php

namespace App\Identity\Termination;

use App\Audit\SecurityEventRecorder;
use App\Identity\Actions\RevokeIdentityGameAuthorizations;
use App\Identity\Actions\RevokeIdentityWebSessions;
use App\Identity\Models\Identity;
use Illuminate\Support\Facades\DB;
use LogicException;

final class RequestIdentityTermination
{
    public function __construct(
        private readonly IdentityTerminationGuard $guard,
        private readonly RevokeIdentityWebSessions $webSessions,
        private readonly RevokeIdentityGameAuthorizations $gameAuthorizations,
        private readonly SecurityEventRecorder $securityEvents,
    ) {}

    public function execute(Identity $identity): Identity
    {
        return DB::transaction(function () use ($identity): Identity {
            $locked = Identity::query()->lockForUpdate()->find($identity->id);
            if (! $locked instanceof Identity || $locked->disabled_at !== null || $locked->terminated_at !== null) {
                throw new AccountTerminationRejected(__('identity.errors.termination_unavailable'));
            }

            if ($locked->hasPendingTermination()) {
                return $locked;
            }

            $this->guard->assertAvailable($locked);
            $graceDays = config('identity_security.termination.grace_days');
            if (! is_int($graceDays) || $graceDays < 1 || $graceDays > 90) {
                throw new LogicException('The account termination grace period is invalid.');
            }

            $now = now();
            $locked->forceFill([
                'termination_requested_at' => $now,
                'termination_scheduled_for' => $now->copy()->addDays($graceDays),
            ])->save();

            $this->webSessions->execute($locked);
            $this->gameAuthorizations->execute($locked);
            $this->securityEvents->recordIdentityTerminationRequested($locked->id);

            return $locked->refresh();
        });
    }
}
