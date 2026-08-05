<?php

namespace Tests\Feature\Operations;

use App\Operations\ProductionConfigurationVerifier;
use App\Payments\Infrastructure\DeterministicTestPaymentProvider;
use Illuminate\Support\Facades\Artisan;
use Tests\TestCase;

final class ProductionConfigurationVerifierTest extends TestCase
{
    protected function setUp(): void
    {
        parent::setUp();

        config([
            'app.env' => 'production',
            'app.debug' => false,
            'app.key' => 'base64:dGVzdC1vbmx5LXByb2R1Y3Rpb24ta2V5',
            'app.url' => 'https://platform.oteryn.com',
            'session.secure' => true,
            'session.http_only' => true,
            'mail.default' => 'smtp',
            'mail.mailers.smtp.transport' => 'smtp',
            'mail.from.address' => 'noreply@oteryn.com',
            'marketplace.enabled' => true,
            'marketplace.escrow_canary_account_id' => 9999,
            'marketplace.allowed_duration_days' => [1, 3, 7],
            'marketplace.minimum_starting_bid' => 100,
            'marketplace.minimum_bid_increment' => 10,
            'marketplace.commission_basis_points' => 1000,
            'marketplace.escrow_quiescence_seconds' => 30,
            'marketplace.public_bid_history_limit' => 20,
            'marketplace.character_limit' => 10,
            'database.connections.canary_character_transfer.username' => 'oteryn_character_transfer',
            'payments.enabled' => false,
        ]);
    }

    public function test_compliant_provider_independent_configuration_passes(): void
    {
        self::assertSame([], app(ProductionConfigurationVerifier::class)->inspect());

        self::assertSame(0, Artisan::call('production:verify-configuration'));
        self::assertStringContainsString(
            'Production configuration invariant checks passed.',
            Artisan::output(),
        );
    }

    public function test_non_production_environment_is_rejected(): void
    {
        config(['app.env' => 'local']);

        $this->assertViolation('APP_ENV must be production.');
    }

    public function test_debug_mode_is_rejected(): void
    {
        config(['app.debug' => true]);

        $this->assertViolation('APP_DEBUG must be disabled.');
    }

    public function test_missing_application_key_is_rejected(): void
    {
        config(['app.key' => '']);

        $this->assertViolation('APP_KEY must be configured.');
    }

    public function test_non_https_application_url_is_rejected(): void
    {
        config(['app.url' => 'http://platform.oteryn.com']);

        $this->assertViolation('APP_URL must use HTTPS.');
    }

    public function test_localhost_and_loopback_application_urls_are_rejected(): void
    {
        config(['app.url' => 'https://localhost']);
        $this->assertViolation('APP_URL must not use a localhost or loopback host.');

        config(['app.url' => 'https://127.0.0.5']);
        $this->assertViolation('APP_URL must not use a localhost or loopback host.');

        config(['app.url' => 'https://[::1]']);
        $this->assertViolation('APP_URL must not use a localhost or loopback host.');
    }

    public function test_insecure_session_cookie_is_rejected(): void
    {
        config(['session.secure' => false]);

        $this->assertViolation('Secure session cookies must be enabled.');
    }

    public function test_non_http_only_session_cookie_is_rejected(): void
    {
        config(['session.http_only' => false]);

        $this->assertViolation('HttpOnly session cookies must be enabled.');
    }

    public function test_non_delivery_mail_transports_are_rejected(): void
    {
        config([
            'mail.default' => 'array',
            'mail.mailers.array.transport' => 'array',
        ]);
        $this->assertViolation('The default mailer must use a delivery-capable transport.');

        config([
            'mail.default' => 'log',
            'mail.mailers.log.transport' => 'log',
        ]);
        $this->assertViolation('The default mailer must use a delivery-capable transport.');
    }

    public function test_invalid_or_reserved_test_sender_address_is_rejected(): void
    {
        config(['mail.from.address' => 'not-an-email']);
        $this->assertViolation('MAIL_FROM_ADDRESS must be a valid email address.');

        config(['mail.from.address' => 'noreply@example.test']);
        $this->assertViolation('MAIL_FROM_ADDRESS must not use a reserved test domain.');
    }

