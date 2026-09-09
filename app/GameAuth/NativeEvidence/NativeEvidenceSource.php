<?php

namespace App\GameAuth\NativeEvidence;

use Closure;
use Illuminate\Support\Facades\DB;
use InvalidArgumentException;
use stdClass;

final class NativeEvidenceSource
{
    public function __construct(private readonly NativeEvidenceHighWaterWitness $witness) {}

    /**
     * @param  array<string, int|string>  $request
     * @return array<string, bool|int|string>
     */
    public function observe(array $request): array
    {
        if (config('game-auth.native_evidence.activated') !== true) {
            throw new NativeEvidenceUnavailable('Native evidence producer is not activated.');
        }

        $operation = (string) $request['operation'];
        $version = (int) $request['version'];

        return match ($operation) {
            NativeEvidenceContract::FRESH_ACCOUNT => $this->observeAccount($request, $version, false),
            NativeEvidenceContract::RECOVERY_ACCOUNT => $this->observeAccount($request, $version, true),
            NativeEvidenceContract::FRESH_TRUST => $this->observeTrust($request, $version, false),
            NativeEvidenceContract::RECOVERY_TRUST => $this->observeTrust($request, $version, true),
            default => NativeEvidenceContract::failure($version, $operation, 'unsupported'),
        };
    }

    /**
     * @param  array<string, int|string>  $request
     * @return array<string, bool|int|string>
     */
    private function observeAccount(array $request, int $version, bool $recovery): array
    {
        $operation = (string) $request['operation'];
        $accountId = (string) $request['account_id'];
        $purpose = (string) $request['purpose'];
        $scope = (string) $request['scope'];

        if ($recovery) {
            if ($purpose !== NativeEvidenceContract::RECOVERY_PURPOSE || $scope !== NativeEvidenceContract::RECOVERY_SCOPE) {
                return NativeEvidenceContract::failure($version, $operation, 'unsupported');
            }
        } else {
            $expectedPurpose = $this->requiredConfiguredBinding('fresh_account_purpose');
            $expectedScope = $this->requiredConfiguredBinding('fresh_account_scope');
            if ($purpose !== $expectedPurpose || $scope !== $expectedScope) {
                return NativeEvidenceContract::failure($version, $operation, 'unsupported');
            }
        }

        $sourceNamespace = NativeEvidenceNamespace::accountSource($accountId);
        $stateNamespace = NativeEvidenceNamespace::accountState($accountId);

        if (! DB::table('identities')->where('account_id', $accountId)->exists()) {
            if ($this->namespaceHasHistory($sourceNamespace) || $this->witness->peek($stateNamespace) !== null) {
                throw new NativeEvidenceUnavailable('Canonical account state disappeared below retained native evidence high-water.');
            }

            return NativeEvidenceContract::failure($version, $operation, 'not_found');
        }

        return $this->withOrderedNamespace($sourceNamespace, function (int $nextRevision) use (
            $sourceNamespace,
            $stateNamespace,
            $version,
            $operation,
            $accountId,
            $purpose,
            $scope,
        ): ?array {
            return DB::transaction(function () use (
                $sourceNamespace,
                $stateNamespace,
                $nextRevision,
                $version,
                $operation,
                $accountId,
                $purpose,
                $scope,
            ): ?array {
                $identity = DB::table('identities')->where('account_id', $accountId)->lockForUpdate()->first();
                if (! $identity instanceof stdClass) {
                    return null;
                }

                $generation = $this->positiveDatabaseInt($identity->native_security_generation ?? null, 'native security generation');
                $this->assertOrBootstrapAccountStateHighWater($sourceNamespace, $stateNamespace, $generation);
                $observedAt = $this->nonNegativeDatabaseInt(now()->timestamp, 'source observed time');

                $response = [
                    'version' => $version,
                    'operation' => $operation,
                    'result' => 'observed',
                    'source_authority' => $this->sourceAuthority(),
                    'source_revision' => (string) $nextRevision,
                    'decision_identity' => (string) $nextRevision,
                    'source_observed_at' => (string) $observedAt,
                    'clock_uncertainty_seconds' => (string) $this->clockUncertaintySeconds(),
                    'account_id' => $accountId,
                    'purpose' => $purpose,
                    'scope' => $scope,
                    'allowed' => ($identity->disabled_at ?? null) === null && ($identity->terminated_at ?? null) === null,
                    'minimum_valid_generation' => (string) $generation,
                ];

                $this->persistObservation($sourceNamespace, $nextRevision, $version, $operation, $observedAt, $response);

                return $response;
            }, 3);
        }, static fn () => NativeEvidenceContract::failure($version, $operation, 'not_found'));
    }

