<?php

namespace App\Identity\Email;

use App\Audit\SecurityEventRecorder;
use App\Identity\Actions\RevokeIdentityGameAuthorizations;
use App\Identity\Actions\RevokeIdentityWebSessions;
use App\Identity\Models\Identity;
use App\Identity\Models\IdentityEmailChangeRequest;
use App\Identity\Support\IdentitySecret;
use Illuminate\Database\QueryException;
use Illuminate\Support\Facades\DB;

final class RecoverIdentityEmailChange
{
    public function __construct(
        private readonly RevokeIdentityWebSessions $webSessions,
        private readonly RevokeIdentityGameAuthorizations $gameAuthorizations,
        private readonly SecurityEventRecorder $securityEvents,
    ) {}

    /** @return 'cancelled'|'recovered' */
    public function execute(string $token): string
    {
        try {
            return DB::transaction(function () use ($token): string {
                $change = IdentityEmailChangeRequest::query()
                    ->where('recovery_token_hash', IdentitySecret::hash($token))
                    ->lockForUpdate()
                    ->first();

                if (! $change instanceof IdentityEmailChangeRequest) {
                    throw new EmailChangeRejected('The email recovery link is invalid or has expired.');
                }

                if ($change->isPending()) {
                    $change->forceFill(['cancelled_at' => now()])->save();
                    $this->securityEvents->recordIdentityEmailChangeCancelled($change->identity_id);

                    return 'cancelled';
                }

                if (! $change->isRecoverable()) {
                    throw new EmailChangeRejected('The email recovery link is invalid or has expired.');
                }

                $identity = Identity::query()->lockForUpdate()->find($change->identity_id);
                if (! $identity instanceof Identity || $identity->terminated_at !== null) {
                    throw new EmailChangeRejected('The email recovery link is invalid or has expired.');
                }

                $oldEmailInUse = Identity::query()
                    ->where('email', $change->old_email)
                    ->whereKeyNot($identity->id)
                    ->exists();
                if ($oldEmailInUse) {
                    throw new EmailChangeRejected('The previous email address cannot be restored automatically.');
                }

                $identity->forceFill([
                    'email' => $change->old_email,
                    'email_change_available_at' => now()->addDays((int) config('identity_security.email_change.cooldown_days', 7)),
                ])->save();
                $change->forceFill(['recovered_at' => now()])->save();

                $this->webSessions->execute($identity);
                $this->gameAuthorizations->execute($identity);
                $this->securityEvents->recordIdentityEmailChangeRecovered($identity->id);

                return 'recovered';
            });
        } catch (QueryException $exception) {
            throw new EmailChangeRejected('The previous email address cannot be restored automatically.', previous: $exception);
        }
    }
}
