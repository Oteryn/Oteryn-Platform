<?php

namespace App\Identity\Mfa;

use App\Audit\SecurityEventRecorder;
use App\Identity\Actions\RevokeIdentityGameAuthorizations;
use App\Identity\Actions\RevokeIdentityWebSessions;
use App\Identity\Models\Identity;
use Illuminate\Support\Facades\DB;
use Illuminate\Support\Facades\Hash;
use PragmaRX\Google2FA\Google2FA;

final class ConfirmIdentityMfaEnrollment
{
    public function __construct(
        private readonly Google2FA $google2fa,
        private readonly MfaRecoveryCodes $recoveryCodes,
        private readonly RevokeIdentityWebSessions $webSessions,
        private readonly RevokeIdentityGameAuthorizations $gameAuthorizations,
        private readonly SecurityEventRecorder $securityEvents,
    ) {}

    public function execute(
        Identity $identity,
        string $currentPassword,
        string $code,
    ): MfaEnrollmentConfirmation {
        return DB::transaction(function () use ($identity, $currentPassword, $code): MfaEnrollmentConfirmation {
            $lockedIdentity = Identity::query()
                ->lockForUpdate()
                ->find($identity->id);

            if (! $lockedIdentity instanceof Identity
                || $lockedIdentity->disabled_at !== null
                || $lockedIdentity->hasConfirmedMfa()
                || ! is_string($lockedIdentity->two_factor_secret)
            ) {
                throw new MfaStateRejected;
            }

            if (! Hash::check($currentPassword, $lockedIdentity->password)) {
                throw new MfaCodeRejected;
            }

            $matchedTimestamp = $this->google2fa->verifyKeyNewer(
                $lockedIdentity->two_factor_secret,
                trim($code),
                0,
                1,
            );

            if (! is_int($matchedTimestamp)) {
                throw new MfaCodeRejected;
            }

            $recoveryCodes = $this->recoveryCodes->generate();

            $lockedIdentity->forceFill([
                'two_factor_recovery_codes' => $recoveryCodes['hashes'],
                'two_factor_confirmed_at' => now(),
                'two_factor_last_used_timestep' => $matchedTimestamp,
            ])->save();

            $this->webSessions->execute($lockedIdentity);
            $this->gameAuthorizations->execute($lockedIdentity);
            $this->securityEvents->recordIdentityMfaEnrolled($lockedIdentity->id);

            return new MfaEnrollmentConfirmation(
                $lockedIdentity,
                $recoveryCodes['plain'],
            );
        });
    }
}
