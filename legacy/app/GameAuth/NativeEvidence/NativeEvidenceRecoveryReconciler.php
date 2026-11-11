<?php

namespace App\GameAuth\NativeEvidence;

use App\Identity\Support\CanonicalAccountId;
use Illuminate\Support\Facades\DB;
use InvalidArgumentException;
use stdClass;

final class NativeEvidenceRecoveryReconciler
{
    public function __construct(private readonly NativeEvidenceHighWaterWitness $witness) {}

    public function reconcileAccountGeneration(string $accountId): int
    {
        $this->assertActivated();
        if (! CanonicalAccountId::isValid($accountId)) {
            throw new InvalidArgumentException('Native evidence reconciliation requires a canonical AccountId.');
        }

        $stateNamespace = NativeEvidenceNamespace::accountState($accountId);

        return DB::transaction(function () use ($accountId, $stateNamespace): int {
            $identity = DB::table('identities')->where('account_id', $accountId)->lockForUpdate()->first();
            if (! $identity instanceof stdClass) {
                throw new NativeEvidenceUnavailable('Canonical account state is unavailable for reconciliation.');
            }

            $current = $this->positiveDatabaseInt($identity->native_security_generation ?? null, 'native security generation');

            return $this->witness->withNamespace($stateNamespace, function (?int $floor) use ($accountId, $current): int {
                if ($floor === null || $floor < 1) {
                    throw new NativeEvidenceUnavailable('Native security-generation retained high-water is unavailable for reconciliation.');
                }
                if ($floor < $current) {
                    throw new NativeEvidenceUnavailable('Native security-generation retained high-water is behind canonical state.');
                }
                if ($floor === $current) {
                    return $current;
                }

                $updated = DB::table('identities')
                    ->where('account_id', $accountId)
                    ->where('native_security_generation', $current)
                    ->update(['native_security_generation' => $floor]);
                if ($updated !== 1) {
                    throw new NativeEvidenceUnavailable('Native security generation changed during reconciliation.');
                }

                return $floor;
            });
        }, 3);
    }

    public function reconcileSigningTrustAsRevoked(string $issuer, string $profile, string $keyPurpose): int
    {
        $this->assertActivated();
        $this->assertSupportedScope($issuer, $profile, $keyPurpose);
        $stateNamespace = NativeEvidenceNamespace::trustState($issuer, $profile, $keyPurpose);

        return DB::transaction(function () use ($issuer, $profile, $keyPurpose, $stateNamespace): int {
            $trustProfile = DB::table('native_game_signing_trust_profiles')
                ->where('issuer', $issuer)
                ->where('profile', $profile)
                ->where('key_purpose', $keyPurpose)
                ->orderByDesc('profile_version')
                ->lockForUpdate()
                ->first();
            if (! $trustProfile instanceof stdClass) {
                throw new NativeEvidenceUnavailable('Native signing trust profile is unavailable for reconciliation.');
            }

            $profileId = $this->positiveDatabaseInt($trustProfile->id ?? null, 'native signing trust profile id');
            $current = $this->positiveDatabaseInt($trustProfile->issuer_revision ?? null, 'native signing trust issuer revision');

            return $this->witness->withNamespace($stateNamespace, function (?int $floor) use ($profileId, $current, $trustProfile): int {
                if ($floor === null || $floor < 1) {
                    throw new NativeEvidenceUnavailable('Native signing trust retained high-water is unavailable for reconciliation.');
                }
                if ($floor < $current) {
                    throw new NativeEvidenceUnavailable('Native signing trust retained high-water is behind canonical state.');
                }
                if ($floor === $current) {
                    if (($trustProfile->revoked_at ?? null) !== null) {
                        return $current;
                    }

                    throw new NativeEvidenceUnavailable('Native signing trust has no witness-ahead ambiguity to reconcile.');
                }

                $reconciledAt = now();
                $updated = DB::table('native_game_signing_trust_profiles')
                    ->where('id', $profileId)
                    ->where('issuer_revision', $current)
                    ->update([
                        'issuer_revision' => $floor,
                        'revoked_at' => $reconciledAt,
                        'updated_at' => $reconciledAt,
                    ]);
                if ($updated !== 1) {
                    throw new NativeEvidenceUnavailable('Native signing trust changed during reconciliation.');
                }

                return $floor;
            });
        }, 3);
    }

    private function assertSupportedScope(string $issuer, string $profile, string $keyPurpose): void
    {
        $freshPurpose = config('game-auth.native_evidence.fresh_key_purpose');
        $fresh = $issuer === NativeEvidenceContract::FRESH_ISSUER
            && $profile === NativeEvidenceContract::FRESH_PROFILE
            && is_string($freshPurpose)
            && $freshPurpose !== ''
            && $keyPurpose === $freshPurpose;
        $recovery = $issuer === NativeEvidenceContract::RECOVERY_ISSUER
            && $profile === NativeEvidenceContract::RECOVERY_PROFILE
            && $keyPurpose === NativeEvidenceContract::RECOVERY_KEY_PURPOSE;

        if (! $fresh && ! $recovery) {
            throw new InvalidArgumentException('Unsupported native signing trust reconciliation scope.');
        }
    }

    private function assertActivated(): void
    {
        if (config('game-auth.native_evidence.activated') !== true) {
            throw new NativeEvidenceUnavailable('Native evidence reconciliation requires an activated producer.');
        }
    }

    private function positiveDatabaseInt(mixed $value, string $name): int
    {
        if (is_int($value) && $value >= 1) {
            return $value;
        }
        if (is_string($value) && preg_match('/^[1-9][0-9]{0,18}$/', $value) === 1) {
            $parsed = filter_var($value, FILTER_VALIDATE_INT, ['options' => ['min_range' => 1]]);
            if (is_int($parsed)) {
                return $parsed;
            }
        }

        throw new NativeEvidenceUnavailable("{$name} is invalid.");
    }
}
