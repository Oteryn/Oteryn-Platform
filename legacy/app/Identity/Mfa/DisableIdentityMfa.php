<?php

namespace App\Identity\Mfa;

use App\Audit\SecurityEventRecorder;
use App\Identity\Actions\RevokeIdentityGameAuthorizations;
use App\Identity\Actions\RevokeIdentityWebSessions;
use App\Identity\Models\Identity;
use Illuminate\Support\Facades\DB;
use Illuminate\Support\Facades\Hash;

final class DisableIdentityMfa
{
    public function __construct(
        private readonly MfaCodeConsumer $codes,
        private readonly RevokeIdentityWebSessions $webSessions,
        private readonly RevokeIdentityGameAuthorizations $gameAuthorizations,
        private readonly SecurityEventRecorder $securityEvents,
    ) {}

    public function execute(Identity $identity, string $currentPassword, string $code): void
    {
        DB::transaction(function () use ($identity, $currentPassword, $code): void {
            $lockedIdentity = Identity::query()
                ->lockForUpdate()
                ->find($identity->id);

            if (! $lockedIdentity instanceof Identity
                || $lockedIdentity->disabled_at !== null
                || ! $lockedIdentity->hasConfirmedMfa()
            ) {
                throw new MfaStateRejected;
            }

            if (! Hash::check($currentPassword, $lockedIdentity->password)) {
                throw new MfaCodeRejected;
            }

            $this->codes->consumeLockedIdentity($lockedIdentity, $code);
            $lockedIdentity->forceFill([
                'two_factor_secret' => null,
                'two_factor_recovery_codes' => null,
                'two_factor_confirmed_at' => null,
                'two_factor_last_used_timestep' => null,
            ])->save();

            $this->webSessions->execute($lockedIdentity);
            $this->gameAuthorizations->execute($lockedIdentity);
            $this->securityEvents->recordIdentityMfaDisabled($lockedIdentity->id);
        });
    }
}
