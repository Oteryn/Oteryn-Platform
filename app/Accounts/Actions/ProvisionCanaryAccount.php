<?php

namespace App\Accounts\Actions;

use App\Accounts\Contracts\CanaryAccountProvisioningGateway;
use App\Accounts\Exceptions\CanaryAccountProvisioningConflict;
use App\Accounts\Exceptions\CanaryAccountProvisioningException;
use App\Accounts\Exceptions\CanaryAccountProvisioningUnavailable;
use App\Accounts\Models\IdentityCanaryAccount;
use App\Audit\SecurityEventRecorder;
use Illuminate\Database\QueryException;
use Illuminate\Support\Facades\DB;
use Throwable;

final class ProvisionCanaryAccount
{
    public const FAILURE_DEPENDENCY_UNAVAILABLE = 'dependency_unavailable';

    public const FAILURE_BINDING_CONFLICT = 'binding_conflict';

    public function __construct(
        private readonly CanaryAccountProvisioningGateway $gateway,
        private readonly SecurityEventRecorder $securityEvents,
    ) {}

    public function execute(int $identityId): IdentityCanaryAccount
    {
        $intent = DB::transaction(function () use ($identityId): IdentityCanaryAccount {
            $binding = IdentityCanaryAccount::query()
                ->whereKey($identityId)
                ->lockForUpdate()
                ->firstOrFail();

            if ($binding->isReady()) {
                return $binding;
            }

            if ($binding->isConflict()) {
                throw new CanaryAccountProvisioningConflict('Canary account provisioning is in a hard conflict state.');
            }

            $binding->forceFill([
                'last_attempt_at' => now(),
                'last_failure_code' => null,
            ])->save();

            return $binding;
        });

        if ($intent->isReady()) {
            return $intent;
        }

        try {
            $canaryAccountId = $this->gateway->provision(
                $intent->provisioning_name,
                $intent->canary_creation_epoch,
            );
        } catch (CanaryAccountProvisioningConflict $exception) {
            $this->markConflict($identityId);

            throw $exception;
        } catch (CanaryAccountProvisioningException $exception) {
            $this->markFailure($identityId, self::FAILURE_DEPENDENCY_UNAVAILABLE);

            throw $exception;
        } catch (Throwable) {
            $this->markFailure($identityId, self::FAILURE_DEPENDENCY_UNAVAILABLE);

            throw new CanaryAccountProvisioningUnavailable('Canary account provisioning dependency is unavailable.');
        }

        try {
            return DB::transaction(function () use ($identityId, $canaryAccountId): IdentityCanaryAccount {
                $binding = IdentityCanaryAccount::query()
                    ->whereKey($identityId)
                    ->lockForUpdate()
                    ->firstOrFail();

                if ($binding->isConflict()) {
                    throw new CanaryAccountProvisioningConflict('Canary account provisioning entered a hard conflict state.');
                }

                if ($binding->canary_account_id !== null && $binding->canary_account_id !== $canaryAccountId) {
                    throw new CanaryAccountProvisioningConflict('Identity is already associated with a different Canary account.');
                }

                $completedNow = ! $binding->isReady();

                $binding->forceFill([
                    'canary_account_id' => $canaryAccountId,
                    'status' => IdentityCanaryAccount::STATUS_READY,
                    'last_failure_code' => null,
                    'ready_at' => $binding->ready_at ?? now(),
                ])->save();

                if ($completedNow) {
                    $this->securityEvents->recordCanaryAccountProvisioningCompleted($identityId);
                }

                return $binding;
            });
        } catch (CanaryAccountProvisioningConflict $exception) {
            $this->markConflict($identityId);

            throw $exception;
        } catch (QueryException $exception) {
            if ($this->isIntegrityViolation($exception)) {
                $this->markConflict($identityId);

                throw new CanaryAccountProvisioningConflict('Canary account binding conflicts with an existing ownership record.');
            }

            throw new CanaryAccountProvisioningUnavailable('Platform account binding finalization is unavailable.');
        } catch (Throwable) {
            throw new CanaryAccountProvisioningUnavailable('Platform account binding finalization is unavailable.');
        }
    }

    private function markFailure(int $identityId, string $failureCode): void
    {
        try {
            DB::transaction(function () use ($identityId, $failureCode): void {
                $binding = IdentityCanaryAccount::query()
                    ->whereKey($identityId)
                    ->lockForUpdate()
                    ->first();

                if ($binding === null || $binding->isReady() || $binding->isConflict()) {
                    return;
                }

                $binding->forceFill([
                    'status' => IdentityCanaryAccount::STATUS_PENDING,
                    'last_failure_code' => $failureCode,
                ])->save();

                $this->securityEvents->recordCanaryAccountProvisioningFailed($identityId);
            });
        } catch (Throwable) {
            // Preserve the original safe provisioning exception. A Platform DB
            // outage may prevent recording the failure, but must not expose the
            // underlying Canary query or sink credential material.
        }
    }

    private function markConflict(int $identityId): void
    {
        try {
            DB::transaction(function () use ($identityId): void {
                $binding = IdentityCanaryAccount::query()
                    ->whereKey($identityId)
                    ->lockForUpdate()
                    ->first();

                if ($binding === null) {
                    return;
                }

                $binding->forceFill([
                    'status' => IdentityCanaryAccount::STATUS_CONFLICT,
                    'last_failure_code' => self::FAILURE_BINDING_CONFLICT,
                ])->save();

                $this->securityEvents->recordCanaryAccountProvisioningConflict($identityId);
            });
        } catch (Throwable) {
            // Fail closed at the caller even when conflict-state persistence is
            // temporarily unavailable.
        }
    }

    private function isIntegrityViolation(QueryException $exception): bool
    {
        return (string) $exception->getCode() === '23000';
    }
}