    /**
     * @param  array<string, int|string>  $request
     * @return array<string, bool|int|string>
     */
    private function observeTrust(array $request, int $version, bool $recovery): array
    {
        $operation = (string) $request['operation'];
        $issuer = (string) $request['issuer'];
        $profileName = (string) $request['profile'];
        $keyPurpose = (string) $request['key_purpose'];
        $keyId = (string) $request['key_id'];

        if ($recovery) {
            $supported = $issuer === NativeEvidenceContract::RECOVERY_ISSUER
                && $profileName === NativeEvidenceContract::RECOVERY_PROFILE
                && $keyPurpose === NativeEvidenceContract::RECOVERY_KEY_PURPOSE;
        } else {
            $supported = $issuer === NativeEvidenceContract::FRESH_ISSUER
                && $profileName === NativeEvidenceContract::FRESH_PROFILE
                && $keyPurpose === $this->requiredConfiguredBinding('fresh_key_purpose');
        }
        if (! $supported) {
            return NativeEvidenceContract::failure($version, $operation, 'unsupported');
        }

        $sourceNamespace = NativeEvidenceNamespace::trustSource($issuer, $profileName, $keyPurpose);
        $stateNamespace = NativeEvidenceNamespace::trustState($issuer, $profileName, $keyPurpose);
        $profile = $this->trustProfile($issuer, $profileName, $keyPurpose);

        if (! $profile instanceof stdClass) {
            if ($this->namespaceHasHistory($sourceNamespace) || $this->witness->peek($stateNamespace) !== null) {
                throw new NativeEvidenceUnavailable('Signing trust profile disappeared below retained high-water.');
            }

            return NativeEvidenceContract::failure($version, $operation, 'not_found');
        }

        $issuerRevision = $this->positiveDatabaseInt($profile->issuer_revision ?? null, 'signing trust issuer revision');
        $profileId = $this->positiveDatabaseInt($profile->id ?? null, 'signing trust profile id');
        $this->assertTrustStateHighWater($stateNamespace, $issuerRevision);

        if (! $this->trustKeyExists($profileId, $keyId)) {
            return NativeEvidenceContract::failure($version, $operation, 'not_found');
        }

        return $this->withOrderedNamespace($sourceNamespace, function (int $nextRevision) use (
            $stateNamespace,
            $version,
            $operation,
            $issuer,
            $profileName,
            $keyPurpose,
            $keyId,
        ): ?array {
            return DB::transaction(function () use (
                $stateNamespace,
                $nextRevision,
                $version,
                $operation,
                $issuer,
                $profileName,
                $keyPurpose,
                $keyId,
            ): ?array {
                $profile = DB::table('native_game_signing_trust_profiles')
                    ->where('issuer', $issuer)
                    ->where('profile', $profileName)
                    ->where('key_purpose', $keyPurpose)
                    ->lockForUpdate()
                    ->first();
                if (! $profile instanceof stdClass) {
                    return null;
                }

                $issuerRevision = $this->positiveDatabaseInt($profile->issuer_revision ?? null, 'signing trust issuer revision');
                $profileId = $this->positiveDatabaseInt($profile->id ?? null, 'signing trust profile id');
                $this->assertTrustStateHighWater($stateNamespace, $issuerRevision);

                $key = DB::table('native_game_signing_trust_key_versions')
                    ->where('profile_id', $profileId)
                    ->where('key_id', $keyId)
                    ->orderByDesc('key_revision')
                    ->lockForUpdate()
                    ->first();
                if (! $key instanceof stdClass || ! is_string($key->public_key ?? null)) {
                    return null;
                }

                $this->positiveDatabaseInt($key->key_revision ?? null, 'signing key revision');
                NativeEvidenceContract::assertEncodedPublicKey($key->public_key);
                $observedAt = $this->nonNegativeDatabaseInt(now()->timestamp, 'source observed time');

                $response = [
                    'version' => $version,
                    'operation' => $operation,
                    'result' => 'observed',
                    'source_authority' => $this->sourceAuthority(),
                    'source_revision' => (string) $nextRevision,
                    'decision_identity' => (string) $nextRevision,
                    'source_observed_at' => (string) $observedAt,
                    'clock_uncertainty_seconds' => (string) $this->clockUncertaintySeconds(),
                    'issuer' => $issuer,
                    'profile' => $profileName,
                    'key_purpose' => $keyPurpose,
                    'key_id' => $keyId,
                    'trusted' => ($profile->revoked_at ?? null) === null && (bool) ($key->trusted ?? false),
                    'public_key' => $key->public_key,
                ];

                $this->persistObservation(
                    NativeEvidenceNamespace::trustSource($issuer, $profileName, $keyPurpose),
                    $nextRevision,
                    $version,
                    $operation,
                    $observedAt,
                    $response,
                );

                return $response;
            }, 3);
        }, static fn () => NativeEvidenceContract::failure($version, $operation, 'not_found'));
    }

