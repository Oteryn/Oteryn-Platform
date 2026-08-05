<?php

namespace App\Payments\Contracts;

use App\Payments\Data\VerifiedProviderEvent;
use Carbon\CarbonImmutable;

interface PaymentWebhookVerifier
{
    /**
     * @param  array<string, string|list<string>>  $headers
     */
    public function verify(
        string $rawPayload,
        array $headers,
        ?CarbonImmutable $now = null,
    ): VerifiedProviderEvent;
}
