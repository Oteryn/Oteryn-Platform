<?php

use App\Payments\Infrastructure\DeterministicTestPaymentProvider;

$currencies = array_values(array_filter(
    array_map(
        static fn (string $currency): string => strtoupper(trim($currency)),
        explode(',', (string) env('PAYMENTS_ALLOWED_CURRENCIES', 'PLN,EUR')),
    ),
    static fn (string $currency): bool => $currency !== '',
));

return [
    'enabled' => (bool) env('PAYMENTS_ENABLED', false),
    'provider' => env('PAYMENTS_PROVIDER'),
    'provider_verified' => (bool) env('PAYMENTS_PROVIDER_VERIFIED', false),
    'provider_adapter_class' => env('PAYMENTS_PROVIDER_ADAPTER_CLASS'),
    'webhook_verifier_class' => env('PAYMENTS_WEBHOOK_VERIFIER_CLASS'),
    'allowed_currencies' => $currencies,
    'maximum_order_amount_minor' => (int) env('PAYMENTS_MAXIMUM_ORDER_AMOUNT_MINOR', 100_000_000),
    'webhook' => [
        'maximum_payload_bytes' => (int) env('PAYMENTS_WEBHOOK_MAXIMUM_PAYLOAD_BYTES', 32_768),
        'signature_tolerance_seconds' => (int) env('PAYMENTS_WEBHOOK_SIGNATURE_TOLERANCE_SECONDS', 300),
        'test_secret' => env('PAYMENTS_TEST_SECRET'),
    ],
    'test_adapter_class' => DeterministicTestPaymentProvider::class,
];
