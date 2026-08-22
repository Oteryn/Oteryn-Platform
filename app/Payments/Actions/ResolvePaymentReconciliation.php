<?php

namespace App\Payments\Actions;

use App\Audit\AdminAuditRecorder;
use App\Identity\Models\Identity;
use App\Payments\Exceptions\PaymentException;
use App\Payments\Models\PaymentReconciliationEntry;
use App\Payments\Models\PaymentReconciliationResolution;
use App\Payments\PaymentTestSurface;
use Illuminate\Database\QueryException;
use Illuminate\Support\Facades\DB;

final class ResolvePaymentReconciliation
{
    public function __construct(
        private readonly PaymentTestSurface $testSurface,
        private readonly AdminAuditRecorder $audit,
    ) {}

    public function execute(
        Identity $actor,
        PaymentReconciliationEntry $entry,
        string $resolutionCode,
        string $idempotencyKey,
    ): PaymentReconciliationResolution {
        $this->testSurface->ensureAvailable();

        if ($idempotencyKey === '' || strlen($idempotencyKey) > 120) {
            throw new PaymentException('idempotency_key_invalid', 'The reconciliation request identifier is invalid.');
        }

        if ($resolutionCode !== PaymentReconciliationResolution::REVIEWED_NO_PAYMENT_STATE_CHANGE) {
            throw new PaymentException('resolution_code_invalid', 'The reconciliation resolution is invalid.');
        }

        $existing = PaymentReconciliationResolution::query()
            ->where('idempotency_key', $idempotencyKey)
            ->first();

        if ($existing instanceof PaymentReconciliationResolution) {
            return $this->existingResult($existing, $actor, $entry, $resolutionCode);
        }

        try {
            return DB::transaction(function () use ($actor, $entry, $resolutionCode, $idempotencyKey): PaymentReconciliationResolution {
                $lockedEntry = PaymentReconciliationEntry::query()
                    ->whereKey($entry->id)
                    ->lockForUpdate()
                    ->first();

                if (! $lockedEntry instanceof PaymentReconciliationEntry) {
                    throw new PaymentException('reconciliation_missing', 'The reconciliation entry no longer exists.');
                }

                $existing = PaymentReconciliationResolution::query()
                    ->where('idempotency_key', $idempotencyKey)
                    ->lockForUpdate()
                    ->first();

                if ($existing instanceof PaymentReconciliationResolution) {
                    return $this->existingResult($existing, $actor, $lockedEntry, $resolutionCode);
                }

                if ($lockedEntry->state !== PaymentReconciliationEntry::STATE_OPEN) {
                    throw new PaymentException(
                        'reconciliation_already_resolved',
                        'The reconciliation entry has already been resolved.',
                    );
                }

                $resolution = PaymentReconciliationResolution::query()->create([
                    'payment_reconciliation_entry_id' => $lockedEntry->id,
                    'actor_identity_id' => $actor->id,
                    'resolution_code' => $resolutionCode,
                    'idempotency_key' => $idempotencyKey,
                    'created_at' => now(),
                ]);

                $lockedEntry->state = PaymentReconciliationEntry::STATE_RESOLVED;
                $lockedEntry->resolved_at = now();
                $lockedEntry->save();

                $this->audit->record(
                    $actor->id,
                    'payments.reconciliation_resolved_without_payment_state_change',
                    'payment_reconciliation_entry',
                    (string) $lockedEntry->id,
                    [
                        'issue_type' => $lockedEntry->issue_type,
                        'resolution_code' => $resolutionCode,
                    ],
                );

                return $resolution;
            }, 3);
        } catch (QueryException $exception) {
            if ($this->isDuplicateKey($exception)) {
                $existing = PaymentReconciliationResolution::query()
                    ->where('idempotency_key', $idempotencyKey)
                    ->first();

                if ($existing instanceof PaymentReconciliationResolution) {
                    return $this->existingResult($existing, $actor, $entry, $resolutionCode);
                }
            }

            throw new PaymentException(
                'dependency_unavailable',
                'The payment database is temporarily unavailable.',
            );
        }
    }

    private function existingResult(
        PaymentReconciliationResolution $existing,
        Identity $actor,
        PaymentReconciliationEntry $entry,
        string $resolutionCode,
    ): PaymentReconciliationResolution {
        if ($existing->actor_identity_id !== $actor->id
            || $existing->payment_reconciliation_entry_id !== $entry->id
            || $existing->resolution_code !== $resolutionCode) {
            throw new PaymentException(
                'idempotency_conflict',
                'The reconciliation request identifier is already in use.',
            );
        }

        return $existing;
    }

    private function isDuplicateKey(QueryException $exception): bool
    {
        $sqlState = (string) $exception->getCode();
        $driverCode = $exception->errorInfo[1] ?? null;

        return $sqlState === '23000'
            && (
                (is_int($driverCode) || is_string($driverCode))
                    ? in_array((int) $driverCode, [19, 1062], true)
                    : false
            );
    }
}
