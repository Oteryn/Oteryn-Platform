<?php

namespace App\Operations;

use App\Payments\Contracts\PaymentProviderGateway;
use App\Payments\Contracts\PaymentWebhookVerifier;

final class ProductionConfigurationVerifier
{
    /**
     * @return list<string>
     */
    public function inspect(): array
    {
        $violations = [];

        if (config('app.env') !== 'production') {
            $violations[] = 'APP_ENV must be production.';
        }

        if (config('app.debug') !== false) {
            $violations[] = 'APP_DEBUG must be disabled.';
        }

        $appKey = config('app.key');
        if (! is_string($appKey) || trim($appKey) === '') {
            $violations[] = 'APP_KEY must be configured.';
        }

        $appUrl = config('app.url');
        if (! is_string($appUrl) || trim($appUrl) === '') {
            $violations[] = 'APP_URL must be configured as an absolute HTTPS URL.';
        } else {
            $scheme = parse_url($appUrl, PHP_URL_SCHEME);
            $host = parse_url($appUrl, PHP_URL_HOST);

            if (! is_string($scheme) || strtolower($scheme) !== 'https') {
                $violations[] = 'APP_URL must use HTTPS.';
            }

            if (! is_string($host) || $host === '') {
                $violations[] = 'APP_URL must include a valid host.';
            } elseif ($this->isLocalHost($host)) {
                $violations[] = 'APP_URL must not use a localhost or loopback host.';
            }
        }

        if (config('session.secure') !== true) {
            $violations[] = 'Secure session cookies must be enabled.';
        }

        if (config('session.http_only') !== true) {
            $violations[] = 'HttpOnly session cookies must be enabled.';
        }

        if (! $this->hasDeliveryCapableMailer()) {
            $violations[] = 'The default mailer must use a delivery-capable transport.';
        }

        $fromAddress = config('mail.from.address');
        if (! is_string($fromAddress) || ! filter_var($fromAddress, FILTER_VALIDATE_EMAIL)) {
            $violations[] = 'MAIL_FROM_ADDRESS must be a valid email address.';
        } elseif ($this->usesReservedTestDomain($fromAddress)) {
            $violations[] = 'MAIL_FROM_ADDRESS must not use a reserved test domain.';
        }

        if (config('marketplace.enabled')) {
            array_push($violations, ...$this->marketplaceViolations());
        }

        if (config('payments.enabled')) {
            array_push($violations, ...$this->paymentViolations());
        }

        return $violations;
    }

    /** @return list<string> */
    private function marketplaceViolations(): array
    {
        $violations = [];
        $escrowAccountId = config('marketplace.escrow_canary_account_id');
        if (! is_int($escrowAccountId) || $escrowAccountId <= 0) {
            $violations[] = 'MARKETPLACE_ESCROW_CANARY_ACCOUNT_ID must be a positive Canary account ID.';
        }

        $durations = config('marketplace.allowed_duration_days');
        if (! is_array($durations) || $durations === []) {
            $violations[] = 'Character Bazaar auction durations must be a non-empty unique list.';
        } elseif (array_filter($durations, static fn (mixed $duration): bool => ! is_int($duration)) !== []) {
            $violations[] = 'Character Bazaar auction durations must be integers between 1 and 30 days.';
        } else {
            /** @var array<array-key, int> $durations */
            if (array_values(array_unique($durations)) !== array_values($durations)) {
                $violations[] = 'Character Bazaar auction durations must be a non-empty unique list.';
            } elseif (array_filter($durations, static fn (int $duration): bool => $duration < 1 || $duration > 30) !== []) {
                $violations[] = 'Character Bazaar auction durations must be integers between 1 and 30 days.';
            }
        }

        foreach (['minimum_starting_bid', 'minimum_bid_increment', 'escrow_quiescence_seconds', 'public_bid_history_limit', 'character_limit'] as $key) {
            $value = config("marketplace.{$key}");
            if (! is_int($value) || $value < 1) {
                $violations[] = "Character Bazaar {$key} must be a positive integer.";
            }
        }

        $commission = config('marketplace.commission_basis_points');
        if (! is_int($commission) || $commission < 0 || $commission > 10_000) {
            $violations[] = 'Character Bazaar commission must be between 0 and 10000 basis points.';
        }

        $transferUsername = config('database.connections.canary_character_transfer.username');
        if (! is_string($transferUsername) || trim($transferUsername) === '' || $transferUsername === 'root') {
            $violations[] = 'The dedicated Canary character-transfer database username must be configured and must not be root.';
        }

        return $violations;
    }

