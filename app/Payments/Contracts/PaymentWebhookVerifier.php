<?php

namespace App\Payments\Contracts;

use App\Payments\Data\VerifiedProviderEvent;
use Carbon\CarbonImmutable;

interface PaymentWebhookVerifier
{
    /**
     * Authenticate the provider message before parsing and return only bounded,
     * provider-authenticated facts. Currency and minor-unit amount are required
     * so settlement integrity can be checked before payment truth changes.
     *
     * @param  array<string, string|list<string>>  $headers
     */
    public function verify(
        string $rawPayload,
        array $headers,
        ?CarbonImmutable $now = null,
    ): VerifiedProviderEvent;
}
