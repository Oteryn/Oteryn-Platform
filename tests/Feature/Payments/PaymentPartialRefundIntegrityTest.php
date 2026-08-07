<?php

namespace Tests\Feature\Payments;

use App\Identity\Models\Identity;
use App\Payments\Actions\CreatePaymentOrder;
use App\Payments\Actions\ProcessPaymentProviderEvent;
use App\Payments\Infrastructure\DeterministicTestPaymentProvider;
use App\Payments\Models\PaymentOrder;
use App\Payments\Models\PaymentOrderTransition;
use App\Payments\Models\PaymentProviderEvent;
use App\Payments\Models\PaymentReconciliationEntry;
use Carbon\CarbonImmutable;
use Illuminate\Foundation\Testing\RefreshDatabase;
use Illuminate\Support\Facades\Hash;
use Illuminate\Support\Str;
use Tests\TestCase;

final class PaymentPartialRefundIntegrityTest extends TestCase
{
    use RefreshDatabase;

    private const SECRET = 'partial-refund-integrity-secret';

    private CarbonImmutable $now;

    protected function setUp(): void
    {
        parent::setUp();

        $this->now = CarbonImmutable::parse('2026-08-07T14:30:00Z');

        config([
            'app.env' => 'testing',
            'payments.enabled' => true,
            'payments.provider' => DeterministicTestPaymentProvider::PROVIDER,
            'payments.provider_verified' => false,
            'payments.allowed_currencies' => ['PLN', 'EUR'],
            'payments.maximum_order_amount_minor' => 100_000_000,
            'payments.webhook.maximum_payload_bytes' => 32_768,
            'payments.webhook.signature_tolerance_seconds' => 300,
            'payments.webhook.test_secret' => self::SECRET,
        ]);
    }

    public function test_distinct_partial_refunds_accumulate_replay_is_idempotent_over_refund_reconciles_and_full_refund_closes_truth(): void
    {
        $order = app(CreatePaymentOrder::class)->execute(
            $this->identity('partial-refund@example.com'),
            'PLN',
            1_000,
            (string) Str::uuid(),
        );

        $this->process((string) Str::uuid(), 'payment.succeeded', $order, 1_000);

        $firstPartial = $this->process(
            (string) Str::uuid(),
            'payment.partially_refunded',
            $order,
            300,
        );
        $secondEventId = (string) Str::uuid();
        $secondPartial = $this->process(
            $secondEventId,
            'payment.partially_refunded',
            $order,
            200,
        );
        $replayedSecond = $this->process(
            $secondEventId,
            'payment.partially_refunded',
            $order,
            200,
        );

        self::assertSame($secondPartial->id, $replayedSecond->id);
        self::assertSame(PaymentProviderEvent::STATE_PROCESSED, $firstPartial->processing_state);
        self::assertSame(PaymentProviderEvent::STATE_PROCESSED, $secondPartial->processing_state);
        self::assertSame(PaymentOrder::STATUS_PARTIALLY_REFUNDED, $order->refresh()->status);
        self::assertSame(4, $order->version);

        $refundTransitions = PaymentOrderTransition::query()
            ->where('payment_order_id', $order->id)
            ->whereNotNull('refunded_total_minor')
            ->orderBy('version')
            ->get();

        self::assertCount(2, $refundTransitions);
        $firstRefundTransition = $refundTransitions->get(0);
        $secondRefundTransition = $refundTransitions->get(1);
        self::assertInstanceOf(PaymentOrderTransition::class, $firstRefundTransition);
        self::assertInstanceOf(PaymentOrderTransition::class, $secondRefundTransition);
        self::assertSame(300, $firstRefundTransition->verified_refund_amount_minor);
        self::assertSame(300, $firstRefundTransition->refunded_total_minor);
        self::assertSame(200, $secondRefundTransition->verified_refund_amount_minor);
        self::assertSame(500, $secondRefundTransition->refunded_total_minor);

        $overRefund = $this->process(
            (string) Str::uuid(),
            'payment.partially_refunded',
            $order,
            600,
        );

        self::assertSame(PaymentProviderEvent::STATE_RECONCILIATION, $overRefund->processing_state);
        self::assertSame('refund_integrity_mismatch', $overRefund->failure_code);
        self::assertSame(PaymentOrder::STATUS_PARTIALLY_REFUNDED, $order->refresh()->status);
        self::assertSame(4, $order->version);
        self::assertSame(2, PaymentOrderTransition::query()
            ->where('payment_order_id', $order->id)
            ->whereNotNull('refunded_total_minor')
            ->count());
        self::assertSame(1, PaymentReconciliationEntry::query()
            ->where('payment_order_id', $order->id)
            ->where('issue_type', 'refund_integrity_mismatch')
            ->count());

        $fullRefund = $this->process(
            (string) Str::uuid(),
            'payment.refunded',
            $order,
            1_000,
        );

        self::assertSame(PaymentProviderEvent::STATE_PROCESSED, $fullRefund->processing_state);
        self::assertSame(PaymentOrder::STATUS_REFUNDED, $order->refresh()->status);
        self::assertSame(5, $order->version);
        self::assertSame(5, PaymentProviderEvent::query()->count());
        self::assertSame(5, PaymentOrderTransition::query()->count());

        $terminalRefund = PaymentOrderTransition::query()
            ->where('payment_order_id', $order->id)
            ->where('to_status', PaymentOrder::STATUS_REFUNDED)
            ->firstOrFail();

        self::assertSame(1_000, $terminalRefund->verified_refund_amount_minor);
        self::assertSame(1_000, $terminalRefund->refunded_total_minor);
    }

