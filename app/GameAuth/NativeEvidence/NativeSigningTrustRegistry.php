<?php

namespace App\GameAuth\NativeEvidence;

use Closure;
use Illuminate\Database\Query\JoinClause;
use Illuminate\Support\Facades\DB;
use InvalidArgumentException;
use LogicException;
use stdClass;

final class NativeSigningTrustRegistry
{
    /** Current and retiring key (OTERYN_V2_NATIVE_GATEWAY_LOGIN_CONTRACT §9.2, §9.3). */
    private const MAX_FRESH_TRUSTED_KEYS = 2;

    public function __construct(private readonly NativeEvidenceHighWaterWitness $witness) {}

    public function publishTrustedKey(
        string $issuer,
        string $profile,
        string $keyPurpose,
        string $keyId,
        string $publicKeyBytes,
    ): stdClass {
        $this->assertSupportedScope($issuer, $profile, $keyPurpose);
        NativeEvidenceContract::assertKeyId($keyId);
        $encoded = NativeEvidenceContract::encodePublicKey($publicKeyBytes);
        $stateNamespace = NativeEvidenceNamespace::trustState($issuer, $profile, $keyPurpose);

        return $this->witness->withNamespace($stateNamespace, function (?int $floor, Closure $advance) use (
            $issuer,
            $profile,
            $keyPurpose,
            $keyId,
            $encoded,
        ): stdClass {
            return DB::transaction(function () use (
                $floor,
                $advance,
                $issuer,
                $profile,
                $keyPurpose,
                $keyId,
                $encoded,
            ): stdClass {
                $trustProfile = $this->lockedProfileOrNull($issuer, $profile, $keyPurpose);
                if (! $trustProfile instanceof stdClass) {
                    if ($floor !== null) {
                        throw new NativeEvidenceUnavailable('Signing trust profile is missing below retained high-water.');
                    }

                    $advance(1);
                    $profileId = DB::table('native_game_signing_trust_profiles')->insertGetId([
                        'issuer' => $issuer,
                        'profile' => $profile,
                        'key_purpose' => $keyPurpose,
                        'profile_version' => 1,
                        'issuer_revision' => 1,
                        'revoked_at' => null,
                        'created_at' => now(),
                        'updated_at' => now(),
                    ]);

                    return $this->insertKeyVersion($profileId, $keyId, 1, $encoded, true, null);
                }

                $issuerRevision = $this->positiveDatabaseInt($trustProfile->issuer_revision ?? null, 'native signing trust issuer revision');
                $profileId = $this->positiveDatabaseInt($trustProfile->id ?? null, 'native signing trust profile id');
                $this->positiveDatabaseInt($trustProfile->profile_version ?? null, 'native signing trust profile version');
                $this->assertWitnessMatches($floor, $issuerRevision, 'native signing trust state');
                if (($trustProfile->revoked_at ?? null) !== null) {
                    throw new LogicException('Revoked native signing trust profile version cannot accept a trusted key.');
                }

                $latest = $this->latestKeyVersion($profileId, $keyId, true);
                if ($latest instanceof stdClass) {
                    $currentKeyRevision = $this->positiveDatabaseInt($latest->key_revision ?? null, 'native signing key revision');
                    if (! (bool) $latest->trusted) {
                        throw new LogicException('Revoked native signing key id cannot be re-trusted.');
                    }
                    if (($latest->public_key ?? null) === $encoded) {
                        return $latest;
                    }
                    if ($this->isFreshAdmissionScope($issuer, $profile)) {
                        throw new LogicException('A fresh admission signing key id is bound to one public key forever.');
                    }
                    $keyRevision = $this->nextPositive($currentKeyRevision, 'native signing key revision');
                } else {
                    if ($this->isFreshAdmissionScope($issuer, $profile)
                        && $this->trustedKeyCount($profileId) >= self::MAX_FRESH_TRUSTED_KEYS) {
                        throw new LogicException('At most two fresh admission signing keys may be trusted at once.');
                    }
                    $keyRevision = 1;
                }

                $nextIssuerRevision = $this->nextPositive($issuerRevision, 'native signing trust issuer revision');
                $advance($nextIssuerRevision);
                DB::table('native_game_signing_trust_profiles')->where('id', $profileId)->update([
                    'issuer_revision' => $nextIssuerRevision,
                    'updated_at' => now(),
                ]);

                return $this->insertKeyVersion($profileId, $keyId, $keyRevision, $encoded, true, null);
            }, 3);
        });
    }

