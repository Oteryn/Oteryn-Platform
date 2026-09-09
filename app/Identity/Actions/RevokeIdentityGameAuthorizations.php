<?php

namespace App\Identity\Actions;

use App\Audit\SecurityEventRecorder;
use App\GameAuth\NativeEvidence\NativeEvidenceHighWaterWitness;
use App\GameAuth\NativeEvidence\NativeEvidenceNamespace;
use App\GameAuth\NativeEvidence\NativeEvidenceUnavailable;
use App\GameAuth\OAuth\NativeOAuthGenerationBinding;
use App\Identity\Models\Identity;
use App\Identity\Support\CanonicalAccountId;
use Illuminate\Support\Facades\DB;
use LogicException;

final class RevokeIdentityGameAuthorizations
{
    public function __construct(
        private readonly SecurityEventRecorder $securityEvents,
        private readonly ?NativeOAuthGenerationBinding $oauthBindings = null,
        private readonly ?NativeEvidenceHighWaterWitness $nativeEvidenceWitness = null,
    ) {}

    public function execute(Identity $identity): int
    {
        return DB::transaction(function () use ($identity): int {
            $lockedIdentity = Identity::query()
                ->whereKey($identity->id)
                ->lockForUpdate()
                ->first();

            if (! $lockedIdentity instanceof Identity) {
                throw new LogicException('Identity is unavailable for game authorization revocation.');
            }

            $legacyGeneration = $lockedIdentity->game_auth_generation;
            $nativeGeneration = $lockedIdentity->native_security_generation;
            if (! is_int($legacyGeneration) || $legacyGeneration < 0 || $legacyGeneration >= PHP_INT_MAX) {
                throw new LogicException('Legacy game authorization generation cannot advance safely.');
            }
            if (! is_int($nativeGeneration) || $nativeGeneration < 1 || $nativeGeneration >= PHP_INT_MAX) {
                throw new LogicException('Native security generation cannot advance safely.');
            }

            $generation = $legacyGeneration + 1;
            $nextNativeGeneration = $nativeGeneration + 1;
            $this->fenceNativeGeneration($lockedIdentity, $nativeGeneration, $nextNativeGeneration);

            $lockedIdentity->forceFill([
                'game_auth_generation' => $generation,
                'native_security_generation' => $nextNativeGeneration,
            ])->save();

            ($this->oauthBindings ?? app(NativeOAuthGenerationBinding::class))
                ->revokeForIdentity($lockedIdentity);
            $this->securityEvents->recordIdentityGameAuthorizationsRevoked($lockedIdentity->id);

            $identity->setAttribute('game_auth_generation', $generation);
            $identity->setAttribute('native_security_generation', $nextNativeGeneration);

            return $generation;
        });
    }

    private function fenceNativeGeneration(Identity $identity, int $currentGeneration, int $nextGeneration): void
    {
        $witness = $this->nativeEvidenceWitness ?? app(NativeEvidenceHighWaterWitness::class);
        if (! $witness->isConfigured()) {
            return;
        }

        $accountId = $identity->account_id;
        if (! is_string($accountId) || ! CanonicalAccountId::isValid($accountId)) {
            throw new LogicException('Canonical AccountId is unavailable for native security revocation.');
        }

        $sourceNamespace = NativeEvidenceNamespace::accountSource($accountId);
        $stateNamespace = NativeEvidenceNamespace::accountState($accountId);

        try {
            $witness->withNamespace($stateNamespace, function (?int $floor, \Closure $advance) use (
                $witness,
                $sourceNamespace,
                $currentGeneration,
                $nextGeneration,
            ): void {
                if ($floor === null) {
                    $sourceFloor = $witness->peek($sourceNamespace);
                    $databaseHighWater = DB::table('native_game_evidence_observations')
                        ->where('namespace_hash', $sourceNamespace)
                        ->max('source_revision');
                    if ($sourceFloor !== null || $databaseHighWater !== null) {
                        throw new NativeEvidenceUnavailable('Native security-generation witness is missing for active source history.');
                    }
                    $advance($currentGeneration);
                    $floor = $currentGeneration;
                }

                if ($floor !== $currentGeneration) {
                    throw new NativeEvidenceUnavailable('Native security generation does not match retained high-water.');
                }

                $advance($nextGeneration);
            });
        } catch (NativeEvidenceUnavailable $exception) {
            throw new LogicException('Native security revocation cannot advance its non-rollback witness.', previous: $exception);
        }
    }
}
