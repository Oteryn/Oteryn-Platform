<?php

namespace App\Identity\Termination;

use App\Audit\SecurityEventRecorder;
use App\Identity\Actions\RevokeIdentityGameAuthorizations;
use App\Identity\Actions\RevokeIdentityWebSessions;
use App\Identity\Models\Identity;
use App\Identity\Models\IdentityEmailChangeRequest;
use App\Identity\Models\IdentityRecoveryKey;
use App\Identity\Support\IdentitySecret;
use Illuminate\Support\Facades\DB;
use Illuminate\Support\Facades\Hash;

final class FinalizeIdentityTermination
{
    public function __construct(
        private readonly IdentityTerminationGuard $guard,
        private readonly RevokeIdentityWebSessions $webSessions,
        private readonly RevokeIdentityGameAuthorizations $gameAuthorizations,
        private readonly SecurityEventRecorder $securityEvents,
    ) {}

    public function execute(int $identityId): bool
    {
        return DB::transaction(function () use ($identityId): bool {
            $identity = Identity::query()->lockForUpdate()->find($identityId);
            if (! $identity instanceof Identity || $identity->terminated_at !== null) {
                return false;
            }

            if (! $identity->hasPendingTermination() || $identity->termination_scheduled_for?->isFuture() !== false) {
                return false;
            }

            $this->guard->assertAvailable($identity);
            $oldEmail = $identity->email;
            $now = now();

            $identity->forceFill([
                'email' => "terminated+{$identity->id}@invalid.oteryn",
                'password' => Hash::make(IdentitySecret::generate()),
                'disabled_at' => $now,
                'terminated_at' => $now,
                'terminated_email_hash' => IdentitySecret::keyedHash($oldEmail),
                'public_account_association' => false,
                'public_status_visible' => false,
                'two_factor_secret' => null,
                'two_factor_recovery_codes' => null,
                'two_factor_confirmed_at' => null,
                'two_factor_last_used_timestep' => null,
                'email_change_available_at' => null,
            ])->save();

            IdentityEmailChangeRequest::query()
                ->where('identity_id', $identity->id)
                ->whereNull('recovered_at')
                ->whereNull('cancelled_at')
                ->update(['cancelled_at' => $now, 'updated_at' => $now]);
            IdentityRecoveryKey::query()->where('identity_id', $identity->id)->delete();
            DB::table('password_reset_tokens')->where('email', $oldEmail)->delete();

            $this->webSessions->execute($identity);
            $this->gameAuthorizations->execute($identity);
            $this->securityEvents->recordIdentityTerminationFinalized($identity->id);

            return true;
        });
    }
}