    /**
     * @template T of array<string, bool|int|string>|null
     *
     * @param  Closure(int): T  $observer
     * @param  Closure(): array<string, bool|int|string>  $notFound
     * @return array<string, bool|int|string>
     */
    private function withOrderedNamespace(string $namespace, Closure $observer, Closure $notFound): array
    {
        return $this->witness->withNamespace($namespace, function (?int $floor, Closure $advance) use ($namespace, $observer, $notFound): array {
            $databaseHighWater = $this->databaseHighWater($namespace);
            if ($floor === null) {
                if ($databaseHighWater !== 0) {
                    throw new NativeEvidenceUnavailable('Native evidence witness is missing for existing source history.');
                }
            } elseif ($floor > $databaseHighWater) {
                throw new NativeEvidenceUnavailable('Native evidence source rollback detected.');
            } elseif ($floor < $databaseHighWater) {
                $advance($databaseHighWater);
            }

            if ($databaseHighWater >= PHP_INT_MAX) {
                throw new NativeEvidenceUnavailable('Native evidence source revision is exhausted.');
            }
            $nextRevision = $databaseHighWater + 1;
            $response = $observer($nextRevision);
            if ($response === null) {
                return $notFound();
            }

            $advance($nextRevision);

            return $response;
        });
    }

    private function assertOrBootstrapAccountStateHighWater(string $sourceNamespace, string $stateNamespace, int $generation): void
    {
        $this->witness->withNamespace($stateNamespace, function (?int $floor, Closure $advance) use ($sourceNamespace, $generation): void {
            if ($floor === null) {
                if ($this->namespaceHasHistory($sourceNamespace)) {
                    throw new NativeEvidenceUnavailable('Native security-generation witness is missing for active source history.');
                }
                $advance($generation);

                return;
            }

            if ($floor !== $generation) {
                throw new NativeEvidenceUnavailable('Native security generation does not match retained high-water.');
            }
        });
    }

    private function assertTrustStateHighWater(string $stateNamespace, int $issuerRevision): void
    {
        $floor = $this->witness->peek($stateNamespace);
        if ($floor === null || $floor !== $issuerRevision) {
            throw new NativeEvidenceUnavailable('Signing trust state does not match retained high-water.');
        }
    }

    private function namespaceHasHistory(string $namespace): bool
    {
        return $this->witness->peek($namespace) !== null || $this->databaseHighWater($namespace) !== 0;
    }

