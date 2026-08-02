#!/usr/bin/env python3
from __future__ import annotations

from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]


def replace_once(path: str, old: str, new: str) -> None:
    target = ROOT / path
    text = target.read_text(encoding="utf-8")
    if text.count(old) != 1:
        raise RuntimeError(f"{path}: expected one marker, found {text.count(old)}")
    target.write_text(text.replace(old, new, 1), encoding="utf-8")


def patch_provider() -> None:
    replace_once(
        "app/Providers/AppServiceProvider.php",
        "use App\\Marketplace\\Contracts\\CanaryCharacterTransferGateway;\n",
        "use App\\Marketplace\\Contracts\\CanaryCharacterTransferGateway;\n"
        "use App\\Payments\\Contracts\\PaymentProviderGateway;\n"
        "use App\\Payments\\Contracts\\PaymentWebhookVerifier;\n"
        "use App\\Payments\\Infrastructure\\DeterministicTestPaymentProvider;\n",
    )
    replace_once(
        "app/Providers/AppServiceProvider.php",
        "        $this->app->bind(CanaryCharacterTransferGateway::class, CanaryCharacterTransfer::class);\n",
        "        $this->app->bind(CanaryCharacterTransferGateway::class, CanaryCharacterTransfer::class);\n"
        "        $this->app->bind(\n"
        "            PaymentProviderGateway::class,\n"
        "            fn (): PaymentProviderGateway => $this->deterministicTestPaymentProvider(),\n"
        "        );\n"
        "        $this->app->bind(\n"
        "            PaymentWebhookVerifier::class,\n"
        "            fn (): PaymentWebhookVerifier => $this->deterministicTestPaymentProvider(),\n"
        "        );\n",
    )
    replace_once(
        "app/Providers/AppServiceProvider.php",
        "    private function configureLocalization(): void\n",
        "    private function deterministicTestPaymentProvider(): DeterministicTestPaymentProvider\n"
        "    {\n"
        "        if (config('payments.provider') !== DeterministicTestPaymentProvider::PROVIDER) {\n"
        "            throw new LogicException('No approved payment provider adapter is bound.');\n"
        "        }\n\n"
        "        $secret = config('payments.webhook.test_secret');\n"
        "        $maximumPayloadBytes = config('payments.webhook.maximum_payload_bytes');\n"
        "        $signatureToleranceSeconds = config('payments.webhook.signature_tolerance_seconds');\n\n"
        "        if (! is_string($secret)\n"
        "            || ! is_int($maximumPayloadBytes)\n"
        "            || ! is_int($signatureToleranceSeconds)) {\n"
        "            throw new LogicException('The deterministic test payment provider is not configured.');\n"
        "        }\n\n"
        "        return new DeterministicTestPaymentProvider(\n"
        "            $secret,\n"
        "            $maximumPayloadBytes,\n"
        "            $signatureToleranceSeconds,\n"
        "        );\n"
        "    }\n\n"
        "    private function configureLocalization(): void\n",
    )


