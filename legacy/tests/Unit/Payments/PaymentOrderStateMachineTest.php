<?php

namespace Tests\Unit\Payments;

use App\Payments\Data\PaymentStateDecision;
use App\Payments\Models\PaymentOrder;
use App\Payments\PaymentOrderStateMachine;
use PHPUnit\Framework\TestCase;

final class PaymentOrderStateMachineTest extends TestCase
{
    public function test_pending_payment_accepts_only_initial_settlement_outcomes(): void
    {
        $machine = new PaymentOrderStateMachine;

        foreach ([
            'payment.succeeded' => PaymentOrder::STATUS_SUCCEEDED,
            'payment.failed' => PaymentOrder::STATUS_FAILED,
            'payment.cancelled' => PaymentOrder::STATUS_CANCELLED,
            'payment.expired' => PaymentOrder::STATUS_EXPIRED,
        ] as $eventType => $expectedStatus) {
            $decision = $machine->decide(PaymentOrder::STATUS_PENDING, $eventType);

            self::assertSame(PaymentStateDecision::APPLY, $decision->action);
            self::assertSame($expectedStatus, $decision->targetStatus);
        }
    }

    public function test_terminal_payment_truth_cannot_regress(): void
    {
        $machine = new PaymentOrderStateMachine;

        foreach ([
            'payment.failed',
            'payment.cancelled',
            'payment.expired',
        ] as $eventType) {
            $decision = $machine->decide(PaymentOrder::STATUS_SUCCEEDED, $eventType);

            self::assertSame(PaymentStateDecision::RECONCILE, $decision->action);
            self::assertSame('out_of_order_transition', $decision->reason);
        }
    }

    public function test_duplicate_state_is_an_idempotent_noop(): void
    {
        $decision = (new PaymentOrderStateMachine)->decide(
            PaymentOrder::STATUS_SUCCEEDED,
            'payment.succeeded',
        );

        self::assertSame(PaymentStateDecision::NOOP, $decision->action);
        self::assertSame(PaymentOrder::STATUS_SUCCEEDED, $decision->targetStatus);
    }

    public function test_unknown_event_requires_reconciliation(): void
    {
        $decision = (new PaymentOrderStateMachine)->decide(
            PaymentOrder::STATUS_PENDING,
            'payment.unknown',
        );

        self::assertSame(PaymentStateDecision::RECONCILE, $decision->action);
        self::assertNull($decision->targetStatus);
        self::assertSame('unsupported_event_type', $decision->reason);
    }
}
