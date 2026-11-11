<?php

namespace App\Payments\Http;

use App\Identity\Models\Identity;
use App\Payments\Actions\CreatePaymentCheckout;
use App\Payments\Actions\CreatePaymentOrder;
use App\Payments\Exceptions\PaymentException;
use App\Payments\Models\PaymentOrder;
use App\Payments\PaymentTestSurface;
use Illuminate\Contracts\View\View;
use Illuminate\Http\RedirectResponse;
use Illuminate\Http\Request;
use Illuminate\Validation\Rule;

final class PaymentAccountController
{
    public function __construct(
        private readonly CreatePaymentOrder $createOrder,
        private readonly CreatePaymentCheckout $createCheckout,
        private readonly PaymentTestSurface $testSurface,
    ) {}

    public function index(Request $request): View
    {
        $identity = $this->identity($request);

        return view('payments.index', [
            'orders' => PaymentOrder::query()
                ->where('identity_id', $identity->id)
                ->orderByDesc('created_at')
                ->paginate(20),
            'testSurfaceAvailable' => $this->testSurface->isAvailable(),
            'allowedCurrencies' => $this->allowedCurrencies(),
            'testAmountMinor' => $this->safeTestAmountMinor(),
        ]);
    }

    public function storeTestCheckout(Request $request): RedirectResponse
    {
        $identity = $this->identity($request);

        try {
            $this->testSurface->ensureAvailable();
            $allowedCurrencies = $this->allowedCurrencies();

            /** @var array{request_id: string, currency: string} $validated */
            $validated = $request->validate([
                'request_id' => ['required', 'uuid'],
                'currency' => ['required', 'string', 'size:3', Rule::in($allowedCurrencies)],
            ]);

            $requestId = $validated['request_id'];
            $order = $this->createOrder->execute(
                $identity,
                strtoupper($validated['currency']),
                $this->testSurface->amountMinor(),
                'payments-test-order:'.$requestId,
            );
            $this->createCheckout->execute(
                $order,
                'payments-test-checkout:'.$requestId,
            );
        } catch (PaymentException $exception) {
            return redirect()
                ->route('payments.account.index')
                ->withErrors(['payments' => $this->safeError($exception)]);
        }

        return redirect()->route('payments.account.return', [
            'orderPublicId' => $order->public_id,
        ]);
    }

    public function returned(Request $request, string $orderPublicId): View
    {
        $identity = $this->identity($request);
        $order = PaymentOrder::query()
            ->where('public_id', $orderPublicId)
            ->where('identity_id', $identity->id)
            ->firstOrFail();

        return view('payments.return', [
            'order' => $order,
        ]);
    }

    private function identity(Request $request): Identity
    {
        $identity = $request->user();
        abort_unless($identity instanceof Identity, 403);

        return $identity;
    }

    /** @return list<string> */
    private function allowedCurrencies(): array
    {
        $currencies = config('payments.allowed_currencies');

        if (! is_array($currencies)) {
            return [];
        }

        return array_values(array_filter(
            $currencies,
            static fn (mixed $currency): bool => is_string($currency) && preg_match('/^[A-Z]{3}$/D', $currency) === 1,
        ));
    }

    private function safeTestAmountMinor(): ?int
    {
        if (! $this->testSurface->isAvailable()) {
            return null;
        }

        try {
            return $this->testSurface->amountMinor();
        } catch (PaymentException) {
            return null;
        }
    }

    private function safeError(PaymentException $exception): string
    {
        return match ($exception->reason) {
            'payments_disabled', 'test_surface_unavailable' => __('payments.errors.unavailable'),
            'test_surface_misconfigured', 'provider_unavailable', 'dependency_unavailable' => __('payments.errors.temporary'),
            'currency_unsupported', 'amount_invalid' => __('payments.errors.invalid_request'),
            'checkout_unavailable', 'checkout_state_conflict', 'idempotency_conflict' => __('payments.errors.recovery'),
            default => __('payments.errors.generic'),
        };
    }
}