def patch_verifier() -> None:
    replace_once(
        "app/Operations/ProductionConfigurationVerifier.php",
        "        if (config('marketplace.enabled')) {\n"
        "            array_push($violations, ...$this->marketplaceViolations());\n"
        "        }\n\n"
        "        return $violations;\n",
        "        if (config('marketplace.enabled')) {\n"
        "            array_push($violations, ...$this->marketplaceViolations());\n"
        "        }\n\n"
        "        if (config('payments.enabled')) {\n"
        "            array_push($violations, ...$this->paymentViolations());\n"
        "        }\n\n"
        "        return $violations;\n",
    )
    replace_once(
        "app/Operations/ProductionConfigurationVerifier.php",
        "    private function hasDeliveryCapableMailer(): bool\n",
        "    /** @return list<string> */\n"
        "    private function paymentViolations(): array\n"
        "    {\n"
        "        $violations = [];\n"
        "        $provider = config('payments.provider');\n"
        "        if (! is_string($provider) || trim($provider) === '' || strtolower($provider) === 'test') {\n"
        "            $violations[] = 'PAYMENTS_PROVIDER must identify an approved non-test provider.';\n"
        "        }\n\n"
        "        if (config('payments.provider_verified') !== true) {\n"
        "            $violations[] = 'The payment provider profile must be directly verified before activation.';\n"
        "        }\n\n"
        "        $adapter = config('payments.provider_adapter_class');\n"
        "        if (! is_string($adapter)\n"
        "            || ! class_exists($adapter)\n"
        "            || ! is_a($adapter, \\App\\Payments\\Contracts\\PaymentProviderGateway::class, true)) {\n"
        "            $violations[] = 'PAYMENTS_PROVIDER_ADAPTER_CLASS must implement PaymentProviderGateway.';\n"
        "        }\n\n"
        "        $verifier = config('payments.webhook_verifier_class');\n"
        "        if (! is_string($verifier)\n"
        "            || ! class_exists($verifier)\n"
        "            || ! is_a($verifier, \\App\\Payments\\Contracts\\PaymentWebhookVerifier::class, true)) {\n"
        "            $violations[] = 'PAYMENTS_WEBHOOK_VERIFIER_CLASS must implement PaymentWebhookVerifier.';\n"
        "        }\n\n"
        "        $currencies = config('payments.allowed_currencies');\n"
        "        if (! is_array($currencies)\n"
        "            || $currencies === []\n"
        "            || array_filter(\n"
        "                $currencies,\n"
        "                static fn (mixed $currency): bool => ! is_string($currency)\n"
        "                    || preg_match('/^[A-Z]{3}$/', $currency) !== 1,\n"
        "            ) !== []) {\n"
        "            $violations[] = 'Payment currencies must be a non-empty list of ISO-style uppercase codes.';\n"
        "        }\n\n"
        "        $maximumAmount = config('payments.maximum_order_amount_minor');\n"
        "        if (! is_int($maximumAmount) || $maximumAmount < 1) {\n"
        "            $violations[] = 'The maximum payment order amount must be a positive integer in minor units.';\n"
        "        }\n\n"
        "        $maximumPayloadBytes = config('payments.webhook.maximum_payload_bytes');\n"
        "        if (! is_int($maximumPayloadBytes)\n"
        "            || $maximumPayloadBytes < 1\n"
        "            || $maximumPayloadBytes > 1_048_576) {\n"
        "            $violations[] = 'The payment webhook payload limit must be between 1 and 1048576 bytes.';\n"
        "        }\n\n"
        "        $signatureTolerance = config('payments.webhook.signature_tolerance_seconds');\n"
        "        if (! is_int($signatureTolerance)\n"
        "            || $signatureTolerance < 1\n"
        "            || $signatureTolerance > 900) {\n"
        "            $violations[] = 'The payment webhook signature tolerance must be between 1 and 900 seconds.';\n"
        "        }\n\n"
        "        $testSecret = config('payments.webhook.test_secret');\n"
        "        if (is_string($testSecret) && trim($testSecret) !== '') {\n"
        "            $violations[] = 'PAYMENTS_TEST_SECRET must not be configured for an enabled production provider.';\n"
        "        }\n\n"
        "        return $violations;\n"
        "    }\n\n"
        "    private function hasDeliveryCapableMailer(): bool\n",
    )


