<?php

namespace App\Payments\Actions;

use App\Identity\Models\Identity;
use App\Payments\Exceptions\PaymentException;
use App\Payments\Models\PaymentOrder;
use App\Payments\Models\PaymentOrderTransition;
use App\Payments\PaymentAvailability;
use Illuminate\Database\QueryException;
use Illuminate\Support\Facades\DB;
use Illuminate\Support\Str;

final class CreatePaymentOrder
{
    public function __construct(
        private readonly PaymentAvailability $availability,
    ) {}

    public function execute(
        Identity $identity,
        string $currency,
        int $amountMinor,
        string $idempotencyKey,
    ): PaymentOrder {
        $this->availability->ensureEnabled();

        $normalizedCurrency = strtoupper(trim($currency));
        $provider = config('payments.provider');
        $maximumAmount = config('payments.maximum_order_amount_minor');
        $allowedCurrencies = config('payments.allowed_currencies');

        if ($idempotencyKey === '' || strlen($idempotencyKey) > 120) {
            throw new PaymentException('idempotency_key_invalid', 'The payment order request identifier is invalid.');
        }

        if (! is_string($provider) || trim($provider) === '') {
            throw new PaymentException('provider_unavailable', 'Payments are not configured.');
        }

        if (! is_array($allowedCurrencies)
            || ! in_array($normalizedCurrency, $allowedCurrencies, true)) {
            throw new PaymentException('currency_unsupported', 'The selected payment currency is not supported.');
        }

        if (! is_int($maximumAmount)
            || $amountMinor < 1
            || $amountMinor > $maximumAmount) {
            throw new PaymentException('amount_invalid', 'The payment amount is invalid.');
        }

        $existing = PaymentOrder::query()
            ->where('idempotency_key', $idempotencyKey)
            ->first();

        if ($existing instanceof PaymentOrder) {
            return $this->existingResult(
                $existing,
                $identity,
                $provider,
                $normalizedCurrency,
                $amountMinor,
            );
        }

        try {
            return DB::transaction(function () use (
                $identity,
                $provider,
                $normalizedCurrency,
                $amountMinor,
                $idempotencyKey,
            ): PaymentOrder {
                $existing = PaymentOrder::query()
                    ->where('idempotency_key', $idempotencyKey)
                    ->lockForUpdate()
                    ->first();

                if ($existing instanceof PaymentOrder) {
                    return $this->existingResult(
                        $existing,
                        $identity,
                        $provider,
                        $normalizedCurrency,
                        $amountMinor,
                    );
                }

                $order = PaymentOrder::query()->create([
                    'public_id' => (string) Str::uuid(),
                    'identity_id' => $identity->id,
                    'provider' => $provider,
                    'currency' => $normalizedCurrency,
                    'amount_minor' => $amountMinor,
                    'status' => PaymentOrder::STATUS_PENDING,
                    'idempotency_key' => $idempotencyKey,
                    'version' => 1,
                ]);

                PaymentOrderTransition::query()->create([
                    'payment_order_id' => $order->id,
                    'payment_provider_event_id' => null,
                    'from_status' => null,
                    'to_status' => PaymentOrder::STATUS_PENDING,
                    'reason' => 'order_created',
                    'version' => 1,
                    'created_at' => now(),
                ]);

                return $order;
            }, 3);
        } catch (QueryException $exception) {
            if ($this->isDuplicateKey($exception)) {
                $existing = PaymentOrder::query()
                    ->where('idempotency_key', $idempotencyKey)
                    ->first();

                if ($existing instanceof PaymentOrder) {
                    return $this->existingResult(
                        $existing,
                        $identity,
                        $provider,
                        $normalizedCurrency,
                        $amountMinor,
                    );
                }
            }

            throw new PaymentException('dependency_unavailable', 'The payment database is temporarily unavailable.');
        }
    }

    private function existingResult(
        PaymentOrder $existing,
        Identity $identity,
        string $provider,
        string $currency,
        int $amountMinor,
    ): PaymentOrder {
        if ($existing->identity_id !== $identity->id
            || $existing->provider !== $provider
            || $existing->currency !== $currency
            || $existing->amount_minor !== $amountMinor) {
            throw new PaymentException(
                'idempotency_conflict',
                'The payment order request identifier is already in use.',
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
