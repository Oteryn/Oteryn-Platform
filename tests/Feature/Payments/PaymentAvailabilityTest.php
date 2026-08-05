<?php

namespace Tests\Feature\Payments;

use App\Identity\Models\Identity;
use App\Payments\Actions\CreatePaymentCheckout;
use App\Payments\Actions\CreatePaymentOrder;
use App\Payments\Actions\ProcessPaymentProviderEvent;
use App\Payments\Exceptions\PaymentException;
use App\Payments\Models\PaymentAttempt;
use App\Payments\Models\PaymentOrder;
use App\Payments\Models\PaymentOrderTransition;
use App\Payments\Models\PaymentProviderEvent;
use App\Payments\Models\PaymentReconciliationEntry;
use Illuminate\Foundation\Testing\RefreshDatabase;
use Illuminate\Support\Facades\Hash;
use Illuminate\Support\Str;
use Tests\TestCase;

final class PaymentAvailabilityTest extends TestCase
{
    use RefreshDatabase;

    protected function setUp(): void
    {
        parent::setUp();

        config([
            'app.env' => 'testing',
            'payments.enabled' => false,
            'payments.provider' => null,
            'payments.webhook.test_secret' => null,
        ]);
    }

    public function test_default_disabled_configuration_rejects_every_domain_action_before_provider_resolution(): void
    {
        $identity = Identity::query()->create([
            'email' => 'payment-default-disabled@example.com',
            'password' => Hash::make('Correct-Horse-9!Battery'),
        ]);

        $this->assertPaymentsDisabled(fn () => app(CreatePaymentOrder::class)->execute(
            $identity,
            'PLN',
            1_000,
            (string) Str::uuid(),
        ));

        $this->assertPaymentsDisabled(fn () => app(CreatePaymentCheckout::class)->execute(
            new PaymentOrder,
            (string) Str::uuid(),
        ));

        $this->assertPaymentsDisabled(fn () => app(ProcessPaymentProviderEvent::class)->execute(
            '{"malformed":',
            [],
        ));

        self::assertSame(0, PaymentOrder::query()->count());
        self::assertSame(0, PaymentAttempt::query()->count());
        self::assertSame(0, PaymentOrderTransition::query()->count());
        self::assertSame(0, PaymentProviderEvent::query()->count());
        self::assertSame(0, PaymentReconciliationEntry::query()->count());
    }

    /**
     * @param  callable(): mixed  $operation
     */
    private function assertPaymentsDisabled(callable $operation): void
    {
        try {
            $operation();
            self::fail('Disabled payments must reject the domain action.');
        } catch (PaymentException $exception) {
            self::assertSame('payments_disabled', $exception->reason);
        }
    }
}
