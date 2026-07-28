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
use LogicException;

final class ConfirmIdentityEmailChange
{
    public function __construct(
        private readonly RevokeIdentityWebSessions $webSessions,
        private readonly RevokeIdentityGameAuthorizations $gameAuthorizations,
        private readonly SecurityEventRecorder $securityEvents,
    ) {}

    public function execute(string $token): Identity
    {
        try {
            return DB::transaction(function () use ($token): Identity {
                $change = IdentityEmailChangeRequest::query()
                    ->where('verification_token_hash', IdentitySecret::hash($token))
                    ->lockForUpdate()
                    ->first();

                if (! $change instanceof IdentityEmailChangeRequest || ! $change->isPending()) {
                    throw new EmailChangeRejected('The email verification link is invalid or has expired.');
                }

                $identity = Identity::query()->lockForUpdate()->find($change->identity_id);
                if (! $identity instanceof Identity || $identity->disabled_at !== null || $identity->terminated_at !== null) {
                    throw new EmailChangeRejected('Email change is not available for this account.');
                }

                $emailInUse = Identity::query()
                    ->where('email', $change->new_email)
                    ->whereKeyNot($identity->id)
                    ->exists();
                if ($emailInUse) {
                    throw new EmailChangeRejected('The requested email address cannot be used.');
                }

                $now = now();
                $recoveryHours = $this->boundedConfig('identity_security.email_change.recovery_window_hours', 1, 168);
                $cooldownDays = $this->boundedConfig('identity_security.email_change.cooldown_days', 1, 90);

                $identity->forceFill([
                    'email' => $change->new_email,
                    'email_change_available_at' => $now->copy()->addDays($cooldownDays),
                ])->save();

                $change->forceFill([
                    'confirmed_at' => $now,
                    'recoverable_until' => $now->copy()->addHours($recoveryHours),
                ])->save();

                $this->webSessions->execute($identity);
                $this->gameAuthorizations->execute($identity);
                $this->securityEvents->recordIdentityEmailChangeConfirmed($identity->id);

                return $identity->refresh();
            });
        } catch (QueryException $exception) {
            throw new EmailChangeRejected('The requested email address cannot be used.', previous: $exception);
        }
    }

    private function boundedConfig(string $key, int $minimum, int $maximum): int
    {
        $value = config($key);
        if (! is_int($value) || $value < $minimum || $value > $maximum) {
            throw new LogicException("Invalid bounded identity security configuration: {$key}.");
        }

        return $value;
    }
}
