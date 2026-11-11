<?php

namespace App\Payments\Data;

final readonly class VerifiedProviderEvent
{
    /**
     * `amountMinor` is provider-authenticated settlement truth. For
     * `payment.partially_refunded` it is an incremental refund delta. For
     * `payment.refunded` it is the cumulative terminal refunded amount and
     * must equal the immutable payment-order total.
     *
     * @param  array<string, bool|int|string|null>  $metadata
     */
    public function __construct(
        public string $provider,
        public string $eventId,
        public string $eventType,
        public string $orderPublicId,
        public string $currency,
        public int $amountMinor,
        public ?string $providerObjectReference,
        public string $payloadSha256,
        public int $signatureTimestamp,
        public array $metadata,
    ) {}
}