    public function test_partial_refund_currency_amount_and_ordering_mismatches_never_create_refund_truth(): void
    {
        $pending = app(CreatePaymentOrder::class)->execute(
            $this->identity('partial-ordering@example.com'),
            'PLN',
            1_000,
            (string) Str::uuid(),
        );
        $outOfOrder = $this->process(
            (string) Str::uuid(),
            'payment.partially_refunded',
            $pending,
            200,
        );

        self::assertSame(PaymentProviderEvent::STATE_RECONCILIATION, $outOfOrder->processing_state);
        self::assertSame('out_of_order_transition', $outOfOrder->failure_code);
        self::assertSame(PaymentOrder::STATUS_PENDING, $pending->refresh()->status);

        $settled = app(CreatePaymentOrder::class)->execute(
            $this->identity('partial-mismatch@example.com'),
            'PLN',
            1_000,
            (string) Str::uuid(),
        );
        $this->process((string) Str::uuid(), 'payment.succeeded', $settled, 1_000);

        $currencyMismatch = $this->process(
            (string) Str::uuid(),
            'payment.partially_refunded',
            $settled,
            200,
            'EUR',
        );
        $amountMismatch = $this->process(
            (string) Str::uuid(),
            'payment.partially_refunded',
            $settled,
            1_000,
        );

        self::assertSame(PaymentProviderEvent::STATE_RECONCILIATION, $currencyMismatch->processing_state);
        self::assertSame('settlement_integrity_mismatch', $currencyMismatch->failure_code);
        self::assertSame(PaymentProviderEvent::STATE_RECONCILIATION, $amountMismatch->processing_state);
        self::assertSame('settlement_integrity_mismatch', $amountMismatch->failure_code);
        self::assertSame(PaymentOrder::STATUS_SUCCEEDED, $settled->refresh()->status);
        self::assertSame(2, $settled->version);
        self::assertSame(0, PaymentOrderTransition::query()->whereNotNull('refunded_total_minor')->count());
    }

    public function test_legacy_partial_state_without_durable_refund_history_fails_closed(): void
    {
        $order = app(CreatePaymentOrder::class)->execute(
            $this->identity('partial-legacy-gap@example.com'),
            'PLN',
            1_000,
            (string) Str::uuid(),
        );
        $this->process((string) Str::uuid(), 'payment.succeeded', $order, 1_000);

        $order->status = PaymentOrder::STATUS_PARTIALLY_REFUNDED;
        $order->save();

        $event = $this->process(
            (string) Str::uuid(),
            'payment.partially_refunded',
            $order,
            100,
        );

        self::assertSame(PaymentProviderEvent::STATE_RECONCILIATION, $event->processing_state);
        self::assertSame('refund_integrity_mismatch', $event->failure_code);
        self::assertSame(PaymentOrder::STATUS_PARTIALLY_REFUNDED, $order->refresh()->status);
        self::assertSame(2, $order->version);
        self::assertSame(0, PaymentOrderTransition::query()->whereNotNull('refunded_total_minor')->count());
    }

    private function identity(string $email): Identity
    {
        return Identity::query()->create([
            'email' => $email,
            'password' => Hash::make('Correct-Horse-9!Battery'),
        ]);
    }

    private function process(
        string $eventId,
        string $eventType,
        PaymentOrder $order,
        int $amountMinor,
        string $currency = 'PLN',
    ): PaymentProviderEvent {
        $payload = json_encode([
            'id' => $eventId,
            'type' => $eventType,
            'created' => $this->now->getTimestamp(),
            'data' => [
                'order_public_id' => $order->public_id,
                'currency' => $currency,
                'amount_minor' => $amountMinor,
                'provider_object_reference' => null,
            ],
        ], JSON_THROW_ON_ERROR);

        return app(ProcessPaymentProviderEvent::class)->execute(
            $payload,
            [
                DeterministicTestPaymentProvider::TIMESTAMP_HEADER => (string) $this->now->getTimestamp(),
                DeterministicTestPaymentProvider::SIGNATURE_HEADER => DeterministicTestPaymentProvider::signature(
                    self::SECRET,
                    $this->now->getTimestamp(),
                    $payload,
                ),
            ],
            $this->now,
        );
    }
}
