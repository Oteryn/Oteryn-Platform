<?php

namespace App\Console\Commands;

use App\GameAuth\NativeEvidence\NativeEvidenceContract;
use App\GameAuth\NativeEvidence\NativeEvidenceRecoveryReconciler;
use Illuminate\Console\Command;
use Throwable;

final class ReconcileNativeEvidence extends Command
{
    protected $signature = 'game-auth:native-evidence:reconcile
        {--account-id= : Canonical AccountId whose retained native generation should be reconciled}
        {--trust= : Signing-trust scope to reconcile conservatively: fresh or recovery}';

    protected $description = 'Reconcile witness-ahead native evidence state without lowering retained authority.';

    public function handle(NativeEvidenceRecoveryReconciler $reconciler): int
    {
        $accountId = $this->option('account-id');
        $trust = $this->option('trust');
        $hasAccount = is_string($accountId) && $accountId !== '';
        $hasTrust = is_string($trust) && $trust !== '';

        if ($hasAccount === $hasTrust) {
            $this->components->error('Specify exactly one of --account-id or --trust=fresh|recovery.');

            return self::FAILURE;
        }

        try {
            if ($hasAccount) {
                $generation = $reconciler->reconcileAccountGeneration($accountId);
                $this->components->info("Reconciled native account generation to {$generation}.");

                return self::SUCCESS;
            }

            if ($trust === 'fresh') {
                $purpose = config('game-auth.native_evidence.fresh_key_purpose');
                if (! is_string($purpose) || $purpose === '') {
                    $this->components->error('Fresh native signing key purpose is not configured.');

                    return self::FAILURE;
                }
                $revision = $reconciler->reconcileSigningTrustAsRevoked(
                    NativeEvidenceContract::FRESH_ISSUER,
                    NativeEvidenceContract::FRESH_PROFILE,
                    $purpose,
                );
            } elseif ($trust === 'recovery') {
                $revision = $reconciler->reconcileSigningTrustAsRevoked(
                    NativeEvidenceContract::RECOVERY_ISSUER,
                    NativeEvidenceContract::RECOVERY_PROFILE,
                    NativeEvidenceContract::RECOVERY_KEY_PURPOSE,
                );
            } else {
                $this->components->error('The --trust option must be fresh or recovery.');

                return self::FAILURE;
            }

            $this->components->info("Conservatively reconciled native signing trust to revision {$revision}; the affected profile version remains revoked.");

            return self::SUCCESS;
        } catch (Throwable $exception) {
            $this->components->error($exception->getMessage());

            return self::FAILURE;
        }
    }
}
