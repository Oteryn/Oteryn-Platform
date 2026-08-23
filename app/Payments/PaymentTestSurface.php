<?php

namespace App\Payments;

use App\Payments\Exceptions\PaymentException;
use App\Payments\Infrastructure\DeterministicTestPaymentProvider;

final class PaymentTestSurface
{
    public function isAvailable(): bool
    {
        return config('app.env') !== 'production'
            && config('payments.enabled') === true
            && config('payments.provider') === DeterministicTestPaymentProvider::PROVIDER;
    }

    public function ensureAvailable(): void
    {
        if (! $this->isAvailable()) {
            throw new PaymentException(
                'test_surface_unavailable',
                'The deterministic payment test surface is unavailable.',
            );
        }
    }

    public function amountMinor(): int
    {
        $amount = config('payments.test_surface.amount_minor');

        if (! is_int($amount) || $amount < 1) {
            throw new PaymentException(
                'test_surface_misconfigured',
                'The deterministic payment test surface is misconfigured.',
            );
        }

        return $amount;
    }
}
