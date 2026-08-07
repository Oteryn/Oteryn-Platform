<?php

namespace App\Payments;

use App\Payments\Data\PaymentStateDecision;
use App\Payments\Models\PaymentOrder;

final class PaymentOrderStateMachine
{
    /**
     * @var array<string, string>
     */
    private const EVENT_TARGETS = [
        'payment.succeeded' => PaymentOrder::STATUS_SUCCEEDED,
        'payment.failed' => PaymentOrder::STATUS_FAILED,
        'payment.cancelled' => PaymentOrder::STATUS_CANCELLED,
        'payment.expired' => PaymentOrder::STATUS_EXPIRED,
        'payment.partially_refunded' => PaymentOrder::STATUS_PARTIALLY_REFUNDED,
        'payment.refunded' => PaymentOrder::STATUS_REFUNDED,
        'payment.disputed' => PaymentOrder::STATUS_DISPUTED,
        'payment.charged_back' => PaymentOrder::STATUS_CHARGED_BACK,
    ];

    /**
     * @var array<string, list<string>>
     */
    private const ALLOWED_TARGETS = [
        PaymentOrder::STATUS_PENDING => [
            PaymentOrder::STATUS_SUCCEEDED,
            PaymentOrder::STATUS_FAILED,
            PaymentOrder::STATUS_CANCELLED,
            PaymentOrder::STATUS_EXPIRED,
        ],
        PaymentOrder::STATUS_CHECKOUT_CREATED => [
            PaymentOrder::STATUS_SUCCEEDED,
            PaymentOrder::STATUS_FAILED,
            PaymentOrder::STATUS_CANCELLED,
            PaymentOrder::STATUS_EXPIRED,
        ],
        PaymentOrder::STATUS_SUCCEEDED => [
            PaymentOrder::STATUS_PARTIALLY_REFUNDED,
            PaymentOrder::STATUS_REFUNDED,
            PaymentOrder::STATUS_DISPUTED,
            PaymentOrder::STATUS_CHARGED_BACK,
        ],
        PaymentOrder::STATUS_PARTIALLY_REFUNDED => [
            PaymentOrder::STATUS_REFUNDED,
            PaymentOrder::STATUS_DISPUTED,
            PaymentOrder::STATUS_CHARGED_BACK,
        ],
        PaymentOrder::STATUS_DISPUTED => [
            PaymentOrder::STATUS_REFUNDED,
            PaymentOrder::STATUS_CHARGED_BACK,
        ],
    ];

    public function decide(string $currentStatus, string $eventType): PaymentStateDecision
    {
        $targetStatus = self::EVENT_TARGETS[$eventType] ?? null;

        if ($targetStatus === null) {
            return new PaymentStateDecision(
                PaymentStateDecision::RECONCILE,
                $currentStatus,
                null,
                'unsupported_event_type',
            );
        }

        if ($eventType === 'payment.partially_refunded'
            && $currentStatus === PaymentOrder::STATUS_PARTIALLY_REFUNDED) {
            return new PaymentStateDecision(
                PaymentStateDecision::APPLY,
                $currentStatus,
                $targetStatus,
                'provider_partial_refund',
            );
        }

        if ($targetStatus === $currentStatus) {
            return new PaymentStateDecision(
                PaymentStateDecision::NOOP,
                $currentStatus,
                $targetStatus,
                'duplicate_state',
            );
        }

        if (in_array($targetStatus, self::ALLOWED_TARGETS[$currentStatus] ?? [], true)) {
            return new PaymentStateDecision(
                PaymentStateDecision::APPLY,
                $currentStatus,
                $targetStatus,
                'provider_event',
            );
        }

        return new PaymentStateDecision(
            PaymentStateDecision::RECONCILE,
            $currentStatus,
            $targetStatus,
            'out_of_order_transition',
        );
    }
}
