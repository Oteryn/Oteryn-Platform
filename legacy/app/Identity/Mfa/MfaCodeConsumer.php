<?php

namespace App\Identity\Mfa;

use App\Audit\SecurityEventRecorder;
use App\Identity\Models\Identity;
use Illuminate\Support\Facades\DB;
use Illuminate\Support\Facades\Hash;
use PragmaRX\Google2FA\Google2FA;

final class MfaCodeConsumer
{
    public function __construct(
        private readonly Google2FA $google2fa,
        private readonly MfaRecoveryCodes $recoveryCodes,
        private readonly SecurityEventRecorder $securityEvents,
    ) {}

    public function consumeForPendingLogin(
        int $identityId,
        int $expectedGeneration,
        int $expectedConfirmedAt,
        string $code,
    ): Identity {
        return DB::transaction(function () use (
            $identityId,
            $expectedGeneration,
            $expectedConfirmedAt,
            $code,
        ): Identity {
            $identity = Identity::query()
                ->lockForUpdate()
                ->find($identityId);

            if (! $identity instanceof Identity
                || $identity->disabled_at !== null
                || ! $identity->hasConfirmedMfa()
                || $identity->web_session_generation !== $expectedGeneration
                || $identity->two_factor_confirmed_at?->getTimestamp() !== $expectedConfirmedAt
            ) {
                throw new MfaStateRejected;
            }

            $this->consumeLockedIdentity($identity, $code);

            return $identity;
        });
    }

    public function consumeLockedIdentity(Identity $identity, string $code): MfaFactor
    {
        $secret = $identity->two_factor_secret;

        if ($identity->disabled_at !== null || ! $identity->hasConfirmedMfa() || ! is_string($secret)) {
            throw new MfaStateRejected;
        }

        $trimmedCode = trim($code);

        if (preg_match('/^\d{6}$/D', $trimmedCode) === 1) {
            $oldTimestamp = $identity->two_factor_last_used_timestep ?? 0;
            $matchedTimestamp = $this->google2fa->verifyKeyNewer(
                $secret,
                $trimmedCode,
                $oldTimestamp,
                1,
            );

            if (is_int($matchedTimestamp)) {
                $identity->forceFill([
                    'two_factor_last_used_timestep' => $matchedTimestamp,
                ])->save();

                return MfaFactor::Totp;
            }
        }

        $normalizedRecoveryCode = $this->recoveryCodes->normalize($trimmedCode);
        $storedRecoveryCodes = $identity->two_factor_recovery_codes ?? [];

        foreach ($storedRecoveryCodes as $index => $storedHash) {
            if ($normalizedRecoveryCode !== '' && Hash::check($normalizedRecoveryCode, $storedHash)) {
                unset($storedRecoveryCodes[$index]);
                $identity->forceFill([
                    'two_factor_recovery_codes' => array_values($storedRecoveryCodes),
                ])->save();
                $this->securityEvents->recordIdentityMfaRecoveryCodeUsed($identity->id);

                return MfaFactor::RecoveryCode;
            }
        }

        throw new MfaCodeRejected;
    }
}