def patch_verifier_test() -> None:
    replace_once(
        "tests/Feature/Operations/ProductionConfigurationVerifierTest.php",
        "            'database.connections.canary_character_transfer.username' => 'oteryn_character_transfer',\n",
        "            'database.connections.canary_character_transfer.username' => 'oteryn_character_transfer',\n"
        "            'payments.enabled' => false,\n",
    )
    replace_once(
        "tests/Feature/Operations/ProductionConfigurationVerifierTest.php",
        "    public function test_command_fails_closed_without_printing_application_key(): void\n",
        "    public function test_enabled_payments_reject_the_deterministic_test_provider(): void\n"
        "    {\n"
        "        config([\n"
        "            'payments.enabled' => true,\n"
        "            'payments.provider' => 'test',\n"
        "            'payments.provider_verified' => true,\n"
        "            'payments.provider_adapter_class' => \\App\\Payments\\Infrastructure\\DeterministicTestPaymentProvider::class,\n"
        "            'payments.webhook_verifier_class' => \\App\\Payments\\Infrastructure\\DeterministicTestPaymentProvider::class,\n"
        "            'payments.allowed_currencies' => ['PLN', 'EUR'],\n"
        "            'payments.maximum_order_amount_minor' => 100_000,\n"
        "            'payments.webhook.maximum_payload_bytes' => 32_768,\n"
        "            'payments.webhook.signature_tolerance_seconds' => 300,\n"
        "            'payments.webhook.test_secret' => 'must-not-be-used-in-production',\n"
        "        ]);\n\n"
        "        $this->assertViolation('PAYMENTS_PROVIDER must identify an approved non-test provider.');\n"
        "        $this->assertViolation('PAYMENTS_TEST_SECRET must not be configured for an enabled production provider.');\n"
        "    }\n\n"
        "    public function test_enabled_payments_require_a_verified_real_provider_profile(): void\n"
        "    {\n"
        "        config([\n"
        "            'payments.enabled' => true,\n"
        "            'payments.provider' => 'future-provider',\n"
        "            'payments.provider_verified' => false,\n"
        "            'payments.provider_adapter_class' => null,\n"
        "            'payments.webhook_verifier_class' => null,\n"
        "            'payments.allowed_currencies' => ['PLN'],\n"
        "            'payments.maximum_order_amount_minor' => 100_000,\n"
        "            'payments.webhook.maximum_payload_bytes' => 32_768,\n"
        "            'payments.webhook.signature_tolerance_seconds' => 300,\n"
        "            'payments.webhook.test_secret' => null,\n"
        "        ]);\n\n"
        "        $violations = app(ProductionConfigurationVerifier::class)->inspect();\n"
        "        self::assertContains('The payment provider profile must be directly verified before activation.', $violations);\n"
        "        self::assertContains('PAYMENTS_PROVIDER_ADAPTER_CLASS must implement PaymentProviderGateway.', $violations);\n"
        "        self::assertContains('PAYMENTS_WEBHOOK_VERIFIER_CLASS must implement PaymentWebhookVerifier.', $violations);\n"
        "    }\n\n"
        "    public function test_disabled_payments_do_not_require_provider_configuration(): void\n"
        "    {\n"
        "        config([\n"
        "            'payments.enabled' => false,\n"
        "            'payments.provider' => null,\n"
        "            'payments.provider_verified' => false,\n"
        "            'payments.provider_adapter_class' => null,\n"
        "            'payments.webhook_verifier_class' => null,\n"
        "        ]);\n\n"
        "        self::assertSame([], app(ProductionConfigurationVerifier::class)->inspect());\n"
        "    }\n\n"
        "    public function test_command_fails_closed_without_printing_application_key(): void\n",
    )


def patch_environment_example() -> None:
    path = ROOT / ".env.example"
    text = path.read_text(encoding="utf-8")
    marker = "# Provider-neutral payment settlement remains disabled by default."
    if marker in text:
        raise RuntimeError(".env.example: payment block already exists")
    block = """

# Provider-neutral payment settlement remains disabled by default.
# Never select the deterministic test adapter in production. A real provider,
# sandbox evidence, secrets ownership and explicit production authorization are required.
PAYMENTS_ENABLED=false
PAYMENTS_PROVIDER=
PAYMENTS_PROVIDER_VERIFIED=false
PAYMENTS_PROVIDER_ADAPTER_CLASS=
PAYMENTS_WEBHOOK_VERIFIER_CLASS=
PAYMENTS_ALLOWED_CURRENCIES=PLN,EUR
PAYMENTS_MAXIMUM_ORDER_AMOUNT_MINOR=100000000
PAYMENTS_WEBHOOK_MAXIMUM_PAYLOAD_BYTES=32768
PAYMENTS_WEBHOOK_SIGNATURE_TOLERANCE_SECONDS=300
PAYMENTS_TEST_SECRET=
"""
    path.write_text(text.rstrip() + block + "\n", encoding="utf-8")


def patch_concurrency_test() -> None:
    replace_once(
        "tests/Feature/Payments/PaymentEventConcurrencyMariaDbTest.php",
        "$order->freshOrFail()->status",
        "PaymentOrder::query()->findOrFail($order->id)->status",
    )


def main() -> int:
    patch_provider()
    patch_verifier()
    patch_verifier_test()
    patch_environment_example()
    patch_concurrency_test()
    Path(__file__).unlink()
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
