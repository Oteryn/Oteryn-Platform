<?php

namespace App\Payments\Data;

final readonly class VerifiedProviderEvent
{
    /**
     * @param  array<string, bool|int|string|null>  $metadata
     */
    public function __construct(
        public string $provider,
        public string $eventId,
        public string $eventType,
        public string $orderPublicId,
        public ?string $providerObjectReference,
        public string $payloadSha256,
        public int $signatureTimestamp,
        public array $metadata,
    ) {}
}