    public function test_enabled_marketplace_requires_valid_escrow_and_transfer_configuration(): void
    {
        config(['marketplace.escrow_canary_account_id' => 0]);
        $this->assertViolation('MARKETPLACE_ESCROW_CANARY_ACCOUNT_ID must be a positive Canary account ID.');

        config([
            'marketplace.escrow_canary_account_id' => 9999,
            'database.connections.canary_character_transfer.username' => 'root',
        ]);
        $this->assertViolation('The dedicated Canary character-transfer database username must be configured and must not be root.');
    }

    public function test_disabled_marketplace_does_not_require_transfer_prerequisites(): void
    {
        config([
            'marketplace.enabled' => false,
            'marketplace.escrow_canary_account_id' => 0,
            'database.connections.canary_character_transfer.username' => 'root',
        ]);

        self::assertSame([], app(ProductionConfigurationVerifier::class)->inspect());
    }

    public function test_enabled_payments_reject_the_deterministic_test_provider(): void
    {
        config([
            'payments.enabled' => true,
            'payments.provider' => 'test',
            'payments.provider_verified' => true,
            'payments.provider_adapter_class' => DeterministicTestPaymentProvider::class,
            'payments.webhook_verifier_class' => DeterministicTestPaymentProvider::class,
            'payments.allowed_currencies' => ['PLN', 'EUR'],
            'payments.maximum_order_amount_minor' => 100_000,
            'payments.webhook.maximum_payload_bytes' => 32_768,
            'payments.webhook.signature_tolerance_seconds' => 300,
            'payments.webhook.test_secret' => 'must-not-be-used-in-production',
        ]);

        $this->assertViolation('PAYMENTS_PROVIDER must identify an approved non-test provider.');
        $this->assertViolation('PAYMENTS_TEST_SECRET must not be configured for an enabled production provider.');
    }

    public function test_enabled_payments_require_a_verified_real_provider_profile(): void
    {
        config([
            'payments.enabled' => true,
            'payments.provider' => 'future-provider',
            'payments.provider_verified' => false,
            'payments.provider_adapter_class' => null,
            'payments.webhook_verifier_class' => null,
            'payments.allowed_currencies' => ['PLN'],
            'payments.maximum_order_amount_minor' => 100_000,
            'payments.webhook.maximum_payload_bytes' => 32_768,
            'payments.webhook.signature_tolerance_seconds' => 300,
            'payments.webhook.test_secret' => null,
        ]);

        $violations = app(ProductionConfigurationVerifier::class)->inspect();
        self::assertContains('The payment provider profile must be directly verified before activation.', $violations);
        self::assertContains('PAYMENTS_PROVIDER_ADAPTER_CLASS must implement PaymentProviderGateway.', $violations);
        self::assertContains('PAYMENTS_WEBHOOK_VERIFIER_CLASS must implement PaymentWebhookVerifier.', $violations);
    }

    public function test_disabled_payments_do_not_require_provider_configuration(): void
    {
        config([
            'payments.enabled' => false,
            'payments.provider' => null,
            'payments.provider_verified' => false,
            'payments.provider_adapter_class' => null,
            'payments.webhook_verifier_class' => null,
        ]);

        self::assertSame([], app(ProductionConfigurationVerifier::class)->inspect());
    }

    public function test_command_fails_closed_without_printing_application_key(): void
    {
        config([
            'app.env' => 'local',
            'app.key' => 'do-not-print-this-secret-value',
        ]);

        self::assertSame(1, Artisan::call('production:verify-configuration'));
        self::assertStringContainsString('Production configuration verification failed.', Artisan::output());
        self::assertStringNotContainsString('do-not-print-this-secret-value', Artisan::output());
    }

    private function assertViolation(string $message): void
    {
        self::assertContains($message, app(ProductionConfigurationVerifier::class)->inspect());
    }
}
