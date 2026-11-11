<?php

namespace App\Payments\Data;

use Carbon\CarbonImmutable;

final readonly class CheckoutSession
{
    public function __construct(
        public string $providerReference,
        public CarbonImmutable $expiresAt,
    ) {}
}