    public function publishNextProfileVersion(
        string $issuer,
        string $profile,
        string $keyPurpose,
        string $keyId,
        string $publicKeyBytes,
    ): stdClass {
        $this->assertSupportedScope($issuer, $profile, $keyPurpose);
        NativeEvidenceContract::assertKeyId($keyId);
        $encoded = NativeEvidenceContract::encodePublicKey($publicKeyBytes);
        $stateNamespace = NativeEvidenceNamespace::trustState($issuer, $profile, $keyPurpose);

        return $this->witness->withNamespace($stateNamespace, function (?int $floor, Closure $advance) use (
            $issuer,
            $profile,
            $keyPurpose,
            $keyId,
            $encoded,
        ): stdClass {
            return DB::transaction(function () use (
                $floor,
                $advance,
                $issuer,
                $profile,
                $keyPurpose,
                $keyId,
                $encoded,
            ): stdClass {
                $trustProfile = $this->lockedProfile($issuer, $profile, $keyPurpose);
                $issuerRevision = $this->positiveDatabaseInt($trustProfile->issuer_revision ?? null, 'native signing trust issuer revision');
                $profileVersion = $this->positiveDatabaseInt($trustProfile->profile_version ?? null, 'native signing trust profile version');
                $this->assertWitnessMatches($floor, $issuerRevision, 'native signing trust state');
                if (($trustProfile->revoked_at ?? null) === null) {
                    throw new LogicException('A successor native signing trust profile version requires terminal revocation of the current version.');
                }
                if ($this->historicalKeyIdExists($issuer, $profile, $keyPurpose, $keyId)) {
                    throw new LogicException('A native signing key id cannot be reused across trust profile versions.');
                }

                $nextIssuerRevision = $this->nextPositive($issuerRevision, 'native signing trust issuer revision');
                $nextProfileVersion = $this->nextPositive($profileVersion, 'native signing trust profile version');
                $advance($nextIssuerRevision);
                $profileId = DB::table('native_game_signing_trust_profiles')->insertGetId([
                    'issuer' => $issuer,
                    'profile' => $profile,
                    'key_purpose' => $keyPurpose,
                    'profile_version' => $nextProfileVersion,
                    'issuer_revision' => $nextIssuerRevision,
                    'revoked_at' => null,
                    'created_at' => now(),
                    'updated_at' => now(),
                ]);

                return $this->insertKeyVersion($profileId, $keyId, 1, $encoded, true, null);
            }, 3);
        });
    }

    public function revokeKey(string $issuer, string $profile, string $keyPurpose, string $keyId): stdClass
    {
        $this->assertSupportedScope($issuer, $profile, $keyPurpose);
        NativeEvidenceContract::assertKeyId($keyId);
        $stateNamespace = NativeEvidenceNamespace::trustState($issuer, $profile, $keyPurpose);

        return $this->witness->withNamespace($stateNamespace, function (?int $floor, Closure $advance) use (
            $issuer,
            $profile,
            $keyPurpose,
            $keyId,
        ): stdClass {
            return DB::transaction(function () use ($floor, $advance, $issuer, $profile, $keyPurpose, $keyId): stdClass {
                $trustProfile = $this->lockedProfile($issuer, $profile, $keyPurpose);
                $issuerRevision = $this->positiveDatabaseInt($trustProfile->issuer_revision ?? null, 'native signing trust issuer revision');
                $profileId = $this->positiveDatabaseInt($trustProfile->id ?? null, 'native signing trust profile id');
                $this->assertWitnessMatches($floor, $issuerRevision, 'native signing trust state');

                $latest = $this->latestKeyVersion($profileId, $keyId, true);
                if (! $latest instanceof stdClass) {
                    throw new InvalidArgumentException('Unknown native signing key id.');
                }
                $keyRevision = $this->positiveDatabaseInt($latest->key_revision ?? null, 'native signing key revision');
                if (! (bool) $latest->trusted) {
                    return $latest;
                }
                if (! is_string($latest->public_key ?? null)) {
                    throw new LogicException('Native signing public key state is invalid.');
                }
                NativeEvidenceContract::assertEncodedPublicKey($latest->public_key);

                $nextIssuerRevision = $this->nextPositive($issuerRevision, 'native signing trust issuer revision');
                $nextKeyRevision = $this->nextPositive($keyRevision, 'native signing key revision');
                $advance($nextIssuerRevision);
                $revokedAt = now();
                DB::table('native_game_signing_trust_profiles')->where('id', $profileId)->update([
                    'issuer_revision' => $nextIssuerRevision,
                    'updated_at' => $revokedAt,
                ]);

                return $this->insertKeyVersion(
                    $profileId,
                    $keyId,
                    $nextKeyRevision,
                    $latest->public_key,
                    false,
                    $revokedAt,
                );
            }, 3);
        });
    }