    /** @return list<string> */
    private function paymentViolations(): array
    {
        $violations = [];
        $provider = config('payments.provider');
        if (! is_string($provider) || trim($provider) === '' || strtolower($provider) === 'test') {
            $violations[] = 'PAYMENTS_PROVIDER must identify an approved non-test provider.';
        }

        if (config('payments.provider_verified') !== true) {
            $violations[] = 'The payment provider profile must be directly verified before activation.';
        }

        $adapter = config('payments.provider_adapter_class');
        if (! is_string($adapter)
            || ! class_exists($adapter)
            || ! is_a($adapter, PaymentProviderGateway::class, true)) {
            $violations[] = 'PAYMENTS_PROVIDER_ADAPTER_CLASS must implement PaymentProviderGateway.';
        }

        $verifier = config('payments.webhook_verifier_class');
        if (! is_string($verifier)
            || ! class_exists($verifier)
            || ! is_a($verifier, PaymentWebhookVerifier::class, true)) {
            $violations[] = 'PAYMENTS_WEBHOOK_VERIFIER_CLASS must implement PaymentWebhookVerifier.';
        }

        $currencies = config('payments.allowed_currencies');
        if (! is_array($currencies)
            || $currencies === []
            || array_filter(
                $currencies,
                static fn (mixed $currency): bool => ! is_string($currency)
                    || preg_match('/^[A-Z]{3}$/', $currency) !== 1,
            ) !== []) {
            $violations[] = 'Payment currencies must be a non-empty list of ISO-style uppercase codes.';
        }

        $maximumAmount = config('payments.maximum_order_amount_minor');
        if (! is_int($maximumAmount) || $maximumAmount < 1) {
            $violations[] = 'The maximum payment order amount must be a positive integer in minor units.';
        }

        $maximumPayloadBytes = config('payments.webhook.maximum_payload_bytes');
        if (! is_int($maximumPayloadBytes)
            || $maximumPayloadBytes < 1
            || $maximumPayloadBytes > 1_048_576) {
            $violations[] = 'The payment webhook payload limit must be between 1 and 1048576 bytes.';
        }

        $signatureTolerance = config('payments.webhook.signature_tolerance_seconds');
        if (! is_int($signatureTolerance)
            || $signatureTolerance < 1
            || $signatureTolerance > 900) {
            $violations[] = 'The payment webhook signature tolerance must be between 1 and 900 seconds.';
        }

        $testSecret = config('payments.webhook.test_secret');
        if (is_string($testSecret) && trim($testSecret) !== '') {
            $violations[] = 'PAYMENTS_TEST_SECRET must not be configured for an enabled production provider.';
        }

        return $violations;
    }

    private function hasDeliveryCapableMailer(): bool
    {
        $defaultMailer = config('mail.default');

        if (! is_string($defaultMailer) || $defaultMailer === '') {
            return false;
        }

        $transport = config("mail.mailers.{$defaultMailer}.transport");

        return is_string($transport)
            && $transport !== ''
            && ! in_array(strtolower($transport), ['array', 'log'], true);
    }

    private function isLocalHost(string $host): bool
    {
        $host = strtolower(trim($host, '[]'));

        return $host === 'localhost'
            || str_ends_with($host, '.localhost')
            || str_starts_with($host, '127.')
            || $host === '::1';
    }

    private function usesReservedTestDomain(string $email): bool
    {
        $separator = strrpos($email, '@');

        if ($separator === false) {
            return true;
        }

        $domain = strtolower(substr($email, $separator + 1));

        foreach (['.test', '.example', '.invalid', '.localhost'] as $suffix) {
            if ($domain === ltrim($suffix, '.') || str_ends_with($domain, $suffix)) {
                return true;
            }
        }

        return false;
    }
}
