<?php

namespace App\Payments\Http;

use App\Identity\Models\Identity;
use App\Payments\Actions\ResolvePaymentReconciliation;
use App\Payments\Exceptions\PaymentException;
use App\Payments\Models\PaymentReconciliationEntry;
use App\Payments\Models\PaymentReconciliationResolution;
use App\Payments\PaymentTestSurface;
use Illuminate\Contracts\View\View;
use Illuminate\Http\RedirectResponse;
use Illuminate\Http\Request;
use Illuminate\Validation\Rule;

final class AdminPaymentReconciliationController
{
    public function __construct(
        private readonly ResolvePaymentReconciliation $resolve,
        private readonly PaymentTestSurface $testSurface,
    ) {}

    public function index(): View
    {
        return view('admin.payments.reconciliation', [
            'entries' => PaymentReconciliationEntry::query()
                ->with(['paymentOrder', 'providerEvent', 'resolutions'])
                ->orderByRaw("CASE WHEN state = 'open' THEN 0 ELSE 1 END")
                ->orderByDesc('created_at')
                ->limit(100)
                ->get(),
            'testSurfaceAvailable' => $this->testSurface->isAvailable(),
        ]);
    }

    public function resolve(Request $request, PaymentReconciliationEntry $reconciliation): RedirectResponse
    {
        $actor = $request->user();
        abort_unless($actor instanceof Identity, 403);

        /** @var array{request_id: string, resolution_code: string} $validated */
        $validated = $request->validate([
            'request_id' => ['required', 'uuid'],
            'resolution_code' => [
                'required',
                'string',
                Rule::in([PaymentReconciliationResolution::REVIEWED_NO_PAYMENT_STATE_CHANGE]),
            ],
        ]);

        try {
            $this->resolve->execute(
                $actor,
                $reconciliation,
                $validated['resolution_code'],
                'payments-reconciliation:'.$validated['request_id'],
            );
        } catch (PaymentException $exception) {
            return back()->withErrors([
                'payments' => $this->safeError($exception),
            ]);
        }

        return redirect()
            ->route('admin.payments.reconciliation.index')
            ->with('status', __('payments.admin.resolved'));
    }

    private function safeError(PaymentException $exception): string
    {
        return match ($exception->reason) {
            'test_surface_unavailable', 'payments_disabled' => __('payments.errors.unavailable'),
            'reconciliation_already_resolved' => __('payments.errors.already_resolved'),
            'idempotency_conflict', 'resolution_code_invalid' => __('payments.errors.invalid_request'),
            'dependency_unavailable', 'test_surface_misconfigured' => __('payments.errors.temporary'),
            default => __('payments.errors.generic'),
        };
    }
}
