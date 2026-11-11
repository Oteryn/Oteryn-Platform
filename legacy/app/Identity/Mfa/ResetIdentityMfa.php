<?php

namespace App\Identity\Mfa;

use App\Audit\SecurityEventRecorder;
use App\Identity\Actions\RevokeIdentityGameAuthorizations;
use App\Identity\Actions\RevokeIdentityWebSessions;
use App\Identity\Models\Identity;
use Illuminate\Support\Facades\DB;

final class ResetIdentityMfa
{
    public function __construct(
        private readonly RevokeIdentityWebSessions $webSessions,
        private readonly RevokeIdentityGameAuthorizations $gameAuthorizations,
        private readonly SecurityEventRecorder $securityEvents,
    ) {}

    public function execute(Identity $identity): void
    {
        DB::transaction(function () use ($identity): void {
            $identity->forceFill([
                'two_factor_secret' => null,
                'two_factor_recovery_codes' => null,
                'two_factor_confirmed_at' => null,
                'two_factor_last_used_timestep' => null,
            ])->save();

            $this->webSessions->execute($identity);
            $this->gameAuthorizations->execute($identity);
            $this->securityEvents->recordIdentityMfaReset($identity->id);
        });
    }
}
