<?php

namespace App\Payments\Actions;

use App\Payments\Exceptions\PaymentException;
use App\Payments\Models\PaymentAttempt;
use App\Payments\Models\PaymentOrder;
use App\Payments\Models\PaymentOrderTransition;
use App\Payments\Models\PaymentReconciliationEntry;
use App\Payments\PaymentAvailability;
use App\Payments\PaymentProviderResolver;
use Illuminate\Support\Facades\DB;
use Illuminate\Support\Str;
use Throwable;

final class CreatePaymentCheckout
{
    public function __construct(
        private readonly PaymentProviderResolver $providers,
        private readonly PaymentAvailability $availability,
    ) {}

    public function execute(
        PaymentOrder $order,
        string $idempotencyKey,
    ): PaymentAttempt {
        $this->availability->ensureEnabled();

        if ($idempotencyKey === '' || strlen($idempotencyKey) > 120) {
            throw new PaymentException('idempotency_key_invalid', 'The payment checkout request identifier is invalid.');
        }

        $existing = PaymentAttempt::query()
            ->where('idempotency_key', $idempotencyKey)
            ->first();

        if ($existing instanceof PaymentAttempt) {
            return $this->existingResult($existing, $order);
        }

        $attempt = DB::transaction(function () use ($order, $idempotencyKey): PaymentAttempt {
            $lockedOrder = PaymentOrder::query()
                ->whereKey($order->id)
                ->lockForUpdate()
                ->first();

            if (! $lockedOrder instanceof PaymentOrder) {
                throw new PaymentException('order_missing', 'The payment order no longer exists.');
            }

            $existing = PaymentAttempt::query()
                ->where('idempotency_key', $idempotencyKey)
                ->lockForUpdate()
                ->first();

            if ($existing instanceof PaymentAttempt) {
                return $this->existingResult($existing, $lockedOrder);
            }

            if ($lockedOrder->status !== PaymentOrder::STATUS_PENDING) {
                throw new PaymentException('checkout_unavailable', 'A checkout cannot be created for this payment order.');
            }

            return PaymentAttempt::query()->create([
                'public_id' => (string) Str::uuid(),
                'payment_order_id' => $lockedOrder->id,
                'provider' => $lockedOrder->provider,
                'status' => PaymentAttempt::STATUS_CREATING,
                'provider_checkout_reference' => null,
                'idempotency_key' => $idempotencyKey,
                'sanitized_error_code' => null,
            ]);
        }, 3);

        if ($attempt->status !== PaymentAttempt::STATUS_CREATING) {
            return $attempt;
        }

        try {
            $session = $this->providers->gateway()->createCheckout($order, $idempotencyKey);
        } catch (Throwable) {
            DB::transaction(function () use ($attempt, $order): void {
                $lockedAttempt = PaymentAttempt::query()
                    ->whereKey($attempt->id)
                    ->lockForUpdate()
                    ->first();
                $lockedOrder = PaymentOrder::query()
                    ->whereKey($order->id)
                    ->lockForUpdate()
                    ->first();

                if (! $lockedAttempt instanceof PaymentAttempt
                    || ! $lockedOrder instanceof PaymentOrder
                    || $lockedAttempt->status !== PaymentAttempt::STATUS_CREATING) {
                    return;
                }

                $lockedAttempt->status = PaymentAttempt::STATUS_AMBIGUOUS;
                $lockedAttempt->sanitized_error_code = 'provider_unavailable';
                $lockedAttempt->save();

                PaymentReconciliationEntry::query()->create([
                    'payment_order_id' => $lockedOrder->id,
                    'payment_provider_event_id' => null,
                    'issue_type' => 'ambiguous_checkout_creation',
                    'state' => PaymentReconciliationEntry::STATE_OPEN,
                    'metadata' => [
                        'attempt_public_id' => $lockedAttempt->public_id,
                    ],
                    'created_at' => now(),
                    'resolved_at' => null,
                ]);
            }, 3);

            throw new PaymentException(
                'provider_unavailable',
                'The payment provider response is ambiguous and requires reconciliation.',
            );
        }

        return DB::transaction(function () use ($attempt, $order, $session): PaymentAttempt {
            $lockedAttempt = PaymentAttempt::query()
                ->whereKey($attempt->id)
                ->lockForUpdate()
                ->first();
            $lockedOrder = PaymentOrder::query()
                ->whereKey($order->id)
                ->lockForUpdate()
                ->first();

            if (! $lockedAttempt instanceof PaymentAttempt
                || ! $lockedOrder instanceof PaymentOrder) {
                throw new PaymentException('dependency_unavailable', 'The payment database is temporarily unavailable.');
            }

            if ($lockedAttempt->status === PaymentAttempt::STATUS_READY) {
                return $lockedAttempt;
            }

            if ($lockedAttempt->status !== PaymentAttempt::STATUS_CREATING
                || $lockedOrder->status !== PaymentOrder::STATUS_PENDING) {
                throw new PaymentException('checkout_state_conflict', 'The payment checkout requires reconciliation.');
            }

            $lockedAttempt->status = PaymentAttempt::STATUS_READY;
            $lockedAttempt->provider_checkout_reference = $session->providerReference;
            $lockedAttempt->sanitized_error_code = null;
            $lockedAttempt->save();

            $fromStatus = $lockedOrder->status;
            $lockedOrder->status = PaymentOrder::STATUS_CHECKOUT_CREATED;
            $lockedOrder->version++;
            $lockedOrder->save();

            PaymentOrderTransition::query()->create([
                'payment_order_id' => $lockedOrder->id,
                'payment_provider_event_id' => null,
                'from_status' => $fromStatus,
                'to_status' => PaymentOrder::STATUS_CHECKOUT_CREATED,
                'reason' => 'checkout_created',
                'version' => $lockedOrder->version,
                'created_at' => now(),
            ]);

            return $lockedAttempt;
        }, 3);
    }

    private function existingResult(
        PaymentAttempt $existing,
        PaymentOrder $order,
    ): PaymentAttempt {
        if ($existing->payment_order_id !== $order->id
            || $existing->provider !== $order->provider) {
            throw new PaymentException(
                'idempotency_conflict',
                'The payment checkout request identifier is already in use.',
            );
        }

        return $existing;
    }
}