    private function trustProfile(string $issuer, string $profile, string $keyPurpose): ?stdClass
    {
        $value = DB::table('native_game_signing_trust_profiles')
            ->where('issuer', $issuer)
            ->where('profile', $profile)
            ->where('key_purpose', $keyPurpose)
            ->first();

        return $value instanceof stdClass ? $value : null;
    }

    private function trustKeyExists(int $profileId, string $keyId): bool
    {
        return DB::table('native_game_signing_trust_key_versions')
            ->where('profile_id', $profileId)
            ->where('key_id', $keyId)
            ->exists();
    }

    private function databaseHighWater(string $namespace): int
    {
        $value = DB::table('native_game_evidence_observations')
            ->where('namespace_hash', $namespace)
            ->max('source_revision');

        return $value === null ? 0 : $this->nonNegativeDatabaseInt($value, 'source revision');
    }

    /** @param array<string, bool|int|string> $response */
    private function persistObservation(
        string $namespace,
        int $sourceRevision,
        int $version,
        string $operation,
        int $observedAt,
        array $response,
    ): void {
        $json = json_encode($response, JSON_UNESCAPED_SLASHES | JSON_THROW_ON_ERROR);
        if (strlen($json) > NativeEvidenceContract::MAX_RESPONSE_BYTES) {
            throw new NativeEvidenceUnavailable('Native evidence response exceeds the accepted bound.');
        }

        DB::table('native_game_evidence_observations')->insert([
            'namespace_hash' => $namespace,
            'source_revision' => $sourceRevision,
            'version' => $version,
            'operation' => $operation,
            'source_observed_at' => $observedAt,
            'response_json' => $json,
            'created_at' => now(),
        ]);
    }

    private function sourceAuthority(): string
    {
        $value = config('game-auth.native_evidence.source_authority');
        if (! is_string($value) || strlen($value) < 1 || strlen($value) > 128 || preg_match('/^[A-Za-z0-9._:\/-]+$/', $value) !== 1) {
            throw new NativeEvidenceUnavailable('Native evidence source authority is not configured safely.');
        }

        return $value;
    }

    private function requiredConfiguredBinding(string $key): string
    {
        $value = config("game-auth.native_evidence.{$key}");
        if (! is_string($value) || $value === '') {
            throw new NativeEvidenceUnavailable("Native evidence {$key} is not configured.");
        }

        try {
            NativeEvidenceContract::assertBinding($value);
        } catch (InvalidArgumentException) {
            throw new NativeEvidenceUnavailable("Native evidence {$key} is not configured safely.");
        }

        return $value;
    }

    private function clockUncertaintySeconds(): int
    {
        $value = config('game-auth.native_evidence.clock_uncertainty_seconds');
        if (is_string($value)) {
            if (preg_match('/^(0|[1-5])$/D', $value) !== 1) {
                throw new NativeEvidenceUnavailable('Native evidence clock uncertainty is not configured safely.');
            }

            $value = (int) $value;
        }

        if (! is_int($value) || $value < 0 || $value > 5) {
            throw new NativeEvidenceUnavailable('Native evidence clock uncertainty is not configured safely.');
        }

        return $value;
    }

    private function positiveDatabaseInt(mixed $value, string $name): int
    {
        $parsed = $this->nonNegativeDatabaseInt($value, $name);
        if ($parsed < 1) {
            throw new NativeEvidenceUnavailable("Native evidence {$name} must be positive.");
        }

        return $parsed;
    }

    private function nonNegativeDatabaseInt(mixed $value, string $name): int
    {
        if (is_int($value) && $value >= 0) {
            return $value;
        }
        if (is_string($value) && preg_match('/^(0|[1-9][0-9]{0,18})$/', $value) === 1) {
            $parsed = filter_var($value, FILTER_VALIDATE_INT, ['options' => ['min_range' => 0]]);
            if (is_int($parsed)) {
                return $parsed;
            }
        }

        throw new NativeEvidenceUnavailable("Native evidence {$name} is out of range.");
    }
}