    public function revokeProfile(string $issuer, string $profile, string $keyPurpose): void
    {
        $this->assertSupportedScope($issuer, $profile, $keyPurpose);
        $stateNamespace = NativeEvidenceNamespace::trustState($issuer, $profile, $keyPurpose);

        $this->witness->withNamespace($stateNamespace, function (?int $floor, Closure $advance) use (
            $issuer,
            $profile,
            $keyPurpose,
        ): void {
            DB::transaction(function () use ($floor, $advance, $issuer, $profile, $keyPurpose): void {
                $trustProfile = $this->lockedProfile($issuer, $profile, $keyPurpose);
                $issuerRevision = $this->positiveDatabaseInt($trustProfile->issuer_revision ?? null, 'native signing trust issuer revision');
                $profileId = $this->positiveDatabaseInt($trustProfile->id ?? null, 'native signing trust profile id');
                $this->assertWitnessMatches($floor, $issuerRevision, 'native signing trust state');
                if (($trustProfile->revoked_at ?? null) !== null) {
                    return;
                }

                $nextIssuerRevision = $this->nextPositive($issuerRevision, 'native signing trust issuer revision');
                $advance($nextIssuerRevision);
                $revokedAt = now();
                DB::table('native_game_signing_trust_profiles')->where('id', $profileId)->update([
                    'issuer_revision' => $nextIssuerRevision,
                    'revoked_at' => $revokedAt,
                    'updated_at' => $revokedAt,
                ]);
            }, 3);
        });
    }

    /**
     * Encoded public key of a key id that is currently trusted in a non-revoked profile version,
     * or null. Read-only; used by the native admission issuer self-check.
     */
    public function trustedPublicKey(string $issuer, string $profile, string $keyPurpose, string $keyId): ?string
    {
        $this->assertSupportedScope($issuer, $profile, $keyPurpose);
        NativeEvidenceContract::assertKeyId($keyId);

        $trustProfile = DB::table('native_game_signing_trust_profiles')
            ->where('issuer', $issuer)
            ->where('profile', $profile)
            ->where('key_purpose', $keyPurpose)
            ->orderByDesc('profile_version')
            ->first();
        if (! $trustProfile instanceof stdClass || ($trustProfile->revoked_at ?? null) !== null) {
            return null;
        }
        $profileId = $this->positiveDatabaseInt($trustProfile->id ?? null, 'native signing trust profile id');
        $latest = $this->latestKeyVersion($profileId, $keyId, false);
        if (! $latest instanceof stdClass || ! (bool) $latest->trusted || ! is_string($latest->public_key ?? null)) {
            return null;
        }
        NativeEvidenceContract::assertEncodedPublicKey($latest->public_key);

        return $latest->public_key;
    }

    private function isFreshAdmissionScope(string $issuer, string $profile): bool
    {
        return $issuer === NativeEvidenceContract::FRESH_ISSUER && $profile === NativeEvidenceContract::FRESH_PROFILE;
    }

