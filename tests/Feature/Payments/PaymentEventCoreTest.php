<?php

namespace Tests\Feature\Payments;

use App\Identity\Models\Identity;
use App\Payments\Actions\CreatePaymentCheckout;
use App\Payments\Actions\CreatePaymentOrder;
use App\Payments\Actions\ProcessPaymentProviderEvent;
use App\Payments\Contracts\PaymentProviderGateway;
use App\Payments\Data\CheckoutSession;
use App\Payments\Exceptions\PaymentException;
use App\Payments\Infrastructure\DeterministicTestPaymentProvider;
use App\Payments\Models\PaymentAttempt;
use App\Payments\Models\PaymentOrder;
use App\Payments\Models\PaymentOrderTransition;
use App\Payments\Models\PaymentProviderEvent;
use App\Payments\Models\PaymentReconciliationEntry;
use Carbon\CarbonImmutable;
use Illuminate\Foundation\Testing\RefreshDatabase;
use Illuminate\Support\Facades\Hash;
use Illuminate\Support\Str;
use Tests\TestCase;

final class PaymentEventCoreTest extends TestCase
{
    use RefreshDatabase;

    private const SECRET = 'deterministic-test-payment-secret';

    protected function setUp(): void
    {
        parent::setUp();

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

    public function test_disabled_payments_reject_all_domain_actions_without_persistence(): void
    {
        $identity = $this->identity('payment-disabled@example.com');
        config(['payments.enabled' => false]);

        try {
            app(CreatePaymentOrder::class)->execute(
                $identity,
                'PLN',
                1_000,
                (string) Str::uuid(),
            );
            self::fail('Disabled payments must reject order creation.');
        } catch (PaymentException $exception) {
            self::assertSame('payments_disabled', $exception->reason);
        }

        self::assertSame(0, PaymentOrder::query()->count());
        self::assertSame(0, PaymentOrderTransition::query()->count());

        config(['payments.enabled' => true]);
        $order = app(CreatePaymentOrder::class)->execute(
            $identity,
            'PLN',
            1_000,
            (string) Str::uuid(),
        );
        config(['payments.enabled' => false]);

        try {
            app(CreatePaymentCheckout::class)->execute($order, (string) Str::uuid());
            self::fail('Disabled payments must reject checkout creation.');
        } catch (PaymentException $exception) {
            self::assertSame('payments_disabled', $exception->reason);
        }

        try {
            app(ProcessPaymentProviderEvent::class)->execute('{"malformed":', []);
            self::fail('Disabled payments must reject provider-event processing.');
        } catch (PaymentException $exception) {
            self::assertSame('payments_disabled', $exception->reason);
        }

        self::assertSame(PaymentOrder::STATUS_PENDING, $order->refresh()->status);
        self::assertSame(1, $order->version);
        self::assertSame(1, PaymentOrderTransition::query()->count());
        self::assertSame(0, PaymentAttempt::query()->count());
        self::assertSame(0, PaymentProviderEvent::query()->count());
        self::assertSame(0, PaymentReconciliationEntry::query()->count());
    }

    public function test_order_and_checkout_replay_require_the_exact_original_request(): void
    {
        $identity = $this->identity('payment-order@example.com');
        $orderAction = app(CreatePaymentOrder::class);
        $orderRequestId = (string) Str::uuid();

        $order = $orderAction->execute($identity, 'pln', 12_345, $orderRequestId);
        $replayedOrder = $orderAction->execute($identity, 'PLN', 12_345, $orderRequestId);

        self::assertSame($order->id, $replayedOrder->id);
        self::assertSame(PaymentOrder::STATUS_PENDING, $order->status);
        self::assertSame(1, PaymentOrder::query()->count());
        self::assertSame(1, PaymentOrderTransition::query()->count());

        try {
            $orderAction->execute($identity, 'PLN', 12_346, $orderRequestId);
            self::fail('An order idempotency key must not accept a different amount.');
        } catch (PaymentException $exception) {
            self::assertSame('idempotency_conflict', $exception->reason);
        }

        $checkoutAction = app(CreatePaymentCheckout::class);
        $checkoutRequestId = (string) Str::uuid();
        $attempt = $checkoutAction->execute($order, $checkoutRequestId);
        $replayedAttempt = $checkoutAction->execute($order->refresh(), $checkoutRequestId);

        self::assertSame($attempt->id, $replayedAttempt->id);
        self::assertSame(PaymentAttempt::STATUS_READY, $attempt->status);
        self::assertStringStartsWith('test_checkout_', (string) $attempt->provider_checkout_reference);
        self::assertSame(PaymentOrder::STATUS_CHECKOUT_CREATED, $order->refresh()->status);
        self::assertSame(2, PaymentOrderTransition::query()->count());
        self::assertSame(0, PaymentProviderEvent::query()->count());
    }

    public function test_invalid_signature_is_rejected_before_malformed_payload_parsing(): void
    {
        $now = CarbonImmutable::parse('2026-08-02T12:00:00Z');

        try {
            app(ProcessPaymentProviderEvent::class)->execute(
                '{"malformed":',
                [
                    DeterministicTestPaymentProvider::TIMESTAMP_HEADER => (string) $now->getTimestamp(),
                    DeterministicTestPaymentProvider::SIGNATURE_HEADER => str_repeat('0', 64),
                ],
                $now,
            );
            self::fail('An invalid signature must be rejected.');
        } catch (PaymentException $exception) {
            self::assertSame('invalid_signature', $exception->reason);
        }

        self::assertSame(0, PaymentProviderEvent::query()->count());
        self::assertSame(0, PaymentReconciliationEntry::query()->count());
    }

    public function test_expired_signature_is_rejected_without_persistence(): void
    {
        $now = CarbonImmutable::parse('2026-08-02T12:00:00Z');
        $timestamp = $now->subMinutes(10)->getTimestamp();
        $payload = $this->payload((string) Str::uuid(), 'payment.succeeded', (string) Str::uuid());

        try {
            app(ProcessPaymentProviderEvent::class)->execute(
                $payload,
                $this->headers($payload, $timestamp),
                $now,
            );
            self::fail('An expired signature must be rejected.');
        } catch (PaymentException $exception) {
            self::assertSame('expired_signature', $exception->reason);
        }

        self::assertSame(0, PaymentProviderEvent::query()->count());
    }

    public function test_signed_success_is_persisted_once_without_raw_payload_or_personal_data(): void
    {
        $identity = $this->identity('payment-success@example.com');
        $order = app(CreatePaymentOrder::class)->execute(
            $identity,
            'EUR',
            9_999,
            (string) Str::uuid(),
        );
        $now = CarbonImmutable::parse('2026-08-02T12:00:00Z');
        $eventId = (string) Str::uuid();
        $payload = $this->payload(
            $eventId,
            'payment.succeeded',
            $order->public_id,
            ['customer_email' => 'must-not-be-stored@example.test'],
        );

        $event = app(ProcessPaymentProviderEvent::class)->execute(
            $payload,
            $this->headers($payload, $now->getTimestamp()),
            $now,
        );
        $replayed = app(ProcessPaymentProviderEvent::class)->execute(
            $payload,
            $this->headers($payload, $now->getTimestamp()),
            $now,
        );

        self::assertSame($event->id, $replayed->id);
        self::assertSame(PaymentProviderEvent::STATE_PROCESSED, $event->processing_state);
        self::assertSame(PaymentOrder::STATUS_SUCCEEDED, $order->refresh()->status);
        self::assertSame(2, $order->version);
        self::assertSame(1, PaymentProviderEvent::query()->count());
        self::assertSame(2, PaymentOrderTransition::query()->count());
        self::assertSame(0, PaymentReconciliationEntry::query()->count());

        $stored = PaymentProviderEvent::query()->firstOrFail();
        self::assertSame(hash('sha256', $payload), $stored->payload_sha256);
        self::assertSame(['event_created' => $now->getTimestamp()], $stored->metadata);
        self::assertStringNotContainsString(
            'must-not-be-stored@example.test',
            json_encode($stored->getAttributes(), JSON_THROW_ON_ERROR),
        );
        self::assertStringNotContainsString(
            $payload,
            json_encode($stored->getAttributes(), JSON_THROW_ON_ERROR),
        );
    }

    public function test_same_event_id_with_different_payload_fails_closed(): void
    {
        $identity = $this->identity('payment-conflict@example.com');
        $order = app(CreatePaymentOrder::class)->execute(
            $identity,
            'PLN',
            1_000,
            (string) Str::uuid(),
        );
        $now = CarbonImmutable::parse('2026-08-02T12:00:00Z');
        $eventId = (string) Str::uuid();
        $success = $this->payload($eventId, 'payment.succeeded', $order->public_id);
        $failed = $this->payload($eventId, 'payment.failed', $order->public_id);

        app(ProcessPaymentProviderEvent::class)->execute(
            $success,
            $this->headers($success, $now->getTimestamp()),
            $now,
        );

        try {
            app(ProcessPaymentProviderEvent::class)->execute(
                $failed,
                $this->headers($failed, $now->getTimestamp()),
                $now,
            );
            self::fail('A provider event identifier must not accept another payload.');
        } catch (PaymentException $exception) {
            self::assertSame('event_id_conflict', $exception->reason);
        }

        self::assertSame(PaymentOrder::STATUS_SUCCEEDED, $order->refresh()->status);
        self::assertSame(1, PaymentProviderEvent::query()->count());
    }

    public function test_out_of_order_event_cannot_regress_terminal_payment_truth(): void
    {
        $identity = $this->identity('payment-ordering@example.com');
        $order = app(CreatePaymentOrder::class)->execute(
            $identity,
            'PLN',
            1_000,
            (string) Str::uuid(),
        );
        $now = CarbonImmutable::parse('2026-08-02T12:00:00Z');

        foreach ([
            [(string) Str::uuid(), 'payment.succeeded'],
            [(string) Str::uuid(), 'payment.failed'],
        ] as [$eventId, $eventType]) {
            $payload = $this->payload($eventId, $eventType, $order->public_id);
            app(ProcessPaymentProviderEvent::class)->execute(
                $payload,
                $this->headers($payload, $now->getTimestamp()),
                $now,
            );
        }

        self::assertSame(PaymentOrder::STATUS_SUCCEEDED, $order->refresh()->status);
        self::assertSame(2, $order->version);
        self::assertSame(2, PaymentProviderEvent::query()->count());
        self::assertSame(1, PaymentReconciliationEntry::query()
            ->where('issue_type', 'out_of_order_transition')
            ->count());
        self::assertSame(
            PaymentProviderEvent::STATE_RECONCILIATION,
            PaymentProviderEvent::query()->latest('id')->firstOrFail()->processing_state,
        );
    }

    public function test_unknown_order_event_is_reconciled_without_creating_payment_truth(): void
    {
        $now = CarbonImmutable::parse('2026-08-02T12:00:00Z');
        $payload = $this->payload(
            (string) Str::uuid(),
            'payment.succeeded',
            (string) Str::uuid(),
        );

        $event = app(ProcessPaymentProviderEvent::class)->execute(
            $payload,
            $this->headers($payload, $now->getTimestamp()),
            $now,
        );

        self::assertNull($event->payment_order_id);
        self::assertSame(PaymentProviderEvent::STATE_RECONCILIATION, $event->processing_state);
        self::assertSame('order_not_found', $event->failure_code);
        self::assertSame(0, PaymentOrder::query()->count());
        self::assertSame(1, PaymentReconciliationEntry::query()->count());
    }

    public function test_ambiguous_checkout_is_persisted_for_reconciliation(): void
    {
        $identity = $this->identity('payment-ambiguous@example.com');
        $order = app(CreatePaymentOrder::class)->execute(
            $identity,
            'PLN',
            2_000,
            (string) Str::uuid(),
        );
        $this->app->instance(PaymentProviderGateway::class, new class implements PaymentProviderGateway
        {
            public function createCheckout(PaymentOrder $order, string $idempotencyKey): CheckoutSession
            {
                throw new PaymentException('timeout', 'Synthetic provider timeout.');
            }
        });

        try {
            app(CreatePaymentCheckout::class)->execute($order, (string) Str::uuid());
            self::fail('An ambiguous provider result must not be reported as checkout success.');
        } catch (PaymentException $exception) {
            self::assertSame('provider_unavailable', $exception->reason);
        }

        self::assertSame(PaymentOrder::STATUS_PENDING, $order->refresh()->status);
        self::assertSame(
            PaymentAttempt::STATUS_AMBIGUOUS,
            PaymentAttempt::query()->firstOrFail()->status,
        );
        self::assertSame(
            'ambiguous_checkout_creation',
            PaymentReconciliationEntry::query()->firstOrFail()->issue_type,
        );
    }

    public function test_test_provider_refuses_production_execution(): void
    {
        config(['app.env' => 'production']);
        $provider = new DeterministicTestPaymentProvider(self::SECRET, 32_768, 300);

        try {
            $provider->verify('{}', [], CarbonImmutable::now());
            self::fail('The test payment provider must refuse production execution.');
        } catch (PaymentException $exception) {
            self::assertSame('test_provider_forbidden', $exception->reason);
        }
    }

    private function identity(string $email): Identity
    {
        return Identity::query()->create([
            'email' => $email,
            'password' => Hash::make('Correct-Horse-9!Battery'),
        ]);
    }

    /**
     * @param  array<string, string>  $extraData
     */
    private function payload(
        string $eventId,
        string $eventType,
        string $orderPublicId,
        array $extraData = [],
    ): string {
        return json_encode([
            'id' => $eventId,
            'type' => $eventType,
            'created' => CarbonImmutable::parse('2026-08-02T12:00:00Z')->getTimestamp(),
            'data' => array_merge([
                'order_public_id' => $orderPublicId,
                'provider_object_reference' => 'test_object_'.substr(hash('sha256', $eventId), 0, 16),
            ], $extraData),
        ], JSON_THROW_ON_ERROR);
    }

    /**
     * @return array<string, string>
     */
    private function headers(string $payload, int $timestamp): array
    {
        return [
            DeterministicTestPaymentProvider::TIMESTAMP_HEADER => (string) $timestamp,
            DeterministicTestPaymentProvider::SIGNATURE_HEADER => DeterministicTestPaymentProvider::signature(
                self::SECRET,
                $timestamp,
                $payload,
            ),
        ];
    }
}
