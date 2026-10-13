<?php

namespace App\Console\Commands;

use App\GameAuth\NativeAdmission\NativeAdmissionKeyring;
use App\GameAuth\NativeEvidence\NativeEvidenceContract;
use App\GameAuth\NativeEvidence\NativeEvidenceDatabaseIdentity;
use App\GameAuth\NativeEvidence\NativeEvidenceHighWaterWitness;
use App\GameAuth\NativeEvidence\NativeEvidenceNamespace;
use App\GameAuth\NativeEvidence\NativeEvidencePublicationFence;
use App\GameAuth\NativeEvidence\NativeEvidenceUnavailable;
use App\GameAuth\NativeEvidence\NativeSigningTrustRegistry;
use Illuminate\Console\Command;
use Illuminate\Support\Facades\DB;
use Throwable;

/**
 * Read-only observation of the fresh signing trust state (PostgreSQL P6a, design note D2). Creates no file,
 * lock file, row or marker and never calls peek or a witness write path. Output is one JSON line; the exit
 * status is non-zero unless the result is `observed`.
 */
final class ObserveNativeSigningTrust extends Command
{
    protected $signature = 'game-auth:native-evidence:observe-trust
        {--trust= : Trust scope to observe; only "fresh" is supported}
        {--key-id= : Key id to report on; defaults to the configured current signing key id}';

    protected $description = 'Read-only observation of the fresh signing trust state: witness store, floor, profile, key and database identity.';

    public function handle(
        NativeEvidenceHighWaterWitness $witness,
        NativeSigningTrustRegistry $trust,
        NativeEvidenceDatabaseIdentity $database,
    ): int {
        if ($this->option('trust') !== 'fresh') {
            $this->components->error('Supply --trust=fresh.');

            return self::FAILURE;
        }

        try {
            $observation = $this->observe($witness, $trust, $database);
        } catch (NativeEvidenceUnavailable $exception) {
            $this->components->error('Native signing trust observation failed: '.$exception->getMessage());

            return self::FAILURE;
        } catch (Throwable $exception) {
            $this->components->error('Native signing trust observation failed: Unexpected '.class_basename($exception).'.');

            return self::FAILURE;
        }

        $this->line((string) json_encode($observation, JSON_UNESCAPED_SLASHES));

        return ($observation['result'] ?? null) === 'observed' ? self::SUCCESS : self::FAILURE;
    }

    /** @return array<string, mixed> */
    private function observe(
        NativeEvidenceHighWaterWitness $witness,
        NativeSigningTrustRegistry $trust,
        NativeEvidenceDatabaseIdentity $database,
    ): array {
        $keyPurpose = config('game-auth.native_evidence.fresh_key_purpose');
        if (! is_string($keyPurpose) || $keyPurpose === '') {
            return ['result' => 'scope_unconfigured'];
        }
        $issuer = NativeEvidenceContract::FRESH_ISSUER;
        $profile = NativeEvidenceContract::FRESH_PROFILE;

        $configured = $this->configuredKeyId();
        $requested = $this->option('key-id');
        $keyId = is_string($requested) && $requested !== '' ? $requested : $configured;
        if (! is_string($keyId) || preg_match('/\A[A-Za-z0-9._-]{1,64}\z/', $keyId) !== 1) {
            throw new NativeEvidenceUnavailable('Supply a valid --key-id or configure the current signing key id.');
        }

        // The database reads run in one read-only transaction opened after the shared locks are taken and
        // closed before they are released.
        $observed = $witness->observe(
            NativeEvidenceNamespace::trustState($issuer, $profile, $keyPurpose),
            function () use ($trust, $database, $issuer, $profile, $keyPurpose, $keyId): array {
                return DB::transaction(function () use ($trust, $database, $issuer, $profile, $keyPurpose, $keyId): array {
                    // Only the outermost transaction can be switched; a caller's wider transaction is left alone.
                    if ($database->supported() && DB::transactionLevel() === 1) {
                        DB::statement('SET TRANSACTION READ ONLY');
                    }

                    return [
                        'trust' => $trust->observe($issuer, $profile, $keyPurpose, $keyId),
                        'database' => $database->read(),
                    ];
                });
            },
        );
        if ($observed['result'] !== 'observed') {
            return ['result' => $observed['result']];
        }

        /** @var array{trust: array{profile: array<string, mixed>, in_history: bool, trusted: bool}, database: array<string, mixed>|null} $reads */
        $reads = $observed['reads'];

        return [
            'result' => 'observed',
            'fence_required' => NativeEvidencePublicationFence::required(),
            'store' => $observed['store'],
            'scope' => ['issuer' => $issuer, 'profile' => $profile, 'key_purpose' => $keyPurpose],
            'floor' => $observed['floor'],
            'profile' => $reads['trust']['profile'],
            'key' => [
                'requested' => $keyId,
                'configured_key_id' => $configured,
                'in_history' => $reads['trust']['in_history'],
                'trusted' => $reads['trust']['trusted'],
            ],
            'database' => $reads['database'],
        ];
    }

    /** The key id the entrypoint exported, or null when none is configured or loadable. */
    private function configuredKeyId(): ?string
    {
        $configured = config('game-auth.native_admission.signing_key_id');
        if (! is_string($configured) || $configured === '') {
            return null;
        }

        try {
            return app(NativeAdmissionKeyring::class)->currentKeyId();
        } catch (Throwable) {
            return $configured;
        }
    }
}
