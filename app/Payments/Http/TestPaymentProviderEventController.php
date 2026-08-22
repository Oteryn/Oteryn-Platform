<?php

namespace App\Payments\Http;

use App\Payments\Actions\ProcessPaymentProviderEvent;
use App\Payments\Exceptions\PaymentException;
use App\Payments\PaymentTestSurface;
use Illuminate\Http\JsonResponse;
use Illuminate\Http\Request;

final class TestPaymentProviderEventController
{
    public function __construct(
        private readonly ProcessPaymentProviderEvent $processEvent,
        private readonly PaymentTestSurface $testSurface,
    ) {}

    public function __invoke(Request $request): JsonResponse
    {
        try {
            $this->testSurface->ensureAvailable();
            $event = $this->processEvent->execute(
                $request->getContent(),
                $request->headers->all(),
            );
        } catch (PaymentException $exception) {
            $status = in_array($exception->reason, ['invalid_signature', 'expired_signature'], true)
                ? 401
                : 400;

            return response()->json([
                'status' => 'rejected',
                'reason' => $exception->reason,
            ], $status);
        }

        return response()->json([
            'status' => $event->processing_state,
            'reconciliation_reason' => $event->failure_code,
        ], 202);
    }
}
