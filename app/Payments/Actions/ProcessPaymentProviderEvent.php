<?php

namespace App\Payments\Actions;

use App\Payments\Data\PaymentStateDecision;
use App\Payments\Data\VerifiedProviderEvent;
use App\Payments\Exceptions\PaymentException;
use App\Payments\Models\PaymentOrder;
use App\Payments\Models\PaymentOrderTransition;
use App\Payments\Models\PaymentProviderEvent;
use App\Payments\Models\PaymentReconciliationEntry;
use App\Payments\PaymentAvailability;
use App\Payments\PaymentOrderStateMachine;
use App\Payments\PaymentProviderResolver;
use Carbon\CarbonImmutable;
use Illuminate\Database\QueryException;
use Illuminate\Support\Facades\DB;

final class ProcessPaymentProviderEvent
{
    public function __construct(
        private readonly PaymentProviderResolver $providers,
        private readonly PaymentOrderStateMachine $stateMachine,
        private readonly PaymentAvailability $availability,
    ) {}

    /**
     * @param  array<string, string|list<string>>  $headers
     */
    public function execute(
        string $rawPayload,
        array $headers,
        ?CarbonImmutable $now = null,
    ): PaymentProviderEvent {
        $this->availability->ensureEnabled();

        $verified = $this->providers->webhookVerifier()->verify($rawPayload, $headers, $now);
        $configuredProvider = config('payments.provider');

        if (! is_string($configuredProvider) || $verified->provider !== $configuredProvider) {
            throw new PaymentException('provider_mismatch', 'The payment event provider is not configured.');
        }

        try {
            return DB::transaction(
                fn (): PaymentProviderEvent => $this->processVerified($verified),
                3,
            );
        } catch (QueryException $exception) {
            if ($this->isDuplicateKey($exception)) {
                $existing = PaymentProviderEvent::query()
                    ->where('provider', $verified->provider)
                    ->where('provider_event_id', $verified->eventId)
                    ->first();

                if ($existing instanceof PaymentProviderEvent) {
                    return $this->existingResult($existing, $verified);
                }
            }

            throw new PaymentException('dependency_unavailable', 'The payment database is temporarily unavailable.');
        }
    }

    private function processVerified(
        VerifiedProviderEvent $verified,
    ): PaymentProviderEvent {
        $existing = PaymentProviderEvent::query()
            ->where('provider', $verified->provider)
            ->where('provider_event_id', $verified->eventId)
            ->lockForUpdate()
            ->first();

        if ($existing instanceof PaymentProviderEvent) {
            return $this->existingResult($existing, $verified);
        }

        $order = PaymentOrder::query()
            ->where('public_id', $verified->orderPublicId)
            ->lockForUpdate()
            ->first();

        $event = PaymentProviderEvent::query()->create([
            'payment_order_id' => $order?->id,
            'provider' => $verified->provider,
            'provider_event_id' => $verified->eventId,
            'event_type' => $verified->eventType,
            'provider_object_reference' => $verified->providerObjectReference,
            'payload_sha256' => $verified->payloadSha256,
            'signature_timestamp' => $verified->signatureTimestamp,
            'processing_state' => PaymentProviderEvent::STATE_PROCESSED,
            'failure_code' => null,
            'metadata' => $verified->metadata,
            'received_at' => now(),
            'processed_at' => null,
        ]);

        if (! $order instanceof PaymentOrder) {
            return $this->reconcile(
                $event,
                null,
                'order_not_found',
                [
                    'order_public_id' => $verified->orderPublicId,
                ],
            );
        }

        if ($order->provider !== $verified->provider) {
            return $this->reconcile(
                $event,
                $order,
                'provider_mismatch',
                [
                    'order_provider' => $order->provider,
                    'event_provider' => $verified->provider,
                ],
            );
        }

        $decision = $this->stateMachine->decide($order->status, $verified->eventType);

        if ($decision->action === PaymentStateDecision::NOOP) {
            $event->processing_state = PaymentProviderEvent::STATE_PROCESSED;
            $event->processed_at = now();
            $event->metadata = array_merge($verified->metadata, [
                'decision' => $decision->reason,
            ]);
            $event->save();

            return $event;
        }

        if ($decision->action === PaymentStateDecision::RECONCILE
            || $decision->targetStatus === null) {
            return $this->reconcile(
                $event,
                $order,
                $decision->reason,
                [
                    'current_status' => $decision->currentStatus,
                    'target_status' => $decision->targetStatus,
                ],
            );
        }

        $fromStatus = $order->status;
        $order->status = $decision->targetStatus;
        $order->version++;
        $order->save();

        PaymentOrderTransition::query()->create([
            'payment_order_id' => $order->id,
            'payment_provider_event_id' => $event->id,
            'from_status' => $fromStatus,
            'to_status' => $decision->targetStatus,
            'reason' => $decision->reason,
            'version' => $order->version,
            'created_at' => now(),
        ]);

        $event->processing_state = PaymentProviderEvent::STATE_PROCESSED;
        $event->processed_at = now();
        $event->save();

        return $event;
    }

    /**
     * @param  array<string, bool|int|string|null>  $metadata
     */
    private function reconcile(
        PaymentProviderEvent $event,
        ?PaymentOrder $order,
        string $issueType,
        array $metadata,
    ): PaymentProviderEvent {
        $event->processing_state = PaymentProviderEvent::STATE_RECONCILIATION;
        $event->failure_code = $issueType;
        $event->processed_at = now();
        $event->save();

        PaymentReconciliationEntry::query()->create([
            'payment_order_id' => $order?->id,
            'payment_provider_event_id' => $event->id,
            'issue_type' => $issueType,
            'state' => PaymentReconciliationEntry::STATE_OPEN,
            'metadata' => $metadata === [] ? null : $metadata,
            'created_at' => now(),
            'resolved_at' => null,
        ]);

        return $event;
    }

    private function existingResult(
        PaymentProviderEvent $existing,
        VerifiedProviderEvent $verified,
    ): PaymentProviderEvent {
        if ($existing->payload_sha256 !== $verified->payloadSha256
            || $existing->event_type !== $verified->eventType
            || $existing->provider_object_reference !== $verified->providerObjectReference) {
            throw new PaymentException(
                'event_id_conflict',
                'The payment event identifier is already in use.',
            );
        }

        return $existing;
    }

    private function isDuplicateKey(QueryException $exception): bool
    {
        $sqlState = (string) $exception->getCode();
        $driverCode = $exception->errorInfo[1] ?? null;

        return $sqlState === '23000'
            && (
                (is_int($driverCode) || is_string($driverCode))
                    ? in_array((int) $driverCode, [19, 1062], true)
                    : false
            );
    }
}
