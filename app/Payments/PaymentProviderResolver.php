<?php

namespace App\Payments;

use App\Payments\Contracts\PaymentProviderGateway;
use App\Payments\Contracts\PaymentWebhookVerifier;
use Illuminate\Contracts\Container\Container;

final class PaymentProviderResolver
{
    public function __construct(
        private readonly Container $container,
    ) {}

    public function gateway(): PaymentProviderGateway
    {
        return $this->container->make(PaymentProviderGateway::class);
    }

    public function webhookVerifier(): PaymentWebhookVerifier
    {
        return $this->container->make(PaymentWebhookVerifier::class);
    }
}
