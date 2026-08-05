<?php

namespace App\Payments\Contracts;

use App\Payments\Data\CheckoutSession;
use App\Payments\Models\PaymentOrder;

interface PaymentProviderGateway
{
    public function createCheckout(PaymentOrder $order, string $idempotencyKey): CheckoutSession;
}
