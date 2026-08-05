<?php

namespace App\Payments;

use App\Payments\Contracts\PaymentProviderGateway;
use App\Payments\Contracts\PaymentWebhookVerifier;
use Illuminate\Contracts\Container\Container;
use LogicException;

final class PaymentProviderResolver
{
    public function __construct(
        private readonly Container $container,
    ) {}

    public function gateway(): PaymentProviderGateway
    {
        $gateway = $this->container->make(PaymentProviderGateway::class);

        if (! $gateway instanceof PaymentProviderGateway) {
            throw new LogicException('The configured payment provider gateway is invalid.');
        }

        return $gateway;
    }

    public function webhookVerifier(): PaymentWebhookVerifier
    {
        $verifier = $this->container->make(PaymentWebhookVerifier::class);

        if (! $verifier instanceof PaymentWebhookVerifier) {
            throw new LogicException('The configured payment webhook verifier is invalid.');
        }

        return $verifier;
    }
}
