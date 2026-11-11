<?php

namespace App\Identity\Mfa;

use App\Identity\Models\Identity;
use Illuminate\Support\Facades\DB;
use PragmaRX\Google2FA\Google2FA;

final class StartIdentityMfaEnrollment
{
    public function __construct(
        private readonly Google2FA $google2fa,
    ) {}

    public function execute(Identity $identity): Identity
    {
        return DB::transaction(function () use ($identity): Identity {
            $lockedIdentity = Identity::query()
                ->lockForUpdate()
                ->find($identity->id);

            if (! $lockedIdentity instanceof Identity
                || $lockedIdentity->disabled_at !== null
                || $lockedIdentity->hasConfirmedMfa()
            ) {
                throw new MfaStateRejected;
            }

            if ($lockedIdentity->two_factor_secret === null) {
                $lockedIdentity->forceFill([
                    'two_factor_secret' => $this->google2fa->generateSecretKey(),
                    'two_factor_recovery_codes' => null,
                    'two_factor_confirmed_at' => null,
                    'two_factor_last_used_timestep' => null,
                ])->save();
            }

            return $lockedIdentity;
        });
    }
}
