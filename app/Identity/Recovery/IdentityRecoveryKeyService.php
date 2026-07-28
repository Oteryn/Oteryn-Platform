<?php

namespace App\Identity\Recovery;

use App\Audit\SecurityEventRecorder;
use App\Identity\Actions\RevokeIdentityGameAuthorizations;
use App\Identity\Actions\RevokeIdentityWebSessions;
use App\Identity\Models\Identity;
use App\Identity\Models\IdentityRecoveryKey;
use App\Identity\Support\IdentitySecret;
use Illuminate\Support\Facades\DB;
use Illuminate\Support\Facades\Hash;
use LogicException;

final class IdentityRecoveryKeyService
{
    public function __construct(
        private readonly RevokeIdentityWebSessions $webSessions,
        private readonly RevokeIdentityGameAuthorizations $gameAuthorizations,
        private readonly SecurityEventRecorder $securityEvents,
    ) {}

    public function generate(Identity $identity): string
    {
        $rawKey = $this->format(random_bytes(20));
        $normalized = $this->normalize($rawKey);

        DB::transaction(function () use ($identity, $normalized): void {
            $lockedIdentity = Identity::query()->lockForUpdate()->find($identity->id);
            if (! $lockedIdentity instanceof Identity || $lockedIdentity->disabled_at !== null || $lockedIdentity->terminated_at !== null) {
                throw new RecoveryKeyRejected('Recovery key management is not available for this account.');
            }

            IdentityRecoveryKey::query()->updateOrCreate(
                ['identity_id' => $lockedIdentity->id],
                [
                    'key_hash' => IdentitySecret::keyedHash($normalized),
                    'generated_at' => now(),
                    'used_at' => null,
                    'revoked_at' => null,
                ],
            );
            $this->securityEvents->recordIdentityRecoveryKeyGenerated($lockedIdentity->id);
        });

        return $rawKey;
    }

    public function revoke(Identity $identity): void
    {
        DB::transaction(function () use ($identity): void {
            $key = IdentityRecoveryKey::query()
                ->where('identity_id', $identity->id)
                ->lockForUpdate()
                ->first();

            if (! $key instanceof IdentityRecoveryKey || ! $key->isActive()) {
                throw new RecoveryKeyRejected('No active recovery key exists.');
            }

            $key->forceFill(['revoked_at' => now()])->save();
            $this->securityEvents->recordIdentityRecoveryKeyRevoked($identity->id);
        });
    }

    public function recover(string $email, string $rawKey, string $newPassword): Identity
    {
        $normalized = $this->normalize($rawKey);
        if ($normalized === '') {
            throw new RecoveryKeyRejected('The recovery credentials are invalid.');
        }

        return DB::transaction(function () use ($email, $normalized, $newPassword): Identity {
            $identity = Identity::query()->where('email', $email)->lockForUpdate()->first();
            if (! $identity instanceof Identity || $identity->disabled_at !== null || $identity->terminated_at !== null) {
                throw new RecoveryKeyRejected('The recovery credentials are invalid.');
            }

            $key = IdentityRecoveryKey::query()
                ->where('identity_id', $identity->id)
                ->lockForUpdate()
                ->first();
            $candidate = IdentitySecret::keyedHash($normalized);
            if (! $key instanceof IdentityRecoveryKey || ! $key->isActive() || ! hash_equals($key->key_hash, $candidate)) {
                throw new RecoveryKeyRejected('The recovery credentials are invalid.');
            }

            $identity->forceFill([
                'password' => Hash::make($newPassword),
                'two_factor_secret' => null,
                'two_factor_recovery_codes' => null,
                'two_factor_confirmed_at' => null,
                'two_factor_last_used_timestep' => null,
            ])->save();
            $key->forceFill(['used_at' => now()])->save();
            DB::table('password_reset_tokens')->where('email', $identity->email)->delete();

            $this->webSessions->execute($identity);
            $this->gameAuthorizations->execute($identity);
            $this->securityEvents->recordIdentityRecoveryKeyUsed($identity->id);

            return $identity->refresh();
        });
    }

    public function normalize(string $rawKey): string
    {
        return strtoupper((string) preg_replace('/[^A-Z0-9]/i', '', $rawKey));
    }

    private function format(string $bytes): string
    {
        $prefix = config('identity_security.recovery_key.prefix');
        if (! is_string($prefix) || $prefix === '' || ! preg_match('/^[A-Z0-9]+$/', $prefix)) {
            throw new LogicException('The recovery key prefix configuration is invalid.');
        }

        $encoded = strtoupper(bin2hex($bytes));
        $groups = str_split($encoded, 8);

        return $prefix.'-'.implode('-', $groups);
    }
}
