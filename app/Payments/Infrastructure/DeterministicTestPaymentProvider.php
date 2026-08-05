<?php

namespace App\Payments\Infrastructure;

use App\Payments\Contracts\PaymentProviderGateway;
use App\Payments\Contracts\PaymentWebhookVerifier;
use App\Payments\Data\CheckoutSession;
use App\Payments\Data\VerifiedProviderEvent;
use App\Payments\Exceptions\PaymentException;
use App\Payments\Models\PaymentOrder;
use Carbon\CarbonImmutable;
use JsonException;

final class DeterministicTestPaymentProvider implements PaymentProviderGateway, PaymentWebhookVerifier
{
    public const PROVIDER = 'test';

    public const TIMESTAMP_HEADER = 'x-oteryn-test-timestamp';

    public const SIGNATURE_HEADER = 'x-oteryn-test-signature';

    public function __construct(
        private readonly string $secret,
        private readonly int $maximumPayloadBytes,
        private readonly int $signatureToleranceSeconds,
    ) {
        if ($secret === '') {
            throw new PaymentException('test_provider_misconfigured', 'The test payment provider is not configured.');
        }

        if ($maximumPayloadBytes < 1 || $signatureToleranceSeconds < 1) {
            throw new PaymentException('test_provider_misconfigured', 'The test payment provider limits are invalid.');
        }
    }

    public function createCheckout(PaymentOrder $order, string $idempotencyKey): CheckoutSession
    {
        $this->ensureNonProduction();

        if ($idempotencyKey === '') {
            throw new PaymentException('idempotency_key_required', 'A payment checkout request identifier is required.');
        }

        return new CheckoutSession(
            providerReference: 'test_checkout_'.substr(
                hash('sha256', $order->public_id.'|'.$idempotencyKey),
                0,
                32,
            ),
            expiresAt: CarbonImmutable::now()->addMinutes(30),
        );
    }

    /**
     * @param  array<string, string|list<string>>  $headers
     */
    public function verify(
        string $rawPayload,
        array $headers,
        ?CarbonImmutable $now = null,
    ): VerifiedProviderEvent {
        $this->ensureNonProduction();

        if (strlen($rawPayload) > $this->maximumPayloadBytes) {
            throw new PaymentException('payload_too_large', 'The payment event payload is too large.');
        }

        $timestampValue = $this->header($headers, self::TIMESTAMP_HEADER);
        $signature = $this->header($headers, self::SIGNATURE_HEADER);

        if ($timestampValue === null || ! ctype_digit($timestampValue) || $signature === null) {
            throw new PaymentException('invalid_signature', 'The payment event signature is invalid.');
        }

        $timestamp = (int) $timestampValue;
        $currentTimestamp = ($now ?? CarbonImmutable::now())->getTimestamp();

        if (abs($currentTimestamp - $timestamp) > $this->signatureToleranceSeconds) {
            throw new PaymentException('expired_signature', 'The payment event signature has expired.');
        }

        $expectedSignature = hash_hmac('sha256', $timestampValue.'.'.$rawPayload, $this->secret);
        if (! hash_equals($expectedSignature, strtolower($signature))) {
            throw new PaymentException('invalid_signature', 'The payment event signature is invalid.');
        }

        try {
            $decoded = json_decode($rawPayload, true, 16, JSON_THROW_ON_ERROR);
        } catch (JsonException) {
            throw new PaymentException('invalid_payload', 'The payment event payload is invalid.');
        }

        if (! is_array($decoded)) {
            throw new PaymentException('invalid_payload', 'The payment event payload is invalid.');
        }

        $eventId = $decoded['id'] ?? null;
        $eventType = $decoded['type'] ?? null;
        $eventCreated = $decoded['created'] ?? null;
        $data = $decoded['data'] ?? null;

        if (! is_string($eventId) || $eventId === '' || strlen($eventId) > 120
            || ! is_string($eventType) || $eventType === '' || strlen($eventType) > 80
            || ! is_int($eventCreated)
            || ! is_array($data)) {
            throw new PaymentException('invalid_payload', 'The payment event payload is invalid.');
        }

        $orderPublicId = $data['order_public_id'] ?? null;
        $providerObjectReference = $data['provider_object_reference'] ?? null;

        if (! is_string($orderPublicId) || $orderPublicId === '' || strlen($orderPublicId) > 36
            || ($providerObjectReference !== null
                && (! is_string($providerObjectReference)
                    || $providerObjectReference === ''
                    || strlen($providerObjectReference) > 120))) {
            throw new PaymentException('invalid_payload', 'The payment event payload is invalid.');
        }

        return new VerifiedProviderEvent(
            provider: self::PROVIDER,
            eventId: $eventId,
            eventType: $eventType,
            orderPublicId: $orderPublicId,
            providerObjectReference: $providerObjectReference,
            payloadSha256: hash('sha256', $rawPayload),
            signatureTimestamp: $timestamp,
            metadata: [
                'event_created' => $eventCreated,
            ],
        );
    }

    public static function signature(string $secret, int $timestamp, string $rawPayload): string
    {
        return hash_hmac('sha256', $timestamp.'.'.$rawPayload, $secret);
    }

    /**
     * @param  array<string, string|list<string>>  $headers
     */
    private function header(array $headers, string $name): ?string
    {
        foreach ($headers as $headerName => $value) {
            if (strtolower($headerName) !== $name) {
                continue;
            }

            if (is_string($value)) {
                return trim($value);
            }

            $first = $value[0] ?? null;

            return is_string($first) ? trim($first) : null;
        }

        return null;
    }

    private function ensureNonProduction(): void
    {
        if (config('app.env') === 'production') {
            throw new PaymentException(
                'test_provider_forbidden',
                'The deterministic test payment provider cannot run in production.',
            );
        }
    }
}
