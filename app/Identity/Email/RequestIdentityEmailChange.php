<?php

namespace App\Identity\Email;

use App\Audit\SecurityEventRecorder;
use App\Identity\Models\Identity;
use App\Identity\Models\IdentityEmailChangeRequest;
use App\Identity\Support\IdentitySecret;
use App\Notifications\Identity\IdentityEmailChangeNotice;
use App\Notifications\Identity\VerifyIdentityEmailChange;
use Illuminate\Support\Facades\DB;
use Illuminate\Support\Facades\Notification;
use Illuminate\Support\Str;
use LogicException;

final class RequestIdentityEmailChange
{
    public function __construct(
        private readonly SecurityEventRecorder $securityEvents,
    ) {}

    public function execute(Identity $identity, string $newEmail): IdentityEmailChangeRequest
    {
        $verificationToken = IdentitySecret::generate();
        $recoveryToken = IdentitySecret::generate();
        $now = now();
        $ttlHours = $this->boundedConfig('identity_security.email_change.verification_ttl_hours', 1, 168);
        $locale = app()->getLocale();

        $request = DB::transaction(function () use ($identity, $newEmail, $verificationToken, $recoveryToken, $now, $ttlHours): IdentityEmailChangeRequest {
            $lockedIdentity = Identity::query()->lockForUpdate()->find($identity->id);
            if (! $lockedIdentity instanceof Identity || $lockedIdentity->disabled_at !== null || $lockedIdentity->terminated_at !== null) {
                throw new EmailChangeRejected(__('identity.errors.email_change_unavailable'));
            }

            if ($lockedIdentity->email_change_available_at?->isFuture() === true) {
                throw new EmailChangeRejected(__('identity.errors.email_change_cooldown'));
            }

            $emailInUse = Identity::query()
                ->where('email', $newEmail)
                ->whereKeyNot($lockedIdentity->id)
                ->exists();
            if ($emailInUse) {
                throw new EmailChangeRejected(__('identity.errors.email_unusable'));
            }

            $pendingCollision = IdentityEmailChangeRequest::query()
                ->where('new_email', $newEmail)
                ->where('identity_id', '!=', $lockedIdentity->id)
                ->whereNull('confirmed_at')
                ->whereNull('cancelled_at')
                ->whereNull('recovered_at')
                ->where('expires_at', '>', $now)
                ->exists();
            if ($pendingCollision) {
                throw new EmailChangeRejected(__('identity.errors.email_unusable'));
            }

            IdentityEmailChangeRequest::query()
                ->where('identity_id', $lockedIdentity->id)
                ->whereNull('confirmed_at')
                ->whereNull('cancelled_at')
                ->whereNull('recovered_at')
                ->update(['cancelled_at' => $now, 'updated_at' => $now]);

            $change = IdentityEmailChangeRequest::query()->create([
                'id' => (string) Str::uuid(),
                'identity_id' => $lockedIdentity->id,
                'old_email' => $lockedIdentity->email,
                'new_email' => $newEmail,
                'verification_token_hash' => IdentitySecret::hash($verificationToken),
                'recovery_token_hash' => IdentitySecret::hash($recoveryToken),
                'requested_at' => $now,
                'expires_at' => $now->copy()->addHours($ttlHours),
            ]);

            $this->securityEvents->recordIdentityEmailChangeRequested($lockedIdentity->id);

            return $change;
        });

        Notification::route('mail', $request->new_email)
            ->notify(new VerifyIdentityEmailChange($verificationToken, $locale));
        Notification::route('mail', $request->old_email)
            ->notify(new IdentityEmailChangeNotice($recoveryToken, $locale));

        return $request;
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