    /** Key ids of one profile version whose latest revision is trusted. Caller holds the profile lock. */
    private function trustedKeyCount(int $profileId): int
    {
        $latestRevisions = DB::table('native_game_signing_trust_key_versions')
            ->select('key_id', DB::raw('MAX(key_revision) as key_revision'))
            ->where('profile_id', $profileId)
            ->groupBy('key_id');

        return DB::table('native_game_signing_trust_key_versions as key_versions')
            ->joinSub($latestRevisions, 'latest', function (JoinClause $join): void {
                $join->on('latest.key_id', '=', 'key_versions.key_id')
                    ->on('latest.key_revision', '=', 'key_versions.key_revision');
            })
            ->where('key_versions.profile_id', $profileId)
            ->where('key_versions.trusted', true)
            ->count();
    }

    private function lockedProfile(string $issuer, string $profile, string $keyPurpose): stdClass
    {
        $trustProfile = $this->lockedProfileOrNull($issuer, $profile, $keyPurpose);
        if (! $trustProfile instanceof stdClass) {
            throw new InvalidArgumentException('Unknown native signing trust profile.');
        }

        return $trustProfile;
    }

    private function lockedProfileOrNull(string $issuer, string $profile, string $keyPurpose): ?stdClass
    {
        $value = DB::table('native_game_signing_trust_profiles')
            ->where('issuer', $issuer)
            ->where('profile', $profile)
            ->where('key_purpose', $keyPurpose)
            ->orderByDesc('profile_version')
            ->lockForUpdate()
            ->first();

        return $value instanceof stdClass ? $value : null;
    }

    private function latestKeyVersion(int $profileId, string $keyId, bool $lock): ?stdClass
    {
        $query = DB::table('native_game_signing_trust_key_versions')
            ->where('profile_id', $profileId)
            ->where('key_id', $keyId)
            ->orderByDesc('key_revision');
        if ($lock) {
            $query->lockForUpdate();
        }
        $value = $query->first();

        return $value instanceof stdClass ? $value : null;
    }

    private function historicalKeyIdExists(string $issuer, string $profile, string $keyPurpose, string $keyId): bool
    {
        return DB::table('native_game_signing_trust_key_versions as key_versions')
            ->join('native_game_signing_trust_profiles as profiles', 'profiles.id', '=', 'key_versions.profile_id')
            ->where('profiles.issuer', $issuer)
            ->where('profiles.profile', $profile)
            ->where('profiles.key_purpose', $keyPurpose)
            ->where('key_versions.key_id', $keyId)
            ->exists();
    }

    private function insertKeyVersion(
        int $profileId,
        string $keyId,
        int $keyRevision,
        string $publicKey,
        bool $trusted,
        mixed $revokedAt,
    ): stdClass {
        $id = DB::table('native_game_signing_trust_key_versions')->insertGetId([
            'profile_id' => $profileId,
            'key_id' => $keyId,
            'key_revision' => $keyRevision,
            'public_key' => $publicKey,
            'trusted' => $trusted,
            'revoked_at' => $revokedAt,
            'created_at' => now(),
        ]);
        $stored = DB::table('native_game_signing_trust_key_versions')->where('id', $id)->first();
        if (! $stored instanceof stdClass) {
            throw new LogicException('Native signing key version was not persisted.');
        }

        return $stored;
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
            throw new InvalidArgumentException('Unsupported native signing trust scope.');
        }
        NativeEvidenceContract::assertBinding($issuer);
        NativeEvidenceContract::assertBinding($profile);
        NativeEvidenceContract::assertBinding($keyPurpose);
    }

    private function assertWitnessMatches(?int $floor, int $current, string $name): void
    {
        if ($floor === null || $floor !== $current) {
            throw new NativeEvidenceUnavailable("{$name} does not match retained high-water.");
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

        throw new LogicException("{$name} is invalid.");
    }

    private function nextPositive(int $current, string $name): int
    {
        if ($current < 1 || $current >= PHP_INT_MAX) {
            throw new LogicException("{$name} cannot advance safely.");
        }

        return $current + 1;
    }
}
