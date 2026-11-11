<?php

namespace App\Payments;

use App\Payments\Exceptions\PaymentException;

final class PaymentAvailability
{
    public function ensureEnabled(): void
    {
        if (config('payments.enabled') !== true) {
            throw new PaymentException('payments_disabled', 'Payments are disabled.');
        }
    }
}
